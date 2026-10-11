# Mouse navigation

Wavalyze separates horizontal navigation (time/X) from vertical navigation (sample value/Y). Vertical actions affect only the track under the pointer; horizontal actions affect all tracks.

## Waveform

| Input | Action |
|---|---|
| Right-drag | Pan time (X) |
| Shift + right-drag | Pan sample value (Y) for hovered track |
| Ctrl + right-drag | Pan time and sample value together |
| Shift + horizontal scroll | Pan time |
| Ctrl + scroll | Zoom time around pointer |
| Alt + Shift + scroll | Pan sample value for hovered track |
| Alt + Ctrl + scroll | Zoom sample value around pointer |
| Ctrl + left-drag | Rectangle zoom in time and sample value |

Plain scrolling does not navigate.

### Rectangle zoom

Start inside waveform with Ctrl + left-button press, then drag. Rectangle is clipped to waveform bounds. Release left button to zoom both axes to rectangle; vertical zoom affects only dragged track.

- Ctrl may be released after drag starts.
- Right-click cancels gesture. Gesture stays canceled until left button is released.
- Selection and right-drag panning are blocked while gesture owns pointer, including after cancellation.
- Click without dragging, or rectangle with zero width or height, does nothing.

## Files sidebar

Drag an open file header onto another to choose channel pairs for a diff.
The dragged file is A and the target is B (A minus B). Dropping one new WAV onto
an open file loads it and opens the same pairing dialog. On confirmation, selected
source tracks and diffs are grouped together, and unpaired source tracks are
removed from the view. See [channel orders and active layouts](channel_orders.md)
for loaded-channel behavior and restoring removed tracks.

## Time ruler

| Input | Action |
|---|---|
| Drag horizontally | Pan time |
| Shift + horizontal scroll | Pan time |
| Ctrl + scroll | Zoom time around pointer |

Ctrl + Shift scrolling has no action on time ruler.

## Overview strip

Overview strip sits above time ruler. Highlighted viewport represents visible time range.

| Input | Action |
|---|---|
| Drag viewport center | Pan time |
| Drag left or right viewport edge | Zoom time by resizing visible range |
| Hold Shift while dragging | Make finer pan or edge-resize adjustments |
| Double-click strip | Fit full visible duration |

Fine adjustment uses the visible time span captured at the start of the drag.
Holding Shift limits sensitivity to one tenth of that span across the overview
width (or less when already zoomed out beyond the overview). Pressing or releasing
Shift during a drag changes sensitivity without moving the viewport suddenly.

## Amplitude and dB rulers

Both vertical rulers use same navigation controls.

The dB ruler fills available space with intermediate labels as you zoom or change
the display skew. Familiar integer values take priority; narrow ranges also show
fractional dB values, with up to six decimal places. Labels stay inside the ruler
and leave room for hover readouts. Positive and negative sample values show their
magnitude in dB, so the labels are mirrored around zero.

| Input | Action |
|---|---|
| Drag vertically | Pan sample value for track |
| Shift + scroll | Pan sample value for track |
| Ctrl + scroll | Zoom sample value around pointer |

Plain scrolling does not navigate.

## Value overview strip

Each track has a narrow vertical overview immediately left of its enabled
amplitude/dB rulers. The strip represents +1 at the top, zero in the middle, and
−1 at the bottom. Its thumb shows the track's visible value range; arrows indicate
limits extending beyond full scale. The reference stays linear in sample values,
including when the waveform uses skewed display scaling.

| Input | Action |
|---|---|
| Drag thumb center | Pan this track's value range, preserving its span |
| Drag top or bottom edge | Change that limit, keeping the opposite limit fixed |
| Wheel over strip | Zoom around the current range's center |
| Hold Shift while dragging or scrolling | Make finer adjustments |
| Double-click strip | Reset this track to −1…+1 |
| Hover | Show current limits, zoom factor, and controls |

At high zoom, the visible thumb stays at least 2 px tall. Invisible grab areas
remain larger, with a center area for panning and separate edge resize areas.
The cursor indicates which action will start. Very short tracks reduce these
areas to fit, leaving the track-height resize handle clear.

When the actual range occupies less than 18 px, edge drags adjust the starting
range by at most 2.5% per pixel. Larger ranges follow the full-scale reference.
The drag sensitivity is captured at the start and remains stable throughout the
gesture; Shift can be pressed or released without a jump. Resizing cannot cross
the opposite limit. Wheel zoom uses the **Zoom Y** sensitivity and inversion
settings, and does not also scroll the track list.

## Other mouse zoom controls

- **zoom full x** toolbar button fits full visible duration.
- **Zoom to selection** button beside selection fields fits selected sample range.

## Direction and sensitivity

Settings under **Navigation (scroll wheel)** configure sensitivity and inversion independently for Pan X, Pan Y, Zoom X, and Zoom Y. These settings affect scrolling only. Overview drags use the reference and fine-adjustment behavior described above; other drag gestures remain direct 1:1 movement.

Trackpads may report Shift + wheel gestures as horizontal scroll. Value-ruler scrolling accepts vertical or horizontal scroll input; waveform and time-ruler X panning requires horizontal scroll input.
