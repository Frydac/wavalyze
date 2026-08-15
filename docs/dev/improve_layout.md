# Improve track layout

## Move the track UI to explicit rectangles

The track view currently mixes two layout models:

- `Model` owns a fixed track height, which the parent reserves with `allocate_exact_size`.
- The children are arranged with egui's `horizontal`, `vertical`, and `allocate_ui*` layouts, which
  are content-driven and may report a size larger than requested.

This can produce contradictory geometry. For example, a vertical `ScrollArea` has a default
`min_scrolled_height` of 64 points. If it is placed in a 25-point stats rectangle using an
allocating child UI, it can enlarge the horizontal row. The waveform then lays itself out using
that enlarged row while the next track still starts at the fixed height reserved by the parent.
The result is overlapping tracks.

The stats panel now avoids that particular issue by using a non-allocating, clipped child UI and a
scroll area that accepts a zero-point minimum viewport height. The complete track is also clipped
to its reserved rectangle as a safety net. These are local protections, not a complete geometry
model.

A future cleanup should make the entire track rectangle-driven:

1. Reserve the model-owned `track_rect` exactly once.
2. Derive non-overlapping rectangles from it for the sidebar and waveform area.
3. Split those rectangles into the sidebar header, stats viewport, ruler columns, waveform header,
   waveform canvas, and resize handle.
4. Pass the resulting rectangles to the rendering functions. Use non-allocating child UIs or
   direct painting/interactions inside those rectangles.
5. Intersect each component's clip rectangle with its assigned rectangle.
6. Keep intrinsic widget sizes local: widgets may scroll, clip, truncate, or simplify their
   presentation, but must never change the track's outer geometry.

This would establish one source of truth for track geometry and remove the current dependency on
sibling layout order. It should be done as a focused refactor because waveform hover, selection,
value-ruler interaction, resize handling, and the outer track-list scroll area all rely on the
current screen rectangles.
