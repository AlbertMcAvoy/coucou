// Whether each hook-driven pill is connected.
//
// A pill fed by hook events has no key: it is connected once the agent's config
// routes its events to coucou-hook (Mac #183 — the idle card used to say "Key
// not configured" for these). Only reads, and only the agents' own config files;
// nothing here ever writes.
//
// The checks themselves are pure functions over parsed JSON, so they are tested
// on fixtures rather than on the real files.

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::Value;

use crate::platform::home_dir;

/// Port of `coucouHooksPresent(inSettings:)` (ClaudeHookDetection.swift): true
/// when a parsed `~/.claude/settings.json` routes Claude Code's SessionStart
/// events to Coucou. The command text is what tells: Coucou's relay here is
/// `coucou-hook`, the Mac's is `~/.claude/coucou/nb-hook` (or NotchBuddy in the
/// App Store build), so a settings file shared between machines reads the same.
pub fn claude_hooks_present(settings: &Value) -> bool {
    let Some(groups) = settings
        .get("hooks")
        .and_then(|h| h.get("SessionStart"))
        .and_then(Value::as_array)
    else {
        return false;
    };
    groups.iter().any(|group| {
        group.get("hooks").and_then(Value::as_array).is_some_and(|hooks| {
            hooks.iter().any(|hook| {
                hook.get("command")
                    .and_then(Value::as_str)
                    .is_some_and(|c| c.contains("NotchBuddy") || c.contains("coucou"))
            })
        })
    })
}

/// True when some string in `config` runs the relay for `agent`: it names
/// `coucou-hook` (or the Mac's `nb-hook`) and passes `--agent <agent>`.
///
/// Every agent nests its hooks differently (Gemini and Codex under
/// `hooks.<event>[].hooks[].command`, Copilot under `hooks.<event>[].bash` or
/// `.powershell`, Antigravity under its own key), and the README tells people to
/// write these files by hand, so the whole document is searched rather than one
/// layout trusted.
pub fn agent_hooks_present(config: &Value, agent: &str) -> bool {
    match config {
        Value::String(s) => runs_relay_for(s, agent),
        Value::Array(items) => items.iter().any(|v| agent_hooks_present(v, agent)),
        Value::Object(map) => map.values().any(|v| agent_hooks_present(v, agent)),
        _ => false,
    }
}

fn runs_relay_for(command: &str, agent: &str) -> bool {
    if !(command.contains("coucou-hook") || command.contains("nb-hook")) {
        return false;
    }
    // `--agent gemini` must not also match `--agent gemini-beta`.
    let flag = format!("--agent {agent}");
    command.match_indices(&flag).any(|(at, _)| {
        command[at + flag.len()..]
            .chars()
            .next()
            .is_none_or(|c| c.is_whitespace() || c == '"' || c == '\'')
    })
}

/// Agents configured through their own hook file, with the `--agent` tag the
/// relay is given and where that file lives. Antigravity has two places: the
/// one this README names, and the Mac's.
fn agent_configs() -> Vec<(&'static str, &'static str, Vec<PathBuf>)> {
    let home = home_dir();
    vec![
        ("agent_gemini", "gemini", vec![home.join(".gemini").join("settings.json")]),
        (
            "agent_antigravity",
            "antigravity",
            vec![
                home.join(".config").join("antigravity").join("hooks.json"),
                home.join(".gemini").join("config").join("hooks.json"),
            ],
        ),
        ("agent_codex", "codex", vec![home.join(".codex").join("hooks.json")]),
        ("agent_copilot", "copilot", vec![home.join(".copilot").join("hooks").join("coucou.json")]),
        ("agent_muse", "muse", vec![home.join(".config").join("muse").join("settings.json")]),
    ]
}

/// A JSON file as a value; anything missing or unreadable is "nothing there".
fn read_json(path: &PathBuf) -> Value {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| {
            let text = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes).to_vec();
            serde_json::from_slice(&text).ok()
        })
        .unwrap_or(Value::Null)
}

/// Pill ID → connected, for every hook-driven pill.
pub fn status() -> HashMap<String, bool> {
    let claude = claude_hooks_present(&read_json(&crate::hooks::settings_path()));
    let mut out = HashMap::new();
    // Cursor sessions are Claude Code in Cursor's terminal: the same hooks.
    out.insert("integration_claude".to_string(), claude);
    out.insert("agent_cursor".to_string(), claude);
    for (pill, agent, paths) in agent_configs() {
        let present = paths.iter().any(|p| agent_hooks_present(&read_json(p), agent));
        out.insert(pill.to_string(), present);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(json: &str) -> Value {
        serde_json::from_str(json).unwrap_or(Value::Null)
    }

    // The ten cases of tests/ClaudeHookDetectionTests.swift, plus this build's relay.

    #[test]
    fn the_hook_coucou_writes_is_installed() {
        assert!(claude_hooks_present(&settings(
            r#"{"hooks":{"SessionStart":[{"hooks":[
              {"type":"command","command":"\"C:/Users/me/AppData/Local/Coucou/bin/coucou-hook.exe\" SessionStart"}]}]}}"#
        )));
        assert!(claude_hooks_present(&settings(
            r#"{"hooks":{"SessionStart":[{"hooks":[
              {"type":"command","command":"'/home/me/.local/share/coucou/bin/coucou-hook' SessionStart"}]}]}}"#
        )));
    }

    #[test]
    fn the_mac_builds_hooks_count_too() {
        assert!(claude_hooks_present(&settings(
            r#"{"hooks":{"SessionStart":[{"hooks":[
              {"type":"command","command":"$HOME/.claude/coucou/nb-hook"}]}]}}"#
        )));
        assert!(claude_hooks_present(&settings(
            r#"{"hooks":{"SessionStart":[{"hooks":[
              {"type":"command","command":"/Applications/NotchBuddy.app/.../nb-hook"}]}]}}"#
        )));
    }

    #[test]
    fn our_hook_next_to_somebody_elses_is_installed() {
        assert!(claude_hooks_present(&settings(
            r#"{"hooks":{"SessionStart":[
              {"hooks":[{"type":"command","command":"/usr/local/bin/other-tool"}]},
              {"hooks":[{"type":"command","command":"$HOME/.claude/coucou/nb-hook"}]}]}}"#
        )));
    }

    #[test]
    fn other_tools_only_is_not_installed() {
        // The case that showed "Key not configured" on macOS.
        assert!(!claude_hooks_present(&settings(
            r#"{"hooks":{"SessionStart":[{"hooks":[
              {"type":"command","command":"$HOME/.vibe-island/bin/vibe-island-bridge"},
              {"type":"command","command":"python3 /Users/me/.claude/skills/harness/hook.py"}]}]}}"#
        )));
    }

    #[test]
    fn hooks_for_other_events_do_not_count() {
        assert!(!claude_hooks_present(&settings(
            r#"{"hooks":{"PreToolUse":[{"hooks":[
              {"type":"command","command":"$HOME/.claude/coucou/nb-hook"}]}]}}"#
        )));
    }

    #[test]
    fn malformed_or_empty_settings_never_read_as_installed() {
        for json in [
            "{}",
            r#"{"hooks":{"SessionStart":[]}}"#,
            r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command"}]}]}}"#,
            r#"{"hooks":{"SessionStart":"not-an-array"}}"#,
            r#"{"hooks":"not-an-object"}"#,
            "not json",
        ] {
            assert!(!claude_hooks_present(&settings(json)), "{json}");
        }
    }

    // Other agents: any place in their config that runs the relay with their tag.

    #[test]
    fn an_agent_is_connected_when_its_config_runs_the_relay_with_its_tag() {
        let gemini = settings(
            r#"{"hooks":{"BeforeTool":[{"hooks":[
              {"type":"command","command":"\"C:/Users/me/AppData/Local/Coucou/bin/coucou-hook.exe\" --agent gemini BeforeTool"}]}]}}"#,
        );
        assert!(agent_hooks_present(&gemini, "gemini"));
        assert!(!agent_hooks_present(&gemini, "codex"));

        // Copilot keeps the command under `bash` / `powershell`.
        let copilot = settings(
            r#"{"version":1,"hooks":{"preToolUse":[{"type":"command",
              "powershell":"& 'C:\\Users\\me\\AppData\\Local\\Coucou\\bin\\coucou-hook.exe' --agent copilot preToolUse"}]}}"#,
        );
        assert!(agent_hooks_present(&copilot, "copilot"));

        // The Mac's relay, in a config synced from a Mac.
        let codex = settings(r#"{"hooks":{"Stop":[{"hooks":[{"command":"~/.claude/coucou/nb-hook --agent codex"}]}]}}"#);
        assert!(agent_hooks_present(&codex, "codex"));
    }

    #[test]
    fn a_longer_tag_or_another_program_is_not_the_agent() {
        let beta = settings(r#"{"hooks":{"x":[{"command":"coucou-hook --agent gemini-beta x"}]}}"#);
        assert!(!agent_hooks_present(&beta, "gemini"));
        let other = settings(r#"{"hooks":{"x":[{"command":"other-relay --agent gemini x"}]}}"#);
        assert!(!agent_hooks_present(&other, "gemini"));
        let quoted = settings(r#"{"cmd":["coucou-hook", "--agent gemini"]}"#);
        // Split over two strings: no single command runs the relay with the tag.
        assert!(!agent_hooks_present(&quoted, "gemini"));
        assert!(!agent_hooks_present(&Value::Null, "gemini"));
        let at_end = settings(r#"{"c":"coucou-hook --agent muse"}"#);
        assert!(agent_hooks_present(&at_end, "muse"));
    }
}
