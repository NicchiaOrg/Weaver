# Workspace

The project folder on disk, changed also outside Weaver: by the developer's editor, by git (a pull, a branch switch) or by the agent. Weaver watches each open project and turns what it sees into the facts below.

## Events

### PROJECT_FOLDER_REMOVED
The `.weaver` folder of the project disappeared.

### PROJECT_FOLDER_APPEARED
A `.weaver` folder appeared in a folder that was not a project.

### FILES_CHANGED
Files of the project's Model, Requirements, Specs or cards changed outside Weaver.

### REQUEST_DELETED
The file of a design request was deleted outside Weaver.
