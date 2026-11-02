# omp

The omp program on the developer's computer: the only agent Weaver works with and the only source of the models it knows. Weaver asks it about itself (where it is, its version, its usable models, a newer version) and starts it as an ACP server for every job and for General; what omp answers and what its ACP sessions do are the facts below.

## Events

### READY
omp answers with at least the minimum version Weaver needs and at least one usable model.

### MISSING
omp cannot be found, or does not answer.

### OUTDATED
omp answers with a version older than the minimum Weaver needs.

### NO_PROVIDER
omp answers but has no usable model, because no model provider is connected.

### UPDATE_AVAILABLE
omp says a newer version of itself exists.

### UPDATE_ENDED
omp's own update finished well.

### UPDATE_FAILED
omp's own update failed or took too long.

### SESSION_STARTED
omp started, opened the ACP session and accepted the instruction Weaver sent.

### START_FAILED
omp could not start, open the session or accept the instruction within the time allowed, with the reason.

### SESSION_RESUMED
omp started again and reloaded an earlier ACP session.

### SESSION_NOT_RESUMED
omp could not reload an earlier ACP session.

### PERMISSION_REQUESTED
omp asks the developer's permission before doing something, only when its permissions are set to ask.

### TURN_ENDED
omp answered the last prompt normally, or after being cancelled.

### TURN_FAILED
omp ended the prompt abnormally: it refused, ran out of tokens or turns, or refused the prompt itself, with the reason.

### EXITED
The omp process ended, or its ACP connection closed, with the last lines it wrote about the error.
