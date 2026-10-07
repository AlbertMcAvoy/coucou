<div align="center">

<img src="src-tauri/icons/128x128.png" width="96" alt="Coucou icon">

# Coucou for Windows

**Mochi doesn't get a notch on a PC — so it lives at the top of your screen instead.**

Approve Claude Code permissions, watch your session work, drop a file, chat with Claude, keep an eye on your services — without leaving what you're doing.

![Windows 10/11](https://img.shields.io/badge/Windows-10%2F11-0078D4?logo=windows)
![Tauri 2](https://img.shields.io/badge/Tauri-2-FFC131?logo=tauri&logoColor=black)
![Rust](https://img.shields.io/badge/Rust-backend-000?logo=rust)
![Code: MIT](https://img.shields.io/badge/code-MIT-green)

</div>

<img src="screenshots/greeting.png" width="640" alt="Mochi waving hello at launch">

---

## Install

The downloadable installer is **temporarily unavailable**. Microsoft Defender
wrongly flags the unsigned installer as malware (`Trojan:Win32/Wacatac.H!ml`, a
machine-learning false positive). A report is under review at Microsoft, and the
installer will be published again once it is cleared and code-signed.

Until then, [build it yourself](#build-it-yourself): it takes a few minutes and
installs for the current user only — no admin prompt.

## Using it

<img src="screenshots/compact.png" width="292" alt="The compact island, with the integration pills as mini Mochis">
<img src="screenshots/overview.png" width="640" alt="The overview: the focused integration on the left, the other pills on the right">
<img src="screenshots/approval.png" width="640" alt="A Claude Code permission request, with Deny and Allow">
<img src="screenshots/chat.png" width="640" alt="Chatting with Claude from the island">
<img src="screenshots/drop.png" width="640" alt="Mochi turned into a box, waiting for a file">

| What you do | What happens |
|---|---|
| Move the mouse to the very top-centre of the screen | Mochi peeks out |
| Click the small island | It opens |
| Click Mochi | It gets annoyed. Three times in a row and it goes dizzy |
| Rest the pointer on Mochi for two seconds | Hearts |
| Drag a file onto the island | Mochi turns into a box, swallows it, then offers to answer questions about it |
| `Esc` | Closes the island |
| Tray icon | Open, Settings…, Pause, Quit |

Everything else happens on its own: a Claude Code permission request opens the
island with **Deny / Allow**, a question from Claude Code shows its options to
pick from, a finished session shows what it did, and
your integrations sit in the coloured pills next to Mochi.

A permission card or a question stays until you answer it: the mouse leaving
never folds it, it comes up even when the island is already open or another
pill is in front, and the pill you were on comes back once you answer. To keep
it for later, fold it with the **⌃** in its corner (or `Esc` in the island): the
island shrinks to its compact size and stays on screen, nothing is answered, and
opening it again shows the card. **Open terminal** brings the window the session
runs in to the front.

## Your pills

**Settings… → Active pills** lists the tools you use, from the same catalog as
the Mac app. Pick your **main tool** — VS Code, Cursor, Codex or Antigravity —
which is always there and doesn't take a slot, then declare up to four more:
agents (Gemini CLI, Copilot CLI, Muse Code, OpenCode, Amp, Hermes, Claude
Desktop), the chat providers (Anthropic, Google AI, OpenAI, Ollama, LM Studio),
and the services under **Integrations**. A pill fed by hooks says whether its
hooks are installed, never asks for a key; a local model server's pill says
whether the chat is connected to it. A session on a pill you didn't
declare still shows up, for as long as it runs.

## Claude Code

<img src="screenshots/settings.png" width="562" alt="The settings window">

Open **Settings… → Claude Code → Install hooks…**. You get the exact diff of what
will change in `%USERPROFILE%\.claude\settings.json`, the path of the dated backup
that will be taken, and nothing is written until you click. Your own hooks are
never touched, and uninstalling removes only Coucou's entries.

The relay is a tiny executable, `coucou-hook.exe`, copied to
`%LOCALAPPDATA%\Coucou\bin\` at launch. It is given 300 ms to reach Coucou and
exits cleanly if the app is closed, slow or crashed — **a Claude Code session is
never blocked or slowed down by Coucou.** If nobody answers a permission request
in time, Coucou stays quiet and Claude Code asks in the terminal as usual.

It works from any terminal — Windows Terminal, PowerShell, VS Code, Git Bash.

### Plan usage

As on the Mac, the island's header can show your plan limits: a small pill
("Claude 73%", green below 50 %, orange up to 80 %, red above) for the 5-hour and
weekly Claude limits, and another for Codex. Click one for the details and the
reset times. Both are off by default; turn them on in **Settings… → Plan usage**.

- **Claude** (Pro and Max plans): the numbers come from Claude Code's own status
  line. **Show in notch** first shows you the diff of the `statusLine` change in
  `%USERPROFILE%\.claude\settings.json`, takes a dated backup and writes only
  after your click, with the same writer as the hooks: the status line becomes `coucou-hook --statusline`, which
  passes only the limits on (300 ms at most) and runs the status line you had
  before — kept in `statusline-previous.json` next to the relay — with the same
  input, printing what it prints. On Windows that one runs through Git Bash, as
  Claude Code runs it (`CLAUDE_CODE_GIT_BASH_PATH`, then the Git for Windows that
  `git.exe` on `PATH` belongs to, then the usual install folders); it gets 10 s
  and 64 KB of output. **Uninstall relay** puts your status line back. The
  numbers arrive with Claude Code's replies.
- **Codex**: nothing is installed. When the pill shows (or is clicked, at most
  once a minute) Coucou starts `codex app-server` and asks it
  `account/rateLimits/read`, as Codex's `/status` does, then stops it (15 s at
  most, never while paused). Codex must be signed in with ChatGPT.

## Chat and keys

**Settings… → Claude** takes your Anthropic API key. Keys live in the **Windows
Credential Manager**, never on disk and never in the interface — the island can
only ask whether a key exists. Same for every integration key.

The chat also talks to **Google AI (Gemini)**, **OpenAI** and **OpenRouter**:
add their keys in **Settings… → Chat providers**, then click the model name
above the chat box to switch provider and model, as on the Mac. The model list
is fetched from the provider only once you pick it and it has a key. Switching
mid-conversation carries the conversation over as plain text, so nothing in one
provider's format is ever sent to another. These providers get no web search
and no tools — they answer, they never act on your PC.

**Local models**: **Settings… → Local models** connects **Ollama** or **LM
Studio** (leave the address empty for the usual one on this PC; Ollama's
`OLLAMA_HOST` is honoured) or any server that speaks the OpenAI API (vLLM,
llama.cpp…), with an optional key kept in the credential store. Answers stream
in as they are written, and the `<think>` blocks of reasoning models stay
hidden. A text file you dropped goes along inline (24 000 characters at most);
images and PDFs by name only. Settings tells you whether the address is this
PC — nothing leaves it then — and warns before a key would travel over plain
`http://` to another machine.

Answers from every provider are shown as **Markdown**: headings, lists, bold,
inline code, quotes, and code blocks with a copy button. It is built from text
nodes, never parsed as HTML, and only `http`/`https` links open. Mochi greets
you by your first name when your account has one (the Windows display name or
the Linux GECOS full name; a bare login name is not used).

To send the Claude chat through an Anthropic-compatible gateway, set
`COUCOU_ANTHROPIC_BASE_URL` (for example `https://gateway.example.com`;
`/v1/messages` is added). It must be `https://`, or `http://` to this PC only.
Claude Code's own `ANTHROPIC_BASE_URL` is deliberately ignored: your key only
goes where you told Coucou to send it. The gateway's host is written to the log
once; the key never is.

No telemetry. The only network requests Coucou makes are to the services you
configure yourself.

## Build it yourself

You need [Rust](https://rustup.rs), [Node 20+](https://nodejs.org), and the
**MSVC build tools** (Visual Studio Build Tools with "Desktop development with
C++"). WebView2 ships with Windows 10/11.

```powershell
cd windows
npm install
npm run tauri dev      # live-reloading development build
npm run pack           # builds the installer and drops it in windows/release/
```

`npm run dev` alone serves the front end in an ordinary browser, which is enough
to work on the island's looks. It also serves `dev/upload-preview.html`, which
replays the whole file-drop choreography on a loop — the one part of the UI that
otherwise needs a real drag from Explorer to see. Neither page ships in the app.

`npm run pack` leaves two files in `windows/release/`, the same names the release
workflow publishes:

```
Coucou-Windows-X.Y.Z-setup.exe    the versioned installer
Coucou-Windows-setup.exe          the same file under the rolling name
```

Installing is optional — `target/release/coucou.exe` runs on its own. There is no
window in the taskbar and no console: the island at the top of the screen and the
Mochi in the notification area are the whole app, and Quit lives in its menu.

The 28 sounds are the macOS app's own files; they are never duplicated in this
folder. The path is declared once, in `SOUNDS_DIR` at the top of
`vite.config.ts` — when they move to `shared/sounds/`, change that one line.

The app icon and the tray icon are drawn in code, like Mochi itself:

```powershell
npm run icons          # regenerates src-tauri/icons from scripts/gen-icons.mjs
```

### Layout

```
windows/
  src/                 island front end (TypeScript, no framework)
    mochi/             Mochi and the launch greeting, in Canvas 2D
    island/            state machine, hooks, integrations
    views/             every island view
    settings/          the settings window
  src-tauri/           Rust backend: window, named pipe, Claude API, pollers
  hook/                coucou-hook.exe, the Claude Code relay
  scripts/             icon generator
```

### Log

`%LOCALAPPDATA%\Coucou\coucou.log` — hook events, permission decisions, poller
problems. It stays on your machine.

## Supported agents

Every agent below is installed from **Settings → Agents** with the same steps as
Claude Code: the exact diff, the path of the dated backup, nothing written until
you click, and uninstalling removes only Coucou's entries. A config Coucou cannot
read, or where it finds something it does not expect, is left alone and the
reason is shown. Each agent gets its own pill (`agent_<name>`, the Mac's ids and
colours). The files are the Mac's, under `%USERPROFILE%` on Windows and `~` on
Linux.

| Agent | Installs | Permissions |
|---|---|---|
| Claude Code | `.claude\settings.json` (**Settings → Claude Code**) | Allow / Deny and questions in the island |
| Codex | `.codex\hooks.json` — then trust the hooks once with `/hooks` in Codex | Allow / Deny in the island |
| GitHub Copilot CLI | `.copilot\hooks\coucou.json` | Allow / Deny in the island |
| Muse Code | `.config\muse\settings.json` | Allow / Deny in the island |
| Gemini CLI | `.gemini\settings.json` | asked in Gemini CLI |
| Antigravity | `.gemini\config\hooks.json` (a `coucou` hook group) | asked in Antigravity |
| Cursor Agent | `.cursor\hooks.json` — Claude Code in Cursor's terminal also goes on the Cursor pill, through the Claude Code hooks | asked in Cursor |
| Claude Desktop (Windows) | nothing to install: Claude Code sessions from the Claude app are tagged by the relay | asked in the Claude app |
| OpenCode | plugin `.config\opencode\plugins\coucou.js` | asked in OpenCode |
| Amp | plugin `.config\amp\plugins\coucou.ts` | asked in Amp |
| Hermes Agent | plugin `.hermes\plugins\coucou\` — then `hermes plugins enable coucou` once | asked in Hermes |
| Any other | run `coucou-hook --agent <name> [<Event>]` from your tool's hooks | asked in the tool |

The relay maps every agent's event and field names onto Claude Code's (Gemini
CLI's `BeforeTool`, Copilot's `preToolUse`, Cursor's `beforeSubmitPrompt`…), and
answers each agent the way it expects. It never lets anything through on its
own: with no click it prints no decision at all (`{}` for the agents that need
JSON, `"ask"` for Copilot, which is fail-closed), so the agent asks in its own
terminal exactly as without Coucou — including when Coucou is closed.

The plugins start the relay directly, with no shell in between, and never wait
for it. Amp's steps appear as each tool finishes: its "before" hook must return
a verdict, and Coucou never gives one.

**How each agent runs the relay on Windows.** Hook commands are written for the
shell that runs them: Git Bash for Claude Code (quoted, forward slashes),
PowerShell for Gemini CLI and Copilot CLI (`& '…\coucou-hook.exe'`), `cmd /C`
for Codex. Cursor, Antigravity and Muse Code do not document theirs: the relay
path is written bare when it has no space or special character — which works in
cmd, PowerShell and when started directly — and in double quotes otherwise.
These three are untested on Windows.

A pill is **connected** when Coucou finds its own entries in the files above —
the same check as Settings → Agents (for Claude Code: a SessionStart hook
running Coucou's relay; the Cursor pill also counts Claude Code's hooks).
Coucou only reads these files, each time the island opens. Permission requests
get the island's card for Claude Code (in any terminal, and in Cursor's),
Codex, Copilot CLI and Muse Code; other agents and Claude Desktop ask in their
own window.

## What's different from the Mac version

- No notch, so the island lives at the top centre of the screen and retracts into
  the top edge instead of hiding in a notch.
- Permission approval works from **any** terminal; the Mac build only listens to
  VS Code sessions.
- "Open terminal" finds the session's window by walking up from the relay's
  process to the terminal or editor that runs it. A session in a classic
  console window (`cmd.exe` or PowerShell without Windows Terminal) has no such
  ancestor — conhost owns that window — so its folder opens in VS Code instead,
  as it does when `code` is on your `PATH` and nothing was found.
- No global keyboard shortcuts yet: a waiting card is folded with its **⌃** or
  `Esc` in the island, and reopened by clicking the island or Open in the tray.
- Apple Music, the one pill from the Mac catalog with nothing behind it here,
  is left out.
- Not in this version: sending a dropped file by email and dragging Mochi onto
  a window to attach it as context. On the Mac, email goes through Resend or
  Apple Mail's scripting; neither has a safe equivalent that attaches a file
  here, and the drop card would need a third button it doesn't have.
- Cal.com shows the next bookings as a list rather than the Mac's calendar.
- The Cursor pill carries both Cursor Agent's own hooks and Claude Code running
  in Cursor's terminal.
- Hermes: Coucou writes the plugin but does not run the `hermes` CLI, so it is
  turned on once by hand. Hermes runs natively on Linux; on Windows it is
  untested.
- Plan usage: the user's previous status line runs through Git Bash on Windows
  (`/bin/sh` on Linux and Mac); without Git Bash it is not run, rather than
  guessed at with `cmd`. The Codex CLI is looked for on `PATH` and in npm's,
  Volta's, Bun's and pnpm's folders (and nvm's on Linux).
- The chat's model picker opens inside the chat card instead of a popover, and
  it also offers OpenRouter and any OpenAI-compatible server, which the Mac
  does not. Google AI, OpenAI and OpenRouter can see an image you dropped (sent
  inline), where the Mac sends its name only.

## Linux

The same app builds for Linux: everything that differs lives in
`src-tauri/src/platform/`, and the relay's transport in `hook/src/unix.rs`.

```bash
sudo apt install build-essential pkg-config \
  libwebkit2gtk-4.1-dev libgtk-layer-shell-dev libayatana-appindicator3-dev \
  librsvg2-dev libssl-dev libdbus-1-dev patchelf \
  gstreamer1.0-plugins-base gstreamer1.0-plugins-good
npm install
npm run tauri dev      # live-reloading development build
npm run pack           # AppImage, .deb and .rpm in windows/release/
```

On Arch Linux, build and install the package from `linux/arch/`:

```bash
git clone https://github.com/Louis-CFM/coucou.git
cd coucou/linux/arch
makepkg -si
```

It needs `webkit2gtk-4.1`, `gtk-layer-shell` and `libayatana-appindicator`
(pulled in as dependencies); store API keys with GNOME Keyring or KWallet.

What changes on Linux:

- **The island** is a gtk-layer-shell overlay anchored to the top edge, over any
  top panel, on compositors that support it: COSMIC, KDE Plasma, Hyprland, Sway
  and other wlroots compositors. GNOME has no layer-shell and ignores where a
  Wayland window asks to go, so there Coucou runs through XWayland as a dock
  window: top centre, on every workspace, still there after Super+D.
  `COUCOU_X11=0` keeps the native Wayland window, `COUCOU_DOCK=0` makes it a
  utility window instead of a dock. `COUCOU_LAYER_SHELL=0` forces the regular
  window anywhere.
- **Click-through** is the window's input region, kept equal to the island
  shape, so the compositor sends every other click to what is underneath.
- **Mochi's eyes** follow the pointer only while it is over the island: Wayland
  gives no app the cursor position anywhere else.
- **Claude Code hooks** go through `~/.local/share/coucou/bin/coucou-hook` and a
  Unix socket at `$XDG_RUNTIME_DIR/coucou.sock`. Both ends check that the other
  runs as the same user. Every other agent uses the same relay, single-quoted
  for `sh`, and its config under `~` (see Supported agents). A config that is a
  symlink (dotfiles) is written through to its target, with its permissions
  kept.
- **Keys** live in the Secret Service (GNOME Keyring, KWallet).
- **Plan usage**: the status line relay is `~/.local/share/coucou/bin/coucou-hook
  --statusline` and runs your previous status line with `/bin/sh -c`, like Claude
  Code. Codex is found on `$PATH`, in `~/.local/bin`, npm's global prefix, Volta,
  Bun, pnpm, or nvm (newest Node first), since a desktop launch often has a
  shorter `$PATH` than your shell.
- **Mochi's greeting** uses the full name in your account's GECOS field
  (`chfn` sets it); without one the chat stays neutral.
- **Files**: preferences in `~/.config/coucou/`, the log at
  `~/.local/share/coucou/coucou.log`.
- What the Windows build leaves out, this one does too: sending a file by
  email and dragging Mochi onto a window.
- **Open terminal** opens the folder in VS Code: Wayland lets no app bring
  another app's window forward, and X11 would need a window-manager client this
  build doesn't carry.
- No **Claude Desktop** pill: the Claude app has no Linux build.
