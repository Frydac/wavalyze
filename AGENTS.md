# Wavalyze Agent Instructions

This file provides guidance for agents working on the Wavalyze codebase. Wavalyze is a WAV file viewer/analyzer built with Rust and egui, supporting both native and WASM targets.

## Project Overview

- **Language:** Rust (Edition 2024, MSRV 1.92.0)
- **UI Framework:** egui/eframe
- **Audio Processing:** hound, rayon
- **CLI Parsing:** clap
- **Build Tool:** Trunk (for WASM)
- **Structure:** Binary crate with `src/main.rs`, `src/app.rs`, `src/lib.rs`

## Build, Lint, and Test

### Quick Commands
- **Run native app:** `cargo run --release`
- **Run WASM dev server:** `trunk serve` (then open `http://127.0.0.1:8080/index.html#dev`)
- **Add WASM target:** `rustup target add wasm32-unknown-unknown`

### Full CI Check (run after larger edits)
```bash
./check.sh
```
This runs: `cargo check` (workspace, all targets, WASM), `cargo fmt --check`, `cargo clippy` (with `-D warnings`), `cargo test` (unit + doc tests), and `trunk build`.

### Before Committing
- Run `./check.sh` before creating a commit.
- If `cargo fmt` changes files, rerun `./check.sh` so the committed tree matches the verified tree.

### Individual Commands
```bash
# Build
cargo build
trunk build                    # WASM build

# Lint
cargo fmt --all -- --check     # Format check
cargo clippy --quiet --workspace --all-targets --all-features -- -D warnings -W clippy::all

# Test
cargo test --quiet --workspace           # All tests
cargo test --quiet --workspace --doc     # Doc tests only
cargo test <test_name>                   # Single test (e.g., `cargo test test_common`)
```

## Code Style Guidelines

### Formatting
- **Standard:** Follow `rustfmt` with edition 2024
- **Config:** `rustfmt.toml` uses `use_small_heuristics = "Default"`
- **Enforcement:** Run `cargo fmt` before committing

### Imports
Group imports in this order:
1. `std` imports
2. External crate imports (e.g., `egui`, `anyhow`, `clap`)
3. `crate` (project) module imports

Use `use crate::` for project modules, `use self::` for sibling modules:
```rust
use crate::{
    args::{self, Args},
    model::{self, Action},
    view,
    wav::ReadConfig,
};
use eframe::egui;
use tracing::trace;
```

### Naming Conventions
- **Functions/variables:** `snake_case`
- **Types/structs/enums:** `PascalCase`
- **Module names:** `snake_case`
- **Constants:** `SCREAMING_SNAKE_CASE`

### Error Handling
- **Main functions:** Return `anyhow::Result<()>`
- **Error propagation:** Use `?` operator
- **Unrecoverable errors:** Use `expect()` or `unwrap()` with descriptive messages
- **Example:**
  ```rust
  fn main() -> anyhow::Result<()> {
      let config = load_config()?;
      Ok(())
  }
  ```

### Clippy
Code must be clippy-clean:
```bash
cargo clippy --quiet --workspace --all-targets --all-features -- -D warnings -W clippy::all
```
Common rules:
- Allow `unused_variables` where intentionally unused
- Use `#![allow(dead_code)]` sparingly for planned features

### Comments
- Add comments for complex logic and non-obvious code
- Use `// NOTE:`, `// TODO:`, `// FIXME:` prefixes for maintenance notes
- Document public API behavior in doc comments (`///`)

### Dependencies
- Add new dependencies to `[dependencies]` in `Cargo.toml`
- Add test-only dependencies to `[dev-dependencies]`
- Use `cargo add` when possible to maintain proper formatting

## Project Structure

```
src/
├── main.rs          # Entry point, handles CLI args, native/WASM paths
├── lib.rs           # Crate root, exports public modules
├── app.rs           # Main App struct implementing eframe::App
├── args.rs          # CLI argument definitions (clap)
├── log.rs           # Logging/tracing initialization
├── audio/           # Audio processing (buffers, analysis, thumbnails)
├── model/           # Core data (Model, Track, Config, Actions)
├── view/            # UI components and rendering
├── wav/             # WAV file reading
├── widgets/         # Reusable UI widgets
├── math/            # Math utilities
├── pos.rs           # Position/coordinate types
├── rect.rs          # Rectangle types
├── sample.rs        # Sample data types
├── util.rs          # General utilities
├── generator/       # Demo/test waveform generation
└── bin/             # Binary targets (if any)
tests/               # Integration tests (model tests in tests/model/)
```

## WASM Considerations

- WASM code goes in `#[cfg(target_arch = "wasm32")]` blocks
- Use `web-sys` and `wasm-bindgen-futures` for DOM access
- Set up `console_error_panic_hook` in WASM entry point
- Release profile optimizes for small WASM: `opt-level = 2`

## Typos

The project uses `typos-cli` for spell checking. Config is in `.typos.toml`. Run with:
```bash
typos
```

## Common Patterns

### Adding a new module:
1. Create `src/new_module.rs` or `src/new_module/mod.rs`
2. Add `pub mod new_module;` to `src/lib.rs`
3. Export public types with `pub use`

### Adding a new test:
1. Add test in the appropriate module with `#[cfg(test)] mod tests { ... }`
2. Or add integration tests in `tests/` directory
3. Run with `cargo test <test_name>`

### Adding a dependency:
1. Edit `Cargo.toml` under `[dependencies]` or `[dev-dependencies]`
2. Run `cargo check` to verify
3. Import in code using `use crate::` or external crate name
