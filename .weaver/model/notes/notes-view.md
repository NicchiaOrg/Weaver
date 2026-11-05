# Notes view

The Notes view of one project window and the notes sheet it shows. Like every view but Settings, it exists only in a project with a `.weaver` folder. Choosing another view keeps what the developer was doing in the sheet, but Weaver stops following the notes until he comes back.

## States

### Closed (initial)
The view is not shown; Weaver reads nothing of the notes.
- `developer.OPEN_NOTES_VIEW` → Open: Weaver reads the sheet as it is on disk and shows it formatted, with its images; with no sheet yet, the empty sheet says what can be written in it. When the sheet on disk is still the one the view showed when the developer left it, the view comes back as he left it, with the cursor, the scroll and the changes that can be undone.

### Open
The view is shown; it follows the notes on disk by itself, and every change of the developer is written by itself.
- `developer.EDIT_NOTES` → Open: the change shows at once and is written to the sheet a moment after the last one, the notes folder being created first when it does not exist. When the writing fails, the developer is told why, the text stays as it is, and the next change tries again.
- `developer.TICK_NOTES_ITEM` → Open: the checklist item is ticked, its text struck through and dimmed, or unticked back to normal text; it stays where it is, nothing else in the sheet changes, and the sheet is written at once.
- `developer.ADD_NOTES_IMAGES` → Open: each image is copied into the notes folder, created first when it does not exist, under the name of its file when it has one and otherwise under the date and time, with a number added when that name is taken; it shows in the sheet where it was pasted or dropped, and the originals stay where they are. An image larger than the size limit, or of a kind the sheet cannot show, is refused with the reason and not copied; the others are still added. Files that are not images are left out, and when nothing is an image nothing happens.
- `workspace.NOTES_CHANGED` → Open: when no change of the developer waits to be written, the sheet shows the file as it is now, empty when it was deleted; otherwise the sheet's version is written over it and a warning says so. Images that changed on disk show as they are now, and missing ones as a placeholder.
- `developer.LEAVE_NOTES_VIEW` → Closed: a change still waiting is written at once; every image the developer removed from the sheet during this visit, and that the sheet no longer shows anywhere, goes to the system's trash. An image the trash refuses stays, and the developer is told why. Images he never removed stay, even when the sheet does not show them.
- `window.CLOSED` → Closed: as when leaving the view, before the window goes.
