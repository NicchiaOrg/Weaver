---
description: "Use when building or changing the Jobs list, a conversation, General, question rounds or system notifications."
---
# Jobs and conversations

## job
- **Jobs view:** the project's jobs listed on the left (~300 px, resizable 240–420 px), the selected job's conversation on the right. Empty list: `No jobs yet`.
- **List spacing:** 12 px of space between the top edge of the jobs list and its first row, inside the list's scrolling content, so it scrolls away with the rows; no added space below the last row.
- **Fixed names**, set at creation: `Setup`, `Design`, `Import`, `Scan`, `Run T3` (short number, else permanent ID), `All tasks`, `All tasks · <model>`, `Resolve D3`, `Rework P3`, `Refine P3`. Equal names are told apart by started time.
- **Row:** name (semibold), kind in secondary text (`setup`, `design`, `import`, `scan`, `run`, `resolve`, `rework`, `refinement`), state badge, started time, running time, and the latest note on a second line truncated with `…` (full text in the tooltip).
- **Badges**, always with text: `queued` (outline), `running` (blue, spinning `Loader`, static under reduced motion), `waiting` (warning, `MessageCircleQuestion`), `suspended` (outline, `Pause`), `done` (green, `Check`), `failed` (error, `X`), `stopped` (muted, `Square`).
- **Times:** started `HH:MM` today, else `D MMM HH:MM`. Running time counts from the session start (none while queued), ticks every second while working, freezes at the end: `45s`, `4m 05s`, `1h 12m`.
- **Order:** waiting, then running, queued, suspended (each newest start first), then finished jobs, latest end first.
- **Retention:** a finished job stays listed until removed with its row's `X` (no dialog) or the window closes; only the 30 latest finished jobs are kept. Working jobs have no `X`.
- **Conversation:** entries top to bottom, message box pinned at the bottom. The view follows new text unless scrolled up; then `Jump to latest` (`ArrowDown`) appears. Nothing scrolls sideways: tables and code scroll in their own box.
- First entry: Weaver's one-line instruction, shown as a message labelled `Weaver`. Developer messages: right-aligned boxes on the secondary background, plain text. Agent messages: full width, no box, Markdown with the shared renderer; reasoning in a collapsed `Thinking` block.
- **Tool calls:** one compact row each: icon by kind (read `FileText`, search `Search`, command `SquareTerminal`, edit `FilePen`, sub-agent `Bot`, other `Wrench`), omp's title on one line truncated with `…`, status icon (`Loader`, `Check`, `X` in error colour). A row whose text does not fit is a disclosure button (`ChevronRight`/`ChevronDown`, `aria-expanded`) opening the full command (without leading `$ `) or full title in a monospace box, ~12 lines then scrolling. Rows never show results or output. Rows start closed.
- **Sub-agents:** under omp's `task` row, indented, one line each in omp's order: `Bot`, omp's name for it, state as icon and text (`working`, `succeeded`, `failed`, `interrupted`). When the job ends, those still working show `interrupted`. No sub-agent conversation in v1.
- **Notes:** each note appears where it arrived, one secondary line: `StickyNote`, `HH:MM`, the text.
- **Message box:** textarea growing to ~10 lines, placeholder `Message the agent…`; `Enter` sends, `Shift+Enter` new line; `Send` (`ArrowUp`) disabled while empty after trimming. While the job works a message reaches the agent without stopping it (steer).
- **Queued message:** one not delivered yet (a round or permission waits, or omp has not taken it) shows `Clock` + `Queued` under it until delivered; a refused one shows `Not delivered: <reason>.` in the error colour; still queued at the job's end: `Not delivered: the job ended.`
- **Stop:** `Square` button `Stop the agent` beside `Send` while the job is queued, running, waiting or suspended. AlertDialog `Stop <name>?` / `The agent stops at once and every process it started ends.`, `Stop` (destructive) and `Cancel`, focus on `Cancel`.
- **Permission card** (permissions set to `ask`): a box with a warning border headed `omp asks permission`, the compact row of the tool call concerned, then one button per option omp offers, omp's names and order; rejecting options use `outline`. Afterwards it stays read-only with the choice, or `No longer needed.` (voided by the turn's end) or `Cancelled.` (Stop).
- **Queued job:** only `Starting omp…` centred in secondary text, or `Waiting for omp to be ready.` with `Open Settings` while omp is not ready; `Send` disabled.
- **Suspended job:** a secondary line `Weaver stopped; the job continues when its session is reloaded.`; on resume `Session reloaded; the agent continues its work.`
- **End line:** `Finished.` or `Stopped.` (secondary); failed: `TriangleAlert` + `Failed: <reason>.` in the error colour, reasons in plain words (`omp did not answer within 30 s`, `omp exited with code <N>`, `the session could not be resumed`, `the agent refused`, `the agent ran out of tokens`, `the agent reached its turn limit`, a proxy explanation of `requirements/ui#system-certificates`, or omp's message). After an exit, `Show details` opens omp's last error lines in monospace.
- **Finished conversation:** read-only; the message box becomes `This job has ended. Continue in Chat.` with `Open Chat`.
- New agent messages are announced by an `aria-live="polite"` region once per complete message.

## general-chat
- Its own view in every project window, always present, with the job conversation rendering (messages, `Thinking`, tool rows, sub-agents, permission cards, queued messages), but no instruction, notes or end line.
- Every text of the interface names it `Chat`, never `General`: the sidebar entry, its tooltip, the view's title and the texts below. A request refused because it is not open says `Chat is not open for this project`.
- Empty: centred secondary text `Talk with omp in this project. Ask it to start a job of Weaver when you want one.`
- **Loading:** while the kept conversation loads, centred secondary text `Loading conversation…` in place of the empty text, above any message sent meanwhile, until the history shows. Waiting for omp with omp missing adds the `omp is not installed.` line and `Install omp` at once; while omp is only checking, updating or not connected, the loading text shows alone.
- The message box is always enabled; a message sent while omp starts or reloads shows at once with `Starting omp…` under it until delivered. `Stop` (no dialog) shows during a turn.
- **Clear:** `Eraser` button `Clear` in the header, disabled while empty; a kept conversation that is loading is not empty. AlertDialog `Clear Chat?` / `The conversation is deleted and a new one starts.` (also while loading; during a turn: `The agent's turn stops, the conversation is deleted and a new one starts.`), `Clear` (destructive) and `Cancel`, focus on `Cancel`.
- **Started jobs:** a line `Bot` + `Started <job name>` with `Open conversation`, which selects that job in the Jobs view. A refused start: `Weaver did not start <kind>: <reason>.` in the warning colour.
- **Failures:** `omp stopped: <reason>.` in the error colour, then `Your next message starts omp again.` in secondary text. With omp missing: `omp is not installed.` and the `Install omp` button.

## question-round
- **Overlay:** a modal Dialog over the whole window, any view, background dimmed; up to ~760 px wide and 85% of the window high, content scrolling inside. `Esc` and outside clicks do nothing. Focus is trapped and moves to the question text on each question; no transitions under reduced motion.
- **Header:** `Questions from the agent · <job name>` (prepared Drift question: `Question about <D3> · <Drift title>`), below `Question <n> of <N>` or `Summary`; with other rounds waiting, `+<k> more round(s) waiting`. Right: `Minimize` (`Minus`) and `Close` (`X`), which opens the Skip-all confirmation.
- **Index:** left column, one entry per question (number, one-line text with full text in the tooltip, status icon + text: `Answered`, `Skipped`, `Missing`), last entry `Summary`. Below 600 px window width: a row of numbered dots with the same accessible status.
- **Question:** text in full, context below in muted text, both Markdown, in the agent's language; every label of the overlay is English.
- **Options:** full-row radio (single) or checkbox (multiple) with key hint `1`–`9`, bold plain-text label, full Markdown description, `Recommended` badge; the recommended option is preselected on the first visit only. `Show details` / `Hide details`, collapsed on every visit. Clicking a selected single option clears it. A digit selects (single) or toggles (multiple); digits are inert while the textarea has focus. Arrows move within the group.
- **Free answer:** textarea `Your answer (optional)`, placeholder `Write a free answer or a note for the agent`, growing to 10 lines; text reaches the backend within 300 ms of the last keystroke and on blur.
- **Footer:** `Back`, `Skip all`, `Skip`, `Next` (`Review` on the last). `Next`/`Review` disabled, tooltip `Choose an option or write an answer.`, until a choice or non-blank text exists. `Ctrl/Cmd+Enter` = `Next`/`Review`/`Send to agent`.
- **Skip all:** AlertDialog `Skip all unanswered questions?` / `The agent will use its recommendation for each of them.`, `Skip all` and `Go back`, focus on `Go back`. Never kept with the round.
- **Summary:** each answer (labels and text), `Skipped`, or `Missing` (warning colour, `TriangleAlert`), with `Edit` (`Pencil`) going to that question. `Send to agent` (primary) disabled while any is missing, with `Answer or skip <n> more question(s) to send.`
- **Minimized:** a warning-colour bar pinned at the window bottom: `The agent is waiting for your answers (question <n> of <N>)` + `Open questions`.

## notification
| Kind | Title | Message |
|---|---|---|
| Finished | `<project> – <job> finished` | `Weaver is waiting for you.` |
| Several finished | `<project> – <n> jobs finished` | `Weaver is waiting for you.` |
| Failed | `<project> – <job> stopped` | `The agent stopped before finishing its work.` |
| Question round | `<project> – <job> asks a question` | `The agent is waiting for your answers.` |
| Permission | `<project> – <job> asks permission` | `The agent is waiting for your approval.` |
| System Certificates turned on | `System Certificates turned on` | the text of `requirements/ui#system-certificates` |

- `<project>` is the folder name, `<job>` the job's fixed name. Never any message, question, command, path, answer or card. Only the System Certificates notification names a product, and it belongs to no project.
- **Grouping:** a job finishing while a finished notification of the same project is shown replaces it with `<n> jobs finished`; at most one finished notification per project. The other kinds are one per event, never grouped.
- Each notification has an ID of project + job + kind to replace or withdraw it. Where the system cannot remove a delivered one, clicking a withdrawn one only brings the window forward.
- **Click target:** finished/failed → that job's conversation (several → the Jobs view); question → the round overlay; permission → the conversation scrolled to the card; System Certificates turned on → the Settings view of the project window used most recently, else the welcome window.
- App name and icon, system default sound; no setting.
