# Window

The window of one project. A project has at most one window; when no project window is open, Weaver shows a welcome window with the recent projects and a way to open a folder.

## States

### Closed (initial)
The project is not open.
- `developer.OPEN_PROJECT` → Open
- `desktop.APP_STARTED` → Open: only the projects whose window was open when Weaver last quit; their jobs that were working resume.

### Open
The project is shown; its jobs, General and question rounds live in it.
- `developer.OPEN_PROJECT` → Open: the window comes to the front.
- `notification.CLICKED` → Open: the window comes to the front showing what the notification was about.
- `developer.CLOSE_WINDOW` → Closed: its working jobs are suspended; the project will not reopen at the next start.
- `developer.QUIT_APP` → Closed: its working jobs are suspended; the project reopens at the next start.

## Events

### OPENED
Emitted when the window opens.

### CLOSED
Emitted when the window closes, also because Weaver quits.
