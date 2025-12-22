# Wavalyze CLI Interface

Wavalyze provides a flexible and intuitive command-line interface for analyzing WAV files. The interface supports multiple files with optional channel and sample range specifications.

## Quick Start

The simplest usage is to open a file:

```bash
wavalyze song.wav
```

For more complex operations, you can use explicit subcommands:

```bash
wavalyze open song.wav:0,2:1000-5000
wavalyze diff file1.wav file2.wav
```

## File Specifications

When opening files, you can attach optional specifications directly to the filename using colons as separators. Specifications can be provided in any order.

### Syntax

```
FILE[:SPEC[:SPEC]]
```

Where `SPEC` can be:
- **Channels**: Comma-separated channel numbers (e.g., `0,2,4`)
- **Sample Range**: Start and end sample indices (e.g., `1000-5000`)

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

### Combining Channels and Ranges

You can combine channel and range specifications in either order:

```bash
wavalyze song.wav:0,2:1000-5000
wavalyze song.wav:1000-5000:0,2
wavalyze song.wav:-10000:1,3      # First 10000 samples, channels 1 and 3
wavalyze song.wav:0:5000-         # Channel 0, from sample 5000 onwards
```

## Commands

### open (default)

Opens one or more WAV files for editing or analysis.

```bash
wavalyze open file1.wav file2.wav:0,1 file3.wav:1000-5000
```

**Flags:**
- `-v, --verbose`: Enable verbose output

The `open` command is the default, so you can omit it:

```bash
wavalyze file1.wav file2.wav:0,1 file3.wav:1000-5000
```

### diff

Compares two WAV files.

```bash
wavalyze diff original.wav modified.wav
```

**Flags:**
- `-v, --verbose`: Enable verbose output

### info

Displays information about one or more WAV files.

```bash
wavalyze info file1.wav file2.wav file3.wav
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
```

### Open multiple files with different specifications

```bash
wavalyze file1.wav:0,1 file2.wav:1000-50000 file3.wav:0:100000-
```

## Notes

- Channel indices are zero-based (the first channel is channel 0)
- Sample ranges are inclusive of the start index and exclusive of the end index
- Specifications are case-sensitive
- All paths use your platform's standard path separators (forward slashes on Unix-like systems, backslashes on Windows)
- If both channels and range are specified, both filters are applied
