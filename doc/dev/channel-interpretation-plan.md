# Channel interpretation presets and active layouts

Status: steps 1–7 complete. This feature concerns wavalyze only.

## Verified workplace behavior

The QC `sine_auro_1.0.wav` example has a PCM WAV header (format tag 1), three
channels, 24-bit samples, 48 kHz, and no WAVEX channel mask. Across its 10,240
frames, indices 0 and 1 are exactly zero; index 2 contains the center signal.

The signal generator allocates matrix rows through the highest active channel
ID (`highest ID + 1`), leaving the intervening rows silent. Without a supplied
output layout, BB8's WAV writer maps every input matrix row to the same WAV
index and writes the matrix's row count. This is the Auro ID-indexed convention.

Relevant workplace sources:

- `cli/tools/cli-bb8/common/cpp14/protected/app/processor/signal/Oscillator.cpp`:
  the channel-frequency vector is resized to `chid + 1`, then determines the
  output matrix row count.
- `cli/tools/cli-bb8/common/cpp14/protected/app/processor/wav/Write.cpp`:
  without an output layout, `wavix__matix[chid] = chid` for every input row.
- `core/audio/ruby/audio/qc/wav_wavex_layout.rb`: the ordinary Auro reference
  writer is invoked without an output layout; the WAVEX writer receives one.

The writer also supports compact output when an explicit layout is supplied
without WAVEX: it packs the layout's channels into consecutive WAV indices.
That is a different convention and is outside the initial preset set. Its
existence is another reason to expose the interpretation rather than treat
ordinary WAV channel counts as proof of an order.

## Channel order presets

Use the same interpretation model in the Files sidebar and diff pairing.
Initial choices:

| Preset | Roles assigned to WAV indices |
|---|---|
| Automatic | A valid WAVEX mask takes precedence. An absent or zero mask falls back to Auro ID order. An invalid nonzero mask leaves roles unassigned. |
| Auro ID order | Index equals the explicit numeric ID in the imported channel catalog, regardless of header mask. Unknown numeric IDs remain unassigned. |
| Unassigned | No roles are assigned. Indices remain visible. |

Display how Automatic resolved: `WAVEX`, `Auro ID order (assumed)`, or
`Unassigned`. Keep the raw mask and original header metadata separate from the
chosen interpretation. Switching presets must not reorder, drop, or modify
sample buffers or change offsets, channel selections, or diff pairings.

Explicit Auro ID order may override a valid or invalid mask. Switching back to
Automatic restores header-based behavior. This choice is per file; A and B in
the diff dialog have independent choices. Preserve choices across reloads and
carry dialog choices into the loaded files. Extending the set of orders later
should only require another mapping implementation and its compatibility rules.

## Active layout is separate from order

A role mapping identifies an index's potential speaker. It does not establish
which speakers the file actually uses. With Auro ID order, three stored
channels resolve to `0 L`, `1 R`, `2 C`; they do not automatically establish
layout `3.0`.

For Auro ID order, start with no active layout selected. Offer compatible named
layouts from `data/channel/layout.json` and allow the user to choose one. With
`1.0` selected, index 2 is active `C`; indices 0 and 1 are displayed as padding
(with their index and potential role still available). This marks expected
padding; it does not verify silence or remove those channels.

For valid WAVEX interpretation, the mask supplies both order and layout.
Initially, do not offer a conflicting active-layout override in that mode.
For Unassigned or invalid Automatic interpretation, show no inferred layout.
Changing the order clears an incompatible explicit layout selection.

Do not infer a layout from filenames or scan audio for silence in this feature.
A silent channel may still be an active speaker. Metadata must distinguish
header-derived layouts, user-selected layouts, and unknown active layouts.

## Candidate layout rules

Use explicit numeric IDs from the imported catalog, not JSON array positions.
Use the total header channel count, even when only a subset is loaded.

For an N-channel file in Auro ID order, the initial candidate list contains
nonempty catalog layouts whose `max(channel ID) + 1 == N`. This matches the
verified generator's minimal padded storage convention. Deduplicate identical
layout masks and present their names/aliases together. Do not select a candidate
automatically, even if only one is available.

Examples from the current catalog:

| Stored channels | Candidate layouts |
|---|---|
| 2 | 2.0 |
| 3 | 1.0, 3.0 |
| 4 | 0.1, 1.1, 2.1, 3.1 |
| 6 | 4.0, 4.1, 5.0, 5.1 |
| 9 | 7.0, 7.1 and their no-center variants |
| 24 | NHK_22.0, NHK_22.2 (existing internal catalog names) |

Call these compatible candidates, not detected layouts. Layouts that would fit
only with extra trailing padding (`max ID + 1 < N`) are excluded initially;
manual custom mappings and arbitrary trailing padding can be added later.
An empty candidate list still allows role mapping with no active layout.

## Implementation steps and verification

1. **Verify and define behavior — complete.** Inspect the example's header and
   samples, trace the writer/generator, and check candidate counts against JSON.
2. **Shared resolver — complete.** Add preset state and role/layout resolution independent
   of the UI. Test explicit ID values, valid/invalid WAVEX, absent/zero masks,
   unknown indices, candidate deduplication, padding, and partial loading.
3. **Loading and reload — complete.** Connect disk/browser loading and reload to the
   resolver. Preserve explicit choices and validate selected layouts after
   header changes. Test role changes without audio/index/offset changes.
4. **Files sidebar — complete.** Add channel-order selection, interpretation source,
   compatible active-layout selection, and padding labels. Update track titles
   from the same resolved roles. Verify the mono example and unknown layouts.
5. **Diff input presets — complete.** Peek header count and mask for both inputs, add
   independent selectors, and carry their choices through confirmation into
   loaded files. Test confirmation and reload propagation.
6. **Pairing matrix roles — complete.** Display compact headers such as `2 C` and full-role
   hover text, including padding annotations when selected. Keep checked pairs
   unchanged when interpretation changes. Verify mixed Auro/WAVEX inputs.
7. **Review and documentation — complete.** Exercise ordinary, WAVEX, mixed-order, partial
   load, and reload workflows; document assumptions and run relevant checks.

After each implementation step, stop with a short result, verification, and
an explicit statement of the next step. The diff-window path-shortening
changes remain a separate change from this feature.

Step 2 verification: eight resolver tests pass; fourteen JSON generator tests
pass (155 assertions); `rake channels:check` confirms generated output matches
the catalog; formatting and strict all-target/all-feature Clippy pass. No load
or UI behavior has been changed yet. Next: step 3, loading and reload.

Step 3 verification: all 345 library tests and the complete `./check.sh` pass,
including integration tests, strict Clippy, doctests, the WASM check and threaded
web build. Tests cover disk/upload presets, serialized upload compatibility,
partial channel selection, audio/track/offset preservation, layout invalidation
after a channel-count change, and rejecting a reload after a preset change.

Step 4 adds shared selectors, a queued file-interpretation action, interpretation
and layout provenance in metadata, and expected-padding annotations in channel
rows and track titles. Headless UI tests select mono then Unassigned, and verify
that WAVEX layouts are shown without a conflicting active-layout selector.
Sidebar/action tests cover unloaded indices, padding, metadata, and track titles.
Step 4 verification: all 348 library tests and the complete `./check.sh` pass.
Next: step 5, independent interpretation selectors for diff inputs A and B and
propagation through confirmation into loaded files.

Step 5 verification: all 352 library tests and the complete `./check.sh` pass.
The diff dialog peeks full header counts/masks even for selected-channel inputs,
validates index bounds, and keeps independent A/B selector state. Headless UI
tests change B without changing A or checked pairs. Worker integration tests
confirm all three B presets, preserve A's selected mono layout, verify the
selected source indices and actual diff samples, and retain choices on reload.
The existing long-path window-width regression still passes.
Next: step 6, compact role labels and full-role hover text in the pairing matrix.

UI adjustment after step 5: the diff dialog shows channel-order selectors in
a dedicated rightmost column, beside each input's path and offset. Active-layout
controls and layout names are omitted from this dialog; the Files sidebar keeps
its existing active-layout controls. The resolved order source is available on
hover. Existing input choices still propagate through confirmation. Next remains
step 6, compact channel-role annotations in the pairing matrix.

Step 6 verification: all 354 library tests and the complete `./check.sh` pass.
Rows show inline index/role labels; columns stack roles below indices. Hover
shows the full identity, input/index and interpretation source; unknown roles
leave the role blank. A `*` marks expected padding when a selected layout is supplied.
The matrix can scroll horizontally if its annotated columns exceed the window.
Headless tests cover mixed Auro ID/WAVEX 22.2 orders, sparse selected indices,
changing presets without changing checked cells, unknown roles, and actual
full-role/padding tooltips. Next: step 7, final review and documentation.

UI adjustment after step 6: column headers use a smaller centered font, with a
fixed checkbox-sized width and clipping inside each cell to prevent overlapping
roles. Full names remain available on hover. Unassigned rows and columns show
only the channel index. A geometry regression checks that long labels preserve
checkbox width and spacing and stay clipped to their own columns. All five diff
dialog tests pass. Next remains step 7, final review and documentation.

Checkbox hover now identifies both inputs' original WAV indices and short/full
roles, including expected padding where applicable. The corresponding row and
column labels highlight immediately, and clear when the pointer leaves the
matrix. A headless regression verifies a sparse, off-diagonal pair with different
Auro ID and WAVEX orders, tooltip content, both highlighted labels, and unchanged
pair selections. Next remains step 7, final review and documentation.

Files sidebar adjustment: active layout is a value cell inside the collapsed
Metadata grid, replacing the previous static layout row and the prominent
separate control. Auro ID order retains its compatible-layout selector; WAVEX
shows a read-only layout name. Channel order remains above Metadata. Next remains
step 7, final review and documentation.

Step 7 verification: reviewed the resolver, catalog generation, disk/upload
loading, role updates, diff confirmation, reload validation, and final UI.
Automated coverage exercises ordinary and WAVEX inputs, mixed orders, partial
loads, sample/index/offset preservation, independent selectors, invalid masks,
layout invalidation, stale reload rejection, matrix geometry, and hover behavior.
The demo now explicitly uses Auro ID order with 3.1, matching its L/R/C/LFE roles;
its previous 4.0 metadata disagreed with those channels.

Added `doc/channel_orders.md` and a README link, and updated the channel-mapping
documentation for absent/zero-mask fallback, explicit overrides, the active-layout
metadata cell, and the generated candidate catalog. Documentation separates
assumed order from a known active layout and explains expected padding, reload
behavior, and the currently unsupported compact/custom orders.

All 356 library tests, 27 integration tests, and the complete `./check.sh` pass,
including native/WASM checks, formatting, strict Clippy, doctests, and the threaded
browser build. Fourteen Ruby generator tests pass (155 assertions), and
`rake channels:check` confirms the Rust catalog matches the JSON definitions.
`git diff --check` is clean. The existing unstable WASM atomics warning remains.
No further implementation steps remain; changes are ready for commit review.

Track-title refinement: display original WAV index, short role, then path in
separate fields. Index and role use fixed widths with a monospace font; index is
right-aligned, role left-aligned, and path starts at the same position regardless
of role or index length. Expected padding uses `*`; unassigned roles leave their
field blank. Existing title hover and track selection/drag interactions remain.
A geometry regression covers different roles, padding, unknown roles, five-digit
indices, font sizes, and long paths in narrow/wide headers.

Final pre-commit verification after the title refinement: the complete
`./check.sh` passes with 357 library tests and 27 integration tests, native/WASM
checks, formatting, strict Clippy, doctests, and the threaded browser build.
