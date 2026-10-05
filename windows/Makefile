# Coucou for Windows: build it and install it for the current user.
#
#   make              everything: checks the tools, installs the npm packages (npm ci),
#                     builds the installers, installs Coucou and starts it
#   make prereqs      installs Rust, Node and the MSVC build tools with winget
#   make build        builds only: windows/release/ gets the setup .exe and the .msi
#   make install-msi  installs from the MSI instead (per machine, asks for admin)
#   make uninstall    removes the installed Coucou
#   make dev          live-reloading development build
#   make test         type-checks the front end and runs the Rust tests
#   make clean        removes what the builds left behind
#
# The recipes are plain npm, node and cargo commands (the Windows-specific steps
# are in scripts/setup.mjs), so they run the same from PowerShell, cmd or Git
# Bash. GNU Make itself: `winget install ezwinports.make`.
#
# The Claude Code hooks are not installed from here: Settings… → Claude Code →
# Install hooks… shows the diff of settings.json and writes it after your click.

.PHONY: all check prereqs deps build install install-msi uninstall dev test clean

all: install

check:
	node scripts/setup.mjs check

# winget fails on a package that is already installed and up to date: `-` lets
# the next one go on. Open a new terminal afterwards so PATH picks them up.
prereqs:
	-winget install --id Rustlang.Rustup -e
	-winget install --id OpenJS.NodeJS.LTS -e
	-winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

# `ci`, not `install`: exactly what package-lock.json says, and the lock file is
# never rewritten, so a `git pull` afterwards can't trip on it.
deps:
	npm ci

# A running coucou.exe locks target/release/coucou.exe, and the link fails.
build: check deps
	node scripts/setup.mjs stop
	npm run pack

install: build
	node scripts/setup.mjs install

install-msi: build
	node scripts/setup.mjs install --msi

uninstall:
	node scripts/setup.mjs uninstall

dev: deps
	npm run tauri dev

# The app's build script wants the hook relay built first (it is a bundled resource).
test: deps
	npx tsc --noEmit
	cargo build --release -p coucou-hook
	cargo test --workspace

clean:
	cargo clean
	node scripts/setup.mjs clean
