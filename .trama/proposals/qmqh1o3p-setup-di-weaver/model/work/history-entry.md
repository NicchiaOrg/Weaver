# History entry

What a job did, told by its agent for one macro-feature: a title, a one-line summary, a text and the finished Tasks it groups. The agent writes the entries at the end of a Run, one per macro-feature, and may write one for any other job that finished Tasks. Weaver never writes an entry by itself, so a Task finished and never cited stays out of History.

## States

### Written (initial)
The entry exists, dated when it was written or last updated.
- `agent.UPDATE_HISTORY_ENTRY` → Written: the agent rewrites its text or adds finished Tasks, never removes one; the date becomes the moment of the update.

## Events

### TASKS_CITED
Emitted when the entry is written or updated, for the Tasks it cites for the first time.
