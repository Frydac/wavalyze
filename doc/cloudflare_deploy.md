# Cloudflare Deploy Notes

This document records the setup that ended up working for deploying `wavalyze` to Cloudflare.

## Final setup

- Hosting target: `https://wavalyze.emile-vrijdags-github.workers.dev/`
- Build system: GitHub Actions
- Build command: `env -u NO_COLOR ./scripts/trunk-threaded.sh build --release`
- Deploy tool: Wrangler v4 via GitHub Actions
- Deploy target: Cloudflare Worker with static assets

Relevant files:

- [`web/_headers`](../web/_headers)
- [`index.html`](../index.html)
- [`wrangler.jsonc`](../wrangler.jsonc)
- [`.github/workflows/cloudflare-pages.yml`](../.github/workflows/cloudflare-pages.yml)

## Why this setup

The original goal was to move off GitHub Pages so the site could serve the headers needed for cross-origin isolation:

- `Cross-Origin-Opener-Policy: same-origin`
- `Cross-Origin-Embedder-Policy: require-corp`

That is a hosting prerequisite for future multithreaded wasm work.

Cloudflare-managed Git builds did not work for this Rust app because the Cloudflare build environment did not provide the Rust toolchain setup needed for this repository. The working approach was:

1. Build in GitHub Actions
2. Deploy the prebuilt static output with Wrangler
3. Serve the site from Cloudflare

## Files and config

### `web/_headers`

This file defines the COOP/COEP headers:

```text
/*
  Cross-Origin-Opener-Policy: same-origin
  Cross-Origin-Embedder-Policy: require-corp
```

### `index.html`

This file copies `web/_headers` into the Trunk output so it ends up in `dist/`:

```html
<link data-trunk rel="copy-file" href="web/_headers" />
```

### `wrangler.jsonc`

This file tells Wrangler to deploy the built `dist/` directory as static assets for the `wavalyze` Worker:

```jsonc
{
  "name": "wavalyze",
  "compatibility_date": "2026-03-28",
  "assets": {
    "directory": "./dist",
    "html_handling": "auto-trailing-slash",
    "not_found_handling": "none"
  }
}
```

### GitHub Actions workflow

The workflow:

- installs Rust with the `wasm32-unknown-unknown` target
- downloads `trunk`
- builds the app
- runs `wrangler deploy`

Production deploys happen on pushes to `main`. Pull requests only run the build.

## Cloudflare setup steps

### 1. Create the Cloudflare-hosted app

The working setup was created from the Cloudflare `Workers & Pages` area using the static-files upload flow, which resulted in a Worker-hosted static-assets app on:

- `wavalyze.emile-vrijdags-github.workers.dev`

### 2. Get the account ID

In Cloudflare, open `Workers & Pages` and copy the `Account ID` shown in the `Account Details` panel.

### 3. Create the API token

Create a custom token in Cloudflare and use at least:

- `Account -> Workers Scripts -> Edit`

Optional but useful:

- `User -> User Details -> Read`

The earlier `Account -> Cloudflare Pages -> Edit` permission was not enough for the final Worker-based deploy flow.

### 4. Add GitHub Actions secrets

In the GitHub repo settings, add:

- `CLOUDFLARE_ACCOUNT_ID`
- `CLOUDFLARE_API_TOKEN`

### 5. Push to `main`

The GitHub Actions workflow will build and deploy automatically.

## Issues encountered

### Cloudflare Git build failed

Cloudflare's own build step failed because the environment did not have the Rust setup expected by this repo.

Observed symptom:

- `rustup: not found`

Resolution:

- stop using Cloudflare-managed builds
- build in GitHub Actions instead

### Wrong Cloudflare product target

The first workflow targeted a Cloudflare Pages project, but the created deployment target was actually a Worker-hosted static-assets app on `workers.dev`.

Observed symptom:

- `Project not found`

Resolution:

- switch from `wrangler pages deploy ...` to `wrangler deploy`
- add `wrangler.jsonc`

### Wrangler too old

The first Worker deploy attempt used Wrangler v3, which did not handle the assets-only Worker config as needed.

Observed symptom:

- `Missing entry-point`

Resolution:

- force `wranglerVersion: "4.78.0"` in the GitHub Action

### Cloudflare auth failure

The first token only had Pages permission.

Observed symptom:

- `Authentication error [code: 10000]`

Resolution:

- create/update the token with `Workers Scripts -> Edit`

## Verification

### Check response headers

Use curl:

```bash
curl -I https://wavalyze.emile-vrijdags-github.workers.dev/
```

Expected headers:

- `Cross-Origin-Opener-Policy: same-origin`
- `Cross-Origin-Embedder-Policy: require-corp`

### Check browser isolation

In browser devtools console:

```js
window.crossOriginIsolated
```

Expected result:

```js
true
```

### Check build output locally

```bash
env -u NO_COLOR ./scripts/trunk-threaded.sh build --release
find dist -maxdepth 1 -type f | sort
```

Expected relevant output includes:

- `dist/_headers`
- `dist/index.html`
- `dist/wavalyze.js`
- `dist/wavalyze_bg.wasm`

## Notes

- The existing GitHub Pages workflow was intentionally left in place during cutover.
- This deployment change only solves the hosting prerequisite for threaded wasm. It does not enable wasm multithreading in the app by itself.
