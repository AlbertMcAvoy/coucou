// Hook and plugin installers for every agent other than Claude Code (hooks.rs).
//
// Each agent is a list of file edits handed to config_file.rs, which applies the
// CLAUDE.md rules once for all of them: strict read, refuse unexpected types,
// diff, dated backup that must succeed, fingerprint check, atomic write. What
// lives here is only *what* changes in each agent's config, and the command
// line that starts the relay in the shell that agent uses.
//
// Formats and paths follow the Mac app (NotchBuddy/Sources/App/HookServer.swift)
// so a config shared between machines (dotfiles) looks the same everywhere.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::config_file::{self, FileEdit, Plan};
use crate::{platform, settings};

/// Marker that identifies a Coucou entry: the relay's file name.
const MARKER: &str = "coucou-hook";

// ── The relay command line ────────────────────────────────────────────────────

/// The shell an agent runs its hook commands through on Windows. On Linux every
/// agent uses `sh` (or bash), so there is only one way to quote.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shell {
    /// Git Bash on Windows (Claude Code), `sh` on Linux.
    Sh,
    /// `powershell -Command` (Gemini CLI, Copilot CLI's `powershell` field).
    PowerShell,
    /// `cmd /C`, or a process started directly (Codex, Cursor, Antigravity, Muse).
    Cmd,
}

/// Where the relay is and which OS the command is for. Kept apart from the
/// real install so tests can build both the Windows and the Linux command.
pub struct Relay {
    pub exe: String,
    pub windows: bool,
}

impl Relay {
    pub fn current() -> Self {
        Self {
            exe: settings::hook_exe_path().to_string_lossy().to_string(),
            windows: cfg!(windows),
        }
    }

    /// The relay followed by `args`, as one command line for `shell`.
    pub fn command(&self, shell: Shell, args: &str) -> String {
        if !self.windows {
            return format!("{} {args}", sh_quote(&self.exe));
        }
        let exe = match shell {
            // Git Bash reads backslashes as escapes: forward slashes, quoted.
            Shell::Sh => format!("\"{}\"", self.exe.replace('\\', "/")),
            // A quoted string alone is an expression in PowerShell: `&` runs it.
            // Inside single quotes only `'` is special, and it doubles.
            Shell::PowerShell => format!("& '{}'", self.exe.replace('\'', "''")),
            // cmd /C keeps the quotes of a lone quoted program name; a plain path
            // goes unquoted so it also works if the agent turns out to use
            // PowerShell. A Windows path cannot contain `"`.
            Shell::Cmd if is_plain(&self.exe) => self.exe.clone(),
            Shell::Cmd => format!("\"{}\"", self.exe),
        };
        format!("{exe} {args}")
    }
}

/// The relay command for the running OS, for `args`.
pub fn relay_command(shell: Shell, args: &str) -> String {
    Relay::current().command(shell, args)
}

/// `s` as one single-quoted POSIX shell word: `'` becomes `'\''`, nothing else
/// is special inside single quotes.
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// A path no Windows shell would split or interpret.
fn is_plain(path: &str) -> bool {
    path.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '\\' | ':' | '.' | '_' | '-'))
}

// ── Agents ────────────────────────────────────────────────────────────────────

type JsonChange = Box<dyn Fn(&Value) -> Result<Option<Value>, String>>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Agent {
    Gemini,
    Antigravity,
    Cursor,
    Codex,
}

impl Agent {
    pub const ALL: &'static [Agent] = &[Agent::Codex, Agent::Gemini, Agent::Antigravity, Agent::Cursor];

    /// The `--agent` name; the pill is `agent_<id>` (PillCatalog.swift).
    pub fn id(self) -> &'static str {
        match self {
            Agent::Gemini => "gemini",
            Agent::Antigravity => "antigravity",
            Agent::Cursor => "cursor",
            Agent::Codex => "codex",
        }
    }

    pub fn from_id(id: &str) -> Option<Agent> {
        Self::ALL.iter().copied().find(|a| a.id() == id)
    }

    /// The files this agent's install touches, under `home`.
    fn files(self, home: &Path) -> Vec<PathBuf> {
        match self {
            Agent::Gemini => vec![home.join(".gemini").join("settings.json")],
            Agent::Antigravity => vec![home.join(".gemini").join("config").join("hooks.json")],
            Agent::Cursor => vec![home.join(".cursor").join("hooks.json")],
            Agent::Codex => vec![home.join(".codex").join("hooks.json")],
        }
    }

    /// The edits that install (or remove) Coucou for this agent.
    fn edits(self, home: &Path, relay: &Relay, install: bool) -> Vec<FileEdit<'static>> {
        let files = self.files(home);
        let path = files[0].clone();
        let label = path.display().to_string();
        let change = self.json_change(relay, install);
        vec![FileEdit { path, edit: config_file::json_edit(label, change) }]
    }

    /// The change to this agent's JSON config: the new object, or `None` to
    /// remove a file that is Coucou's alone. Command lines are built here, up
    /// front, so the change itself is pure.
    fn json_change(self, relay: &Relay, install: bool) -> JsonChange {
        match (self, install) {
            (Agent::Gemini, false) => Box::new(|v| groups_uninstall(v, "gemini").map(Some)),
            (Agent::Gemini, true) => {
                let commands: Vec<(String, String, u64)> = GEMINI_EVENTS
                    .iter()
                    .map(|(event, said, timeout)| {
                        let command = relay.command(Shell::PowerShell, &format!("--agent gemini {said}"));
                        (event.to_string(), command, *timeout)
                    })
                    .collect();
                Box::new(move |v| gemini_install(v, &commands).map(Some))
            }
            (Agent::Antigravity, false) => Box::new(|v| antigravity_uninstall(v).map(Some)),
            (Agent::Antigravity, true) => {
                let block = antigravity_block(relay);
                Box::new(move |v| antigravity_install(v, &block).map(Some))
            }
            (Agent::Cursor, false) => Box::new(|v| groups_uninstall(v, "cursor").map(Some)),
            (Agent::Cursor, true) => {
                let command = relay.command(Shell::Cmd, "--agent cursor");
                Box::new(move |v| cursor_install(v, &command).map(Some))
            }
            (Agent::Codex, false) => Box::new(|v| groups_uninstall(v, "codex").map(Some)),
            (Agent::Codex, true) => {
                let command = relay.command(Shell::Cmd, "--agent codex");
                Box::new(move |v| codex_install(v, &command).map(Some))
            }
        }
    }

    /// True when the agent's config already routes to Coucou. Never fails: a
    /// file we cannot read just reads as "not installed".
    fn installed(self, home: &Path) -> bool {
        let files = self.files(home);
        let json = || {
            config_file::read(&files[0])
                .ok()
                .flatten()
                .and_then(|b| config_file::parse_json(Some(&b), "").ok())
                .unwrap_or_else(|| json!({}))
        };
        match self {
            Agent::Gemini => groups_have_ours(&json(), "gemini"),
            Agent::Antigravity => json().get("coucou").is_some_and(antigravity_is_ours),
            Agent::Cursor => groups_have_ours(&json(), "cursor"),
            Agent::Codex => groups_have_ours(&json(), "codex"),
        }
    }

    /// Whether the island can answer this agent's permission requests. Must
    /// match `takes_decisions` in the relay (hook/src/reply.rs).
    fn approvals(self) -> bool {
        matches!(self, Agent::Codex)
    }
}

// ── Strings shown in Settings ─────────────────────────────────────────────────

const TEXT_UNTOUCHED: &str = "Coucou has not touched it.";
const TEXT_UNEXPECTED: &str = "has an unexpected type — Coucou has not touched it.";

impl Agent {
    pub fn name(self) -> &'static str {
        match self {
            Agent::Gemini => "Gemini CLI",
            Agent::Antigravity => "Antigravity",
            Agent::Cursor => "Cursor Agent",
            Agent::Codex => "Codex",
        }
    }

    /// What to do once the file is written, shown with the result.
    fn note(self) -> &'static str {
        match self {
            Agent::Gemini => "Start a new Gemini CLI session to pick the hooks up.",
            Agent::Antigravity => "Start a new Antigravity conversation to pick the hooks up.",
            Agent::Cursor => "Restart Cursor to pick the hooks up.",
            Agent::Codex => "Codex runs new hooks only once you trust them: start Codex and review them once with /hooks.",
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub id: &'static str,
    pub name: &'static str,
    pub installed: bool,
    /// The file (or files, one per line) Coucou writes.
    pub path: String,
    pub hook_ready: bool,
    /// The island can allow or deny this agent's permission requests.
    pub approvals: bool,
    pub note: &'static str,
}

fn find(id: &str) -> Result<Agent, String> {
    Agent::from_id(id).ok_or_else(|| format!("Unknown agent \"{id}\"."))
}

pub fn list() -> Vec<AgentStatus> {
    let home = platform::home_dir();
    let hook_ready = settings::hook_exe_path().exists();
    Agent::ALL
        .iter()
        .map(|&a| AgentStatus {
            id: a.id(),
            name: a.name(),
            installed: a.installed(&home),
            path: a.files(&home).iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("\n"),
            hook_ready,
            approvals: a.approvals(),
            note: a.note(),
        })
        .collect()
}

pub fn preview(id: &str, install: bool) -> Result<Plan, String> {
    let agent = find(id)?;
    config_file::preview(&agent.edits(&platform::home_dir(), &Relay::current(), install))
}

/// Only ever called from an explicit click. Returns the backups taken, one per line.
pub fn apply(id: &str, install: bool, fingerprint: &str) -> Result<String, String> {
    let agent = find(id)?;
    apply_in(agent, &platform::home_dir(), &Relay::current(), install, fingerprint)
}

fn apply_in(agent: Agent, home: &Path, relay: &Relay, install: bool, fingerprint: &str) -> Result<String, String> {
    let backups = config_file::apply(&agent.edits(home, relay, install), fingerprint)?;
    Ok(backups.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("\n"))
}

// ── Shared JSON helpers ───────────────────────────────────────────────────────

fn unexpected(what: &str) -> String {
    format!("{what} {TEXT_UNEXPECTED}")
}

/// `root[key]` as an object to edit: absent is empty, anything else is refused.
fn object_at(root: &Map<String, Value>, key: &str) -> Result<Map<String, Value>, String> {
    match root.get(key) {
        None => Ok(Map::new()),
        Some(Value::Object(m)) => Ok(m.clone()),
        Some(_) => Err(unexpected(&format!("\"{key}\""))),
    }
}

/// `hooks[event]` as a list to edit: absent is empty, anything else is refused.
fn list_at(hooks: &Map<String, Value>, event: &str) -> Result<Vec<Value>, String> {
    match hooks.get(event) {
        None => Ok(Vec::new()),
        Some(Value::Array(list)) => Ok(list.clone()),
        Some(_) => Err(unexpected(&format!("\"hooks\".\"{event}\""))),
    }
}

/// A command line written by Coucou for `agent`.
fn is_our_command(command: Option<&Value>, agent: &str) -> bool {
    command
        .and_then(Value::as_str)
        .is_some_and(|c| c.contains(MARKER) && c.contains(&format!("--agent {agent}")))
}

/// Claude-style groups (`{"matcher"?, "hooks": [{"command"}]}`, or a legacy
/// flat `{"command"}`) without Coucou's entries for `agent`. A group left
/// empty goes; everything else stays exactly as it was.
fn without_ours_in_groups(groups: &[Value], agent: &str) -> Vec<Value> {
    groups
        .iter()
        .filter_map(|group| {
            if is_our_command(group.get("command"), agent) {
                return None;
            }
            let Some(inner) = group.get("hooks").and_then(Value::as_array) else {
                return Some(group.clone());
            };
            let kept: Vec<Value> =
                inner.iter().filter(|h| !is_our_command(h.get("command"), agent)).cloned().collect();
            if kept.len() == inner.len() {
                return Some(group.clone());
            }
            if kept.is_empty() {
                return None;
            }
            let mut group = group.clone();
            group["hooks"] = Value::Array(kept);
            Some(group)
        })
        .collect()
}

/// `root` with one Coucou group per event, built by `group(event)`. Earlier
/// Coucou entries for `agent` are replaced; nobody else's are touched.
fn groups_install(
    root: &Value,
    agent: &str,
    events: impl IntoIterator<Item = (String, Value)>,
) -> Result<Map<String, Value>, String> {
    let mut root = root.as_object().cloned().unwrap_or_default();
    let mut hooks = object_at(&root, "hooks")?;
    for (event, group) in events {
        let mut list = without_ours_in_groups(&list_at(&hooks, &event)?, agent);
        list.push(group);
        hooks.insert(event, Value::Array(list));
    }
    root.insert("hooks".into(), Value::Object(hooks));
    Ok(root)
}

/// `root` without any Coucou group for `agent`; an event left empty goes, and
/// so does `hooks` when nothing is left in it.
fn groups_uninstall(root: &Value, agent: &str) -> Result<Value, String> {
    let mut root = root.as_object().cloned().unwrap_or_default();
    if !root.contains_key("hooks") {
        return Ok(Value::Object(root));
    }
    let hooks = object_at(&root, "hooks")?;
    let mut out = Map::new();
    for (event, value) in hooks {
        match value.as_array() {
            Some(list) => {
                let kept = without_ours_in_groups(list, agent);
                if !kept.is_empty() {
                    out.insert(event, Value::Array(kept));
                }
            }
            None => {
                out.insert(event, value);
            }
        }
    }
    if out.is_empty() {
        root.remove("hooks");
    } else {
        root.insert("hooks".into(), Value::Object(out));
    }
    Ok(Value::Object(root))
}

fn groups_have_ours(root: &Value, agent: &str) -> bool {
    let ours = |group: &Value| {
        is_our_command(group.get("command"), agent)
            || group
                .get("hooks")
                .and_then(Value::as_array)
                .is_some_and(|inner| inner.iter().any(|h| is_our_command(h.get("command"), agent)))
    };
    root.get("hooks")
        .and_then(Value::as_object)
        .is_some_and(|hooks| hooks.values().filter_map(Value::as_array).flatten().any(ours))
}

// ── Gemini CLI — ~/.gemini/settings.json ──────────────────────────────────────
//
// Claude-style groups with a `*` matcher and timeouts in milliseconds. The
// event name the relay should report goes on the command line. On Windows,
// Gemini CLI runs hooks with `powershell -Command`. AfterModel is left out: it
// fires on every response chunk and would flood the island.

const GEMINI_EVENTS: &[(&str, &str, u64)] = &[
    ("SessionStart", "SessionStart", 10000),
    ("SessionEnd", "SessionEnd", 10000),
    ("BeforeTool", "PreToolUse", 5000),
    ("AfterTool", "PostToolUse", 5000),
    ("BeforeAgent", "UserPromptSubmit", 5000),
    ("AfterAgent", "Stop", 5000),
];

fn gemini_install(root: &Value, commands: &[(String, String, u64)]) -> Result<Value, String> {
    let events = commands.iter().map(|(event, command, timeout)| {
        (
            event.clone(),
            json!({ "matcher": "*", "hooks": [{ "type": "command", "command": command, "timeout": timeout }] }),
        )
    });
    groups_install(root, "gemini", events).map(Value::Object)
}

// ── Antigravity — ~/.gemini/config/hooks.json ─────────────────────────────────
//
// Hooks are named groups at the top level; Coucou's is `coucou`. Tool events
// take matcher groups, lifecycle events take handlers directly; timeouts are
// in seconds. Merge and removal come from #298 (kobaltgit). The relay answers
// PreToolUse with "{}" — no decision — never with an allow: Antigravity's own
// permission rules stay in charge.

const ANTIGRAVITY_TOOL_EVENTS: &[&str] = &["PreToolUse", "PostToolUse"];
const ANTIGRAVITY_LIFECYCLE_EVENTS: &[&str] = &["PreInvocation", "PostInvocation", "Stop"];

fn antigravity_block(relay: &Relay) -> Value {
    let handler = |event: &str| {
        json!({
            "type": "command",
            "command": relay.command(Shell::Cmd, &format!("--agent antigravity {event}")),
            "timeout": 10,
        })
    };
    let mut block = Map::new();
    for event in ANTIGRAVITY_TOOL_EVENTS {
        block.insert((*event).into(), json!([{ "matcher": "*", "hooks": [handler(event)] }]));
    }
    for event in ANTIGRAVITY_LIFECYCLE_EVENTS {
        block.insert((*event).into(), json!([handler(event)]));
    }
    Value::Object(block)
}

/// A `coucou` group Coucou wrote (it runs the relay as `--agent antigravity`).
fn antigravity_is_ours(group: &Value) -> bool {
    let text = group.to_string();
    text.contains(MARKER) && text.contains("--agent antigravity")
}

fn antigravity_install(root: &Value, block: &Value) -> Result<Value, String> {
    let mut root = root.as_object().cloned().unwrap_or_default();
    if root.get("coucou").is_some_and(|g| !antigravity_is_ours(g)) {
        return Err(format!("A hook group named \"coucou\" that Coucou did not write is already there — {TEXT_UNTOUCHED}"));
    }
    root.insert("coucou".into(), block.clone());
    Ok(Value::Object(root))
}

/// Removes Coucou's group, and only if it is Coucou's.
fn antigravity_uninstall(root: &Value) -> Result<Value, String> {
    let mut root = root.as_object().cloned().unwrap_or_default();
    if root.get("coucou").is_some_and(antigravity_is_ours) {
        root.remove("coucou");
    }
    Ok(Value::Object(root))
}

// ── Cursor Agent — ~/.cursor/hooks.json ───────────────────────────────────────
//
// `{"version": 1, "hooks": {"<event>": [{"command": …}]}}`, camelCase events
// (from #231, BeyondBirthday07). The relay maps them, turns a stop with
// `status: "error"` into StopFailure, and prints nothing back: no permission is
// ever given on Cursor's behalf. Every session lands on the one `agent_cursor`
// pill.

const CURSOR_EVENTS: &[&str] = &[
    "sessionStart",
    "sessionEnd",
    "beforeSubmitPrompt",
    "preToolUse",
    "postToolUse",
    "postToolUseFailure",
    "stop",
];

fn cursor_install(root: &Value, command: &str) -> Result<Value, String> {
    let events = CURSOR_EVENTS.iter().map(|e| (e.to_string(), json!({ "command": command })));
    let mut root = groups_install(root, "cursor", events)?;
    root.entry("version").or_insert(json!(1));
    Ok(Value::Object(root))
}

// ── Codex — ~/.codex/hooks.json ───────────────────────────────────────────────
//
// Claude-style groups without a matcher, timeouts in seconds; Codex sends the
// event name in the payload. PermissionRequest waits for the island (120 s,
// with a status line shown in Codex meanwhile) and is answered with Claude
// Code's hookSpecificOutput — only after a click. Codex runs hook commands with
// `cmd /C` on Windows and `$SHELL -lc` elsewhere, and only once the user has
// trusted them with /hooks.

const CODEX_EVENTS: &[(&str, u64)] = &[
    ("SessionStart", 10),
    ("UserPromptSubmit", 10),
    ("PreToolUse", 10),
    ("PermissionRequest", 120),
    ("PostToolUse", 10),
    ("Stop", 10),
    ("SubagentStart", 10),
    ("SubagentStop", 10),
    ("Interrupt", 3),
    ("SessionEnd", 3),
];

const CODEX_WAITING: &str = "Waiting for your answer in the island (Coucou)";

fn codex_install(root: &Value, command: &str) -> Result<Value, String> {
    let events = CODEX_EVENTS.iter().map(|(event, timeout)| {
        let mut hook = json!({ "type": "command", "command": command, "timeout": timeout });
        if *event == "PermissionRequest" {
            hook["statusMessage"] = json!(CODEX_WAITING);
        }
        (event.to_string(), json!({ "hooks": [hook] }))
    });
    groups_install(root, "codex", events).map(Value::Object)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_file::tests::scratch;

    fn linux() -> Relay {
        Relay { exe: "/home/me/.local/share/coucou/bin/coucou-hook".into(), windows: false }
    }

    fn windows(exe: &str) -> Relay {
        Relay { exe: exe.into(), windows: true }
    }

    const WIN: &str = r"C:\Users\me\AppData\Local\Coucou\bin\coucou-hook.exe";
    const WIN_SPACE: &str = r"C:\Users\Jane O'Neil\AppData\Local\Coucou\bin\coucou-hook.exe";

    /// Installs then uninstalls `agent` in a fresh home holding `existing` in
    /// its first file, and returns (installed file, uninstalled file).
    fn round_trip(agent: Agent, existing: Option<&str>) -> (PathBuf, Value, Option<Value>) {
        let home = scratch(&format!("agent-{}", agent.id()));
        let file = agent.files(&home)[0].clone();
        if let Some(text) = existing {
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, text).unwrap();
        }
        let relay = linux();
        let plan = config_file::preview(&agent.edits(&home, &relay, true)).unwrap();
        assert!(plan.diff.contains(MARKER), "{}", plan.diff);
        apply_in(agent, &home, &relay, true, &plan.fingerprint).unwrap();
        assert!(agent.installed(&home), "{agent:?} should read as installed");
        let installed = read_json(&file);

        let plan = config_file::preview(&agent.edits(&home, &relay, false)).unwrap();
        apply_in(agent, &home, &relay, false, &plan.fingerprint).unwrap();
        assert!(!agent.installed(&home), "{agent:?} should read as removed");
        let removed = file.exists().then(|| read_json(&file));
        (home, installed, removed)
    }

    fn read_json(path: &Path) -> Value {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn the_command_is_quoted_for_the_shell_that_runs_it() {
        assert_eq!(
            linux().command(Shell::Cmd, "--agent codex"),
            "'/home/me/.local/share/coucou/bin/coucou-hook' --agent codex"
        );
        let w = windows(WIN);
        assert_eq!(
            w.command(Shell::Sh, "Stop"),
            "\"C:/Users/me/AppData/Local/Coucou/bin/coucou-hook.exe\" Stop"
        );
        assert_eq!(
            w.command(Shell::PowerShell, "--agent gemini Stop"),
            format!("& '{WIN}' --agent gemini Stop")
        );
        // A plain path is left bare, so cmd and PowerShell both run it.
        assert_eq!(w.command(Shell::Cmd, "--agent cursor"), format!("{WIN} --agent cursor"));

        let spaced = windows(WIN_SPACE);
        assert_eq!(spaced.command(Shell::Cmd, "x"), format!("\"{WIN_SPACE}\" x"));
        assert_eq!(
            spaced.command(Shell::PowerShell, "x"),
            r"& 'C:\Users\Jane O''Neil\AppData\Local\Coucou\bin\coucou-hook.exe' x"
        );
    }

    #[test]
    fn the_hook_path_is_one_shell_word_whatever_it_contains() {
        assert_eq!(sh_quote("/home/a b/x"), "'/home/a b/x'");
        assert_eq!(sh_quote(r#"/h/$(id)`x`\"y"#), r#"'/h/$(id)`x`\"y'"#);
        assert_eq!(sh_quote("/h/it's"), r"'/h/it'\''s'");
    }

    #[test]
    fn every_agent_id_is_a_valid_pill_name() {
        for agent in Agent::ALL {
            let id = agent.id();
            assert!(id.len() <= 24 && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
            assert_eq!(Agent::from_id(id), Some(*agent));
        }
        assert!(find("claude").is_err());
    }

    #[test]
    fn gemini_keeps_foreign_hooks_and_removes_only_its_own() {
        let existing = r#"{"theme":"dark","hooks":{"BeforeTool":[{"matcher":"other","hooks":[{"type":"command","command":"custom.exe"}]}]}}"#;
        let (home, installed, removed) = round_trip(Agent::Gemini, Some(existing));
        assert_eq!(installed["theme"], "dark");
        let before_tool = installed["hooks"]["BeforeTool"].as_array().unwrap();
        assert_eq!(before_tool.len(), 2);
        assert_eq!(before_tool[0]["matcher"], "other");
        assert_eq!(before_tool[1]["matcher"], "*");
        assert_eq!(before_tool[1]["hooks"][0]["timeout"], 5000);
        let cmd = before_tool[1]["hooks"][0]["command"].as_str().unwrap();
        assert!(cmd.ends_with("--agent gemini PreToolUse"), "{cmd}");
        assert!(installed["hooks"]["AfterModel"].is_null());
        assert_eq!(removed.unwrap(), serde_json::from_str::<Value>(existing).unwrap());
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn installing_twice_leaves_one_entry_per_event() {
        let once = gemini_install(&json!({}), &[("BeforeTool".into(), "'x/coucou-hook' --agent gemini PreToolUse".into(), 5000)]).unwrap();
        let twice = gemini_install(&once, &[("BeforeTool".into(), "'y/coucou-hook' --agent gemini PreToolUse".into(), 5000)]).unwrap();
        assert_eq!(twice["hooks"]["BeforeTool"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn unexpected_types_are_refused_for_every_json_agent() {
        for agent in Agent::ALL {
            let home = scratch(&format!("odd-{}", agent.id()));
            let file = agent.files(&home)[0].clone();
            if file.extension().is_some_and(|e| e == "json") {
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                let shaped: &[&str] = match agent {
                    Agent::Antigravity => &[r#"{"coucou":"nope"}"#],
                    _ => &[r#"{"hooks":"nope"}"#, r#"{"hooks":[1]}"#],
                };
                for odd in shaped.iter().copied().chain(["[1,2]", "{ broken", "\"text\""]) {
                    std::fs::write(&file, odd).unwrap();
                    let edits = agent.edits(&home, &linux(), true);
                    assert!(config_file::preview(&edits).is_err(), "{agent:?} accepted {odd}");
                    assert_eq!(std::fs::read_to_string(&file).unwrap(), odd);
                }
            }
            let _ = std::fs::remove_dir_all(home);
        }
    }

    #[test]
    fn antigravity_gets_its_own_named_group_and_nothing_else_changes() {
        let existing = r#"{"my-guard":{"PreToolUse":[{"matcher":"run_command","hooks":[{"command":"/bin/guard"}]}]}}"#;
        let (home, installed, removed) = round_trip(Agent::Antigravity, Some(existing));
        assert_eq!(installed["my-guard"]["PreToolUse"][0]["matcher"], "run_command");
        let ours = &installed["coucou"];
        for event in ANTIGRAVITY_TOOL_EVENTS {
            assert_eq!(ours[event][0]["matcher"], "*");
            let cmd = ours[event][0]["hooks"][0]["command"].as_str().unwrap();
            assert!(cmd.ends_with(&format!("--agent antigravity {event}")), "{cmd}");
        }
        for event in ANTIGRAVITY_LIFECYCLE_EVENTS {
            assert_eq!(ours[event][0]["timeout"], 10);
            assert!(ours[event][0]["command"].as_str().unwrap().contains(MARKER));
        }
        assert_eq!(removed.unwrap(), serde_json::from_str::<Value>(existing).unwrap());
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn a_coucou_group_someone_else_wrote_is_neither_replaced_nor_removed() {
        let theirs = json!({ "coucou": { "Stop": [{ "command": "/bin/notify" }] } });
        assert!(antigravity_install(&theirs, &antigravity_block(&linux())).is_err());
        assert_eq!(antigravity_uninstall(&theirs).unwrap(), theirs);
    }

    #[test]
    fn on_windows_antigravity_runs_the_relay_path_quoted_only_when_needed() {
        let block = antigravity_block(&windows(WIN));
        assert_eq!(block["Stop"][0]["command"], format!("{WIN} --agent antigravity Stop"));
        let block = antigravity_block(&windows(WIN_SPACE));
        assert_eq!(block["Stop"][0]["command"], format!("\"{WIN_SPACE}\" --agent antigravity Stop"));
    }

    #[test]
    fn cursor_uses_its_native_format_and_keeps_the_users_hooks() {
        let existing = r#"{"version":1,"hooks":{"afterFileEdit":[{"command":"./format.sh"}],"stop":[{"command":"./notify.sh"}]}}"#;
        let (home, installed, removed) = round_trip(Agent::Cursor, Some(existing));
        assert_eq!(installed["version"], 1);
        for event in CURSOR_EVENTS {
            let last = installed["hooks"][event].as_array().unwrap().last().unwrap().clone();
            assert!(last["command"].as_str().unwrap().ends_with("--agent cursor"), "{event}");
        }
        assert_eq!(installed["hooks"]["stop"][0]["command"], "./notify.sh");
        assert_eq!(installed["hooks"]["afterFileEdit"][0]["command"], "./format.sh");
        assert_eq!(removed.unwrap(), serde_json::from_str::<Value>(existing).unwrap());
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn a_new_cursor_file_gets_its_version() {
        let after = cursor_install(&json!({}), "'x/coucou-hook' --agent cursor").unwrap();
        assert_eq!(after["version"], 1);
        // A version the user set is theirs.
        let after = cursor_install(&json!({ "version": 2 }), "c").unwrap();
        assert_eq!(after["version"], 2);
    }

    #[test]
    fn codex_hooks_match_the_macs_and_leave_the_rest_alone() {
        let existing = r#"{"description":"mine","hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"guard.sh"}]}]}}"#;
        let (home, installed, removed) = round_trip(Agent::Codex, Some(existing));
        assert_eq!(installed["description"], "mine");
        for (event, timeout) in CODEX_EVENTS {
            let ours = installed["hooks"][event].as_array().unwrap().last().unwrap()["hooks"][0].clone();
            assert_eq!(ours["timeout"], *timeout, "{event}");
            assert!(ours["command"].as_str().unwrap().ends_with("--agent codex"), "{event}");
            assert_eq!(ours.get("statusMessage").is_some(), *event == "PermissionRequest");
        }
        assert_eq!(installed["hooks"]["PreToolUse"][0]["matcher"], "Bash");
        assert_eq!(removed.unwrap(), serde_json::from_str::<Value>(existing).unwrap());
        assert!(Agent::Codex.approvals());
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn on_windows_codex_gets_a_command_cmd_can_run() {
        let after = codex_install(&json!({}), &windows(WIN_SPACE).command(Shell::Cmd, "--agent codex")).unwrap();
        assert_eq!(after["hooks"]["Stop"][0]["hooks"][0]["command"], format!("\"{WIN_SPACE}\" --agent codex"));
    }

    #[test]
    fn another_tools_entry_for_an_event_with_the_wrong_shape_is_refused() {
        let odd = json!({ "hooks": { "BeforeTool": { "matcher": "x" } } });
        assert!(gemini_install(&odd, &[("BeforeTool".into(), "c".into(), 1)]).is_err());
    }
}
