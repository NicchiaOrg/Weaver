# Window

The window of one project. A project has at most one window; when no project window is open, Weaver shows a welcome window with the recent projects and a way to open a folder. A click on the System Certificates notification with no project window open brings the welcome window to the front. The developer can send a project window to the background: it leaves the screen while Weaver keeps working in it, and it still counts as open.

## States

### Closed (initial)
The project is not open.
- `developer.OPEN_PROJECT` → Open
- `desktop.APP_STARTED` → Open: only the projects whose window was open when Weaver last quit, also those whose window was in the background; their jobs that were working resume.

### Open
The project is open; its jobs, General and question rounds live in it, whether its window is on the screen or in the background.
- `developer.OPEN_PROJECT` → Shown: the window comes to the front, back from the background when it was there.
- `notification.CLICKED` → Shown: the window comes to the front, back from the background when it was there, showing what the notification was about; for the System Certificates notification, only the project window used most recently comes to the front, showing Settings.
- `developer.CLOSE_WINDOW` → Closed: the unsaved changes of its File view are saved or discarded as the developer chose; its working jobs are suspended; the project will not reopen at the next start.
- `developer.CLOSE_WINDOW` → Shown: when the developer chose to save all the File view's changes and a file could not be saved; the other files are saved, nothing is suspended, and the window stays on the tab of that file, which says why.
- `developer.QUIT_APP` → Closed: the unsaved changes of its File view are saved or discarded as the developer chose; its working jobs are suspended; the project reopens at the next start.
- `developer.QUIT_APP` → Shown: when the developer chose to save all the File view's changes and a file could not be saved; the other files are saved, Weaver does not quit, and the window, back from the background when it was there, stays on the tab of that file, which says why.

#### Shown (initial)
The window is on the screen, also when minimised.
- `developer.SEND_TO_BACKGROUND` → In background: the window leaves the screen and, on Windows and Linux, the taskbar; nothing is suspended: its jobs, General, Ask more conversations and question rounds go on, and system notifications treat it as being in the background.
- `developer.CLOSE_WINDOW` → In background: only when agent jobs of the project are working, its File view has no unsaved changes, and the developer chose to keep them working in the background instead of suspending them; nothing is suspended and the project stays open.

#### In background
The window is open but not on the screen, nor in the taskbar on Windows and Linux; Weaver keeps working in it.
- `developer.REOPEN_APP` → Shown: every project window in the background comes back on the screen as it was, and the one used most recently gets the focus.

## Events

### OPENED
Emitted when the window opens.

### CLOSED
Emitted when the window closes, also because Weaver quits.
