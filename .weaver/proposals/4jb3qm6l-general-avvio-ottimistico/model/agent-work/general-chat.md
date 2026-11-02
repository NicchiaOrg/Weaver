# General chat

The free conversation with omp of a project, always there in its window: no instruction, no skill, no written request. The developer writes what he wants, and the agent runs Weaver's commands only when he asks. It is not a job: it holds no Task, announces no card, asks no question round and shows nothing on the Board. Its conversation is kept across closing the window and quitting Weaver, and comes back as it was. When the window opens, a kept conversation comes back at once, before the developer writes anything and without sending anything to the agent; with no kept conversation, omp starts only with the first message.

## States

### Idle (initial)
No turn is in progress; the conversation may be empty.
- `window.OPENED` → Reloading: only when a conversation is kept and omp is ready; Weaver starts omp and reloads it, sending nothing.
- `window.OPENED` → Waiting for omp: only when a conversation is kept and omp is not ready.
- `developer.SEND_MESSAGE` → Working: Weaver starts omp when it is not running, reloading the kept conversation when it has not come back yet, and sends the message.
- `developer.CLEAR_GENERAL` → Idle: the conversation starts again empty.

### Loading
The window has just opened and the kept conversation is coming back from omp; nothing is sent to the agent meanwhile.
- `developer.SEND_MESSAGE` → Loading: the message shows at once and waits; it is sent when the conversation has come back, below it.
- `developer.CLEAR_GENERAL` → Idle: the loading stops and the conversation starts again empty.
- `window.CLOSED` → Idle: omp is ended; the conversation is kept.

#### Reloading (initial)
Weaver has started omp and asks it to reload the kept conversation.
- `omp.SESSION_RESUMED` → Idle: when no message waits; the conversation shows as it was.
- `omp.SESSION_RESUMED` → Working: when messages wait; the conversation shows above them and they are sent.
- `omp.SESSION_NOT_RESUMED` → Idle: when no message waits; an empty conversation starts and a notice says the kept one could not be reloaded.
- `omp.SESSION_NOT_RESUMED` → Working: when messages wait; an empty conversation starts with the notice and they are sent.
- `omp.START_FAILED` → Idle: the reason is shown; the next message starts omp again and reloads the kept conversation.
- `omp.EXITED` → Idle: the reason is shown; the next message starts omp again and reloads the kept conversation.

#### Waiting for omp
omp is not ready, so it cannot start yet; when omp is missing, the view says so and offers to install it.
- `omp.READY` → Reloading: Weaver starts omp and reloads the kept conversation, without waiting for a message.

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
