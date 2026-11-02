# Project

A folder opened in Weaver, seen as a Weaver project: whether it has a `.trama` folder and whether its Model is still empty. "Has a Model" means at least one Feature or one Actor; open Drifts and Requirements do not count. Each open project keeps the files Weaver installs for the agent the same as the ones this version of Weaver carries, and the skills generated from the Requirements the same as its Requirements.

## States

### Not initialised (initial)
The folder has no `.trama` folder. Every action that needs a project says to initialise it first; none fails silently. Weaver never looks for a project in the parent folders.
- `developer.INITIALISE_PROJECT` → Empty Model: creates the `.trama` folder with an empty Model, whose System is named after the folder, the starting Areas of Requirements, and every file installed for the agent; when agent files it would install already exist with different content, nothing is created until the developer confirms overwriting them; a failure leaves nothing behind.
- `workspace.PROJECT_FOLDER_APPEARED` → Empty Model: when the new `.trama` folder holds no Feature and no Actor.
- `workspace.PROJECT_FOLDER_APPEARED` → Has a Model: when the new `.trama` folder already holds a Feature or an Actor.

### Empty Model
The project exists but its Model has no Feature and no Actor. The developer can start from scratch with a setup request or import the existing code; a warning says that Start from Scratch ignores the files already in the folder unless the request mentions them.
- `proposal.APPROVED` → Has a Model: when the Model now holds a Feature or an Actor.
- `proposal.APPROVED` → Empty Model: when the Model is still empty; the Requirement skills are generated again.
- `workspace.FILES_CHANGED` → Has a Model: when the Model now holds a Feature or an Actor, for example after a pull.
- `workspace.FILES_CHANGED` → Empty Model: when the Model is still empty; the Requirement skills are generated again if the Requirements changed.
- `developer.UPDATE_SKILLS` → Empty Model: writes again every installed file that differs from the one Weaver carries, creates the starting Areas when the Requirements are missing, and always reports what it changed or the error.
- `workspace.PROJECT_FOLDER_REMOVED` → Not initialised

### Has a Model
The project has a Model. When its window opens Weaver compares, in the background, every installed file with the one it carries and rewrites the ones that differ, overwriting edits by hand; it says something only when it changed a file, and shows a failure at every opening until it is gone.
- `proposal.APPROVED` → Has a Model: the Requirement skills are generated again from the approved Requirements.
- `workspace.FILES_CHANGED` → Has a Model: when the Model still holds a Feature or an Actor; the Requirement skills are generated again if the Requirements changed.
- `workspace.FILES_CHANGED` → Empty Model: when the Model no longer holds any Feature or Actor.
- `developer.UPDATE_SKILLS` → Has a Model: writes again every installed file that differs from the one Weaver carries and always reports what it changed or the error.
- `workspace.PROJECT_FOLDER_REMOVED` → Not initialised: the window goes back to offering initialisation; the agent jobs already working continue.
