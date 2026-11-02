# Proposal

A change to the Model, the Requirements and the Specs that the agent writes and the developer decides on as a whole. It holds the full new version of every file it changes and the list of files it removes, and it is of one kind: an ordinary change, the setup of an empty project, an import that describes existing code, or a resolution that lists decisions on open Drifts. An ordinary change or a setup carries its Tasks; an import and a resolution carry none. A Proposal can require other Proposals, can be bound to the written request it comes from, can rework a rejected Proposal, and can be attached to a Drift, which then decides it.

A Proposal is stale when a file it changes is no longer the one it started from, counting the Proposals it requires as already applied. It is blocked when a Proposal it requires is not approved, was rejected or discarded, is missing or forms a cycle; when it is stale; when the Model and the Requirements with it applied would be invalid; when one of its changes is no longer covered by a Task, or a Task names an element that would not exist; or, for a resolution, when a code fix it records could no longer be applied. Every reason is listed, never only the first.

## States

### Draft (initial)
The agent of a job is writing it, after opening it. Nobody can approve or reject it, and the Board shows only the ghost of the job that writes it.
- `agent.EDIT_PROPOSAL` → Draft
- `agent.SUBMIT_PROPOSAL` → Open: when every check passes; when it is bound to a written request, the request is consumed.
- `agent.SUBMIT_PROPOSAL` → Approved: a setup or import Proposal written by a job that started on an empty Model, when every check passes and nothing blocks it; the Proposals of the same job that it requires are already approved this way, so a chain of imports is approved whole.
- `developer.DISCARD_PROPOSAL` → Discarded: only when no working job is writing it; Proposals that require it stay blocked saying it was discarded. A Draft attached to a Drift can be discarded too; the Drift stays open.
- `developer.APPROVE_PROPOSAL` → Refining: only a Draft left half-way by a refinement; a new refinement job continues it.
- `drift.CODE_TO_BE_FIXED` → Discarded: only the Draft attached to that Drift.
- `drift.DISMISSED` → Discarded: only the Draft attached to that Drift.

### Open
Handed over and waiting for the developer's decision. A Proposal attached to a Drift is never shown on its own and cannot be decided directly: it is decided through its Drift.
- `developer.APPROVE_PROPOSAL` → Approved: only when nothing blocks it, the job that handed it over has ended and no Run is working in the project; its files replace the Model's, its Tasks are created in To do with the same identities, a Task is created to remove the code of each element it removes, and for a resolution every listed Drift still open gets its decision; it either happens whole or changes nothing.
- `developer.APPROVE_PROPOSAL` → Refining: an ordinary change, not attached to a Drift, that is stale and blocked by nothing else; a refinement job adapts it to the current Model and Requirements without a new interview.
- `developer.REJECT_PROPOSAL` → Rejected: the reason, when given, stays with it; a rejected resolution leaves its Drifts open; Proposals that require it stay blocked saying it was rejected.
- `developer.REWORK_PROPOSAL` → Rejected: with the feedback; a rework job writes it again, and the first Proposal it opens takes its place in the order of the Board and in the requirements of the Proposals that waited for it. A Proposal is reworked at most once.
- `developer.CHOOSE_PROPOSAL_MODEL` → Open: only when it carries Tasks; the chosen model replaces the analysis and execution models for the jobs born from it, until none of its Tasks is in To do or In progress.
- `agent.EDIT_PROPOSAL` → Draft: it goes back to the agent, even if nothing ends up different.
- `drift.PROPOSAL_ACCEPTED` → Approved: only the Proposal attached to that Drift, with the same conditions and effects as an approval.
- `drift.CODE_TO_BE_FIXED` → Rejected: only the Proposal attached to that Drift.
- `drift.DISMISSED` → Discarded: only the Proposal attached to that Drift.

### Refining
A refinement job rewrites it on the current Model and Requirements, keeping its identity, short number, place on the Board, label, chosen model and the Proposals that require it. The agent asks a question only when the current Model contradicts a decision already taken; when the whole change is already in the Model it asks the developer whether to remove it or keep it.
- `agent.EDIT_PROPOSAL` → Refining
- `agent.SUBMIT_PROPOSAL` → Approved: when nothing blocks it; it waits for a working Run to end first.
- `agent.SUBMIT_PROPOSAL` → Open: when something other than staleness still blocks it, with the reasons.
- `developer.REJECT_PROPOSAL` → Rejected: when the developer chose to remove a change already in the Model.
- `job.ENDED` → Draft: when the refinement job ended without handing it over; it stays half-way until the developer approves it again or discards it.

### Approved (final)
Its changes are in the Model, the Requirements and the Specs.

### Rejected (final)
The developer refused it, reworked it, or decided its Drift another way.

### Discarded (final)
Thrown away before or instead of a decision.

## Events

### SUBMITTED
Emitted when the agent hands the Proposal over, whether it becomes Open or is approved at once.

### APPROVED
Emitted when the Proposal is approved, after its changes are applied.
