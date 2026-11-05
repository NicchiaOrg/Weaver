---
name: weaver-setup
description: Use when the instruction names a written setup request for starting a project from scratch (a file `.weaver/requests/<id>.md`) and the Model of the project with a `.weaver/` folder is empty. Interviews the developer about the stack, release, data and tests, then writes the first Proposal (kind setup) with the Model, Requirements, Specs and the Tasks that build the project. Not for writing code.
---

# Weaver: setup

This skill starts an empty project. The **Model** in `.weaver/model/` is still empty; the first **Proposal** of the project carries its first Model, its Requirements, its **Specs** and the Tasks that build the project skeleton. You never write code in this skill, and you never edit `.weaver/model/`, `.weaver/requirements/` or the generated `weaver-req-*` skills directly: every change goes through the Proposal, created with `weaver proposal new --input -` and `"kind":"setup"`.

For the format of Model and Requirement files, read [FORMAT.md](../weaver-design/FORMAT.md) of the `weaver-design` skill (installed next to this skill); do not copy it here.

## Hard rules

- Agents may approve, reject or discard a Proposal, decide a Drift or delete a Task in To do (`weaver task delete`) only when the developer explicitly asks for that decision in the conversation. Without that request, do not decide and do not ask the developer about it: stop and tell them the decision is theirs, on the Board. A setup request does not by itself authorise any of them. Approval on hand-over is Weaver's, not yours: hand the Proposal over and stop.
- The Model describes *what can happen*, never *how it is built*: no variables, field names, database columns, endpoints, classes or file paths in Model files. Technology, layout, release and data choices belong in Requirements and Specs.
- In every command and every file use only the **permanent identity** of a card (from `--json`, field `id`, or from the file name). Never write a short number (`P3`, `T3`) in a file.

## Naming cards

Every live Proposal, Task and Drift has a temporary **short number** (`P3`, `T3`, `D3`) besides its **permanent identity** of eight characters.

- Name cards to the developer by short number and title, for example "Proposal P3 – title", also in the final report. A closed card has no number: use its permanent identity.
- In every command you run and every file you write use **only the permanent identity**, taken from `--json` (field `id`; the number is the field `number`) or from the file or folder name. **Never write a short number in a file**: it changes from one working copy to another.
- Give every card you create, and its announcement, a **short title**: at most five words, about 40 characters, a name for the card rather than a sentence describing the work (for example "Ghost text" instead of "Describe the start screen and guide written requests with a ghost text"). Put the detail in the body, even when the title loses information. Only the title of an announcement is checked (one line, at most 60 characters). Titles that a skill composes from another title stay as they are: `Import: <Feature>`.

## Notes on the job

The developer follows the work through notes: `weaver job edit --input -` with `{"note":"<one line>"}`, passed as `weaver job edit --input - <<'JSON'`, the object and `JSON`. Declare a note at the start of each stage of the work: reading the request, each round of questions, writing the Proposal, handing over. Write 3 to 6 notes for the whole job, never one per command. A note is one sentence of at most 120 characters (a longer one is cut) saying what the agent does and why, for the developer: no first person, no `weaver` command names and no short numbers of cards, in the language of the instruction when it asks for one (for example "Explores the order Machine before asking the first questions"). A note changes nothing else and is never needed to deliver a card.

## Language

When the instruction asks for a language, the texts listed below go in that language. Without a language in the instruction, nothing here changes.

- **In the language of the instruction:** the titles and bodies of Tasks; the titles, bodies and Fix options of Drifts; the titles, bodies and reasons of Proposals; the text, context and options of questions; the notes of the job and the titles of announcements; the titles, summaries and texts of History entries; the replies during the work and the final report.
- **Always in English**, whatever the language: the whole Model (the name of the System; the names of Features, Machines, Actors and States; IDs and Event names; the descriptions in Model files); the Requirements (Areas, titles and bodies of fragments); the Specs, also their `notes`; code and its comments, commit messages and the project's own documentation; the headers Weaver reads in files (`States`, `Events`, `General`, `What happens`, `What should happen`, `Where in the code`, `Fix options`, `Already present`), the `(initial)` and `(final)` markers and the front matter fields; technical terms (commands and tools such as `git pull` or `npm install`, names of products, services, libraries and languages) and Weaver's own terms (Proposal, Task, Drift, Board, Run, Scan). Weaver's interface and the `weaver` command line stay in English.
- **A language changes nothing already written.** When you edit a file written in another language, keep its existing text and write only the new or rewritten text in the chosen language; never translate the whole file.
- **No first person in Weaver's texts.** In what you write for Weaver (questions, contexts and options of rounds, titles and bodies of Proposals, Tasks and Drifts, History entries), Weaver is always "Weaver" and you are "the agent": no first-person pronouns or first-person verbs, in any language. Your replies in the conversation are not Weaver's texts and do not change. Text already written is not rewritten.
- **Self-contained Tasks.** The body of a Task is understood without the conversation that produced it: it says what must be true when done, with the conditions and consequences as the Model says them, and every decision of the interview that concerns it, because whoever implements it does not see that conversation.

## Announcing the Proposal

Right after reading the request, announce the Proposal you will write: `weaver announcement new --input -` with `{"kind":"proposal","title":"<title>"}` prints an announcement ID, which you pass as `announcement` to `weaver proposal new` in step 4. The Board shows a ghost of the card meanwhile. Never wait for the ghost to show; an unknown ID is only noted in the output.

## Step 1: read the request

The instruction names a file `.weaver/requests/<id>.md`. Read the **whole** file before anything else. If it does not exist, tell the developer that the request is gone and stop, without opening any Proposal.

The request is the starting point of the interview, never its answers. Every decision it makes still becomes a question with its options, the option the request indicates being your recommendation.

Look at every image the request links, with the same rules as `weaver-design`: local files and web addresses you can open, images you cannot see ignored silently, what matters written in words, and no copy, link or citation of the request's images, or their paths, in the Proposal, the Model, the Requirements, the Specs or the Tasks.

**Images for the Tasks** work as in `weaver-design` ("Images for the Tasks"): when the request links at least one local image you can see, the first round has one single-choice yes/no question about sending them also to the model that implements the Tasks; with yes, bind each local image to at least one Task that needs it with `weaver proposal edit <id> --input -` and `{"images":[{"task":"<task identity>","path":"<image path>"}]}` before the hand-over (which deletes the request's images folder); with no, bind none. Images linked by a web address never count.

## Step 2: check the Model is empty

Run `weaver model show` and `weaver model list --input -` (with `{"text":"<word>"}` in a heredoc) only as needed. If the Model already has a Feature or an Actor, stop and tell the developer that `weaver-design` is the skill for it: `weaver proposal new` with `"kind":"setup"` would be refused.

## Step 3: interview in rounds

Ask in rounds with `weaver question new --input -` (the JSON in a heredoc), as "Asking" below says: one round per call, 2 to 5 options per question with tradeoffs, your recommendation on each, wait for the answers, no "Other", no fallback when the questions are lost.

Ask only what the request leaves ambiguous and never repeat what is clear. When in doubt, ask one question more. Cover, when not already settled:

- **What the developer wants**: the purpose and the first behaviour of the System.
- **Languages and frameworks.**
- **Release**: desktop application, web or other; repository and where it lives (for example GitHub); cloud (Azure, AWS, Google Cloud) or on premise; for cloud, which resources; infrastructure files (Terraform, Bicep, or none, with a manual release).
- **Data and persistence.**
- **Tests and CI.**

A technology the developer names vaguely ("a web framework used today") you choose yourself, without asking. Record it in the Specs with `chosenByAgent` giving the reason.

## Step 4: write one Proposal

Open exactly one Proposal, bound to the request:

`weaver proposal new --input -` with `{"title":"<title>","kind":"setup","body":"<why>","label":"<short label>","request":"<id>","announcement":"<id>"}`

Give the Proposal a short title (see Naming cards) and a `label` of at most 30 characters (for example the project name), because it will carry Tasks.

The body says why and summarises each decision of the interview, one line each, with the assumptions made for skipped questions. Then:

1. **Model**: System, Features, Machines and Actors as they are agreed, each through `weaver proposal edit <id> --input -` with `{"edit":["<model-path>"]}`.
2. **Requirements**: the Areas and fragments the developer gave you, through `{"edit":["requirements/<area>/…"]}` (see FORMAT.md of `weaver-design`).
3. **Specs**: every field clarified in the interview, with `chosenByAgent` and the reason for each technology you chose yourself.
4. **Tasks**: with `{"edit":["tasks/<slug>.md"]}`, one per piece of work. Include the technical Tasks that create the project skeleton, the repository, the CI and the infrastructure files chosen (`technical: true`, no Model element). Non-technical Tasks name at least one Model element, as in `weaver-design`, and every Task is self-contained. With sub-agents, hand the Tasks of each Machine group to one sub-agent, at most N at once, where N is the limit in the instruction ("Run at most <N> sub-agents at once"), as `weaver-design` says in its step 5.

The hand-over below reads its own output: it must match what was agreed and nothing else.

## Step 5: hand over

Run `weaver proposal move <id> open` without `weaver proposal show` before it. If it is refused, or its output does not match what was agreed, reopen the Proposal with `weaver proposal edit <id>`, fix it, and hand over again.

A setup on an empty Model is approved by Weaver at hand-over when nothing blocks it. After a successful hand-over, read the result of `weaver proposal move <id> open`:

- If it says the Proposal was **approved**, tell the developer that the Proposal is approved and that its Tasks start.
- If it says the Proposal was **not approved** (it stays open), tell the developer that it was handed over and why it was not approved on its own, from the hand-over output, and to review it on the Board.

Do not ask the developer to review an approved Proposal. Do not approve, reject or discard it yourself.

## Written design request

The request is consumed by the successful hand-over of the bound Proposal: its file is deleted then. Do not delete the file yourself.

## Reworking a rejected setup Proposal

When the instruction says to rework the rejected Proposal `<id>` with the developer's feedback, follow this skill with these changes; everything else stays as written:

- Read the rejected Proposal with `weaver proposal show <id>` (body, decisions, files, Tasks and `## Feedback`) instead of a request file. Its decisions count as already taken, except what the feedback changes or leaves in doubt. Without `## Feedback`, redo the same setup on the Model as it is now. Step 2 still holds: the Model must be empty.
- The interview is optional: one round of `weaver question new` only for the decisions the feedback opens or leaves open.
- Write the setup Proposal again from scratch, without requesting the rejected one. Bind it with `"reworks":"<id>"` in `weaver proposal new` (never together with `request`); Weaver links it only when the named Proposal is rejected and no other one already reworks it, otherwise it creates it unlinked and the result says why.
- When the rejected Proposal's Tasks had images, bind them to the new Tasks that need them, copying each from the old Task (the path `weaver task show` prints) with `images` in `weaver proposal edit`, without asking the developer again.
- After the hand-over, report its result as Step 5 says. Never approve, reject or discard it yourself.

## Refining a stale setup Proposal

This section applies when the instruction says to refine the stale Proposal `<id>` on the current Model and Requirements, which is the case of a setup Proposal while the Model is empty. Without that sentence, skip it. The rest of this skill (request, interview, announcing, opening a new Proposal) does not apply.

Refining reworks the same Proposal. It keeps its identity, short number, place on the Board, label, chosen model, the images of its Tasks and the Proposals that require it, and it stays a setup: its approval still starts the Run of all the Tasks. You never approve it. At the hand-over Weaver decides where it goes: a Proposal attached to a Drift always goes back Open, and the developer accepts it again from the Drift; a Proposal blocked by a required Proposal that is not approved yet goes back Open; otherwise Weaver approves it (or waits for the end of a Run first).

1. **Read what the Proposal changes.** Run `weaver proposal show <id>`: it gives the body with the decisions, the Tasks, the stale files and their differences from the files as they are now. Then read only the current Model and Requirements of those stale files. Do not read the code, do not archive Tasks or Drifts outside the Proposal, and announce nothing: the Proposal already exists.
2. **The decisions in the body count as taken.** The developer answered them in the interview. Ask nothing yet: a decision that the current Model contradicts and that you cannot choose yourself goes into the single round of step 4, asked after the rebase (see "Asking"). Decide everything else, and write each such decision in the body as an assumption.
3. **When the current Model already has the whole change,** reconcile nothing. Ask one round with one question that explains which approved Proposal already contains the change, with two options: `Remove` (description: "Reject this Proposal: the current Model already has its change.") and `Keep` (description: "Leave it as it is, to refine; nothing changes."), recommended `Keep`. Name that Proposal to the developer by its short number and title.
   With `Remove`, reject it: `weaver proposal move <id> rejected --input -` with `{"feedback":"Already in the current Model."}`, in a heredoc. That is the developer's decision, taken in the round, and the only case in which you reject a Proposal.
   With `Keep`, or when the round is skipped or lost, stop and change nothing.
4. **Otherwise, reconcile the Proposal with the current Model.** Run `weaver proposal edit <id> --input -` with `{"rebase":true}`: the Proposal goes back to Draft and is marked `refining` until you hand it over again. The rebase also drops the broken entries of `requires` (required Proposals that were rejected, discarded or are missing, and, in a cycle, only this Proposal's own entry) and lists each one in its output. Adapt the change without them. Then, when a decision of step 2 needs the developer, or something of a dropped Proposal was needed and you cannot choose yourself, ask one single round of `weaver question new` with all those questions; never a second round, and no round at all when nothing is needed.
   Rewrite in full each file the Proposal touches (Model, Requirements, Specs, Tasks), as the current file plus the Proposal's change, never removing what other approved Proposals changed. Update the Tasks and `## Already present` as `weaver-design` says, and add to the body one line for each thing the refinement changed.
5. **Hand over and stop.** Run `weaver proposal move <id> open`, as Step 5 says. If the hand-over is refused (invalid Model, scope not covered, Tasks that name missing references, a missing label, code fixes that cannot be applied), correct what it reports with `weaver proposal edit <id>` and hand over again. Report its result as Step 5 says, and never approve, reject or discard the Proposal yourself.
6. **A Draft left half-way through a refinement** (still marked `refining`) resumes from there: reread its files, reconcile again as in step 4 if it is stale again, and go on.

## Asking

Run **`weaver question new --input -`** with the JSON request on standard input, in a heredoc, for every round of questions. It is the only way to ask the developer: never use omp's own question tool and never write the questions in the conversation.

- Send one complete round as `{"questions":[...]}`. Every question has a unique non-empty `id`, `question`, `options` as `{label, description}` (2 to 5 of them), a zero-based `recommended` index, and an explicit `multi` boolean. Two fields are optional: `context` on a question and `details` on an option; when present they must not be blank.
- Keep `question` short: one sentence that reads on its own. Put the background of the decision in `context`, the tradeoff in a few words in `description`, and a longer explanation, only when it is needed, in `details`. `question`, `context`, `description` and `details` are Markdown; labels are plain text. Set `multi: true` only when several answers can genuinely hold together. Do not supply "Other": Weaver always offers a free-text answer.
- The developer sees only the round (each question, its context, its options and their own answers), never your conversation. Make every question self-contained: it says everything needed to decide, with no references to the conversation or to another question, and no first person (write "Weaver" or "the agent"). Omit `context` when the question and the options are enough on their own.
- The result is JSON on stdout. With `status: "answered"`, `answers` is keyed by question id: each entry is `{"status":"answered","selected":[<labels>],"text":<optional free text>}` or `{"status":"skipped"}`. A skipped question is an explicit developer skip: use its recommendation for that decision, report it as an assumption for review, and do not ask it again.
- With `status: "unavailable"`, Weaver cannot show questions: ask in no other way, open no Proposal, and tell the developer. When the developer says to continue, run the same `weaver question new` again: it is a new round.
- An error `interrupted` or `cancelled` (exit 1) stops this work at once: no partial answers, no recommendations, no retry, no fallback, no resuming of the round; tell the developer that the questions were lost.
- An error `invalid-input` (exit 2) means the request was refused before opening a round: correct the field it names and submit the corrected request. A usage error is never unavailability.
- **Wait for `weaver question new` to finish**: it blocks until the developer answers, however long that takes. Run it with bash `timeout: 0`, never a fixed value. Never use a timer, `sleep`, periodic reads of the output or interval checks; never end the turn to be woken up; never treat the command as failed and never run it again.

## Sub-agents context sharing

When you hand a part of the work to a sub-agent, in whatever way your agent starts sub-agents, you pass it what you have already read that concerns its part, so it does not look for it again and does not read it again.

- **What you pass.** Of what you have already read, only what concerns that sub-agent's part, plus a short common part (the request and the decisions, when there are any): Weaver's texts quoted as they are (the written request with its images put in words, the output of `weaver model show`, `weaver task show`, `weaver drift show`, `weaver proposal show`, the decisions of the interview) and the places in the code already known, one line per reference, ``- `<ref>` — <where in the code>``. Never code, never everything you gathered, and nothing you read only to pass it on.
- **What the sub-agent does.** It does not reread the Weaver texts it received and reads on its own only the ones that are missing. It always reads the code, also at the places it received, always runs the commands that register its work (`weaver task move <id> doing`, `weaver task new`, `weaver drift new`) and loads its skill as without this feature. A received place that no longer matches the code is ignored: it searches by itself. Context sharing never stops a job.
- **What comes back.** Each sub-agent reports where it found the code of what it saw, one line per reference as above; you reuse those lines in your own work and for the sub-agents that follow.
- **Limits.** No sub-agent runs again a read-only `weaver` command for a text it received, and what you pass to each one concerns only its part: never copy into every sub-agent all that you gathered.

In this skill, a sub-agent that writes the Tasks of a Machine group receives the decisions of the interview and the new text of the Model and of the Requirements for its references.

## Command reference

Every `weaver` command, with its verbs and input fields, is in [COMMANDS.md](../weaver-design/COMMANDS.md), generated from the command registry. `weaver <object> <verb> --help` prints the usage, the fields with their types and limits, an example and the output fields.

## PowerShell

The examples use POSIX syntax. In PowerShell, pass JSON with a pipe and `--input -`, for example `Get-Content -Raw round.json | weaver question new --input -`, instead of `<` or a heredoc. On Windows PowerShell 5.1, set `$OutputEncoding` and `[Console]::OutputEncoding` to UTF-8 first, or non-ASCII characters arrive as `?`. Never pass the JSON as the text of `--input`: Windows PowerShell 5.1 strips the inner quotes of arguments to external programs.
