#!/usr/bin/env bash
set -euo pipefail

if [[ -x "./trunk" ]]; then
    RUSTUP_TOOLCHAIN=nightly-2026-01-15 exec ./trunk "$@"
else
    RUSTUP_TOOLCHAIN=nightly-2026-01-15 exec trunk "$@"
fi
