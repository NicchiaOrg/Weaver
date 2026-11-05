---
description: "Use when implementing identities, short numbers, ordering, scope, staleness, blocking, slots or question checks."
---
# Cards and work

## General
- Permanent identity of a Proposal, Task, Drift or History entry: 8 characters from `[a-z0-9]`, drawn from the OS random generator and redrawn while it equals an identity of the same type in the project (for Tasks: `.weaver/tasks/` and every Proposal's `tasks/`). Commands take it from the file or folder name, never from the content.
- Short numbers: `P`, `T` or `D` plus the lowest free positive integer of that type, assigned in creation order to live cards that have none, freed when the card closes.
  - Live: a Proposal in Draft, Open or Refining; a Task in To do or In progress; a Drift in Open or Code to be fixed. A Done Task of a design keeps its number until every Task of the design is Done, then all free it together.
  - Input accepts either case (`t3`); letter + digits is always a short number; a wrong letter or a number no live card holds is refused.
  - Stored only in the local git-ignored cache under `.weaver/`, written atomically. A missing or unreadable cache, or one with duplicates or unknown cards, is rebuilt silently (valid lines kept, missing numbers assigned); no command fails because of it.
  - Never written into versioned files, History entries or agent instructions. JSON output carries both `id` and `number`.
- Order: oldest `created` first (UTC ISO 8601, milliseconds), ties by identity in byte order. Tasks created by one approval share its instant.
- Every change of cards takes the project's write lock, then reads, validates everything against the state after the change, then writes. A refusal writes nothing (no file, cache or lock leftover). Conditions are checked again when a decision is applied, not only when its button was enabled.

## proposal
- Scope of a change or setup, computed live against the Model, Requirements and Specs with the required Proposals applied below; never stored:
  - `changed`: each added or changed State, Transition, Event; a new Machine as one entry; a Machine, Feature or Actor whose description changed; each added or changed Requirement section, General included; each top-level Specs field whose value changed, as `specs#<field>`.
  - `leads into a change`: each Transition, in any Machine, entering a changed State or reacting to a changed Event.
  - `removed, its container`: a removed element counts as its nearest existing container (Transition → State → Machine → Feature; Event → Machine or Actor). Removed Requirement sections add nothing.
  - Sections compare by Area and reference, whatever fragment holds them; a moved unchanged section, a fragment appearing or vanishing, the System description, `area.md` and fragment titles and descriptions add nothing.
- Coverage: each entry is covered by a Task reference equal to it or containing it (`task` covers `task/To do`; a section by that section or its Area; `specs#<field>` by itself or `specs`), or by a body line under `## Already present`, ``- `<ref>` — <where in the code>``, with the same rule. `removed, its container` entries are covered by removal Tasks: one per topmost removed Model element, ``Remove code of `<ref>` ``, referencing the nearest surviving container; a removed Feature or Actor gives a Technical Task with no reference.
- Staleness: `base` records per touched file the SHA-256 (lowercase hex) of its LF-normalised content when first registered, or `new`. A file is stale when its current content, with required Proposals applied below in dependency order, hashes differently, when a `new` file now exists, or when a hashed file is gone. Editing again keeps `base`; only the refinement's rebase rewrites it. Task files and `proposal.md` never count.
- Reasons Approve is refused, all of them, in this order:
  1. `The agent is still finishing`;
  2. `A run is in progress`;
  3. each required Proposal in `requires` order: still being written, not approved yet, was rejected, was discarded, missing, forms a cycle;
  4. stale, naming each file;
  5. invalid with it applied, naming each error;
  6. each uncovered scope entry, each Task reference that would not exist, a missing label;
  7. for a resolution, each Drift whose code fix could no longer be applied.
- Hand-over is refused, naming every problem, for reasons 5–7 and also: an unregistered file in its folder; Task files in an import, a resolution or a Proposal attached to a Drift; a non-technical Task without Model reference or a Technical Task with one; an `## Already present` reference that would not exist; for a resolution, an empty list, a Drift not open or already in another Draft or Open resolution, `fix-code` without an existing option when the Drift has options, an option or note on another decision. Never for staleness or required Proposals.
- Auto-approval: a setup or import job records whether the Model had no Feature and no Actor when it reached Running. Only a Proposal handed over by such a job is approved at hand-over, with every check of Approve except reason 1; otherwise it stays Open with its reasons.
- Label: required when carrying Tasks; 1–30 characters after trimming, one line.
- Tag colour: FNV-1a 32-bit hash of the label's UTF-8 bytes modulo 6 picks `blue`, `green`, `amber`, `violet`, `cyan`, `pink`, in this order (pink, not red, so that no Task border looks like a Drift's).

## task
- Blocked, computed on every read, never stored: an open Drift names it as origin or concerns the same thing (see `work`), or it is the origin of a Drift in Code to be fixed whose fix Task is not Done. A Done Task is never blocked. A refused take names the Drift (number and title) or the fix Task.
- Runnable: To do and not blocked. The next runnable Task is the oldest one, recomputed at each request.
- Slots: `weaver task list` with `runnable` and `max` N walks runnable Tasks oldest first, picking each that concerns the same thing as no Task already picked and no Task In progress, until N are picked or none is left.
- Groups: To do Tasks linked transitively by concerning the same thing or naming the same backticked file path. With `groups`, each picked Task brings up to 2 more of its group, oldest first, for the same sub-agent: at most 3 Tasks of a group per sub-agent.
- Forgetting: at each `FINISHED`, every Done Task not cited by a History entry counts the Tasks that reached Done by `FINISHED` after it (by `finished`, ties by identity), over Task files and Task copies in History entries; closed-unchanged Tasks never count. At 20 or more it is deleted without trace, unless an open Drift names it as origin.
- Citation: the Task's copy (title, body, references, Proposal, tag, `finished`) is written into every citing entry before the Task file is removed; a failed copy keeps the Task and shows a notice.
- Run all groups runnable Tasks by effective model: their design Proposal's chosen model, else the execution model (trimmed, compared exactly). One Run per group, started together. The sub-agent limit N is shared in proportion to each group's runnable Tasks, at least 1 each, the rest to the largest fractional parts (ties to the group with the oldest Task).

## drift
- Nearest container: a reference that no longer exists is replaced by its nearest existing container (Transition → State → Machine → Feature; Event → Machine or Actor); a missing Requirement section is dropped. Fixing the code is refused before any write when no Model reference is left (or, for a Drift on Requirements only, no section).
- Text fingerprint: SHA-256 (lowercase hex) over the Drift's references in order, each as a line `# <ref>` then the text `weaver model show <ref>` prints and every Requirement section titled with it, LF-normalised; a missing reference adds `(missing)`. One core function, used for dismissals and prepared questions.
- Dismissal records the fingerprint. `weaver drift list` with `refs` marks Dismissed Drifts concerning the same thing whose fingerprint is unchanged; the agent never files those differences again.
- A prepared question is valid while the Drift is Open, it parses, its fingerprint equals the current one, every `fix-code` names an existing fix option and a `proposal` decision has an attached Proposal in Draft or Open. A new preparation replaces it; an invalid one is never changed by itself.
- Prepared question shape: one single-choice question, id = the Drift's identity; one option per fix option in order plus one for the code's current behaviour (`proposal` or `dismissed`); without fix options exactly `proposal` (recommended) and `dismissed`.
- Answer: a fix option is `fix-code` with its free text as note; `proposal` or `dismissed` without text is that decision; Weaver writes the resolution (`Resolve: <Drift title>`) and hands it over with the agent's checks. A refused hand-over discards that Draft and starts a resolve job with the answer.
- Fix options: 2–5, first recommended; none only with an attached Proposal. When the origin is a fix Task, `previous` is set to that fix Task's Drift.

## work
- A reference's key: a Model element is itself; `requirements/<area>#<element>` is the element; `requirements/<area>#General` only itself; `requirements/<area>` the Area; `specs#<field>` the field; `specs` the Specs.
- Containment: Feature ⊃ its Machines; Machine ⊃ its States, Events and their contents; State ⊃ sub-States and Transitions leaving it; Actor ⊃ its Events; Area ⊃ its General section only; `specs` ⊃ each field.
- Two references concern the same thing when keys are equal or one contains the other; two cards when any reference of one concerns any of the other. A card without references concerns nothing.
- One core function serves blocking, slots, groups and Drift lookups by references.

## question-round
- A round is validated whole before queueing; any failure refuses it all, naming each bad field by path (`questions[1].options[0].label`):
  - at least 1 question; ids non-empty, unique in the round; question text non-blank;
  - at least 2 options, each with non-blank label and description; labels unique within the question;
  - `multi` boolean; `recommended` integer from 0 to options − 1;
  - `context` and `details`, when present, non-blank after trimming.
- One round is presented per project; others wait FIFO per project by submission instant, prepared Drift questions included. Projects never wait for each other.
- Confirming needs an option or non-blank text; single choice holds at most one option. Whitespace-only text is absent.
- The backend refuses a send while any question has neither a confirmed answer nor a skip, whatever the interface sent.
- The result has one entry per question id. A skipped question carries nothing: unconfirmed text or options are never delivered and no answer is made up.
