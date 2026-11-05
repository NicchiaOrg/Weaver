---
technical: false
refs:
  - general-chat
  - requirements/tech#general-chat
  - requirements/data#general-chat
  - requirements/ui#general-chat
---
# Chat General

Dipende da "Scheletro del progetto" e "Lavori dell'agente" (client ACP, permessi, chiusura di omp, nascita dei job).

General è la conversazione libera con omp di un progetto, sempre presente nella sua finestra: nessuna istruzione, nessuna skill, nessuna richiesta scritta. È inclusa per scelta del developer. Non è un job: non tiene Task, non annuncia schede, non apre round di domande e non mostra nulla sulla Board. Per scelta del developer, su sua richiesta esplicita l'agente in General avvia lavori di Weaver (design, scan, Run, Resolve) con `trama job new`, che nascono come job normali con la propria conversazione. La conversazione resta alla chiusura della finestra e all'uscita da Weaver e torna com'era.

## Machine `general-chat`
- **Idle** (iniziale, la conversazione può essere vuota): `developer.SEND_MESSAGE` → Working, avviando omp se non gira e ricaricando la conversazione tenuta; `developer.CLEAR_GENERAL` → Idle con conversazione vuota.
- **Working**: `developer.SEND_MESSAGE` arriva mentre l'agente lavora; `omp.PERMISSION_REQUESTED` resta in Working in attesa della risposta, `developer.ANSWER_PERMISSION` idem; `agent.START_JOB` → Working, Weaver avvia il job chiesto (`agent.START_JOB` di `job`); `omp.TURN_ENDED` → Idle; `omp.TURN_FAILED` → Idle con il motivo; `omp.EXITED` → Idle con il motivo, il messaggio successivo riavvia omp; `developer.STOP_AGENT` → Idle, turno annullato; `window.CLOSED` → Idle, turno annullato, conversazione tenuta.

## Requisiti
- `requirements/tech#general-chat`: un omp per progetto, avviato dal primo messaggio e tenuto fino alla chiusura della finestra, che lo spegne come i job. Riga di comando `<omp> [--model <analysis>] --approval-mode <mode> --append-system-prompt <file> acp`, con `<file>` l'istruzione General incorporata scritta in `<app local data>/general.md`; niente `--advisor`, `--slow` né overlay. Env con `TRAMA_GENERAL=<General identity>` e `TRAMA_ENDPOINT`, senza `TRAMA_JOB`, così `trama` rifiuta i comandi legati a un job; eccezione `trama job new`, accettato dall'app solo dall'identità General viva di un progetto aperto, con gli stessi rifiuti della richiesta del developer. Sessione ricaricata con `session/load`; se fallisce parte una sessione vuota con un avviso. Clear chiude la sessione e cancella la voce. Form rifiutati con un testo che chiede all'agente di domandare nella risposta; permessi come nei job.
- `requirements/data#general-chat`: `general/<project hash>.json` nella cartella dati locale con `{project, sessionId, updated}`, scritto quando omp apre o ricarica la sessione, rimosso da Clear; senza record General parte vuota. Nessun testo della conversazione salvato da Weaver.
- `requirements/ui#general-chat`: vista propria con la resa della conversazione dei job (messaggi, `Thinking`, righe tool, sub-agent, card dei permessi, messaggi in coda), senza istruzione, note né riga di fine. Vuota: `Talk with omp in this project. Ask it to start a job of Weaver when you want one.` Message box sempre attivo; durante avvio o ricarica il messaggio mostra `Starting omp…`. `Stop` senza dialogo durante un turno. `Clear` (`Eraser`) disattivato se vuota, con AlertDialog `Clear General?` e testo diverso durante un turno, focus su `Cancel`. Job avviati: riga `Started <job name>` con `Open conversation`; rifiuto: `Weaver did not start <kind>: <reason>.` Errori: `omp stopped: <reason>.` poi `Your next message starts omp again.`; con omp assente `omp is not installed.` e `Install omp`.

## Fatto quando
- Test con agente ACP finto: primo messaggio avvia omp, ricarica della sessione dopo chiusura e riavvio, fallback a sessione vuota con avviso, Stop, uscita di omp.
- `trama job new` da General avvia un job con la sua conversazione e rispetta i rifiuti; da qualunque altro chiamante è rifiutato; i comandi legati a un job sono rifiutati in General.
- Clear cancella record e conversazione; General non notifica mai e non tocca la Board.
- Test Vitest della vista con stati vuoto, avvio, errore, job avviato e rifiutato.
