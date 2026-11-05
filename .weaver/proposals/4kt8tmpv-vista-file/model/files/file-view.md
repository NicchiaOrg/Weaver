# File view

The File view of one project window: the tree of the project's files and the tabs of the files open in it. Like every view but Settings, it exists only in a project with a `.weaver` folder. Choosing another view closes it: its tabs close at once, and what it read and watches is kept for a short while, so that coming back soon reads nothing again.

## States

### Closed (initial)
Weaver reads no folder and watches nothing of the code.
- `developer.OPEN_FILE_VIEW` → Open: Weaver reads only the names and kinds of the entries of the project's root, never the content of a file, leaving out `.git` and what git ignores, and watches the root alone.

### Open
The view is shown: the tree, its folders collapsed until the developer expands them, and the tabs.
- `developer.EXPAND_FOLDER` → Open: Weaver reads only the names and kinds of that folder's entries, as for the root, and watches that folder alone, without the folders inside it.
- `developer.COLLAPSE_FOLDER` → Open: Weaver stops watching that folder and every folder inside it, and forgets their entries.
- `workspace.CODE_FOLDER_CHANGED` → Open: Weaver reads that folder again.
- `developer.CREATE_ENTRY` → Open: the new empty file or folder appears in the tree; refused, with the reason, when the name is not valid, when an entry with that name already exists there, or inside `.weaver/`.
- `developer.RENAME_ENTRY` → Open: a tab open on the entry, or on a file inside it, follows the new path and keeps its unsaved changes; refused, with the reason, when the name is not valid, when an entry with that name already exists, when the entry is inside `.weaver/` or is `.weaver/` itself, or when an agent is changing it or a file inside it.
- `developer.MOVE_ENTRY` → Open: a tab open on the entry, or on a file inside it, follows the new path and keeps its unsaved changes; refused, with the reason, as a rename, and also when the target folder is the entry itself or a folder inside it, or lies inside `.weaver/`.
- `developer.DELETE_ENTRY` → Open: the entry goes to the system's trash and every tab on it, or on a file inside it, closes; refused, with the reason, when the entry is inside `.weaver/` or is `.weaver/` itself, when an agent is changing it or a file inside it, or when the system's trash cannot take it, and then nothing is deleted.
- `developer.LEAVE_FILE_VIEW` → Releasing: every tab closes, after its unsaved changes were saved or discarded as the developer chose.
- `developer.LEAVE_FILE_VIEW` → Open: when the developer chose to save all and a file could not be saved; the other files are saved, the other view is not shown, and the File view stays on the tab of that file, which says why.
- `window.CLOSED` → Closed: every watch and every listing is dropped.

### Releasing
The view is not shown and has no tab; for a short while Weaver keeps the tree and the watches of its folders.
- `developer.OPEN_FILE_VIEW` → Open: the tree shows as it was, with nothing read again.
- `workspace.CODE_FOLDER_CHANGED` → Releasing: Weaver reads that folder again, so that the tree is right if the view opens again.
- `time.FILE_VIEW_RELEASE_DUE` → Closed: every watch and every listing is dropped.
- `window.CLOSED` → Closed: every watch and every listing is dropped.
