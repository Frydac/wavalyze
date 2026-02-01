## Wavalyze Agent Instructions

### Build, Lint, and Test

- **After (larger) code edits:** run `./check.sh`
- **Build:** `cargo build`
- **Build (WASM):** `trunk build`
- **Lint:** `cargo fmt --all -- --check` and `cargo clippy --quiet --workspace --all-targets --all-features --  -D warnings -W clippy::all`
- **Test:** `cargo test --quiet --workspace`
- **Run a single test:** `cargo test <test_name>` (e.g., `cargo test test_common`)

### Code Style Guidelines

- **Formatting:** Adhere to standard Rust formatting (`rustfmt`).
- **Imports:** Group imports: `std`, external crates, then project modules.
- **Types:** Use `anyhow::Result<()>` for functions that can fail in `main`.
- **Naming:** Follow Rust conventions (e.g., `snake_case` for functions/variables, `PascalCase` for structs/enums).
- **Error Handling:** Use `?` for error propagation. Use `expect` for unrecoverable errors.
- **Clippy:** Code must be clippy-clean with `-D warnings -W clippy::all`.
- **Comments:** Add comments to explain complex logic.
- **Dependencies:** Add new dependencies to `Cargo.toml`.
