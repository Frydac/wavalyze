# Channel orders and active layouts

Each file has a **Channel order** choice in the Files sidebar. The diff pairing
window has independent choices for inputs A and B.

| Choice | Interpretation |
|---|---|
| Automatic | Uses a valid WAVEX speaker mask. Without a mask, or with a zero mask, assumes Auro ID order. Invalid nonzero masks leave roles unassigned. |
| Auro ID order | Assigns roles using the numeric Auro channel IDs, overriding any WAVEX mask. |
| Unassigned | Keeps channel indices without assigning speaker roles. |

Automatic is the default. Its resolved source appears beneath the Files
selector and on hover over the diff input selector. The raw WAVEX mask remains
visible in file metadata even when another order is selected.

Track titles show the WAV index, short role, then path. The index and role occupy
fixed-width fields, so paths align across channels. The role field is blank for
unassigned channels, and a `*` marks expected padding. Long paths are shortened;
hover over the title to see the full path and file metadata.

## Ordinary WAV files in Auro ID order

This convention stores each channel at the WAV index equal to its numeric Auro
ID. Channels between active IDs can be silent padding. For example, a mono
center file stores three channels: index 0 is potentially L, index 1 R, and
index 2 C. The signal can be entirely in index 2.

Channel count alone does not establish an active layout. Expand **Metadata**
in the Files sidebar and choose **active layout** if you know which speakers
are active. Compatible named layouts are offered without selecting one
automatically. A three-channel file offers 1.0 and 3.0; selecting 1.0 marks
indices 0 and 1 as expected padding while keeping their potential roles visible.
Choose **Unknown** to clear the selection.

Candidates use the full WAV header count, including channels excluded from
loading. A candidate's highest numeric Auro ID plus one must equal that count.
Names with identical channel membership are grouped together. Additional
trailing padding and custom channel mappings are not supported yet.

Padding labels describe the selected layout; wavalyze does not scan for silence.
An ordinary WAV using another convention, such as a compact mono file with C at
index 0, needs a different interpretation. Until that order is supported, choose
Unassigned to avoid misleading roles. Unknown numeric IDs also remain unassigned.

## WAVEX and diff pairing

With Automatic and a valid WAVEX mask, the header supplies the roles and active
layout. Metadata shows that layout as a read-only value. The decoder follows the
Auro-3D Revision 2 convention, including its 22.x mapping; see the
[channel mapping details](../data/channel/README.md).

In the pairing matrix, rows are input A and columns are input B. Labels show
original WAV indices and short roles. Column labels use a compact font and can
be clipped; hover reveals full names. Unassigned roles are blank. Hovering a
checkbox identifies both channels and highlights its row and column labels.
A `*` on a heading indicates expected padding for an existing selected layout.
Active layout is chosen in Files, rather than in the pairing window.

Changing channel order or active layout updates labels and metadata. It preserves
audio samples, WAV indices, offsets, loaded channel selections, and checked diff
pairs; it does not pair channels by role automatically. Pairing-window choices
are carried into the files loaded when you confirm.

Native reload retains each file's choices and resolves them against the new
header. If an explicit active layout is no longer compatible, it is cleared.
Automatic can therefore resolve differently when the mask changes. These are
per-file choices for the current workspace; they are not global defaults saved
for future opens. Browser uploads support the same selectors but have no disk
reload source.
