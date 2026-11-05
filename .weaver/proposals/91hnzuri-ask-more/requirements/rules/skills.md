---
description: "Use when changing the skills and files Weaver installs for omp, Requirement skills or job instructions."
---
# Agent skills and instructions

## project
- Files Weaver installs for the agent, one list in the core, embedded in the binary that the app and the `weaver` command line share:
  - skills in `.agents/skills/<name>/SKILL.md` (plus their reference files): `trama-setup`, `trama-design` (with its format reference), `trama-import`, `trama-scan`, `trama-compare`, `trama-implement`, `trama-resolve-drifts`;
  - omp sub-agent definitions: `.omp/agents/trama-analysis.md`, which compares Machines on the scan model (`model: "@slow"`, omp's slow role being the scan model), and `.omp/agents/trama-implementer.md`, which names no model and inherits the model of its Run;
  - the instructions of omp's advisor for the Advisor Scanner, `.omp/WATCHDOG.yml`;
  - the generated Requirement skills `trama-req-*`, in the same skills folder.
- Every installed text is in English, LF line endings; each sub-agent definition is under 6 KB. The skills folder is always `.agents/skills` in the project root, with no setting, so the app and the command line find the same one. A file is written, atomically, only when its LF-normalised content differs from what Weaver would write or it is missing; an equal file is left alone. Nothing outside this list is written or deleted, except `trama-req-*` skills the current mode does not produce.
- The installed skills and sub-agent definitions measure the detail of a comparison by the States, Transitions and Events of the Model; they never mention a Diagram, which the app does not have.
- Starting Areas written at initialisation, each with only `area.md`:

  | Area | Title | Description |
  |---|---|---|
  | `ui` | Interface | Use when building or changing screens, layout, styling, components, navigation look or texts shown to users. |
  | `tech` | Technology and constraints | Use when choosing or using languages, frameworks, libraries, platforms, project structure or build and deploy setup. |
  | `data` | Data | Use when defining, storing, validating or showing the information the System keeps and what each piece contains. |
  | `rules` | Business rules | Use when implementing calculations, limits, policies or other business rules behind the Model's conditions and consequences. |
  | `quality` | Quality | Use when the work touches performance, security, privacy, accessibility, offline behaviour, localisation or reliability. |

### Requirement skills
- A fragment has content when at least one of its sections has non-blank text; empty sections are never copied.
- Budget: the sum, over every fragment with content, of the characters (Unicode scalar values) of its skill name `trama-req-<area>-<fragment>` plus its description. Up to 4,000 inclusive: fragment mode; above: index mode. All Areas switch together.
- Fragment mode: one skill `trama-req-<area>-<fragment>` per fragment with content; front matter `name` and the fragment's `description`; body: the Area's H1 and prose when present, then each non-empty section with its reference as `##` heading (`General` for the general one).
- Index mode: one skill `trama-req-<area>` per Area with prose or a fragment with content; front matter `name` and the Area's `description`; body: the Area's prose, the `## General` section in full when non-empty, then for each fragment with content its title, description, the references of its sections and the relative path of `<fragment>.md`, a file in the skill folder holding the fragment's non-empty sections.
- Regeneration happens in the same step as an approval (by the app or the command line), at initialisation, at Update skills, at the opening check, and 1 s after the last change under `.weaver/requirements/` (debounced per project, never across projects).
- Every `trama-req-*` skill the current mode does not produce is removed: the other mode's after a switch, and those of fragments or Areas deleted or left empty.
- Wherever Weaver names a Requirement skill (command output, instructions), it names the one that exists in the current mode.
- `weaver model check` warns, without refusing anything, about each fragment over 12,288 bytes (whole file, front matter included) with path and size, and for a Proposal about the mode switch its approval would cause.
- The report of a regeneration and of Update skills is one structured result for app and command line: files written and removed, mode, budget used out of 4,000, a switch when it happened. A run that changed nothing reports nothing in the app.

## job
- The instruction is one line, in English, built only from fixed text, permanent identities, project-relative paths in backticks, numbers and the language name. It never contains a short number or the text of a request.
- First sentence by job:

  | Job | Sentence |
  |---|---|
  | Setup | ``Use the trama-setup skill on the setup request in `.weaver/requests/<request>.md`.`` |
  | Design | ``Use the trama-design skill on the design request in `.weaver/requests/<request>.md`.`` |
  | Import | ``Use the trama-import skill on this repository.`` then ``Do not import these paths and do not ask which folders to import: `a`, `b/c`.`` or `Import every folder and do not ask which folders to import.` |
  | Scan | `Use the trama-scan skill on the whole System.` |
  | Run of one Task | `Use the trama-implement skill on Task <id>.` |
  | Run all | ``Use the trama-implement skill on the runnable Tasks: take them from `weaver task list --input '{"runnable":true,"max":<N>,"groups":true}'`, one batch at a time, until it says there is none.`` A Run of a model group adds `"proposals":[…]` (the Proposals of its chosen model) or, for the execution-model group, `"notProposals":[…]`, identities in card order. |
  | Resolve | `Use the trama-resolve-drifts skill on Drift <id>.` With an answer already given: `The developer already answered its prepared question: <answer>.`, the answer as one-line JSON `{"selected":[…],"text":…}`. |
  | Rework | `Use the <skill> skill to rework the rejected Proposal <id> with the developer's feedback.` The skill follows the kind: `trama-design` for a change, `trama-setup` for a setup while the Model is empty (`trama-design` otherwise), `trama-import`, or `trama-resolve-drifts` adding ``on the Drifts `<d1>`, `<d2>` `` with its Drifts still open. |
  | Refinement | `Use the trama-design skill to refine the stale Proposal <id> on the current Model and Requirements.` |

- Sentences appended on the same line, in this order:
  1. `Run at most <N> sub-agents at once.`, with N the sub-agent limit setting (for a Run all group, its share; the same number is the `max` of `weaver task list`).
  2. `Pass each sub-agent what you have already read that concerns its part, as the "Sub-agents context sharing" section of your skill says.`
  3. ``Ask the developer only with `weaver question new`, as the skill says, never with your own question tool; run it with no time limit and wait until it ends.``
  4. Only when a language is set: `Write the texts for the developer in <language>, as the Language section of your skill says.`
- Always-on features (advisor, prepared Drift questions, parallel slots) add no sentence: the skills always do them.
- Bindings are enforced by the command line, not by the instruction: only the job whose instruction named a request may bind a Proposal to it, and only its first one; only a rework job may bind with `reworks`, to the Proposal it names.

## ask-conversation
- The instruction of an Ask more conversation is a fixed text in English carried by the app, starting with the line `Weaver: Ask more`, followed by the part that describes the chosen thing, read from the files when the conversation opens (and again when a lost session is replaced). It names no skill and, unlike `## job`, spans several lines and holds texts of cards and of the Requirements.
- Added sentence: only, when a language is set, `Write the texts for the developer in <language>.` None of the sentences of `## job` (sub-agent limit, context sharing, question rounds).
- The fixed text asks the agent to: read with the listed commands what it needs; answer the developer's message and explain; change a Task, a Drift or a Proposal only when the developer asks, and only with the `weaver` commands that allow it; never change the Model or the Requirements, which change only through an approved Proposal; never write by hand in files under `.weaver/`; start a job of Weaver only on the developer's explicit request, as in Chat; ask the developer only in its replies.
- Identities: only permanent identities, except the short number of the chosen card and of the overlay Proposal, each stated as the short number the developer sees.
- **Task:** permanent identity, short number, title, full body, references, paths of its images, and, when present, the Proposal whose design it belongs to (identity and title) and the Drift it fixes (identity).
- **Proposal:** permanent identity, short number, title, state, full body, its Tasks (identity and title) and, for a resolution, each Drift it resolves with its decision.
- **Drift:** permanent identity, short number, title, `What happens` and `What should happen` in full, references, and, when present, the attached Proposal, the Task it was born from and the Drift it repeats.
- **Task of a History entry:** from the copy in the entry: identity, title, body, references, Proposal, label and finished time; then the entry's identity, title and summary.
- **Question:** the question in view in full (text, context, every option with label, description and details, the recommended option, whether several can be chosen), then the text and options (label and description) of the other questions of the round in their order, marking the one in view. Then the name and identity of the job that asked the round or, for a prepared question, the Drift's identity. It says the round is still open and the agent must not answer for the developer. Never the developer's choices, written text, answers or skips.
- **Requirement section:** the reference `requirements/<area>#<section>`, the Area, the fragment, the title and the full text as the view shows it. With an overlay: the Proposal's identity, short number and title, and the change it brings to the section: `added`, `changed` with the text before and the new one, `removed`, `moved from <fragment>`.
- **Read commands**, listed after each part:
  - cards: `weaver task show <id>`, `weaver proposal show <id>` or `weaver drift show <id>` for the chosen card and each linked card, and `weaver model show <ref>` for each reference;
  - Task of a History entry: `weaver history show <entry>`, `weaver proposal show <id>` when its Proposal still exists, and `weaver model show <ref>` for each reference;
  - question: `weaver drift show <id>` for a prepared question;
  - Requirement section: `weaver model show requirements/<area>#<section>`, except for an `added` one, and with an overlay `weaver proposal show <id>`.
