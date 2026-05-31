# Wavalyze CLI Interface

Wavalyze provides a command-line interface for opening and diffing WAV files. File arguments can include optional channel, sample range, and sample offset specifications.

## Quick Start

The simplest usage is to open a file:

```bash
wavalyze song.wav
```

For more complex operations, you can use explicit subcommands:

```bash
wavalyze open song.wav:0,2:1000-5000
wavalyze diff file1.wav file2.wav:0:o=-10
```

## File Specifications

When opening or diffing files, you can attach optional specifications directly to the filename using colons as separators. Specifications can be provided in any order.

### Syntax

```
FILE[:SPEC[:SPEC...]]
```

Where `SPEC` can be:
- **Channels**: Comma-separated channel numbers (e.g., `0,2,4`)
- **Sample Range**: Start and end sample indices (e.g., `1000-5000`)
- **Sample Offset**: Signed sample offset using `offset=N` or `o=N` (e.g., `offset=-10`, `o=25`)

### Channel Selection

Specify one or more channels to open by their zero-indexed numbers separated by commas:

```bash
wavalyze song.wav:0
wavalyze song.wav:0,2,4
wavalyze song.wav:1,3
```

If no channels are specified, all channels are opened.

### Sample Range Selection

Specify a range of samples using the format `START-END`. You can also use shorthand notation to specify open-ended ranges:

```bash
wavalyze song.wav:1000-5000      # From sample 1000 to 5000
wavalyze song.wav:1000-           # From sample 1000 to the end
wavalyze song.wav:-5000           # From the start to sample 5000
```

If no range is specified, the entire file is opened.

### Sample Offset

Specify a signed sample offset with `offset=N` or the short form `o=N`:

```bash
wavalyze song.wav:0:offset=-10
wavalyze song.wav:0:o=25
```

Offsets place a buffer on the shared absolute sample timeline. A positive offset means absolute sample `n` reads local buffer sample `n + offset`.

### Combining Specifications

You can combine channel, range, and offset specifications in any order:

```bash
wavalyze song.wav:0,2:1000-5000
wavalyze song.wav:1000-5000:0,2
wavalyze song.wav:-10000:1,3      # First 10000 samples, channels 1 and 3
wavalyze song.wav:0:5000-         # Channel 0, from sample 5000 onwards
wavalyze song.wav:0:5000-:o=-32   # Channel 0, range from 5000, offset by -32 samples
wavalyze song.wav:o=12:0:100-200  # Same syntax, different order
```

## Commands

### open (default)

Opens one or more WAV files for editing or analysis.

```bash
wavalyze open file1.wav file2.wav:0,1 file3.wav:1000-5000
```

The `open` command is the default, so you can omit it:

```bash
wavalyze file1.wav file2.wav:0,1 file3.wav:1000-5000
```

### diff

Loads two source tracks and creates a third Diff track. The Diff track renders a generated diff buffer computed as:

```text
A[n + offset_a] - B[n + offset_b]
```

Out-of-range samples are treated as zero. Both inputs must have the same sample rate.

```bash
wavalyze diff original.wav processed.wav
wavalyze diff original.wav:0 processed.wav:0:o=-10
```

For each diff input, exactly one channel must be selected. If no channel is specified, the file is accepted only when it is mono. Multi-channel files must specify one channel explicitly:

```bash
wavalyze diff mono_a.wav mono_b.wav       # OK if both files are mono
wavalyze diff stereo_a.wav:0 stereo_b.wav:1
wavalyze diff stereo_a.wav stereo_b.wav   # Error if either file has multiple channels
```

The diff command runs as one background Diff job. It reuses the normal WAV loading pipeline and reports detailed progress for both inputs (`A: reading samples`, `B: thumbnails`, etc.), then computes the diff and integrates the three tracks together.

## Global Options

### `--log-level`

Sets the tracing/log level. Examples:

```bash
wavalyze --log-level debug song.wav
wavalyze --log-level wavalyze=debug,eframe=info song.wav
```

## Examples

### Open a stereo file and view only the left channel

```bash
wavalyze music.wav:0
```

### Open a mono file from 1 minute to 2 minutes (in samples)

Assuming 44.1 kHz sample rate, 1 minute = 2,646,000 samples:

```bash
wavalyze music.wav:2646000-5292000
```

### Analyze first 10 seconds of a multichannel file, specific channels only

For a 48 kHz file, 10 seconds = 480,000 samples:

```bash
wavalyze music.wav:0,2,4:-480000
```

### Compare original and processed versions

```bash
wavalyze diff original.wav processed.wav
wavalyze diff original.wav:0:o=-12 processed.wav:0
```

### Open multiple files with different specifications

```bash
wavalyze file1.wav:0,1 file2.wav:1000-50000 file3.wav:0:100000-
```

## Notes

- Channel indices are zero-based (the first channel is channel 0)
- Sample ranges are inclusive of the start index and exclusive of the end index
- Offsets are signed sample counts and use `offset=N` or `o=N`
- Specifications are case-sensitive
- All paths use your platform's standard path separators (forward slashes on Unix-like systems, backslashes on Windows)
- If channels, range, and offset are specified, all specifications are applied
- `diff` currently produces three tracks: source A, source B, and the generated Diff track
