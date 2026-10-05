// The Windows steps behind the Makefile, kept in Node so the Makefile's recipes
// stay plain commands that run the same from PowerShell, cmd or Git Bash.
//
//   node scripts/setup.mjs check            the tools a build needs are here
//   node scripts/setup.mjs stop             quits a running Coucou
//   node scripts/setup.mjs install [--msi]  installs what `npm run pack` built, starts it
//   node scripts/setup.mjs uninstall        removes the installed Coucou
//   node scripts/setup.mjs clean            removes dist/ and release/

import { execFileSync, spawn } from "node:child_process";
import { existsSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const release = join(root, "release");
const [command, ...flags] = process.argv.slice(2);

// Where each installer puts the app: the NSIS one installs for the current user
// (installMode "currentUser"), the MSI one for the machine.
const NSIS_DIR = join(process.env.LOCALAPPDATA ?? "", "Coucou");
const MSI_DIR = join(process.env.ProgramFiles ?? "C:\\Program Files", "Coucou");

function fail(message) {
  console.error(message);
  process.exit(1);
}

function run(file, args) {
  execFileSync(file, args, { stdio: "inherit" });
}

function check() {
  const missing = [];
  const [major] = process.versions.node.split(".").map(Number);
  if (major < 20) missing.push(`Node 20 or newer (this is ${process.version})`);
  for (const tool of ["cargo", "rustc"]) {
    try {
      execFileSync(tool, ["--version"], { stdio: "ignore" });
    } catch {
      missing.push(`${tool} (Rust, https://rustup.rs)`);
    }
  }
  if (process.platform === "win32") {
    const vswhere = join(
      process.env["ProgramFiles(x86)"] ?? "C:\\Program Files (x86)",
      "Microsoft Visual Studio", "Installer", "vswhere.exe",
    );
    let msvc = "";
    try {
      msvc = execFileSync(vswhere, [
        "-latest", "-products", "*",
        "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
        "-property", "installationPath",
      ], { encoding: "utf8" }).trim();
    } catch {}
    if (!msvc) missing.push("the MSVC build tools (\"Desktop development with C++\")");
  }
  if (missing.length) {
    fail(`Missing:\n${missing.map((m) => `  - ${m}`).join("\n")}\n\`make prereqs\` installs them with winget.`);
  }
  console.log("Node, Rust and the MSVC build tools are here.");
}

function stop() {
  if (process.platform !== "win32") return;
  try {
    execFileSync("taskkill", ["/IM", "coucou.exe", "/F"], { stdio: "ignore" });
    console.log("Quit the running Coucou.");
  } catch {
    // Not running.
  }
}

function install() {
  if (process.platform !== "win32") {
    fail("On Linux, install the .deb, .rpm or AppImage that `npm run pack` left in windows/release/.");
  }
  const msi = flags.includes("--msi");
  const installer = join(release, msi ? "Coucou-Windows.msi" : "Coucou-Windows-setup.exe");
  if (!existsSync(installer)) fail(`${installer} isn't there: run \`make build\` first.`);

  stop();
  console.log(`Installing ${installer}…`);
  if (msi) {
    // Per machine: Windows asks for admin rights.
    run("msiexec", ["/i", installer, "/passive", "/norestart"]);
  } else {
    run(installer, ["/S"]);
  }

  const exe = join(msi ? MSI_DIR : NSIS_DIR, "coucou.exe");
  if (!existsSync(exe)) fail(`Installed, but ${exe} isn't there.`);
  spawn(exe, [], { detached: true, stdio: "ignore" }).unref();
  console.log(`Coucou is installed in ${dirname(exe)} and running: Mochi is at the top of the screen.`);
}

function uninstall() {
  if (process.platform !== "win32") fail("Uninstall it with your package manager.");
  stop();
  let removed = false;
  const nsis = join(NSIS_DIR, "uninstall.exe");
  if (existsSync(nsis)) {
    run(nsis, ["/S"]);
    removed = true;
  }
  if (existsSync(join(MSI_DIR, "coucou.exe"))) {
    const msi = join(release, "Coucou-Windows.msi");
    if (existsSync(msi)) {
      run("msiexec", ["/x", msi, "/passive", "/norestart"]);
      removed = true;
    } else {
      console.log("The MSI install is left: remove Coucou from Settings → Apps.");
    }
  }
  console.log(removed ? "Coucou is uninstalled." : "No installed Coucou found.");
  // Claude Code's settings.json is left alone, as the uninstaller does:
  // Settings… → Claude Code → Uninstall hooks removes Coucou's entries first.
}

function clean() {
  for (const dir of ["dist", "release"]) rmSync(join(root, dir), { recursive: true, force: true });
}

const COMMANDS = { check, stop, install, uninstall, clean };
const action = COMMANDS[command];
if (!action) fail(`Usage: node scripts/setup.mjs ${Object.keys(COMMANDS).join("|")} [--msi]`);
action();
