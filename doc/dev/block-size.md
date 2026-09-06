# Block size

## Scope

Add one global processing block size. UI edits this value beside Selection controls; ruler hover and selection-edge labels show block coordinates. Selection start, length, and inclusive end are editable as absolute samples or block index plus offset. Per-file and per-track block sizes are deferred.

File/track sample offset acts as processing latency for block-coordinate display. This does not change existing waveform placement semantics.

## Coordinate semantics

Existing track placement maps source samples onto the time ruler by subtracting effective track offset:

```text
ruler_sample = source_sample - effective_track_offset
```

The effective track offset is also processing latency, so processing-local index is the ruler sample already. It must not be subtracted again:

```text
processing_local_index = ruler_sample
block_index = processing_local_index.div_euclid(block_size)
in_block_offset = processing_local_index.rem_euclid(block_size)
```

Block indices are zero-based. Euclidean division keeps `in_block_offset` in `0..block_size`, including before ruler origin.

Example: source sample `512` with effective track offset `512` appears at ruler sample `0`. With block size `1024`, it produces block index `0` and in-block offset `0`.

## Selection behavior

Selection start and end are non-negative global ruler samples; displayed end remains inclusive. Their ruler labels use the hover format:

```text
s: 2,049
b: 2 + 1
```

Selection start, length, and inclusive end each expose absolute samples, zero-based block index, and block offset. Values use quotient/remainder, so length `2050` at block size `1024` is `2` blocks plus `2` samples. Block edits convert back with:

```text
samples = block_index * block_size + block_offset
```

Block indices are non-negative, offsets stay in `0..block_size`, and converted values clamp to the existing `999,999,999` sample editor maximum. Changing runtime block size only recomputes displayed block fields; it does not change selection.

## Implementation plan

1. Add persisted startup default and validated global runtime state.
2. Add runtime editor beside Selection and startup-default editor in Settings.
3. Add hovered track context and block coordinates to time-ruler hover label.
4. Run native and WASM validation; record results here.
5. Share ruler block formatting and add block coordinates to selection-edge labels.
6. Add editable selection block index and offset fields.
7. Add focused selection tests and repeat native/WASM validation.

## Decisions

- Default block size: `1024` samples.
- Block size must be positive; runtime state clamps zero to `1`.
- Startup default and current runtime value remain separate.
- No new block-size type or dependency until stronger invariants require one.

## Progress

- [x] Step 1: documented semantics; added persisted `default_block_size`, validated runtime `Model::block_size`, `SetBlockSize`, and unit tests.
  - Validation: `cargo fmt --check` passed.
  - Validation: `cargo test --workspace --all-targets` passed (252 library, 13 audio integration, 13 model integration, and 1 file integration tests).
- [x] Step 2: added global runtime editor beside Selection and separate startup-default editor in Settings.
  - Runtime edits dispatch `SetBlockSize`; startup-default edits only update persisted config.
  - Validation: `cargo fmt --check`, `cargo check --workspace --all-targets`, and `cargo test --workspace --all-targets` passed.
- [x] Step 3: added source track IDs to hover state and block coordinates to ruler labels.
  - Block coordinates use aligned ruler samples because track placement already applies effective offset.
  - Fixed initial double-offset bug; Euclidean division covers samples before ruler origin.
  - Validation: 3 focused coordinate tests and `cargo test --workspace --all-targets` passed.
- [x] Step 4: completed cross-target validation and final notes.
  - Native: `cargo fmt --check` and `cargo test --workspace --all-targets` passed (255 library tests plus integration tests).
  - WASM: pinned nightly `cargo check --workspace --all-features --lib --target wasm32-unknown-unknown` passed; rustc emitted the existing unstable `atomics` target-feature warning.
- [x] Step 5: shared block-coordinate formatting between hover and selection-edge ruler labels.
  - Selection start/end labels preserve sample/block text through paired, overlapping, and fallback placement.
- [x] Step 6: added selection sample, block index, and block offset editors for start, length, and inclusive end.
  - Block edits preserve existing start-edit modes and dispatch `SetSelection`; runtime block-size changes only refresh derived fields.
- [x] Step 7: added focused round-trip, boundary, length, clamping, and ruler-label tests; repeated native and WASM validation.
  - Targeted: 4 selection-editor conversion tests and 7 ruler tests passed.
  - Native: `cargo fmt --all -- --check` and `cargo test --workspace --all-targets` passed (260 library, 13 audio integration, 13 model integration, and 1 file integration tests).
  - WASM: pinned nightly `cargo check --workspace --all-features --lib --target wasm32-unknown-unknown` passed; rustc emitted the existing unstable `atomics` target-feature warning.
  - Deferred: move global block size to per-file/per-track state when files need independent processing layouts.
