#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

git config user.name "PChome Bot"
git config user.email "pchome@example.com"

declare -A files
files[.gitignore]="chore(repo): add .gitignore"
files[.editorconfig]="chore(repo): add .editorconfig"
files[CODEOWNERS]="chore(repo): add CODEOWNERS"
files[.github/workflows/ci.yml]="ci: add basic CI workflow for Rust lint/build"
files[pchome-desktop/Cargo.toml]="chore(desktop): add Cargo.toml workspace and deps"
files[pchome-desktop/src/main.rs]="feat(desktop): add main entry and pin manager startup"
files[pchome-desktop/src/pin.rs]="feat(desktop): add pin manager stub (6-digit TTL)"
files[pchome-desktop/src/uinput.rs]="feat(desktop): add uinput stub"
files[pchome-desktop/src/pipewire.rs]="feat(desktop): add pipewire capture stub"
files[pchome-desktop/src/encoder.rs]="feat(desktop): add encoder stub"
files[pchome-desktop/src/network/mod.rs]="feat(desktop): add network module scaffold"
files[pchome-desktop/src/network/socket.rs]="feat(desktop): add UDP socket stub"
files[pchome-desktop/src/network/webrtc.rs]="feat(desktop): add WebRTC stub"
files[pchome-desktop/src-ui/index.html]="feat(ui): add HUD index HTML"
files[pchome-desktop/src-ui/styles/hud.css]="style(ui): add HUD styles (color palette)"
files[pchome-desktop/src-ui/js/app.js]="feat(ui): add HUD app JS stub"

for path in "${!files[@]}"; do
  # mark checkbox as checked in TODO.md
  sed -i "s|- \[ \] \\`$path\\`|- [x] \\`$path\\`|g" TODO.md
  git add "$path" TODO.md
  git commit -m "${files[$path]}"
  echo "Committed: ${files[$path]}"
done

echo "All commits created."
