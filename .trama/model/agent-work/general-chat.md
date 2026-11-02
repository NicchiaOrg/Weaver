# General chat

The free conversation with omp of a project, always there in its window: no instruction, no skill, no written request. The developer writes what he wants, and the agent runs Weaver's commands only when he asks. It is not a job: it holds no Task, announces no card, asks no question round and shows nothing on the Board. Its conversation is kept across closing the window and quitting Weaver, and comes back as it was.

## States

### Idle (initial)
No turn is in progress; the conversation may be empty.
- `developer.SEND_MESSAGE` → Working: Weaver starts omp when it is not running, reloading the kept conversation, and sends the message.
- `developer.CLEAR_GENERAL` → Idle: the conversation starts again empty.

### Working
The agent answers.
- `developer.SEND_MESSAGE` → Working: the agent receives it while it works.
- `omp.PERMISSION_REQUESTED` → Working: the permission waits for the developer's answer.
- `developer.ANSWER_PERMISSION` → Working
- `agent.START_JOB` → Working: Weaver starts the job the developer explicitly asked for; it gets its own conversation.
- `omp.TURN_ENDED` → Idle
- `omp.TURN_FAILED` → Idle: the reason is shown.
- `omp.EXITED` → Idle: the reason is shown; the next message starts omp again.
- `developer.STOP_AGENT` → Idle: the turn is cancelled.
- `window.CLOSED` → Idle: the turn is cancelled; the conversation is kept.
