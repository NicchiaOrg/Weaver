# Developer

The person who owns the project, looks at it in Weaver and decides. Every decision on the Model and on the work is his: approving, rejecting, reworking or discarding a Proposal, deciding on a Drift, running or deleting a Task. The agent may take one of these decisions through the command line only when the developer explicitly asked it for that decision in a conversation; it then counts as the developer's Event, with the same conditions and consequences. A request to design, scan, implement or resolve never by itself allows the agent to decide. Every decision either happens whole or changes nothing, and its conditions are checked again at the moment it is applied, not when the developer started it.

## Events

### OPEN_PROJECT
The developer opens a folder in Weaver, from the welcome window or from a window already open. A folder already open only comes to the front.

### CLOSE_WINDOW
The developer closes the window of a project. When agent jobs of that project are working he confirms first.

### QUIT_APP
The developer quits Weaver. When agent jobs are working in any project he confirms first.

### INITIALISE_PROJECT
The developer turns a folder without a project into a Weaver project, confirming first when agent files Weaver would install already exist with different content.

### UPDATE_SKILLS
The developer asks Weaver to write again every file it installs for the agent.

### IMPORT_CODE
The developer asks the agent to describe the code already in the project, optionally leaving out some folders for this import only.

### INSTALL_OMP
The developer asks Weaver to install omp with its official installer.

### LOG_IN_PROVIDER
The developer asks to connect omp to a model provider, or to connect it again.

### LOG_OUT_PROVIDER
The developer asks omp to forget a connected provider, after a confirmation.

### CHANGE_SETTINGS
The developer changes one of Weaver's settings.

### REFRESH_SYSTEM_CERTIFICATES
The developer asks Weaver to export the certificate authorities his system trusts again, while System Certificates is on.

### OPEN_USAGE
The developer opens Usage from the window of a project, to see how much is left of the usage limits of the connected providers.

### RETRY_USAGE
The developer asks Weaver to read the usage limits again after reading them failed.

### CLOSE_USAGE
The developer closes Usage, also by choosing to open Settings from it.

### NEW_REQUEST
The developer starts writing a design request: the setup request when the Model is empty and he chose to start from scratch, an ordinary design request otherwise.

### SEND_REQUEST
The developer sends the design request he wrote to the agent.

### DISCARD_REQUEST
The developer throws away a design request he never sent.

### APPROVE_PROPOSAL
The developer approves an open Proposal as a whole.

### REJECT_PROPOSAL
The developer rejects an open Proposal, optionally saying why.

### REWORK_PROPOSAL
The developer rejects an open Proposal with feedback and asks the agent to write it again from that feedback. Empty feedback needs a confirmation.

### DISCARD_PROPOSAL
The developer throws away a Draft the agent never finished.

### CHOOSE_PROPOSAL_MODEL
The developer chooses, or clears, the model that agents use for the work born from one Proposal that carries Tasks.

### RUN_TASK
The developer asks the agent to carry out one Task.

### RUN_ALL
The developer asks the agent to carry out every Task in To do that is not blocked.

### DELETE_TASK
The developer deletes a Task in To do for good, after a confirmation.

### SCAN_SYSTEM
The developer asks the agent to compare the whole System with the code, after a confirmation.

### RESOLVE_DRIFT
The developer asks to decide one open Drift.

### FIX_DRIFT_CODE
The developer decides that the Model and the Requirements are right and the code must change, choosing one of the Drift's fix options and optionally adding a note.

### ACCEPT_DRIFT_PROPOSAL
The developer decides that the code is right and accepts the Proposal attached to the Drift.

### DISMISS_DRIFT
The developer decides that nothing changes, after a confirmation.

### SEND_MESSAGE
The developer writes to the agent in the conversation of a working job, in General or in an Ask more conversation.

### STOP_AGENT
The developer stops the agent of a job, of General or of an Ask more conversation, after a confirmation when it is a job.

### ANSWER_PERMISSION
The developer answers a permission omp asks for: allow once, allow always, or refuse.

### CLEAR_GENERAL
The developer starts General again from an empty conversation.

### ASK_MORE
The developer asks an agent about one thing he is reading: a Proposal, a Task or a Drift in its details, a History entry with the Tasks it cites, the question in view of a question round, or the Requirements as a whole, from the Requirements view.

### CLOSE_ASK
The developer closes an Ask more conversation, after a confirmation when its agent is working.

### SEND_ANSWERS
The developer sends the answers of a question round, every question answered or skipped.

### SKIP_ALL_QUESTIONS
The developer skips every question of the round still without an answer, after a confirmation, and sends the round.
