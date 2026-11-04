---
description: "Use when implementing the limits of the File view and Assets: names, refused actions, binary files, images, size, tabs, line endings, the lock."
---
# Files

## file-view
- **Release delay:** 10 s after the File view stops being shown.
- **Names:** a new name is refused when it is empty, `.` or `..`, holds `/`, `\` or a NUL character, or, on Windows, holds `<>:"|?*`, ends with a space or a dot, or is a reserved device name (`CON`, `NUL`, `COM1`…).
- **Name taken:** a create, rename, move or copy in is refused when the target folder already holds an entry with that name, compared as the file system compares names (case-insensitively on macOS and Windows). A rename that only changes the case of a name is allowed. A drop from outside is also refused when two dropped entries have the same name, compared the same way.
- **Refused actions:**
  - creating inside `.weaver/`;
  - renaming, moving or deleting `.weaver/` or anything inside it, or moving or copying anything into it;
  - moving a folder into itself or into a folder inside it;
  - renaming, moving or deleting a file whose agent lock is held, or a folder holding such a file;
  - copying in, from outside, an entry whose canonical path lies inside the project root;
  - starting a drop from outside while another one of the same window is still copying.
- **Drop from outside:** it copies, never moves; folders are copied with everything inside them and symbolic links as links. It is all or nothing: one refused entry refuses the whole drop, and a failure part way removes everything the drop had copied.

## file-tab
- **Binary:** a file is binary when its first 8 KB (8,192 bytes) hold a NUL byte, or when its bytes, after an optional UTF-8 byte order mark, are not valid UTF-8.
- **Image:** a binary file is an image Weaver can show when its first bytes are the signature of PNG (`89 50 4E 47 0D 0A 1A 0A`), JPEG (`FF D8 FF`), GIF (`GIF87a` or `GIF89a`), WebP (`RIFF`, four bytes, `WEBP`), BMP (`BM`) or ICO (`00 00 01 00`), whatever its name or extension, and it is at most 10 MB (10,485,760 bytes). A larger file with one of those signatures stays binary and the tab says that the image is too large to show. Any other binary file is never shown as an image.
- **SVG:** a file that is not binary and whose name ends in `.svg`, compared case-insensitively, has the drawing of its text; its content is never used to decide it. Every other rule of a text file applies to it unchanged, the 2 MB limit and `.weaver/` included.
- **Too large:** a file that is not binary and larger than 2 MB (2,097,152 bytes) opens read-only and without syntax colours.
- **Read-only:** a file inside `.weaver/` opens read-only.
- **Tabs:** at most 10 open together. Opening an eleventh file closes the tab without unsaved changes that was shown least recently; when all 10 have unsaved changes, nothing opens.
- **Line ending at save:** the one used most in the file as read, counting CRLF and lone LF; LF on a tie or when the file has no line break; a file created in the tree uses LF. A final line break is kept, or left out, as the text has it.
- **Byte order mark:** a UTF-8 byte order mark read at the start of the file is written back at the start; none is ever added.
- **Same file:** two paths name the same file when their canonical forms are equal, case-insensitively on Windows.

## file-lock
- A lock is held while at least one counted tool runs that names the file, or that names no file.
- A tool stops counting once, at the first of its end, its conversation's turn end or failure, its omp's exit, its job's end, General being cleared or its Ask more conversation closing.

## asset-view
- **What is an asset:** a regular file directly in `.weaver/assets/` whose name does not start with `.`. Folders, symbolic links and dot files put there by other means are not shown in the Assets view and are never tagged.
- **Size limit:** an asset is larger than the limit above 10 MB (10 × 1024 × 1024 bytes), the same figure as the request images. It is only a warning and a mark: nothing is ever refused for its size.
- **Names:** the rules of `requirements/rules#file-view` for names, and also refused when the name starts with `.`.
- **Name taken:** a rename or a drop is refused when another asset already has that name, compared as the file system compares names (case-insensitively on macOS and Windows); a rename that only changes the case of a name is allowed. A drop is also refused when two dropped files have the same name, compared the same way.
- **Drop from outside:** it copies, never moves, and only regular files: a dropped folder or symbolic link refuses the whole drop. A file from elsewhere in the project may be copied in. It is all or nothing: one refused file refuses the whole drop, and a failure part way removes everything the drop had copied. At most one drop copies at a time per window; a second one is refused.
- **Back onto itself:** a drop whose files all lie in `.weaver/assets/` of the same project is ignored, without a refusal.
- **Deleting several:** each asset goes to the trash on its own; the ones the trash refuses stay, and the others are not put back.
- **Drag out:** always as a copy, whatever the target app asks.
- **Files view:** `.weaver/assets/` stays read-only there, as the rest of `.weaver/` (`requirements/rules#file-view`).
