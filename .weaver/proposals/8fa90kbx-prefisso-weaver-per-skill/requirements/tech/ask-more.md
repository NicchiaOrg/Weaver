---
description: Use when building Ask more's omp processes, sessions, environment, the reading of the chosen thing and the prompt that carries Weaver's instruction.
---
# Ask more

## ask-conversation
- **Not a job:** no record in the job registry, nothing in `general/`, no `JobKind`. It never enters admission, exclusions, the working jobs that make closing the window or quitting confirm, or the sidebar's waiting count. Everything about it lives in the backend's memory, keyed by a conversation identity generated at opening, and ends with the window.
- **Opening:** the UI sends the kind and the identity of the chosen thing, never its text:
  - Proposal, Task, Drift: the permanent identity; the backend reads the card from the files with `weaver-core`.
  - History: the entry's identity only; the backend reads the entry and the copies of the Tasks it cites from the entry file, and a cited Task without a copy from its Task file as the History view does. Only an entry that is gone opens nothing; a cited Task that cannot be read never prevents opening.
  - Requirements: only the overlay Proposal in view, if any, never a section, Area or focus. The backend checks that the Proposal is still open, as the overlay picker does; when it is not, the instruction is built without it, so a Requirements conversation always opens. It reads no Requirement text and no skill: omp, started in the project root, finds the installed `weaver-req-*` skills by itself, as in every job, and nothing injects them.
  - Question: as `requirements/tech#question-round` (**Ask more**) says.
  - Then the backend builds the instruction (`requirements/rules#ask-conversation`) and keeps it with the conversation until the first prompt. A thing that is missing opens nothing.
- **Process:** one omp per Ask more conversation. It is started by the first message, never at opening, and shut down as in `## job` when the conversation is closed, when the window closes and when Weaver quits. After it stops, the next message starts it again.
- **Command line:** `<omp> [--model <analysis>] --approval-mode <mode> acp`, with the analysis model of the Settings, trimmed, and never a Proposal's chosen model. No `--append-system-prompt`, `--advisor`, `--slow` or overlay.
- **Environment:**
  - `WEAVER_ASK=<conversation identity>` and `WEAVER_ENDPOINT`, with no `WEAVER_JOB` and no `WEAVER_GENERAL`, so `weaver` refuses job-bound commands and `weaver question new` answers `unavailable`.
  - `weaver job new` is accepted from the identity of an open Ask more conversation of the project while it is Working, with the same refusals as the developer's request. The started job is shown in that conversation.
  - PATH and the certificate variables as in `## omp`.
- **First prompt:** one `session/prompt` with one text block: `Instructions from Weaver (not written by the developer):`, a new line, the instruction, an empty line, `Developer's message:`, a new line, the developer's message trimmed as every message. Later messages are sent alone.
- **Session:**
  - The session id stays in memory with the conversation.
  - A later start reloads it with `session/load`, within the 30 s limit and with the replay ordered as in `## general-chat`. In the replay, the first user message shows only the text after `Developer's message:`.
  - If the load fails, an empty session starts and the notice of Chat shows. The message that opens it carries the instruction built again from the files at that moment (for the Requirements, with the overlay Proposal only if it is still open), or goes alone when the chosen thing no longer exists. When no session was ever opened, the instruction prepared at opening goes with the next message.
- **Forms and permissions:** forms are declined with a text telling the agent to ask in its reply, naming Ask more, not General. Permissions work as in `## job`, without notifications.
- **Sub-agents:** read, listed and shown as in `## job` (**Sub-agents**), keyed by the Ask more conversation.
- Neither the instruction nor any prompt is written to disk or logged.
- **Tests:**
  - With the fake ACP agent: nothing is sent at opening; the first prompt carries the instruction and the message, later prompts carry the message alone; closing before the first message starts no omp; a failed reload sends the instruction again with the message.
  - Through the app's endpoint: `weaver job new` is accepted from a working Ask more identity and refused from an idle or closed one.
