# Workspace

The project folder on disk, changed also outside Weaver: by the developer's editor, by git (a pull, a branch switch) or by the agent. Weaver watches each open project, and the folders and files its File view shows, and turns what it sees into the facts below.

## Events

### PROJECT_FOLDER_REMOVED
The `.weaver` folder of the project disappeared.

### PROJECT_FOLDER_APPEARED
A `.weaver` folder appeared in a folder that was not a project.

### FILES_CHANGED
Files of the project's Model, Requirements, Specs or cards changed outside Weaver. Files of the code, assets and notes never count.

### REQUEST_DELETED
The file of a design request was deleted outside Weaver.

### CODE_FOLDER_CHANGED
An entry appeared in, disappeared from or was renamed in a folder the File view watches, by anyone, Weaver included.

### OPEN_FILE_CHANGED
A file open in a tab of the File view changed on disk, or appeared again after it was deleted, by anything other than that tab's own save.

### OPEN_FILE_DELETED
A file open in a tab of the File view disappeared from disk, deleted or moved by anything other than the File view itself.

### ASSETS_CHANGED
An asset appeared in, disappeared from, was renamed in or changed size in the assets folder of the project, by anyone, Weaver included.

### NOTES_CHANGED
The notes sheet of the project, or an image in its notes folder, changed, appeared or disappeared on disk, by anything other than Weaver's own writing.
