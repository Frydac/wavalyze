# Wavalyze

[![CI](https://github.com/frydac/wavalyze/actions/workflows/rust.yml/badge.svg)](https://github.com/frydac/wavalyze/actions/workflows/rust.yml)
[![Pages](https://github.com/frydac/wavalyze/actions/workflows/pages.yml/badge.svg)](https://github.com/frydac/wavalyze/actions/workflows/pages.yml)

<p align="center">
  <img src="doc/images/wavalyze.svg" alt="Wavalyze header graphic" width="50%">
</p>

**Wavalyze** is a WAV file viewer with a long-term goal of becoming a full analysis and diff tool for audio software development.

It is a personal Rust learning project, focused on visual inspection of waveforms and low-level audio data, built on `egui`.

![Wavalyze Screenshot](doc/images/wavalyze_001_small.png)


## Demo

Web demo: [frydac.github.io/wavalyze](https://frydac.github.io/wavalyze/)

## Cloudflare Pages

Cloudflare Pages is a better fit than GitHub Pages if you want to move the WASM app toward web multithreading.

The reason is not Cloudflare Pages by itself, but the response headers it lets you control. WebAssembly threads require the page to be cross-origin isolated, which in practice means sending:

- `Cross-Origin-Opener-Policy: same-origin`
- `Cross-Origin-Embedder-Policy: require-corp`

This repo now includes a Cloudflare Pages [`_headers`](_headers) file and copies it into the `trunk` output, so a Cloudflare Pages deploy can serve those headers.

### Cloudflare Pages setup

This repository deploys to Cloudflare Pages from GitHub Actions. Cloudflare should host the built static files, not run the Rust build itself.

Create a Cloudflare Pages project named `wavalyze` using Direct Upload mode. The GitHub workflow will push the built `dist/` output with Wrangler.

Add these GitHub repository secrets before enabling the workflow:

- `CLOUDFLARE_ACCOUNT_ID`
- `CLOUDFLARE_API_TOKEN`

The API token should have:

- `Account`
- `Cloudflare Pages`
- `Edit`

The workflow in [`cloudflare-pages.yml`](.github/workflows/cloudflare-pages.yml) behaves as follows:

- Push to `main`: build with `trunk` and deploy production to `wavalyze.pages.dev`
- Pull request from a branch in this repository: build and deploy a Cloudflare preview
- Pull request from a fork: build only, no deployment, because secrets are unavailable

After the first deploy, verify in the browser console that:

- `window.crossOriginIsolated === true`

and confirm the deployed responses include the two headers above.

During cutover, the existing GitHub Pages workflow can stay in place. Once Cloudflare is verified, disable [`pages.yml`](.github/workflows/pages.yml) and update the demo URL below.

### Important limitation

This only solves the hosting prerequisite.

The current WASM app is still coded as single-threaded in several places, for example [`src/model/action.rs`](src/model/action.rs) and [`src/wav/read.rs`](src/wav/read.rs), where the web build explicitly avoids worker-thread behavior today.

So the migration path is:

1. Deploy on Cloudflare Pages with the new headers.
2. Confirm cross-origin isolation works in production.
3. Then change the Rust/WASM build and app code to actually use web workers / wasm threads.

## CLI

See [CLI arguments](doc/cli_args.md).

## Features (current)

- Multi-track waveform display
- Zoom and pan on the time axis
- Hover inspection of sample data

## Local dev

Native:
```
cargo run --release
```

Web (WASM):
```
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
trunk serve
```

Open `http://127.0.0.1:8080/index.html#dev` to bypass the service worker cache during development.

## Updating egui

As of 2023, egui is in active development with frequent releases with breaking changes. [eframe_template](https://github.com/emilk/eframe_template/) will be updated in lock-step to always use the latest version of egui.

When updating `egui` and `eframe` it is recommended you do so one version at the time, and read about the changes in [the egui changelog](https://github.com/emilk/egui/blob/master/CHANGELOG.md) and [eframe changelog](https://github.com/emilk/egui/blob/master/crates/eframe/CHANGELOG.md).
