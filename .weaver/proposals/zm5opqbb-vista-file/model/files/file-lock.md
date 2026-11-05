# Agent lock

Whether an agent of the project is changing a file open in a tab of the File view. Only the tools that change files count, the edits, deletions and moves of the agent of a job, of General or of an Ask more conversation of this project, while that tool runs. A tool that names the files it changes counts for those files; a tool that names none counts for every file open in a tab. Tools of sub-agents, commands the agent runs, other programs, and paths outside the project never count. While the lock is held the file's tab cannot be changed and the tree marks the file. It is only an indication: the check at saving stays the protection.

## States

### Not tracked (initial)
The file is not open in a tab.
- `developer.OPEN_FILE` → Held: when a tool that counts for this file is already running.
- `developer.OPEN_FILE` → Free: otherwise.

### Tracked
The file is open in a tab.
- `file-tab.CLOSED` → Not tracked

#### Free (initial)
No tool counts for the file.
- `omp.FILE_TOOL_STARTED` → Held: when the tool names this file, or names no file.

#### Held
At least one running tool counts for the file.
- `omp.FILE_TOOL_STARTED` → Held: the new tool counts too, when it names this file or names no file.
- `omp.FILE_TOOL_ENDED` → Free: when it was the last tool counting for the file.
- `omp.FILE_TOOL_ENDED` → Held: when another tool still counts for the file.
- `omp.TURN_ENDED` → Free: the tools of that conversation stop counting, and none of another conversation still counts.
- `omp.TURN_ENDED` → Held: the tools of that conversation stop counting, but one of another conversation still counts.
- `omp.TURN_FAILED` → Free: the tools of that conversation stop counting, and none of another conversation still counts.
- `omp.TURN_FAILED` → Held: the tools of that conversation stop counting, but one of another conversation still counts.
- `omp.EXITED` → Free: the tools of that conversation stop counting, and none of another conversation still counts.
- `omp.EXITED` → Held: the tools of that conversation stop counting, but one of another conversation still counts.
- `job.ENDED` → Free: the tools of that job stop counting, and none of another conversation still counts.
- `job.ENDED` → Held: the tools of that job stop counting, but one of another conversation still counts.
- `developer.CLEAR_GENERAL` → Free: the tools of General stop counting, and none of another conversation still counts.
- `developer.CLEAR_GENERAL` → Held: the tools of General stop counting, but one of another conversation still counts.
- `developer.CLOSE_ASK` → Free: the tools of that Ask more conversation stop counting, and none of another conversation still counts.
- `developer.CLOSE_ASK` → Held: the tools of that Ask more conversation stop counting, but one of another conversation still counts.

## Events

### LOCKED
Emitted when the lock of a file open in a tab goes from Free to Held.

### RELEASED
Emitted when the lock of a file open in a tab goes from Held to Free, saying where the agent moved the file when omp said so.
