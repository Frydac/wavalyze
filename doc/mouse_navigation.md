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
| Double-click strip | Fit full visible duration |

## Amplitude and dB rulers

Both vertical rulers use same navigation controls.

| Input | Action |
|---|---|
| Drag vertically | Pan sample value for track |
| Shift + scroll | Pan sample value for track |
| Ctrl + scroll | Zoom sample value around pointer |

Plain scrolling does not navigate.

## Other mouse zoom controls

- **zoom full x** toolbar button fits full visible duration.
- **Zoom to selection** button beside selection fields fits selected sample range.

## Direction and sensitivity

Settings under **Navigation (scroll wheel)** configure sensitivity and inversion independently for Pan X, Pan Y, Zoom X, and Zoom Y. These settings affect scrolling only; drag gestures remain direct 1:1 movement.

Trackpads may report Shift + wheel gestures as horizontal scroll. Value-ruler scrolling accepts vertical or horizontal scroll input; waveform and time-ruler X panning requires horizontal scroll input.
