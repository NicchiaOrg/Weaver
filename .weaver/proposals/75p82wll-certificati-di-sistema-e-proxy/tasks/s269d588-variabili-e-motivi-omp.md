---
technical: true
refs:
  - requirements/tech#omp
---
# Certificati nei comandi omp

Quando il lavoro è finito, ogni comando di omp che Weaver esegue da sé (tabella di `requirements/tech#omp`: `omp --version`, `omp models --json`, `omp update --check`, `omp update`, `omp auth-broker logout <provider>`) riceve le variabili di System Certificates, e ogni motivo di fallimento che è un errore di certificato o HTTP viene sostituito dalla spiegazione del proxy.

## Variabili
- Il runner dei comandi di omp aggiunge all'ambiente del figlio, oltre al PATH, le variabili restituite dalla funzione che dà le variabili dei certificati (costruita dal Task del gruppo `system-certificates`): le nove variabili e `PI_REQUEST_COMPRESSION=0`, solo mentre System Certificates è acceso e un bundle esiste. Sovrascrivono quelle ereditate.
- Si leggono a ogni avvio del comando (l'impostazione e il bundle del momento), mai una volta sola.
- Solo per omp: la shell di login (`$SHELL -l -c …`) e `reg.exe` che leggono il PATH non le ricevono. Oggi `SystemExec::run` serve tutti e tre: separare l'ambiente di omp dagli altri usi.
- I processi di omp dei lavori e di General sono coperti dal Task che tocca `requirements/tech#acp`; qui solo i comandi della tabella, ma usando la stessa funzione.

## Motivi
- La regola del motivo resta (ultima riga non vuota di stderr, altrimenti `exit code N`, `could not start omp`, `timed out`), ma prima Weaver prova l'intero stderr con la funzione che classifica il testo come errore di certificato o HTTP (`requirements/rules#system-certificates`, gruppo `system-certificates`). Se lo è, il motivo diventa la spiegazione restituita dalla funzione dei testi del proxy, scelta con l'impostazione com'è nel momento del fallimento (acceso/spento) e mai riscritta dopo.
- Le spiegazioni non hanno punto finale: il riquadro di omp in Missing aggiunge `.`; il `(<reason>)` di un aggiornamento fallito in Missing (`too_old_text`) e in Ready, `Could not log out of <name>: <reason>.` e la riga del perché l'elenco modelli non è disponibile in Settings le incorporano così come sono. Testi esatti in `requirements/ui#system-certificates` (sezione "Proxy explanations" e "Where they replace the reason").
- Gli attuali `CERTIFICATE_MARKERS`, `CERTIFICATE_HINT`, `PROXY_HINT` e `reason_with_hint` vengono sostituiti dalle funzioni nuove (nessun secondo meccanismo); aggiornare i test di `failure.rs`.

## Da dove partire
- `requirements/tech#omp` — crates/core/src/omp/run.rs:`SystemExec::run` (solo PATH oggi), `run_command`
- `requirements/tech#omp` — src-tauri/src/omp.rs:`SystemOps::environment` (costruisce `SystemExec`), crates/core/src/omp/path_env.rs:`run` (shell di login, da lasciare senza variabili)
- `requirements/tech#omp` — crates/core/src/omp/failure.rs:`hint`, `reason_with_hint`, `status_text`
- `requirements/tech#omp` — crates/core/src/omp/probe.rs:`probe_found`, `check_for_update`, `run_update`, `logout`, `too_old_text`
- `requirements/tech#omp` — src-tauri/src/omp.rs: fallimento update (`reason_with_hint`) e logout
