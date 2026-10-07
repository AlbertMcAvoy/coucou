//! coucou-hook — the relay Claude Code runs on every hook event.
//!
//! Reads the hook JSON on stdin, adds a little terminal context, and hands it to
//! Coucou over the named pipe `\\.\pipe\coucou-<sid>` (Windows) or the Unix
//! socket `$XDG_RUNTIME_DIR/coucou.sock` (Linux).
//!
//! Hard rule (docs/CLAUDE.md): **never block Claude Code.**
//! * If the pipe does not exist — Coucou is closed — we exit 0 immediately with
//!   nothing on stdout, and the session carries on untouched.
//! * Every step runs under a deadline enforced by the main thread, so a pipe that
//!   accepts the connection and then stops reading cannot wedge the session
//!   either: we abandon the worker and exit.
//! * Only `PermissionRequest` waits for an answer, because approving from the
//!   island is the whole point. No answer means empty stdout, and Claude Code
//!   asks in the terminal exactly as if Coucou were not installed.
//!
//! Usage: `coucou-hook <EventName>` (the name is also read from the JSON).

use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::Duration;

/// Budget for getting a pipe connection. Beyond this Claude Code wins, always.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);
/// Whole-run budget for an event nobody waits on: connect and write, no more.
const FIRE_AND_FORGET_BUDGET: Duration = Duration::from_secs(2);
/// How long a permission prompt may stay on screen before the terminal takes over.
const DECISION_BUDGET: Duration = Duration::from_secs(110);

/// Fields that are pointless to forward and can be enormous (a whole file read,
/// a full command output). The island never shows them.
const DROPPED_FIELDS: &[&str] = &["tool_response", "transcript_path"];
/// Longest string forwarded for any single field; the island truncates to far
/// less than this anyway.
const MAX_FIELD_LEN: usize = 2_000;

/// The live diff needs the whole text of a file edit, once it has happened:
/// PostToolUse of these tools keeps its edit strings far longer than the rest.
const DIFF_TOOLS: &[&str] = &["Edit", "MultiEdit", "Write"];
/// The `tool_input` keys holding the text being replaced or written.
const DIFF_FIELDS: &[&str] = &["old_string", "new_string", "content"];
/// Per edit string. The island stops diffing at 200 KB anyway (DiffEngine).
const MAX_DIFF_FIELD_LEN: usize = 256 * 1024;
/// For all edit strings of one event together, so the line stays well under the
/// 1 MiB the app reads from the pipe even once JSON-escaped.
const MAX_DIFF_TOTAL: usize = 512 * 1024;

#[cfg(windows)]
mod win;
#[cfg(windows)]
use win::connect;

#[cfg(target_os = "linux")]
mod unix;
#[cfg(target_os = "linux")]
use unix::connect;

fn main() {
    let Some((payload, event)) = read_event() else { std::process::exit(0) };

    let waits_for_answer = event == "PermissionRequest";
    let budget = if waits_for_answer { DECISION_BUDGET } else { FIRE_AND_FORGET_BUDGET };

    // The worker owns every blocking call. If it overruns the budget we simply
    // stop listening and exit: the process dying takes the pipe handle with it.
    // (No catch_unwind here — the release profile is panic = "abort", so it would
    // be dead code. `talk` is written to have nothing to panic on instead.)
    let (tx, rx) = mpsc::channel::<Option<String>>();
    std::thread::spawn(move || {
        let _ = tx.send(talk(&payload, waits_for_answer));
    });

    if let Ok(Some(decision)) = rx.recv_timeout(budget) {
        if let Some(json) = decision_json(&decision) {
            let mut out = std::io::stdout();
            let _ = writeln!(out, "{json}");
            let _ = out.flush();
        }
    }
    // Nothing printed: Claude Code asks in the terminal, as if we were not here.
    std::process::exit(0);
}

/// The documented PermissionRequest output. Anything we do not recognise prints
/// nothing at all rather than guessing — silence is the safe answer.
/// See https://code.claude.com/docs/en/hooks
fn decision_json(decision: &str) -> Option<String> {
    let behavior = match decision.trim() {
        // "always" still answers a plain allow; remembering it is the island's
        // business, not Claude Code's.
        "allow" | "always" => r#"{"behavior":"allow"}"#.to_string(),
        "deny" => r#"{"behavior":"deny","message":"Denied from Coucou"}"#.to_string(),
        _ => return None,
    };
    Some(format!(
        r#"{{"hookSpecificOutput":{{"hookEventName":"PermissionRequest","decision":{behavior}}}}}"#
    ))
}

/// Reads stdin and returns the payload to forward plus the event name.
fn read_event() -> Option<(String, String)> {
    let mut raw = Vec::new();
    if std::io::stdin().read_to_end(&mut raw).is_err() || raw.is_empty() {
        return None;
    }
    // Some shells hand us a UTF-8 BOM; serde_json would choke on it.
    if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        raw.drain(..3);
    }

    let mut payload = serde_json::from_slice::<serde_json::Value>(&raw).ok()?;
    let map = payload.as_object_mut()?;

    // Parse argv: "coucou-hook.exe [--agent <name>] [<EventName>]"
    // --agent tags the payload with coucou_agent so the app routes to the right pill.
    // Absent or invalid names are validated and discarded by the app, not here.
    let mut agent = String::new();
    let mut arg_event = String::new();
    {
        let mut it = std::env::args().skip(1);
        while let Some(arg) = it.next() {
            if arg == "--agent" {
                agent = it.next().unwrap_or_default();
            } else if arg_event.is_empty() {
                arg_event = arg;
            }
        }
    }
    // Which agent this hook was installed for. Absent means Claude Code,
    // so existing hook commands keep working unchanged.
    if !agent.is_empty() {
        map.insert("coucou_agent".into(), serde_json::Value::String(agent));
    }
    let event = map
        .get("hook_event_name")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .unwrap_or(arg_event);
    map.insert("hook_event_name".into(), serde_json::Value::String(event.clone()));

    for field in DROPPED_FIELDS {
        map.remove(*field);
    }

    let cwd_missing = map
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(str::is_empty)
        .unwrap_or(true);
    if cwd_missing {
        if let Ok(cwd) = std::env::current_dir() {
            map.insert(
                "cwd".into(),
                serde_json::Value::String(cwd.to_string_lossy().to_string()),
            );
        }
    }

    // Which terminal the session runs in. Unlike macOS, Coucou here accepts
    // events from every terminal, so this is context only — never a filter.
    for (key, var) in [
        ("term_program", "TERM_PROGRAM"),
        ("wt_session", "WT_SESSION"),
        ("term_session_id", "TERM_SESSION_ID"),
        ("vscode_pid", "VSCODE_PID"),
        ("session_pid", "CLAUDE_CODE_SSE_PORT"),
    ] {
        if !map.contains_key(key) {
            let value = std::env::var(var).unwrap_or_default();
            map.insert(key.into(), serde_json::Value::String(value));
        }
    }

    truncate_payload(&mut payload, &event);

    let mut line = payload.to_string();
    line.push('\n');
    Some((line, event))
}

/// Caps the strings of a payload: every field to MAX_FIELD_LEN, except the edit
/// strings of a finished Edit / MultiEdit / Write, which the live diff needs
/// whole. If even those had to be cut, `coucou_diff_truncated` tells the island
/// not to show counts it cannot trust.
fn truncate_payload(payload: &mut serde_json::Value, event: &str) {
    let keeps_diff = event == "PostToolUse"
        && payload
            .get("tool_name")
            .and_then(|v| v.as_str())
            .is_some_and(|tool| DIFF_TOOLS.contains(&tool));
    let input = if keeps_diff {
        payload.as_object_mut().and_then(|map| map.remove("tool_input"))
    } else {
        None
    };

    truncate_strings(payload);

    if let Some(mut input) = input {
        let mut budget = MAX_DIFF_TOTAL;
        let mut cut_any = false;
        cap_diff_strings(&mut input, &mut budget, &mut cut_any);
        if let Some(map) = payload.as_object_mut() {
            map.insert("tool_input".into(), input);
            if cut_any {
                map.insert("coucou_diff_truncated".into(), serde_json::Value::Bool(true));
            }
        }
    }
}

/// `tool_input` of a diff tool: edit strings share MAX_DIFF_TOTAL, each capped at
/// MAX_DIFF_FIELD_LEN; any other string gets the ordinary cap.
fn cap_diff_strings(value: &mut serde_json::Value, budget: &mut usize, cut_any: &mut bool) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, v) in map.iter_mut() {
                match v {
                    serde_json::Value::String(s) if DIFF_FIELDS.contains(&key.as_str()) => {
                        let limit = MAX_DIFF_FIELD_LEN.min(*budget);
                        if cut(s, limit) {
                            *cut_any = true;
                        }
                        *budget = budget.saturating_sub(s.len());
                    }
                    _ => cap_diff_strings(v, budget, cut_any),
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                cap_diff_strings(item, budget, cut_any);
            }
        }
        serde_json::Value::String(s) => {
            cut(s, MAX_FIELD_LEN);
        }
        _ => {}
    }
}

/// Caps every string in the payload. A single Write can carry a whole file.
fn truncate_strings(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(s) => {
            cut(s, MAX_FIELD_LEN);
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(truncate_strings),
        serde_json::Value::Object(map) => map.values_mut().for_each(truncate_strings),
        _ => {}
    }
}

/// Shortens `s` to at most `max` bytes plus an ellipsis; true if it was cut.
fn cut(s: &mut String, max: usize) -> bool {
    if s.len() <= max {
        return false;
    }
    // Cut on a char boundary; a lone byte index can split UTF-8.
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s.truncate(end);
    s.push('…');
    true
}

/// Connect, send, and — for a permission request — wait for the island's word.
fn talk(payload: &str, waits_for_answer: bool) -> Option<String> {
    let mut pipe = connect()?;

    if pipe.write_all(payload.as_bytes()).is_err() {
        return None;
    }
    let _ = pipe.flush();

    if !waits_for_answer {
        return None;
    }

    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        match pipe.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.contains(&b'\n') {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let answer = String::from_utf8_lossy(&buf).trim().to_string();
    (!answer.is_empty()).then_some(answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_json_matches_the_documented_shape() {
        assert_eq!(
            decision_json("allow").unwrap(),
            r#"{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}"#
        );
        assert_eq!(
            decision_json("deny").unwrap(),
            r#"{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"Denied from Coucou"}}}"#
        );
        // "always" is an island concept; Claude Code just gets an allow.
        assert!(decision_json("always").unwrap().contains(r#""behavior":"allow""#));
    }

    #[test]
    fn anything_unrecognised_prints_nothing() {
        assert!(decision_json("").is_none());
        assert!(decision_json("maybe").is_none());
        // The shape the app used to send must not be mistaken for a decision.
        assert!(decision_json(r#"{"permissionDecision":"allow"}"#).is_none());
    }

    #[test]
    fn long_strings_are_cut_on_a_char_boundary() {
        let mut v = serde_json::json!({ "tool_input": { "content": "é".repeat(4000) } });
        truncate_strings(&mut v);
        let s = v["tool_input"]["content"].as_str().unwrap();
        assert!(s.len() <= MAX_FIELD_LEN + 4);
        assert!(s.ends_with('…'));
    }

    #[test]
    fn a_finished_edit_keeps_its_text_whole_for_the_live_diff() {
        let big = "line\n".repeat(4_000); // 20 KB, well past MAX_FIELD_LEN
        let mut v = serde_json::json!({
            "tool_name": "Edit",
            "tool_input": { "file_path": "/p/a.ts", "old_string": big, "new_string": big },
            "cwd": "x".repeat(4_000),
        });
        truncate_payload(&mut v, "PostToolUse");
        assert_eq!(v["tool_input"]["old_string"].as_str().unwrap().len(), big.len());
        assert_eq!(v["tool_input"]["new_string"].as_str().unwrap().len(), big.len());
        // Everything else keeps the ordinary cap, and nothing says "cut".
        assert!(v["cwd"].as_str().unwrap().len() <= MAX_FIELD_LEN + 4);
        assert!(v.get("coucou_diff_truncated").is_none());

        let mut multi = serde_json::json!({
            "tool_name": "MultiEdit",
            "tool_input": { "edits": [{ "old_string": big, "new_string": "x" }] },
        });
        truncate_payload(&mut multi, "PostToolUse");
        assert_eq!(multi["tool_input"]["edits"][0]["old_string"].as_str().unwrap().len(), big.len());
    }

    #[test]
    fn edits_are_still_capped_before_they_happen_and_for_other_tools() {
        let big = "é".repeat(4_000);
        for (event, tool) in [("PreToolUse", "Edit"), ("PermissionRequest", "Write"), ("PostToolUse", "Bash")] {
            let mut v = serde_json::json!({ "tool_name": tool, "tool_input": { "content": big } });
            truncate_payload(&mut v, event);
            assert!(v["tool_input"]["content"].as_str().unwrap().len() <= MAX_FIELD_LEN + 4, "{event} {tool}");
        }
    }

    #[test]
    fn an_edit_beyond_the_budget_is_cut_and_flagged() {
        let huge = "x".repeat(MAX_DIFF_FIELD_LEN + 10);
        let mut v = serde_json::json!({
            "tool_name": "Write",
            "tool_input": { "file_path": "/p/big.txt", "content": huge },
        });
        truncate_payload(&mut v, "PostToolUse");
        assert!(v["tool_input"]["content"].as_str().unwrap().len() <= MAX_DIFF_FIELD_LEN + 4);
        assert_eq!(v["coucou_diff_truncated"], serde_json::Value::Bool(true));

        // Together, the edit strings never pass the shared budget.
        let half = "y".repeat(MAX_DIFF_FIELD_LEN - 1);
        let edits: Vec<_> = (0..4)
            .map(|_| serde_json::json!({ "old_string": half, "new_string": half }))
            .collect();
        let mut multi = serde_json::json!({ "tool_name": "MultiEdit", "tool_input": { "edits": edits } });
        truncate_payload(&mut multi, "PostToolUse");
        assert!(multi.to_string().len() < MAX_DIFF_TOTAL + 64 * 1024);
        assert_eq!(multi["coucou_diff_truncated"], serde_json::Value::Bool(true));
    }
}
