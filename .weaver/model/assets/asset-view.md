# Assets view

The Assets view of one project window: the assets of the project as a grid of icons, with the developer's selection. Like every view but Settings, it exists only in a project with a `.weaver` folder. Choosing another view closes it and forgets the selection.

## States

### Closed (initial)
The view is not shown; Weaver still knows the names of the assets, so that tags can be recognised.
- `developer.OPEN_ASSETS_VIEW` → Open: Weaver shows the names and sizes of the assets as they are now, each with the icon of its kind of file, and marks those larger than the size limit; nothing is selected. With no asset yet the view says how to add one.

### Open
The view is shown; it follows the assets folder by itself.
- `workspace.ASSETS_CHANGED` → Open: the view shows the assets as they are now, marks included; an asset that is gone leaves the selection.
- `developer.DROP_ASSETS` → Open: the files are copied into the assets folder, created first when it does not exist; the originals stay where they are. While the copy runs the developer sees that it is in progress and can go on using the view; leaving the view or closing the window does not stop it. Once done, the copied assets are selected. When any copied file is larger than the size limit, a warning says so and names it, and the file stays marked. Refused as a whole, with the reason and with nothing copied, when a dropped entry is a folder or a symbolic link, when its name is not valid, when an asset with that name already exists, when two dropped files have the same name, or when another copy dropped in the same window is still running.
- `developer.DROP_ASSETS` → Open: when every dropped file is already an asset of this project, dragged back onto its own view; nothing happens.
- `developer.RENAME_ASSET` → Open: the asset keeps its content under the new name and stays selected; refused, with the reason, when the name is not valid or when another asset already has that name.
- `developer.DELETE_ASSETS` → Open: every selected asset goes to the system's trash and leaves the view; one that the trash refuses stays, and the developer is told how many stayed and why.
- `developer.DRAG_ASSETS_OUT` → Open: the app where the developer drops them receives copies of the dragged assets; the assets stay as they are, whatever that app asks.
- `developer.LEAVE_ASSETS_VIEW` → Closed: the selection is forgotten; a copy still running goes on to its end.
- `window.CLOSED` → Closed
