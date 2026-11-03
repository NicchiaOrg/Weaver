---
technical: true
refs:
  - requirements/ui#job
---
# Spiegazioni proxy in job e Chat

Quando un job o il General falliscono per un errore di certificato o un errore HTTP, la ragione mostrata allo sviluppatore è sostituita dalla spiegazione proxy di `requirements/ui#system-certificates`, scritta al momento del fallimento con il valore dell'impostazione `System Certificates` di quel momento (accesa o spenta danno testi diversi). Le funzioni di classificazione (`requirements/rules#system-certificates`) e di spiegazione sono costruite dal Task del gruppo `system-certificates`; questo Task le collega a ogni superficie di job e Chat.

Quando è fatto:
- **Testi esaminati:** non solo la riga tenuta come ragione, ma l'intero messaggio d'errore ACP e la coda di stderr del job o del General (ultime 40 righe, al massimo 4 KB). Se il testo è un errore di certificato vince sull'HTTP; un errore HTTP vale solo per i codici `403`, `407`, `500`–`599` nelle forme elencate dalla regola.
- **Job, riga finale** `Failed: <reason>.` (`requirements/ui#job`, «End line»): la spiegazione sostituisce la ragione per l'uscita (`omp exited with code <N>` + stderr), per i fallimenti di avvio (`START_FAILED`, errore in risposta al primo `session/prompt`), di apertura (spawn, `initialize`, `session/new` / `session/load`, inclusa la ripresa) e di turno (errori di richiesta di `session/prompt`). Le ragioni fisse in parole semplici (`omp did not answer within 30 s`, `the agent refused`, `the agent ran out of tokens`, `the agent reached its turn limit`, `the session could not be resumed`) restano quando non c'è un errore di certificato o HTTP. La spiegazione non ha punto finale: la superficie aggiunge il proprio `.` e nient'altro.
- **Show details** dopo un'uscita apre sempre le ultime righe d'errore di omp, invariate (monospace); la spiegazione non le sostituisce.
- **Chat (General):** `omp stopped: <reason>.` e la riga del turno fallito usano la spiegazione allo stesso modo; i dettagli restano le righe di omp.
- **Messaggi:** `Not delivered: <reason>.` di un messaggio rifiutato (risposta `failed` o errore di `_session/steering` / del `session/prompt` che lo consegna) usa la spiegazione quando il testo dell'errore la richiede.
- Oggi gli errori ACP non ricevono alcun suggerimento: da cambiare. L'attuale `reason_with_hint`/hint dei certificati in `exit_reason` va sostituito dalle nuove spiegazioni, senza lasciare due convenzioni.
- La ragione salvata (`reason` nel registro, testo della Chat) è quella già sostituita: cambiare l'impostazione dopo non riscrive fallimenti passati.
- Test: per ogni superficie, un testo con `UNABLE_TO_GET_ISSUER_CERT_LOCALLY` e uno con `HTTP 407`, con impostazione accesa e spenta, producono la spiegazione attesa; un errore non riconosciuto mantiene la ragione di oggi.

Sezioni da leggere: `requirements/ui/conversations.md` (`## job` «End line», «Queued message»; `## general-chat` «Failures»), `requirements/ui/system-certificates.md` («Proxy explanations», «Where they replace the reason»), `requirements/rules/system-certificates.md` («Texts tested», «Certificate error», «HTTP error»), `requirements/tech/omp.md` («Failure reasons»), `requirements/tech/acp.md` (`## job` «Opening», «Messages», «Turn end», «Stderr»).

Punti di partenza nel codice:
- `requirements/ui#job` — src-tauri/src/jobs/proc.rs:`OmpProcess::exit_reason` (oggi aggiunge l'hint dei certificati)
- `requirements/ui#job` — src-tauri/src/jobs/session.rs:`OpenError::reason`, `turn_end`, `steering_outcome`
- `requirements/ui#job` — src-tauri/src/jobs/actor.rs:`fail_opening`, gestione di `TurnEnd` fallito e dell'uscita
- `requirements/ui#job` — src-tauri/src/general/actor.rs:`fail_opening`, turno fallito, uscita; src-tauri/src/jobs/conversation.rs:`omp_stopped`
- `requirements/ui#job` — crates/core/src/omp/failure.rs (`CERTIFICATE_MARKERS`, `CERTIFICATE_HINT`/`PROXY_HINT`, `reason_with_hint`: da sostituire con le funzioni del gruppo `system-certificates`)
- `requirements/ui#job` — ui/src/components/jobs/jobs-view.tsx (`FailedLine`, righe della Chat, `Not delivered`): solo visualizzazione, i testi arrivano già sostituiti
