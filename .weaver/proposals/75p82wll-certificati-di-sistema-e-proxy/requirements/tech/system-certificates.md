---
description: "Use when building System Certificates: exporting the system's certificate authorities, the bundle, the variables given to omp, the Zscaler turn-on."
---
# System Certificates

## system-certificates
- **Structure:** the export, the bundle building, the variables and the Zscaler check live in one module of `weaver-core`, as pure functions plus thin I/O; the app backend runs them, keeps the state and sends it to every window. The UI never reads the bundle or the certificates.
- **Reading the system:** `rustls-native-certs` (`load_native_certs`) on a blocking thread, never on the IPC or UI thread.
  - What it reads: on macOS the certificates trusted for TLS in the user, admin and system trust domains; on Windows the `CurrentUser\Root` logical store, which includes `LocalMachine\Root` and the roots of group policies; on Linux the system bundle and folders it probes, `SSL_CERT_FILE` and `SSL_CERT_DIR` of Weaver's own environment included.
  - Errors on single certificates are skipped while at least one certificate loads. When none loads, the export fails with `could not read the system certificates: <first error>`.
  - Weaver waits at most 30 s. Past it the export fails with `the export did not finish within 30 s`, and the late result of the thread is dropped.
- **Public roots:** the certificates of `webpki-root-certs`, compiled in, added after the system's.
- **Bundle building:** one pure function from the DER certificates (system first, then public roots) to the PEM text and the count, with the rules of `requirements/rules#system-certificates`. Certificates are parsed with `x509-parser`. PEM: `-----BEGIN CERTIFICATE-----`, base64 in lines of 64 characters, `-----END CERTIFICATE-----`, LF.
- **Writing:** as `requirements/data#system-certificates`. A failure is `could not write the bundle: <message>`.
- **One export at a time,** app-wide. A request while one runs joins it and starts nothing. An export that ends after the setting was turned off changes no state and emits nothing.
- **State:** in memory: exporting or not, count and write time of the bundle in use, last error, and whether the running export is automatic (start, Zscaler turn-on, daily refresh) or asked with Refresh. At start, count and time come from the bundle on disk. Every change is sent to every window as one event; a window reads the current state with one command when it loads.
- **Daily refresh:** while the setting is on, a timer fires `time.CERTIFICATES_REFRESH_DUE` 24 h after the end of the last export, successful or not. The timer is injected, never slept on in tests.
- **Variables:** one function returns the certificate variables from the setting and the bundle, read when each process is spawned.
  - Setting on and the bundle file present: `NODE_EXTRA_CA_CERTS`, `SSL_CERT_FILE`, `REQUESTS_CA_BUNDLE`, `CURL_CA_BUNDLE`, `GIT_SSL_CAINFO`, `PIP_CERT`, `CARGO_HTTP_CAINFO`, `AWS_CA_BUNDLE` and `DENO_CERT`, each the bundle's absolute path, plus `PI_REQUEST_COMPRESSION=0`.
  - Setting off or no bundle: nothing, so the environment is exactly as without System Certificates.
  - They are applied after the inherited environment and overwrite inherited values of the same names; PATH stays as `requirements/tech#omp` sets it.
  - Every omp spawn uses this one function: the commands of `requirements/tech#omp` (not the login shell nor `reg.exe` that read PATH), jobs, General and their resumes, and the terminals of `requirements/tech#omp-setup`.
- **Zscaler turn-on:** a task started at app start, outside the list of startup steps that report failures; its failures are only logged.
  - Detection: macOS `/Applications/Zscaler/Zscaler.app` exists; Windows `<ProgramFiles>\Zscaler` is a folder, with `ProgramFiles` from the environment, else `C:\Program Files`. Linux and any other system: never. Only file metadata: no process, no content, no process list, no network.
  - Order: the setting is off and `state.json` has no `systemCertificates.autoEnabledAt`; Weaver writes the setting through the settings service, which tells every window; then writes `autoEnabledAt`; then sends the notification and the banners; then exports, as an automatic export.
  - A failed setting write: nothing remembered, nothing shown. A failed `state.json` write after it: logged, the setting stays on.
  - `state.json` has one owner in the backend, shared with the windows' recent projects, so that no write overwrites another.
- **Start order:** the Zscaler check and the bundle's age are decided before omp's first check. When an export starts then, omp's first check waits for its end, at most its 30 s; nothing else of the start waits.
- **omp checks again:** at the end of each export, and when the setting is turned off, through the same entry point as a change of `ompPath`, only in the states the `omp-setup` Machine names.
