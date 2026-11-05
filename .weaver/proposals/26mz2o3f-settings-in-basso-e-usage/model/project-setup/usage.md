# Usage

Usage in the window of one project: how much is left of the usage limits of the model providers omp is connected to, and when each limit resets. Weaver asks omp for them each time the developer opens Usage, never in the background and never by calling a model. Only the providers Weaver offers in the omp setup appear, each connected one with its limits or the reason it has none, and nothing that tells one account from another. The limits read last, in any window, stay known to the whole app until Weaver quits: they are shown while new ones are read and when reading fails, with the time they were read. A reading that ends after Usage was closed still counts for the next opening; a result for a place of omp that is no longer the current one is ignored.

## States

### Closed (initial)
Usage is not shown.
- `developer.OPEN_USAGE` → Reading: only when omp is ready; Weaver asks omp for the usage limits of the connected providers.
- `developer.OPEN_USAGE` → Waiting for omp: when omp is not ready, because it is missing, installing, updating, logging in or connected to no provider.

### Waiting for omp
Usage says why omp is not ready and offers to open Settings.
- `omp.READY` → Reading: Weaver asks omp for the usage limits at once, without Usage being closed and opened again.
- `developer.CLOSE_USAGE` → Closed: when the developer chose to open Settings, the window shows them.

### Reading
Weaver waits for omp's answer. The limits read last, when there are some, stay shown, marked as being updated.
- `omp.USAGE_READ` → Shown: the limits read replace the earlier ones in every window.
- `omp.USAGE_FAILED` → Failed: also when omp does not answer within the time allowed; the limits read last, when there are some, stay shown with the time they were read.
- `developer.CLOSE_USAGE` → Closed: the reading goes on, and its limits count as the ones read last.

### Shown
Each connected provider, in the order of the omp setup, with each of its limits: how much is used, whether it is almost or fully used up, and when it resets. A connected provider without limits says why; when no provider has any limit, Usage says so above them.
- `developer.CLOSE_USAGE` → Closed

### Failed
Usage says why reading the limits failed and offers to try again.
- `developer.RETRY_USAGE` → Reading: only when omp is ready.
- `developer.RETRY_USAGE` → Waiting for omp: when omp is no longer ready.
- `developer.CLOSE_USAGE` → Closed
