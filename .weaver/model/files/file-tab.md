# File tab

One file open in a tab of the File view. A file has at most one tab, and only a limited number of tabs are open together. While its tab is open Weaver keeps the file's text, or its image, in memory and watches that file alone; closing the tab drops the editor, the text or image and the watch at once. The first file of a kind loads the colours of that kind.

The tab remembers the file as it read it. A save writes only when the file on disk is still that one; it keeps the file's line endings, its byte order mark, its permissions and the presence of a final line break, and through a symbolic link it writes the file the link points to inside the project.

A tab on an SVG file shows either the drawing of the tab's text, unsaved changes included, or the text itself, and the developer switches between them at any time. It opens on the drawing, and the choice holds for that tab until it closes. The switch is there whatever the State of a tab that shows text: read-only, locked or deleted alike; it never changes the file. When the text cannot be drawn, the tab says so in place of the drawing.

## States

### Closed (initial)
- `developer.OPEN_FILE` → Image: when the file is an image Weaver can show; the tab shows the image, with its size in pixels and its weight, and no text.
- `developer.OPEN_FILE` → Binary: when the file is binary and not an image Weaver can show; the tab says so and shows no text, and when the file is an image too large to show, it also says that.
- `developer.OPEN_FILE` → Read-only: when the file is too large, then shown without colours, or lies inside `.weaver/`.
- `developer.OPEN_FILE` → Locked: when the file is neither binary nor read-only and an agent is already changing it; the text shows read-only.
- `developer.OPEN_FILE` → Saved: otherwise; the text shows with the colours of its kind, chosen by the file's extension or name, or as plain text when the kind is unknown.

### Open
- `developer.CLOSE_TAB` → Closed: when no unsaved change is left, saved or discarded as the developer chose.
- `developer.LEAVE_FILE_VIEW` → Closed: unless the developer chose to save all and a file could not be saved.
- `developer.OPEN_FILE` → Closed: when the tabs are at their limit and this is the tab without unsaved changes shown least recently; the new file opens in its place.
- `developer.DELETE_ENTRY` → Closed: when the deleted entry is this file or a folder holding it; its unsaved changes are lost.
- `window.CLOSED` → Closed

#### Saved (initial)
The tab shows the file as it is on disk.
- `developer.EDIT_FILE` → Modified
- `workspace.OPEN_FILE_CHANGED` → Saved: the text is read again from disk.
- `workspace.OPEN_FILE_DELETED` → Deleted: the text stays; a bar says that the file was deleted.
- `file-lock.LOCKED` → Locked

#### Modified
The tab has unsaved changes.
- `developer.SAVE_FILE` → Saved: when the file on disk is still the one the tab read; Weaver writes it.
- `developer.SAVE_FILE` → Modified: when the file on disk changed since the tab read it; nothing is written, and the developer chooses to reload it from disk, losing his changes, to overwrite it, or to cancel.
- `developer.SAVE_FILE` → Modified: when the write fails; nothing changes on disk and an error gives the reason.
- `developer.RELOAD_FILE` → Saved: the unsaved changes are lost and the text is read again from disk.
- `developer.OVERWRITE_FILE` → Saved: the tab's text is written over the file on disk, whatever it holds now.
- `workspace.OPEN_FILE_CHANGED` → Modified: a bar says that the file changed on disk; the next save goes through the check.
- `workspace.OPEN_FILE_DELETED` → Deleted: the unsaved changes stay; a bar says that the file was deleted.
- `file-lock.LOCKED` → Locked: the unsaved changes stay.

#### Locked
An agent is changing the file: the text cannot be changed and a line says that the agent is editing this file. Unsaved changes stay in the tab.
- `file-lock.RELEASED` → Saved: when the tab has no unsaved changes; the text is read again from disk, at the new path when omp said where the agent moved the file.
- `file-lock.RELEASED` → Modified: when the tab has unsaved changes; they stay, and a bar offers to take the agent's version, which reloads the file from disk, to keep the developer's text, which overwrites it, or to cancel, after which the next save goes through the check. At the new path when omp said where the agent moved the file.
- `file-lock.RELEASED` → Deleted: when the file no longer exists and omp did not say where the agent moved it; the bar says that the agent deleted it.

#### Deleted
The file no longer exists on disk; the tab keeps its text and a bar says that the file was deleted, by the agent or in another way, and offers to save it again.
- `developer.EDIT_FILE` → Deleted
- `developer.SAVE_FILE` → Saved: the file is written again at its path with the tab's text.
- `workspace.OPEN_FILE_CHANGED` → Saved: when the file appears again and the tab has no unsaved changes; the text is read from disk.
- `workspace.OPEN_FILE_CHANGED` → Modified: when the file appears again and the tab has unsaved changes; a bar says that the file changed on disk, and the next save goes through the check.
- `file-lock.LOCKED` → Locked

#### Read-only
The file is too large or lies inside `.weaver/`: its text cannot be changed.
- `workspace.OPEN_FILE_CHANGED` → Read-only: the text is read again from disk.
- `workspace.OPEN_FILE_DELETED` → Read-only: a bar says that the file was deleted; nothing can be saved.
- `file-lock.LOCKED` → Read-only: a line says that the agent is editing this file.
- `file-lock.RELEASED` → Read-only: the line goes away and the text is read again from disk, at the new path when omp said where the agent moved the file.

#### Binary
The file is binary and no image of it can be shown: the tab says so and shows no text.
- `workspace.OPEN_FILE_DELETED` → Binary: the tab also says that the file was deleted.
- `workspace.OPEN_FILE_CHANGED` → Image: when the file is now an image Weaver can show; it is read and the image shows; the tab no longer says that the file was deleted.
- `workspace.OPEN_FILE_CHANGED` → Binary: otherwise; when the file appears again, the tab no longer says that it was deleted.

#### Image
The file is an image Weaver can show: the tab shows the image, its size in pixels and its weight, and no text. Nothing in the tab can change the file, and an agent changing it shows only through the changes on disk. When the image cannot be drawn, the tab says instead that the file is binary and that the image cannot be shown.
- `workspace.OPEN_FILE_CHANGED` → Image: when the file is still an image Weaver can show; it is read again and the new image shows at once; when the file appears again after a deletion, the bar about it goes away.
- `workspace.OPEN_FILE_CHANGED` → Binary: when the file is no longer an image Weaver can show; the tab says that it is binary, and that the image is too large to show when that is why.
- `workspace.OPEN_FILE_DELETED` → Image: the image stays; a bar says that the file was deleted; nothing can be saved.

## Events

### CLOSED
Emitted when the tab closes, for any reason.
