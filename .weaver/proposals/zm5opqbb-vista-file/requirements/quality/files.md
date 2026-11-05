---
description: "Use when the File view's work must stay light on disk, keep files intact or be tested."
---
# Files

## file-view

- **Performance:** while the File view is closed, Weaver reads no folder of the code and watches nothing outside `.weaver/` but the project root's own watch for `.weaver`. Showing the view reads only the root; expanding reads only that folder. No recursive read, index or search, ever; a listing never opens a file. Expanding a folder starts at most one `git check-ignore`; nothing else of the view starts a process.
- **Release:** 10 s after the view stops being shown, no watch and no listing of the code is left in memory; tested with an injected clock.
- **Tests:** listing (`.git` left out, ignored entries, dot names, no git), the watches added and dropped on expand, collapse, release and window close, the refusals of `requirements/rules#file-view`, rename and move never replacing an entry, deletion to the trash and its refusal, all against temporary folders with injected watcher timing; the tree, its context menu, `F2`, `Delete`, dragging, `Move to…` and the leave dialog in Vitest with their keyboard paths and accessible names.

## file-tab

- **Performance:** opening a file reads it once; a grammar loads only with the first file of its kind; closing a tab frees its editor, text and watch at once. Typing starts no process and no disk work.
- **Reliability:** a save writes only when the file on disk still has the hash the tab read, except `Overwrite` and `Keep mine`; it never changes a byte the developer did not change, its line endings, its byte order mark or its permissions.
- **Logs:** never a file's content; paths are allowed.
- **Tests:** binary detection, the size limit, the line-ending choice and the tab limit as pure functions; saving files with LF, CRLF, mixed endings, a byte order mark, no final line break, the executable bit and through a symbolic link, against temporary folders on every system; the hash check, `Overwrite`, `Save again`, a failed write and the self-write rule; tabs, bars, dialogs and shortcuts (`Cmd/Ctrl+S`, `Cmd/Ctrl+W`, `Ctrl+Tab`) in Vitest with their keyboard paths and accessible names.

## file-lock

- **Reliability:** a lock never outlives its tool's end, its conversation's turn, its omp process, its job or its conversation.
- **Tests:** with the scripted fake ACP agent: tools with and without `locations`, several tools on one file, tools of two conversations, end by `completed`, `failed`, turn end, exit, job end, cleared General and closed Ask more, updates replayed while a session loads, a move with its destination, a deletion, paths outside the root and on Windows.


