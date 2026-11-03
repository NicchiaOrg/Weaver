# Window

The window of one project. A project has at most one window; when no project window is open, Weaver shows a welcome window with the recent projects and a way to open a folder. A click on the System Certificates notification with no project window open brings the welcome window to the front.

## States

### Closed (initial)
The project is not open.
- `developer.OPEN_PROJECT` → Open
- `desktop.APP_STARTED` → Open: only the projects whose window was open when Weaver last quit; their jobs that were working resume.

### Open
The project is shown; its jobs, General and question rounds live in it.
- `developer.OPEN_PROJECT` → Open: the window comes to the front.
- `notification.CLICKED` → Open: the window comes to the front showing what the notification was about; for the System Certificates notification, only the project window used most recently comes to the front, showing Settings.
- `developer.CLOSE_WINDOW` → Closed: the unsaved changes of its File view are saved or discarded as the developer chose; its working jobs are suspended; the project will not reopen at the next start.
- `developer.CLOSE_WINDOW` → Open: when the developer chose to save all the File view's changes and a file could not be saved; the other files are saved, nothing is suspended, and the window stays on the tab of that file, which says why.
- `developer.QUIT_APP` → Closed: the unsaved changes of its File view are saved or discarded as the developer chose; its working jobs are suspended; the project reopens at the next start.
- `developer.QUIT_APP` → Open: when the developer chose to save all the File view's changes and a file could not be saved; the other files are saved, Weaver does not quit, and the window stays on the tab of that file, which says why.

## Events

### OPENED
Emitted when the window opens.

### CLOSED
Emitted when the window closes, also because Weaver quits.
