# Adaptive dB ruler labels

Tick generation retains the existing coarse cadence and supplies additional label
candidates. Placement gives coarse anchors priority, then fills gaps with integer
and fractional values. Each candidate is measured once per frame; accepted labels
reuse that text layout for painting. Labels must fit inside the ruler and leave
four points between their rectangles, including space reserved for hover readouts.

Fractional candidates come from screen-space bands, so zoom and skew determine
their local density. Steps follow a 1/2/5 sequence down to 0.000001 dB. There are
at most 64 bands and seven fractional candidates per sign per band. Combined with
integer candidates, the hard limit is 1,210 candidates per ruler. Placement stops
at the finest generated step. Tests enforce these bounds and label separation.

## CPU measurement

Run the manual measurement with:

```sh
cargo test --release --lib measure_many_db_rulers -- --ignored --nocapture
```

It exercises 24 rulers at 350 points high, with full-scale, asymmetric, quiet,
positive-only and narrow negative-only ranges. Zero-crossing ranges also exercise
skew. It measures 200 frames after 20 warm-up frames, using cached egui fonts,
first with fixed ranges and then continuously varying zoom.

Measured locally on 2026-10-11 (milliseconds for all 24 rulers):

| Ranges | Generation median / p95 | Placement median / p95 | Maximum candidates per ruler |
|---|---|---|---|
| Fixed | 0.296 / 0.351 | 1.491 / 1.713 | 282 |
| Changing zoom | 0.296 / 0.341 | 1.670 / 1.900 | 288 |

These measure tick generation and label placement/painting commands, excluding
waveform rendering, egui tessellation and GPU work. They are diagnostic results,
not timing assertions or a guarantee on other machines. No additional cache is
introduced based on these measurements.

## Display coordinates and skew

Viewport limits are display coordinates. Both lattices convert them back to
sample values before filtering candidates or choosing amplitude tick cadence.
Fractional dB bands also use the inverse mapping without clipping their sample
values against display limits. This prevents visible ticks from disappearing
when zero leaves the viewport and skew is enabled.

The shared mapping transforms each unit interval independently, preserving the
existing integer anchors and positive/negative symmetry. Forward and inverse
mapping use f64 arithmetic until the final screen coordinate, avoiding precision
loss from interpolating between distant f32 screen anchors at extreme zoom.
Regression tests cover both signs, skew up to five, narrow ranges, readable label
placement and subpixel screen round trips.
