//! Cargo "build script": Cargo automatically compiles and runs this file before compiling Wavalyze.
//!
//! Lines printed with a `cargo:` prefix are instructions to Cargo. This script uses them to
//! control when it runs again and to embed build metadata as compile-time environment variables.
//! The finished native binary or WASM file contains the values; Git and `date` are not invoked
//! when the application starts.
//!

use std::process::Command;

// Run a command and return trimmed stdout. Missing commands and failures become `None`, allowing
// builds from source archives without Git metadata to continue with an "unknown" value.
fn command_output(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let output = String::from_utf8(output.stdout).ok()?;
    let output = output.trim();
    (!output.is_empty()).then(|| output.to_owned())
}

fn main() {
    // Cargo caches build-script output. These directives tell it which changes make the embedded
    // metadata stale. Environment variables let release systems provide reproducible values.
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Environment variables intended for overriding derived values, e.g. for reproducible builds.
    println!("cargo:rerun-if-env-changed=WAVALYZE_GIT_HASH");
    println!("cargo:rerun-if-env-changed=WAVALYZE_BUILD_DATE");

    // Watching HEAD handles branch switches and detached HEADs. Watching the current branch ref
    // also catches new commits, because HEAD itself does not change when a branch advances.
    //
    //  NOTE: cargo doesn't run build script each build, so you have to specify that it watches for
    //  some file changes:
    if let Some(head) = command_output("git", &["rev-parse", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={head}");
    }
    if let Some(git_ref) = command_output("git", &["symbolic-ref", "-q", "HEAD"])
        && let Some(path) = command_output("git", &["rev-parse", "--git-path", &git_ref])
    {
        println!("cargo:rerun-if-changed={path}");
    }

    // Prefer caller-supplied values, then local tools, and finally a portable fallback.
    let git_hash = std::env::var("WAVALYZE_GIT_HASH")
        .ok()
        .or_else(|| command_output("git", &["rev-parse", "--short=12", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_owned());
    let build_date = std::env::var("WAVALYZE_BUILD_DATE")
        .ok()
        .or_else(|| command_output("date", &["-u", "+%Y-%m-%d"]))
        .unwrap_or_else(|| "unknown".to_owned());

    // `cargo:rustc-env` makes these available to `env!` while Rust compiles the application.
    println!("cargo:rustc-env=WAVALYZE_GIT_HASH={git_hash}");
    println!("cargo:rustc-env=WAVALYZE_BUILD_DATE={build_date}");
}
