# omp setup

Whether omp is ready for Weaver to start it, one for the whole app. Weaver looks for omp where the settings say, or else in the places its official installer uses, and asks its version and its usable models. A job created while omp is not ready waits; no job and no General conversation starts until it is. A result that comes back for a place of omp that is no longer the current one is ignored.

## States

### Checking (initial)
Weaver asks omp about itself, when the app starts and whenever something may have changed it.
- `omp.READY` → Ready
- `omp.MISSING` → Missing
- `omp.OUTDATED` → Updating: Weaver runs omp's own update.
- `omp.NO_PROVIDER` → Not connected

### Missing
omp cannot be found or does not answer. Weaver offers to install it.
- `developer.INSTALL_OMP` → Installing: Weaver opens the system terminal with omp's official installer for the developer's system, so that he sees everything that happens.
- `developer.CHANGE_SETTINGS` → Checking: only when the place of omp changed.

### Installing
The official installer runs in the system terminal, where Weaver cannot see its end; Weaver looks at omp again every few seconds.
- `time.RECHECK_DUE` → Installing: Weaver asks omp about itself again.
- `omp.MISSING` → Installing: the installer has not finished yet.
- `omp.READY` → Ready
- `omp.OUTDATED` → Updating
- `omp.NO_PROVIDER` → Not connected
- `developer.INSTALL_OMP` → Installing: opens the installer again.
- `developer.CHANGE_SETTINGS` → Checking: only when the place of omp changed.

### Updating
omp updates itself. Jobs created meanwhile wait for the update to end.
- `omp.UPDATE_ENDED` → Checking
- `omp.UPDATE_FAILED` → Ready: when omp still answers with the minimum version and a usable model; the failure is shown and never blocks a job.
- `omp.UPDATE_FAILED` → Missing: when omp is still older than the minimum; it counts as missing and Weaver offers its installer, with the failure.

### Not connected
omp answers but has no usable model. Weaver lists the providers omp can log in to and offers to connect one.
- `developer.LOG_IN_PROVIDER` → Logging in: Weaver opens the system terminal with omp's login for that provider.
- `developer.CHANGE_SETTINGS` → Checking: only when the place of omp changed.

### Logging in
omp's login for a provider runs in the system terminal; Weaver never reads what the developer types there and looks at omp again every few seconds.
- `time.RECHECK_DUE` → Logging in: Weaver asks omp about its models again.
- `omp.NO_PROVIDER` → Logging in: the login has not finished yet.
- `omp.READY` → Ready
- `omp.MISSING` → Missing
- `developer.LOG_IN_PROVIDER` → Logging in: opens the login again, for the same or another provider.

### Ready
omp answers with the minimum version and at least one usable model. Weaver shows its version, its models and which providers are connected.
- `developer.LOG_IN_PROVIDER` → Logging in
- `developer.LOG_OUT_PROVIDER` → Checking: omp forgets only the credentials it stored for that provider; if the provider is still connected afterwards, Weaver says that other credentials of the developer's environment still connect it.
- `developer.CHANGE_SETTINGS` → Checking: only when the place of omp changed.
- `time.UPDATE_CHECK_DUE` → Ready: Weaver asks omp in the background whether a newer version exists, never waiting for the answer to start a job.
- `omp.UPDATE_AVAILABLE` → Updating: the update runs before the next job starts; jobs already working continue.
- `omp.MISSING` → Missing: omp disappeared, for example because it was uninstalled.
