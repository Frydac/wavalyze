# Cargo installation and desktop distribution plan

Status: proposed implementation plan, 2026-09-20. No publishing or release changes have been made.

## Goal

Provide two supported installation routes for the existing egui/eframe application:

- Rust users can run `cargo install wavalyze --locked` and launch `wavalyze`.
- Desktop users can download a prebuilt package, install it without Rust, and launch Wavalyze
  from their operating system's application launcher.

Keep the existing GUI framework. Use established packaging tools rather than developing an
installer application. Cargo publication and prebuilt releases are independent: desktop releases
need not wait for all dependencies to become publishable on crates.io.

## Current state

- `Cargo.toml` defines the package `wavalyze`, but the main executable is `wavalyze-app`.
- `bench_thumbnail` and `digitwise_editor` are also binary targets and would be installed by
  a default `cargo install`.
- `egui_custom_widgets` and `egui_tracing` are Git dependencies without registry versions.
- The package's `include` list excludes the PNG embedded by `src/main.rs`; a published source
  archive would therefore be missing a required build input.
- License files exist, but package description, license declaration, and repository metadata
  still need to be added.
- `scripts/install-macos.sh` builds an app using `cargo-bundle`, installs it into
  `~/Applications`, and creates a `wv` symlink in `~/.cargo/bin`.
- `.github/workflows/rust.yml` is configured to build macOS ARM64, Windows x86-64, and several
  Linux targets, and upload raw binaries on tags. This is configuration evidence, not confirmation
  that every release artifact installs and runs successfully.
- The eframe configuration explicitly enables X11; native Wayland support needs evaluation.

## Phase 1: make the source package installable

- [ ] Choose `wavalyze` as the public executable name. Update Cargo targets, scripts, CI artifact
  paths, documentation, and any other references to `wavalyze-app`. Preserve the existing `wv`
  convenience command in the macOS installation flow where practical.
- [ ] Move development binaries to examples, a separate development package, or non-default
  feature-gated targets so ordinary installation installs only the application.
- [ ] Resolve the Git dependencies with compatible crates.io releases. Publish the maintained
  custom widget crate if needed. For unavailable upstream functionality, choose between an
  upstream release, a separately published fork, or incorporating the required code with its
  license and attribution. Check transitive dependencies too.
- [ ] Add package description, `license = "MIT OR Apache-2.0"`, repository, and README metadata;
  verify that the declaration matches the project's licensing.
- [ ] Correct the package file list to include all compile-time assets and required sources,
  while excluding development data. Verify that the generated archive contains `Cargo.lock`.
- [ ] Check ownership/availability of the crates.io name `wavalyze` and configure publishing
  credentials for the maintainer or release workflow.
- [ ] Verify the declared minimum Rust version against the locked dependency graph; document
  compiler/linker and Linux system package requirements.
- [ ] Run `cargo package --list` and `cargo publish --dry-run`. Build/install from the extracted
  package in a clean environment, outside the checkout, on each initially supported OS.
- [ ] Test both default dependency resolution and `--locked`; document the locked command as
  the recommended installation route.
- [ ] Publish the initial version and verify `cargo install wavalyze --locked` from crates.io.

Acceptance: installation from the registry produces only the intended executable, which launches
the GUI and opens a WAV file on each supported OS. The packaged source does not depend on files
or Cargo configuration available only in the repository checkout.

An interim source installation can use
`cargo install --git https://github.com/frydac/wavalyze --locked --bin wavalyze-app`
with the current target name. Verify this route before documenting it as supported, and update
the command when the executable is renamed.

## Phase 2: produce desktop packages in CI

Start with Linux x86-64, Windows x86-64, and macOS Apple Silicon. Decide whether Intel macOS
is required for the first release; add a separate build or a universal app if it is. Retain other
existing build targets where useful, but distinguish build coverage from tested desktop support.

| Platform | Initial artifact | Packaging work |
| --- | --- | --- |
| macOS | `Wavalyze.app` in a ZIP or DMG | Extend the existing bundle configuration; support copying the app into Applications. |
| Windows | Setup `.exe` or `.msi`, optionally a portable ZIP | Select an installer generator, such as Inno Setup; add icons, Start menu entry, version metadata, and uninstall support. |
| Linux | `.deb`, optionally a portable archive | Install the binary, desktop entry, and icons; declare runtime dependencies. |

- [ ] Select and pin packaging-tool versions. Reuse `cargo-bundle` where suitable; evaluate its
  platform support before committing to one tool for all formats.
- [ ] Build release artifacts using the lockfile on appropriate platform runners. Supply the
  existing build-metadata environment variables from the release commit and release date.
- [ ] Define minimum supported OS versions. For Linux, choose a build environment with a
  suitable glibc baseline and inspect runtime library dependencies.
- [ ] Check native library redistribution requirements and include required license notices.
- [ ] Configure application identity and platform icons consistently. Verify the Windows
  executable's shell icon separately from the icon shown inside the running window.
- [ ] Add Linux desktop-entry metadata, including an appropriate category and `Terminal=false`.
- [ ] Verify X11 operation and decide whether to enable/test native Wayland support.
- [ ] Produce versioned artifacts, checksums, and installation instructions in tagged releases.
- [ ] Test clean installation, launcher startup, upgrade, and uninstall on each platform.
  Preserve user settings during upgrades and define uninstall behavior explicitly.

Acceptance: a user without Rust can install and launch each supported desktop package, and
upgrade it with a documented procedure. A passing compilation alone does not meet this criterion.

## Phase 3: signing and opening files from the desktop

Basic launcher integration belongs in phase 2. File associations and public-download trust require
additional work; signing can be developed alongside phase 2 before a broadly advertised release.

- [ ] Set up macOS Developer ID signing, notarization, and ticket stapling as appropriate for
  the chosen artifact. Store credentials securely in CI and test a downloaded, quarantined app.
- [ ] Decide on Windows code signing, provision credentials if adopted, and test the downloaded
  installer experience. Signing alone does not guarantee the absence of reputation warnings.
- [ ] Register Wavalyze as an optional WAV handler using macOS document types, Windows file
  associations, and Linux MIME/desktop entries. Do not silently replace the user's default player.
- [ ] Verify OS file-open delivery, especially macOS open-document events, beyond CLI parsing.
- [ ] Decide whether opening another file creates a new window/process or forwards it to an
  existing instance; implement forwarding only if that behavior is chosen.
- [ ] Test multiple files, spaces and non-ASCII paths, launch from an unrelated working directory,
  and opening a file when the app is already running. Verify drag-and-drop and file dialogs too.
- [ ] Document how terminal users launch the packaged app, including the existing `wv` convention.
  For Cargo users, consider an explicit user-local desktop-integration command with a matching
  removal command. Do not perform desktop registration from `build.rs`.

Acceptance: downloaded packages have the intended signing behavior, appear in application
launchers, and can open selected WAV files through the OS using the documented instance behavior.

## Later distribution options

After the first packages work reliably, evaluate Homebrew, WinGet, broader Linux formats
(AppImage, Flatpak, RPM), and prebuilt installation through `cargo-binstall`. Each adds release
metadata or maintenance obligations; none is necessary for the initial Cargo and desktop routes.
Start with manual/package-manager upgrades. An in-app automatic updater is a separate project.

## Recommended implementation order

1. Complete source-package cleanup and publication validation in phase 1.
2. Publish when dependency releases and registry ownership are ready; independently extend CI
   with the phase 2 packages.
3. Complete signing and file-open integration, and smoke-test actual downloads on clean systems.
4. Update the README with a platform download/install table, Cargo instructions, prerequisites,
   supported OS versions, upgrade/uninstall instructions, and known limitations.

The main uncertainties are compatible registry releases of the Git dependencies, clean-machine
runtime requirements, macOS file-open handling, and signing setup. Resolve these early rather than
estimating completion solely from the amount of packaging configuration.

## References

- [Cargo install](https://doc.rust-lang.org/cargo/commands/cargo-install.html)
- [Publishing on crates.io](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [Dependency sources and publication](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-locations)
- [cargo-bundle](https://github.com/burtonageo/cargo-bundle)
- [Apple notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
- [Inno Setup](https://jrsoftware.org/isinfo.php)
- [Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry/latest/)
