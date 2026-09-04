#!/usr/bin/env bash
# Build Wavalyze.app, install it for the current user, and expose `wv` on Cargo's PATH.

# Exit on command errors, unset variables, or failures hidden inside pipelines.
set -euo pipefail

# macOS reports its kernel name as "Darwin".
if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "error: this installer only supports macOS" >&2
    exit 1
fi

# `command -v` checks whether each program can be found through PATH.
for command in cargo rustc; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "error: Rust/Cargo not found; install it from https://rustup.rs" >&2
        exit 1
    fi
done

# cargo-bundle is a Cargo plugin. Install it once, using its locked dependencies.
if ! command -v cargo-bundle >/dev/null 2>&1; then
    cargo +1.93.0 install --locked cargo-bundle
fi

# Resolve paths from this script's location, so it works from any directory.
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bundle="$repo_root/target/release/bundle/osx/Wavalyze.app"
install_dir="${HOME:?}/Applications"
installed_app="$install_dir/Wavalyze.app"
installed_executable="$installed_app/Contents/MacOS/wavalyze-app"
cli_dir="$HOME/.cargo/bin"
cli_link="$cli_dir/wv"
# `$$` is this process ID, making the temporary path unique.
staged_app="$install_dir/.Wavalyze.app.installing.$$"

# Do not overwrite an unrelated command already named `wv`.
# `-L` also detects a dangling symbolic link, which `-e` alone misses.
if [[ (-e "$cli_link" || -L "$cli_link") && (! -L "$cli_link" || "$(readlink "$cli_link")" != "$installed_executable") ]]; then
    echo "error: refusing to replace existing $cli_link" >&2
    exit 1
fi

# Finish the release build before touching the currently installed app.
cd "$repo_root"
cargo +1.93.0 bundle --release --bin wavalyze-app

if [[ ! -d "$bundle" || ! -x "$bundle/Contents/MacOS/wavalyze-app" ]]; then
    echo "error: cargo-bundle did not produce a valid $bundle" >&2
    exit 1
fi

mkdir -p "$install_dir" "$cli_dir"

# Copy to a temporary location first. The trap removes a partial copy on failure.
# `ditto` is macOS's bundle-aware copy tool.
trap 'rm -rf "$staged_app"' EXIT
ditto "$bundle" "$staged_app"
[[ -x "$staged_app/Contents/MacOS/wavalyze-app" ]]

# Replace the old app, then point `wv` at the executable inside the new bundle.
rm -rf "$installed_app"
mv "$staged_app" "$installed_app"
ln -sfn "$installed_executable" "$cli_link"
trap - EXIT

# Confirm both Finder launch and command-line launch are usable.
if [[ ! -d "$installed_app" || ! -x "$installed_executable" || ! -L "$cli_link" || ! -x "$cli_link" ]]; then
    echo "error: installed app or CLI link validation failed" >&2
    exit 1
fi

echo "Installed $installed_app"
echo "CLI: $cli_link"
