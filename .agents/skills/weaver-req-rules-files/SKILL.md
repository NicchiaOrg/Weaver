---
name: weaver-req-rules-files
description: "Use when implementing the File view's limits: names, refused tree actions, binary files, size, tabs, line endings, the lock."
---

# Rules

Every rule here is computed in the Rust core, the same for the app and the `weaver` command line, never in the interface.

## file-view

- **Release delay:** 10 s after the File view stops being shown.
- **Names:** a new name is refused when it is empty, `.` or `..`, holds `/`, `\` or a NUL character, or, on Windows, holds `<>:"|?*`, ends with a space or a dot, or is a reserved device name (`CON`, `NUL`, `COM1`…).
- **Name taken:** a create, rename or move is refused when the target folder already holds an entry with that name, compared as the file system compares names (case-insensitively on macOS and Windows). A rename that only changes the case of a name is allowed.
- **Refused actions:**
  - creating inside `.weaver/`;
  - renaming, moving or deleting `.weaver/` or anything inside it, or moving anything into it;
  - moving a folder into itself or into a folder inside it;
  - renaming, moving or deleting a file whose agent lock is held, or a folder holding such a file.

## file-tab

- **Binary:** a file is binary when its first 8 KB (8,192 bytes) hold a NUL byte, or when its bytes, after an optional UTF-8 byte order mark, are not valid UTF-8.
- **Too large:** a file larger than 2 MB (2,097,152 bytes) opens read-only and without syntax colours.
- **Read-only:** a file inside `.weaver/` opens read-only.
- **Tabs:** at most 10 open together. Opening an eleventh file closes the tab without unsaved changes that was shown least recently; when all 10 have unsaved changes, nothing opens.
- **Line ending at save:** the one used most in the file as read, counting CRLF and lone LF; LF on a tie or when the file has no line break; a file created in the tree uses LF. A final line break is kept, or left out, as the text has it.
- **Byte order mark:** a UTF-8 byte order mark read at the start of the file is written back at the start; none is ever added.
- **Same file:** two paths name the same file when their canonical forms are equal, case-insensitively on Windows.

## file-lock

- A lock is held while at least one counted tool runs that names the file, or that names no file.
- A tool stops counting once, at the first of its end, its conversation's turn end or failure, its omp's exit, its job's end, General being cleared or its Ask more conversation closing.
