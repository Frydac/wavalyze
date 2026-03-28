#!/usr/bin/env bash
set -euo pipefail

RUSTUP_TOOLCHAIN=nightly-2026-01-15 exec trunk "$@"
