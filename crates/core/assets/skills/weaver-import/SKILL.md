---
name: weaver-import
description: Use when the instruction asks to import, reverse-engineer or describe existing code as a Model, in a project with a `.weaver/` folder whose Model is empty. Reads the code and writes Proposals for the Model and its evident Requirements, describing the code as it is. Does not change code.
---

# Weaver: import

You describe the **existing code** as a **Model** (what the System does) plus its evident **Requirements** (how it is built), written as **Proposals** with `kind: "import"`. The Model you write describes the code **as it is**, bugs included; everything that looks wrong or that the documentation contradicts goes into the Proposal as a **doubt** for the developer, never silently "fixed". An import carries no Tasks.

For the format of Model, Requirement and Specs files read [FORMAT.md](../weaver-design/FORMAT.md) of the `weaver-design` skill, installed next to this skill.

## Hand-over

A new Proposal starts as a **Draft**, not approvable. Finish every file, then run `weaver proposal move <id> open` without `weaver proposal show` before it: read the changes in the hand-over's output. If they do not match what you meant, reopen the Proposal with `weaver proposal edit <id>`, fix it, and hand it over again. If the hand-over is refused, fix the reported problems and hand it over again. Hand the Proposals over in the order of their `requires`, the ones others build on first. Before every later change, call `weaver proposal edit <id>` again: it returns an open Proposal to Draft, so hand it over again when finished.

## Announcing cards

Once you have planned the Model (step 4), announce every Proposal you will write, each as soon as you know its title: `weaver announcement new --input -` with `{"kind":"proposal","title":"<title>"}` prints an announcement ID. Pass it when you open that Proposal: `weaver proposal new --input -` with `{"title":"<title>","kind":"import","announcement":"<id>"}`. The Board shows a ghost of each Proposal meanwhile. Never wait for a ghost to show and never skip or delay a Proposal because of it; an unknown ID is only noted in the output.

Announce every Drift you will file the same way, before you file it: `weaver announcement new --input -` with `{"kind":"drift","title":"<title>"}` prints an announcement ID, and `weaver drift new --input -` takes it in `announcement`.

## Hard rules

- NEVER edit code, `.weaver/model/` or `.weaver/requirements/` directly. Everything goes through Proposals opened with `kind: "import"`.
- Agents may approve, reject or discard a Proposal, decide a Drift or delete a Task in To do (`weaver task delete`) only when the developer explicitly asks for that decision in the conversation. Without that request, do not decide and do not ask the developer about it: stop and tell them the decision is theirs, on the Board. An import request does not by itself authorise any of them.
- **The code wins.** Model what the code does. Where the documentation (README, `docs/`, comments, API descriptions) says something else, model the code and file a Drift for that difference, as described in "Drifts for documentation differences". Where the code looks buggy or misses a case that the documentation does not mention, model it as it is and record a doubt.
- **Never change what is already modelled.** The one exception is the Specs, which the import updates (see "Specs"). If the Model already has Machines, do not edit their files. You may add new Features, Machines, Actors, Events of existing Actors and Requirement sections. If the code contradicts an existing Machine, do not fix it: list it in your report and suggest a Scan.
- The Model has no code words: no variable, field, function, class, table, endpoint or file names in States, Transitions or Events. Write conditions and consequences in domain words. Code locations belong only in the doubts and in the `where` of a Drift.
- Import only **evident and stable** Requirements: `tech` (constraints and structure only: layers, platform, build and release rules written in the files; never the list of languages, frameworks and libraries, which belongs to the Specs), `data` (main entities and their fields, from schemas or models), `rules` (formulas, limits and policies written explicitly in the code). Never `ui` or `quality`: the code does not tell what was intended there.

## Naming cards

Every live Proposal, Task and Drift has a temporary **short number** (`P3`, `T3`, `D3`) besides its **permanent identity** of eight characters.

- Name cards to the developer by short number and title, for example "Proposal P3 – title", also in the final report. A closed card has no number: use its permanent identity.
- In every command you run and every file you write use **only the permanent identity**, taken from `--json` (field `id`; the number is the field `number`) or from the file or folder name. **Never write a short number in a file**: it changes from one working copy to another.
- Give every card you create, and its announcement, a **short title**: at most five words, about 40 characters, a name for the card rather than a sentence describing the work (for example "Ghost text" instead of "Describe the start screen and guide written requests with a ghost text"). Put the detail in the body, even when the title loses information. Only the title of an announcement is checked (one line, at most 60 characters). Titles that a skill composes from another title stay as they are: `Import: <Feature>`.

## Notes on the job

The developer follows the work through notes: `weaver job edit --input -` with `{"note":"<one line>"}`, passed as `weaver job edit --input - <<'JSON'`, the object and `JSON`. Declare a note at the start of each stage of the work: reading the code, dividing it into Features, writing the Proposals. Write 3 to 6 notes for the whole job, never one per command. A note is one sentence of at most 120 characters (a longer one is cut) saying what the agent does and why, for the developer: no first person, no `weaver` command names and no short numbers of cards, in the language of the instruction when it asks for one (for example "Explores the order Machine before asking the first questions"). A note changes nothing else and is never needed to deliver a card.

## Language

When the instruction asks for a language, the texts listed below go in that language. Without a language in the instruction, nothing here changes.

- **In the language of the instruction:** the titles and bodies of Tasks; the titles, bodies and Fix options of Drifts; the titles, bodies and reasons of Proposals; the text, context and options of questions; the notes of the job and the titles of announcements; the titles, summaries and texts of History entries; the replies during the work and the final report.
- **Always in English**, whatever the language: the whole Model (the name of the System; the names of Features, Machines, Actors and States; IDs and Event names; the descriptions in Model files); the Requirements (Areas, titles and bodies of fragments); the Specs, also their `notes`; code and its comments, commit messages and the project's own documentation; the headers Weaver reads in files (`States`, `Events`, `General`, `What happens`, `What should happen`, `Where in the code`, `Fix options`, `Already present`), the `(initial)` and `(final)` markers and the front matter fields; technical terms (commands and tools such as `git pull` or `npm install`, names of products, services, libraries and languages) and Weaver's own terms (Proposal, Task, Drift, Board, Run, Scan). Weaver's interface and the `weaver` command line stay in English.
- **A language changes nothing already written.** When you edit a file written in another language, keep its existing text and write only the new or rewritten text in the chosen language; never translate the whole file.
- **No first person in Weaver's texts.** In what you write for Weaver (questions, contexts and options of rounds, titles and bodies of Proposals, Tasks and Drifts, History entries), Weaver is always "Weaver" and you are "the agent": no first-person pronouns or first-person verbs, in any language. Your replies in the conversation are not Weaver's texts and do not change. Text already written is not rewritten.

## Steps

1. **Read what is modelled.** `weaver model show`, and `weaver model show <feature>` for each existing Feature. Note what is already covered.
2. **Survey the code yourself.** Manifests and configuration, entry points, routes and handlers, status fields and enums, workflows, jobs and timers, webhooks and integrations, screens and navigation, schemas, tests (they often state the intended behaviour), and the documentation.
3. **Ask one round of questions, then work alone.** The instruction always says which folders to import: it says "Do not import these paths and do not ask which folders to import" followed by the paths, or "Import every folder and do not ask which folders to import". The developer chose them in the setup wizard: never ask which folders to import and do not read the excluded paths. In one call of `weaver question new` (see "Asking" below) ask only how to split the System into Features (offer your own split as the recommended option, plus one or two alternatives) and any other question whose answer changes the whole import. After the answers, do not ask again: record the remaining uncertainty as doubts. List the excluded paths among the folders you skipped in your report.
4. **Plan the Model.** One Machine per kind of thing with a lifecycle (an order, a payment, a subscription, a document), its States taken from the code's status values and the situations its handlers distinguish, its Transitions from the code paths that move between them, and the guards and side effects as prose. Who triggers what from outside (people, other systems, webhooks, time) are Actors that emit Events. Screens and navigation become a UI Machine only when the app has real navigation flows. Follow the format in [FORMAT.md](../weaver-design/FORMAT.md).
5. **First Proposal: the System and its Actors.** `weaver proposal new --input -` with `{"title":"Import: System and Actors","kind":"import","body":"<what was imported and from where>","label":"Import","announcement":"<id>"}`. Write `system.md` (only if it still has placeholder text or does not exist), every new Actor with the Events it emits, the General sections of `tech` and `data`, and the Specs (see "Specs"). Requirements live in Area folders: `area.md` for the Area's description and title, fragments `requirements/<area>/<fragment>.md` for the sections (see "Requirements" in FORMAT.md). On a partly modelled project, skip this Proposal if nothing is new, Specs included; if only the Specs are new or changed, it carries only them.
6. **One Proposal per Feature.** `weaver proposal new --input -` with `{"title":"Import: <Feature>","kind":"import","requires":["<first Proposal ID>","<ID of another Feature's Proposal whose Events this one reacts to>"],"announcement":"<id>"}` (the second entry is optional). Write `<feature>/feature.md`, its Machines, and the `data` and `rules` sections attached to them (`## order`, `## order/Paid`), in a fragment of the Area that suits that Feature, usually one fragment per Feature with a description saying when it is needed. Each Proposal is written and validated on top of the ones it requires, so only list Proposals you actually build on.
7. **Record the differences.** File one Drift for each difference between the documentation and the code, as described in "Drifts for documentation differences". Then end the body of each Proposal (`body`, or edit `proposal.md`) with a `## Doubts` section, one line each:
   - code that looks like a bug, a missing case or dead code, with its location, where the documentation does not say otherwise;
   - behaviour you could not determine;
   - an existing Specs entry that the code no longer uses (it stays in the Specs).
   Write "None." if there are none. Documentation differences are Drifts, never doubts.
   Give each Drift a short title (see Naming cards).
8. **Hand every Proposal over.** Run `weaver proposal move <id> open` for each, the Proposals others require first, without `weaver proposal show` before it: its checks report the problems and its output prints the changes, which must describe the code and nothing else. When it warns of a fragment larger than 12 KB that the Proposal writes into, split that fragment in the same Proposal and hand it over again: cut by Feature or by a group of nearby Machines, give every new fragment a name and a description that say when it is needed, move the sections without changing their text, and prefer a few medium fragments to many tiny ones (every fragment counts against the 4,000-character budget of the Requirement skills). A dependency need not be approved for the hand-over. If a hand-over is refused or its output does not match, reopen the Proposal with `weaver proposal edit <id>`, fix it and hand it over again.
   An import on an empty Model is approved by Weaver at hand-over when nothing blocks it: read each result and tell which Proposals were approved and which stayed open, with the reasons it gives.
9. **Prepare the questions of the Drifts and report.** Follow "Preparing the questions" below, then report: the Proposals in the order to approve them (those that stayed open), with how many doubts each has, whether the Specs were written or updated, the Drifts you filed (each named by short number and title), the differences you could not file yet, the folders you skipped, and any contradiction with Machines that were already modelled (suggest a Scan). Approving an import Proposal starts no Scan: the Model already matches the code.

## Reworking a rejected import Proposal

When the instruction says to rework the rejected Proposal `<id>` with the developer's feedback, follow the Steps with these changes; everything else holds:

- Read the rejected Proposal with `weaver proposal show <id>` (body, decisions, files and `## Feedback`). Its decisions count as already taken, including the folders and the split into Features, except what the feedback changes or leaves in doubt. Without `## Feedback`, redo the same import on the Model as it is now.
- The round of step 3 is optional: ask one round of `weaver question new` only for the decisions the feedback opens or leaves open; otherwise work alone.
- Write the Proposal again from scratch, on the Model as it is now, without requesting the rejected one; the rework may be split into several Proposals, as an import is. Pass `"reworks":"<id>"` only in the first `weaver proposal new` of the rework (never together with `request`); Weaver links it only when the named Proposal is rejected and no other one already reworks it, otherwise it creates it unlinked and the result says why.
- After the hand-over, tell the developer to review the new Proposals that stayed open. Never approve, reject or discard them yourself.

## Refining a stale import Proposal

This section applies when the instruction says to refine the stale Proposal `<id>` on the current Model and Requirements. Without that sentence, skip it. The Steps above (survey, round of questions, planning, new Proposals, Drifts) do not apply: only this Proposal is refined.

Refining reworks the same Proposal. It keeps its identity, short number, place on the Board and the Proposals that require it. It stays an import and carries no Tasks. You never approve it. At the hand-over Weaver decides where it goes: a Proposal attached to a Drift always goes back Open, and the developer accepts it again from the Drift; a Proposal blocked by a required Proposal that is not approved yet goes back Open with its reasons; otherwise Weaver approves it (or waits for the end of a Run first).

1. **Read what the Proposal changes.** Run `weaver proposal show <id>`: it gives the body with the decisions and the doubts, the stale files and their differences from the files as they are now. Then read only the current Model and Requirements of those stale files. Do not read the code, file no Drift, and announce nothing: the Proposal already exists.
2. **The decisions in the body count as taken,** including the split into Features. Ask nothing yet: a decision that the current Model contradicts and that you cannot choose yourself goes into the single round of step 4, asked after the rebase (see "Asking"). Decide everything else, and write each such decision in the body as an assumption.
3. **When the current Model already has the whole change,** reconcile nothing. Ask one round with one question that explains which approved Proposal already contains the change, with two options: `Remove` (description: "Reject this Proposal: the current Model already has its change.") and `Keep` (description: "Leave it as it is, to refine; nothing changes."), recommended `Keep`. Name that Proposal to the developer by its short number and title.
   With `Remove`, reject it: `weaver proposal move <id> rejected --input -` with `{"feedback":"Already in the current Model."}`, in a heredoc. That is the developer's decision, taken in the round, and the only case in which you reject a Proposal.
   With `Keep`, or when the round is skipped or lost, stop and change nothing.
4. **Otherwise, reconcile the Proposal with the current Model.** Run `weaver proposal edit <id> --input -` with `{"rebase":true}`: the Proposal goes back to Draft and is marked `refining` until you hand it over again. The rebase also drops the broken entries of `requires` (required Proposals that were rejected, discarded or are missing, and, in a cycle, only this Proposal's own entry) and lists each one in its output. Adapt the change without them. Then, when a decision of step 2 needs the developer, or something of a dropped Proposal was needed and you cannot choose yourself, ask one single round of `weaver question new` with all those questions; never a second round, and no round at all when nothing is needed.
   Rewrite in full each file the Proposal touches, as the current file plus the Proposal's change, so that the same change is made on the Model as it is now; the Specs too, when it writes them. Never remove what other approved Proposals changed. Keep its `## Doubts` section, and add to the body one line for each thing the refinement changed.
5. **Hand over and stop.** Run `weaver proposal move <id> open`, as the Steps say. If the hand-over is refused (invalid Model, scope not covered, a missing label, code fixes that cannot be applied), correct what it reports with `weaver proposal edit <id>` and hand over again. Tell the developer which result it gave, and never approve, reject or discard the Proposal yourself.
6. **A Draft left half-way through a refinement** (still marked `refining`) resumes from there: reread its files, reconcile again as in step 4 if it is stale again, and go on.

## Specs

The first Proposal describes the technologies of the code in the Specs, as `weaver-setup` does, but deduced from the repository. Write `specs.yaml` with `weaver proposal edit <id> --input -` and `{"edit":["specs.yaml"]}`, in the format of "Specs" in [FORMAT.md](../weaver-design/FORMAT.md).

- **Source.** Fill the fields from what the repository declares: manifests, lockfiles, test and CI configuration, Dockerfiles, infrastructure files. A field that no file declares stays empty.
- **Entries.** None has `chosenByAgent`. The version is only the major one declared. `frameworks` lists only frameworks and libraries that shape the code. `notes` carries only constraints written in the files.
- **Specs already present.** Update them: add missing entries and fields, and correct the version and note of existing entries. An entry the code no longer uses stays, and goes among the `## Doubts` of the first Proposal.

## Drifts for documentation differences

A difference between the documentation and the code is a **Drift**, not a doubt: the developer decides, for each one, whether the code or the documentation is right. File it once with `weaver drift new --input -`, without `task`, and with `proposal` only in the case below: no Proposal fixes it and no Task implements it otherwise.

- Before filing, run `weaver drift list --input -` with `{"all":true,"refs":["<ref>"]}` for each element or Requirement it concerns. A Drift the developer dismissed, whose text has not changed since, is marked in that list: never file that difference again. A difference closed by a fix of the code or by an accepted Proposal stays closed while the Model or the Requirement there still says the same thing; if it is back, set `previous` to the ID of that resolved Drift (the command refuses a previous Drift that is open or unknown).
- `refs` names an element or a Requirement section that already exists in the Model or the Requirements, because `weaver drift new` refuses a reference that does not resolve. Name the nearest one that does, for example the Area's section for a stated rule. If the difference is about an element this import is still to create, record it as a doubt in its Proposal and list it in the report.
- **No fix respects both.** When no fix of the code respects the Model and the Requirements together (for example, two Requirements contradict each other), file the Drift with no `options` and with `proposal` set to a Draft that repairs the Model or the Requirements: a design Proposal, `weaver proposal new --input -` with `kind` `"change"` (not `"import"`), written as `weaver-scan` describes under "Drift without Fix options, with a Proposal". A Drift without Fix options is refused without a Proposal.
- The command: `weaver drift new --input - <<'JSON'` and `JSON`, with `{"title":"<title>","refs":["<ref>"],"happens":"<what the code does>","should":"<what the documentation says and why it differs>","options":[{"title":"<title of the way you recommend>","text":"<what it changes and costs>"},{"title":"<title of another way>","text":"<…>"}],"where":"<where in the code>","announcement":"<id>"}` (up to five options). Re-read the code at the `where` place first, and file only if it really does what `happens` says. The two texts, `where` and two to five options, the recommended one first with different titles, are required, except in the no-fix case above (there `where` is still required, and no option is passed).
- Write both sides and the options at the level of behaviour: what the System does and what the documentation promises, in the words of the domain, not how the code does it. Name no variable, function, class, field or file unless the difference cannot be understood without it. Code locations go only in `where`, the files and no line numbers. Use short Markdown: short paragraphs, a bulleted list for several points, **bold** for the terms that carry the difference, about 100 words per side and about 60 per option.
- Fix options reach what should happen without changing the Model or any Requirement, and respect every Requirement that applies to the elements of the Drift, not only those it names. An option that asks for an exception to, or a change of, the Model or a Requirement is not a fix option.

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

## Preparing the questions

At the very end of the work (before the final report) prepare the question of every Drift this work filed, so Resolve opens at once. You alone do this, never a sub-agent that compares or implements.

1. **Find the Drifts.** Run `weaver drift list --input -` with `{"currentJob":true,"unprepared":true}`; never read `WEAVER_JOB` by hand: the command reads it itself. It lists the open Drifts of this job, the advisor's included, with no valid prepared question and outside every resolution. When it lists none, skip the rest.
2. **Prepare each.** Follow "Writing a prepared question" in the `weaver-resolve-drifts` skill (read it first) for each Drift, with the sub-agents you have: hand the exploration to at most N of them at once (N is the limit in the instruction), one Drift (or a few that concern the same thing) each, and have each record its question with `weaver drift edit`. Without sub-agents, one Drift at a time. A sub-agent asks the developer nothing and registers no other card.
3. **Report.** The final report says how many Drifts were prepared and which were not (with the reason).

## Boundary

Import describes existing code and only creates import Proposals and the Drifts of "Drifts for documentation differences"; it never resolves Drifts or runs implementation. Importing or filing a Drift is not authorisation to decide it: the decision is the developer's.

## Sub-agents context sharing

When you hand a part of the work to a sub-agent, in whatever way your agent starts sub-agents, you pass it what you have already read that concerns its part, so it does not look for it again and does not read it again.

- **What you pass.** Of what you have already read, only what concerns that sub-agent's part, plus a short common part (the request and the decisions, when there are any): Weaver's texts quoted as they are (the written request with its images put in words, the output of `weaver model show`, `weaver task show`, `weaver drift show`, `weaver proposal show`, the decisions of the interview) and the places in the code already known, one line per reference, ``- `<ref>` — <where in the code>``. Never code, never everything you gathered, and nothing you read only to pass it on.
- **What the sub-agent does.** It does not reread the Weaver texts it received and reads on its own only the ones that are missing. It always reads the code, also at the places it received, always runs the commands that register its work (`weaver task move <id> doing`, `weaver task new`, `weaver drift new`) and loads its skill as without this feature. A received place that no longer matches the code is ignored: it searches by itself. Context sharing never stops a job.
- **What comes back.** Each sub-agent reports where it found the code of what it saw, one line per reference as above; you reuse those lines in your own work and for the sub-agents that follow.
- **Limits.** No sub-agent runs again a read-only `weaver` command for a text it received, and what you pass to each one concerns only its part: never copy into every sub-agent all that you gathered.

In this skill, the sub-agents that prepare the questions receive, for each Drift, the output of `weaver drift show`, the Proposal attached to it, the chain of its `previous` Drifts and where the code is. The code at its Where in the code is read again before deciding.

## Command reference

Every `weaver` command, with its verbs and input fields, is in [COMMANDS.md](../weaver-design/COMMANDS.md), generated from the command registry. `weaver <object> <verb> --help` prints the usage, the fields with their types and limits, an example and the output fields.

## PowerShell

The examples use POSIX syntax. In PowerShell, pass JSON with a pipe and `--input -`, for example `Get-Content -Raw round.json | weaver question new --input -`, instead of `<` or a heredoc. On Windows PowerShell 5.1, set `$OutputEncoding` and `[Console]::OutputEncoding` to UTF-8 first, or non-ASCII characters arrive as `?`. Never pass the JSON as the text of `--input`: Windows PowerShell 5.1 strips the inner quotes of arguments to external programs.
