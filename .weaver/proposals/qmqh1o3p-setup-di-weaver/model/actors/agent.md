# Agent

The coding agent, omp, working inside a job or in General that Weaver started. It reaches Weaver only through the `trama` command line, which knows the job it belongs to; what it reports there are the facts below. Only an agent of a job Weaver started can hold a Task, announce a card or ask a question round; a command run outside such a job is refused for those. The agent reads and writes the developer's code, Weaver never does.

## Events

### ANNOUNCE_CARD
The agent says it is preparing a Proposal, a Task or a Drift, with a short title, so that the Board can show it before it exists.

### DECLARE_NOTE
The agent says in one line what it is doing and why.

### EDIT_PROPOSAL
The agent changes a Proposal: it writes or removes one of its files or Tasks, marks a file for deletion, changes its label, records a decision on a Drift in a resolution, or binds it to a Drift.

### SUBMIT_PROPOSAL
The agent hands a finished Proposal over to the developer.

### TAKE_TASK
The agent starts working on a Task.

### RELEASE_TASK
The agent puts a Task it holds back in To do without finishing it.

### FINISH_TASK
The agent says a Task is done.

### CLOSE_TASK_UNCHANGED
The agent closes the fix Task of a Drift without changing the code, with a note saying what it checked and why nothing had to change.

### PREPARE_DRIFT_QUESTION
The agent prepares, ahead of time, the question with which the developer will decide a Drift it filed.

### ASK_QUESTIONS
The agent asks the developer a round of questions and waits for the answers, with no time limit.

### STOP_WAITING
The agent stops waiting for the answers of its round, because its command was interrupted or lost its connection.

### UPDATE_HISTORY_ENTRY
The agent rewrites the text of a History entry or adds Tasks to it.

### START_JOB
The agent in General asks Weaver to start a job (design, scan, Run or Resolve) because the developer explicitly asked it to.
