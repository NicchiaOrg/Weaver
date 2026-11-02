# Drift

A difference between the code and the Model, or between the code and the Requirements, that only the developer can decide. The agent files it after reading the code again, during a scan, during a Run (its advisor files one for code that was already there) or from a job on the developer's request. It names the elements and Requirement sections it concerns and says what happens, what should happen and where in the code; it offers two to five ways to fix the code, the first one recommended, none of which changes the Model. When no change of the code can respect both the Model and the Requirements it offers none, and an attached Proposal is required. It may name the Task during which it was found (its origin) and an earlier Drift it repeats. The agent never files again a difference the developer dismissed while the Model and the Requirements at that point are unchanged.

Before its job ends, the agent that filed it prepares the question with which the developer will decide it; the prepared question holds while the Model and the Requirements it concerns do not change.

## States

### Open (initial)
Waiting for the developer's decision. It blocks the Tasks that concern the same thing.
- `agent.PREPARE_DRIFT_QUESTION` → Open: the prepared question is kept with the Drift.
- `developer.RESOLVE_DRIFT` → Open: with a prepared question still valid, Weaver asks it itself as a question round, with no agent; otherwise a resolve job asks it. Refused while a Run works, while another resolution is being written or waits for a decision, or while the job that filed it is still working.
- `question-round.ANSWERED` → Open: only the prepared question of this Drift; Weaver writes and hands over the resolution Proposal from the answer by itself; a skipped question leaves the Drift as it is; a free answer next to a choice other than a code fix starts a resolve job with the answer already given.
- `developer.FIX_DRIFT_CODE` → Code to be fixed: an option is required when the Drift offers some; references that no longer exist are replaced by their nearest container, and it is refused when no Model element is left. A fix Task is created with both sides, the option, the note and where in the code.
- `developer.ACCEPT_DRIFT_PROPOSAL` → Proposal accepted: only with an attached Proposal that could be approved.
- `developer.DISMISS_DRIFT` → Dismissed: refused while the Drift is in a resolution being written or waiting for a decision.
- `proposal.APPROVED` → Code to be fixed: a resolution that records a code fix for it; the fix Task is created as above.
- `proposal.APPROVED` → Proposal accepted: a resolution that records adapting the Model for it.
- `proposal.APPROVED` → Dismissed: a resolution that records letting it be.

### Code to be fixed
The developer chose to fix the code; its fix Task carries the work. The Drift no longer blocks anything but its origin, which waits for the fix Task.
- `task.FINISHED` → Fixed: only its own fix Task.
- `task.CLOSED_UNCHANGED` → Dismissed: only its own fix Task; the note says why nothing changed.

### Fixed (final)
The code was changed as the developer chose.

### Proposal accepted (final)
The Model and the Requirements now say what the code does.

### Dismissed (final)
Nothing changes.

## Events

### CODE_TO_BE_FIXED
Emitted when the developer decides to fix the code.

### PROPOSAL_ACCEPTED
Emitted when the developer accepts the attached Proposal.

### DISMISSED
Emitted when the developer dismisses the Drift directly.
