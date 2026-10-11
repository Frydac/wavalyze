# Channel definition snapshots

These files are local snapshots of the workplace channel definitions, copied
without changing their JSON schema or contents. They will be the inputs to
wavalyze's explicit Rust generation workflow. Channel IDs are generated from
`id.json`, and layouts and synonym constants from `layout.json`. Cargo and the
application use committed Rust rather than reading these JSON files at build time
or runtime.

## Source

`import.json` records the import date, source directory, SHA-256 checksums, and
entry counts. The initial snapshots came from
`/home/emile/repos/root-all/core/audio/data/channel` on 2026-10-10 and contain
31 channel IDs and 75 layout definitions. Channel entries
provide `name`, explicit numeric `value`, and `short`. Layout entries provide
`name`, `channels`, `synonyms`, and `alternatives`. Preserve all fields on import,
even when Rust generation does not use them.

## IDs, internal layout masks, and WAVEX masks

A channel ID is an identity with an explicit numeric value. For example,
`LeftCenter` has ID 18 and short name `LC`. Its position in a JSON array does not
override that explicit value.

An internal layout mask sets bit `value` for each channel ID present in a layout.
Thus `LeftCenter` occupies internal bit 18 (`0x40000`). These bits describe
membership; they do not establish WAV interleaving order.

A WAVEX mask uses separate speaker-position assignments. `LeftCenter` uses WAVEX
bit 6 (`0x40`), corresponding to front-left-of-center. WAV channels are interleaved
in increasing order of set WAVEX bits, not increasing internal ID values or the
order of a layout's JSON `channels` array.

For another example, the internal `5.1` mask is `0x3F`, while its side-surround
WAVEX mask is `0x60F`. Never cast an internal layout mask to a WAVEX mask; convert
through an independently maintained WAVEX speaker mapping.

The reader follows [Auro-3D in multi-channel WAV, Revision 2, 19 April 2024](https://www.auro-3d.com/wp-content/uploads/2024/04/Auro-3D-in-multi-channel-WAV-White-Paper-20240419.pdf),
pages 1–2. It recognizes the 18 standard WAVEX bits plus six published Auro
extension bits. These extensions are an Auro convention, not Microsoft-standard
positions or FFmpeg's numeric assignments.

TopBackLeft (`0x008000`) and TopBackRight (`0x020000`) depend on the complete
mask. For 22.2 (`0xFFFFFF`, 24 channels) and 22.0 (`0xFBFFF7`, 22 channels),
they map to HeightLeftBack and HeightRightBack. Otherwise they map to
HeightLeftSurround and HeightRightSurround.

| Extension bit | Auro identity |
|---|---|
| `0x040000` | LFE2 |
| `0x080000` | HeightLeftSurround |
| `0x100000` | HeightRightSurround |
| `0x200000` | BottomLeft |
| `0x400000` | BottomCenter |
| `0x800000` | BottomRight |

The resulting 22.2 mask is `0xFFFFFF`, with this interleaving order:

```text
L R C LFE Lb Rb LC RC Cs Ls Rs T HL HC HR HLb HCs HRb LFE2 HLs HRs BL BC BR
```

The PDF labels the bottom channels BtL/BtC/BtR; the imported IDs use BL/BC/BR.
22.0 is a project derivation, not a row supplied by the paper: omit LFE (bit 3)
and LFE2 (bit 18), giving mask `0xFBFFF7`. Both layouts resolve to their internal
catalog definitions, independently of the WAVEX bit numbering.

For sparse masks, project policy is to use ordinary context rather than infer
22.x from extension bits. Top-side extensions can represent height surrounds,
but two file channels must not resolve to the same identity. For example,
`0x088000` combines TopBackLeft and TopSideLeft, both HeightLeftSurround in
ordinary context, so speaker metadata is unassigned. Unknown bits (24..31), zero
masks, and channel-count mismatches also retain their raw mask and decoded samples
but leave layout and all channel IDs unassigned. An absent mask gives no metadata.

The original workplace mapping agrees with the published convention. Earlier
wavalyze versions followed the provisional March 17, 2023 memo instead, with
different extension assignments and 22.2 order despite the same `0xFFFFFF` mask.
Regenerate development fixtures with `rake test_data:wavex`. The reader does not
infer memo-era provenance or migrate files automatically; conversion of real
files requires known provenance, sample reordering, and any necessary mask update.

## Import and regeneration workflow

These tasks import, generate, and check channel IDs and layouts:

```sh
# Validate both source JSON files, preserve their exact bytes, and regenerate Rust.
rake 'channels:import[/home/emile/repos/root-all/core/audio/data/channel]'

# Regenerate channel IDs and layouts from the local definitions.
rake channels:generate

# Check generated channel IDs and layouts without modifying committed files.
rake channels:check

# Run the Ruby generator tests (requires minitest).
ruby scripts/test_generate_channels.rb
```

The generator uses Ruby's JSON and Open3 libraries and requires rustfmt. It keeps
explicit ID values, sorts variants by value for stable output and iteration, and
rejects duplicate names/values, invalid Rust variant names, missing short names,
and values outside 0..63. Layout constants use a `LAYOUT_` prefix with `.`, `+`,
and `-` replaced by `_`, then uppercase (for example, `Auro9.1` becomes
`LAYOUT_AURO9_1`). Individual channel flags use uppercase snake case. Layout
membership is sorted by explicit ID value; synonyms reference the canonical
layout constant. Distinct names may share a mask. `alternatives` are retained in
the snapshot but do not create constants or name lookup.

`Layout::names()` provides the reverse direction for display: an exact internal
mask match returns its catalog names and synonyms (for example, `5.1_4H` and
`Auro9.1`). Equal-mask definitions share one list of names; unmatched masks return
an empty slice. This does not add parsing of input names or alternatives. The
Files panel displays these names before the existing channel-list metadata.

Generation rejects unknown/repeated channel references and constant-name
collisions across IDs, layouts, synonyms, and supplemental masks. Both generated
files are validated and formatted before either is written. Checking uses the
same pipeline without writing. Rust tests live separately in `id_tests.rs` and
`layout_tests.rs` so regeneration does not overwrite them.

## Supplemental masks

`supplemental.json` is locally maintained, not copied from the workplace. It
preserves existing group masks (such as `LOWER_FRONTS` and `HEIGHT_LAYER`) and
the three existing extra Auro `2T` constants. Each entry names a Rust constant
and a list of constant-name `members` whose masks are combined with bitwise OR.
Members can reference generated channel flags, layouts, synonyms, or other
supplemental masks. Cycles, unknown members, duplicate members, and overrides of
imported/generated constants are rejected. `NONE` is generated automatically.

The first generated layouts match the prior handwritten masks except for
`LAYOUT_NHK_22_0`: the JSON excludes both LFE channels, whereas the handwritten
definition excluded only `LFE`. Generation corrects this to 22 channels, with
internal mask `0xEFFFF7`. This is an internal mask, distinct from the derived WAVEX mask
`0xFBFFF7`. The catalog also adds 12 previously missing constants, including
`LAYOUT_AURO8_1` and `LAYOUT_9_1_5H_2T`.

## Import guarantees and review

The source directory is an explicit argument, not a dependency on the local
workplace checkout. Import reads both source files once and validates those exact
bytes against each other and the local `supplemental.json`. It formats both Rust
outputs before writing any snapshot or output. Missing files, invalid definitions,
and formatting failures leave local files untouched. Writes then replace changed
files individually; this is not a filesystem transaction covering the entire batch.

Only `id.json` and `layout.json` are imported. External supplemental files are
ignored; the local supplements remain unchanged. Unknown JSON fields are retained,
and alternative-name arrays are validated even though they do not generate Rust.

Import updates `import.json` when snapshot bytes change or provenance is missing.
Unchanged snapshots retain their prior provenance, even if an identical copy is
imported from another directory. Unchanged files are not rewritten; an identical
import can still repair stale Rust. Review the JSON and generated Rust diffs
together after importing, then run the generator tests, `rake channels:check`,
and the relevant Cargo tests. No Git staging or commits are performed by the task.

CI runs the Ruby generator tests and `rake channels:check`. Source packages include
the JSON definitions, provenance, documentation, generator, tests, and Rakefile.
Ordinary Cargo builds use committed Rust and do not require Ruby or workplace files.

Generation preserves explicit ID values and exact short names, emits layouts
and synonym constants, and retains locally maintained supplementary masks.
Alternative-name lookup is deferred. The WAVEX mapping will remain a separate
local definition following the selected Auro PDF convention.

For future imports, a changed short name, ID value, or layout membership is a
behavior change to review, not something to normalize away during copying.
