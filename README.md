# Wavalyze

[![CI](https://github.com/frydac/wavalyze/actions/workflows/rust.yml/badge.svg)](https://github.com/frydac/wavalyze/actions/workflows/rust.yml)
[![Cloudflare Deploy](https://github.com/frydac/wavalyze/actions/workflows/cloudflare-pages.yml/badge.svg)](https://github.com/frydac/wavalyze/actions/workflows/cloudflare-pages.yml)

**Wavalyze** is a WAV file viewer with the goal of becoming a analysis and diff tool for (my) audio software development.

It is a personal Rust learning project, and curious how well wasm works in a usecase like this.

![Wavalyze Screenshot](doc/images/wavalyze_001_small.png)


## Demo

Web demo: [wavalyze.emile-vrijdags-github.workers.dev](https://wavalyze.emile-vrijdags-github.workers.dev/)

## Docs

- [CLI arguments](doc/cli_args.md)
- [Cloudflare deploy notes](doc/cloudflare_deploy.md)

## CLI

See [CLI arguments](doc/cli_args.md).

## Features (current)

- Multi-file waveform viewing
- Zoom, pan, and region inspection
- Sample-level hover details
- Native and browser builds

## Local dev

Native:
```
cargo run --release
```

Web (WASM):
```
rustup toolchain install nightly-2026-01-15 --target wasm32-unknown-unknown --component rust-src
cargo +1.93.0 install --locked trunk
./trunk-threaded.sh serve
```

Open `http://127.0.0.1:8080/index.html#dev` to bypass the service worker cache during development.

## CI

The main workflows are:

- [`rust.yml`](.github/workflows/rust.yml): native checks, tests, linting, cross-target builds, and threaded wasm validation
- [`cloudflare-pages.yml`](.github/workflows/cloudflare-pages.yml): production web build and deploy to Cloudflare Workers static assets

For a local CI-like run, use:

```bash
./check.sh
```
