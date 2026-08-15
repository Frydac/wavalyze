# Track layout geometry contract

The track view uses explicit rectangles rather than allowing widget content to determine sibling
geometry. This prevents compact tracks from expanding visually into the tracks below them.

## Source of truth

The parent reserves the model-owned `track_rect` exactly once with `allocate_exact_size`.
`TrackLayout` in `src/view/track/layout.rs` is then the source of truth for all geometry inside
that rectangle. It derives rectangles for:

- the sidebar and waveform columns;
- sidebar offset controls and the reset-Y button;
- the stats viewport;
- optional dB and amplitude rulers;
- the waveform header and canvas; and
- the resize handle.

`TrackColumns` performs the same sidebar/content split for both tracks and the top
sidebar-header/time-ruler row. Given the same outer width and configured sidebar width, their
horizontal boundary must therefore be identical.

Geometry construction clamps the sidebar, header, and body to the available space. Narrow windows
and compact tracks may produce zero-width or zero-height component rectangles; they must not
produce negative rectangles or force the containing track to grow. Ruler slots are assigned from
right to left, with the amplitude ruler closest to the waveform.

## Component rendering

Every track component renders through a non-allocating child `Ui` with a stable
per-track/per-component ID salt. Its clip rectangle is the intersection of its assigned rectangle
and its parent's clip rectangle. A component may use local egui layouts internally, but those
layouts must not advance the track's parent cursor or change any sibling rectangle.

The supplied rectangle is authoritative. Rendering and interaction code must not recover geometry
from `Ui::min_rect`, `Ui::max_rect`, or `Ui::available_size`. In particular, the waveform canvas
rectangle is passed explicitly to waveform rendering, hover handling, selection handling, and
`Track::set_screen_rect`; the latter propagates it to `Single::screen_rect` for waveform view
generation and sample/value mapping.

Intrinsic widget size must remain local to the assigned rectangle. If content does not fit, the
component should scroll, clip, truncate, or simplify its presentation. It must never enlarge the
track. The stats panel demonstrates this rule: its vertical `ScrollArea` is capped to the stats
viewport and uses `min_scrolled_height(0.0)` instead of egui's 64-point default.

## Intentional overlap

The resize handle is the only intentional overlap in `TrackLayout`. It occupies a three-point band
over the bottom edge of `track_rect` without reducing the sidebar body or waveform canvas height.
This preserves the existing track-height and waveform-height semantics while giving the entire
bottom edge a resize hit target. All other component rectangles are expected to be disjoint except
for shared boundaries.

The complete track is also clipped to `track_rect` as a final safety net. That clipping is not a
substitute for correct component geometry; layout tests should continue to verify containment,
compact heights, narrow widths, ruler ordering, header/body boundaries, and top-row alignment.
