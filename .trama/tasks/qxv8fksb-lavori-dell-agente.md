---
column: done
technical: false
refs:
  - agent-work
  - job
  - omp
  - requirements/tech#job
  - requirements/data#job
  - requirements/cli#job
  - requirements/rules#job
  - requirements/ui#job
drift: null
proposal: qmqh1o3p
agent: false
created: 2026-11-02T07:29:32.875Z
finished: 2026-11-02T11:07:34.615Z
---
# Lavori dell'agente

Dipende da "Scheletro del progetto", "Core dei file del progetto", "Controllo di omp" e "Impostazioni"; i comandi stanno in "Riga di comando trama".

Un `job` è un lavoro che Weaver affida a omp: setup, design su una richiesta, import, scan di tutto il Sistema, Run di un Task o di tutti i Task eseguibili, resolve di un Drift, rework di una Proposal rifiutata, refinement di una Proposal stale. Ha nome fisso, un processo omp, una sessione ACP e una conversazione in Weaver. Solo omp via ACP: niente terminali, hook, Cursor o Copilot.

## Machine `job`
- **Not started**: un solo Run per progetto (Run all conta come un Run anche con un Run per modello); Run e resolve si escludono; scan, design e import partono sempre. Entrano in Queued: `developer.SEND_REQUEST` (design, o setup a Modello vuoto), `developer.IMPORT_CODE` (senza le cartelle escluse), `developer.SCAN_SYSTEM`, `developer.RUN_TASK` (rifiutato se il Task è blocked o held, o se lavora un altro Run o un resolve), `developer.RUN_ALL` (un Run per modello effettivo dei Task in To do non bloccati, avviati insieme; rifiutato se non ce n'è o con stesso conflitto), `proposal.APPROVED` (Run di tutti i Task dopo una setup approvata alla consegna), `developer.RESOLVE_DRIFT` (senza domanda preparata valida), `question-round.ANSWERED` (risposta a una domanda preparata che richiede l'agente), `developer.REWORK_PROPOSAL` (skill del kind, col feedback; per una resolution solo sui Drift ancora aperti, nessun job se non ce ne sono), `developer.APPROVE_PROPOSAL` (refinement), `agent.START_JOB` (chiesto da General, stessi rifiuti).
- **Queued**: Weaver avvia omp con `trama` nel PATH e il job noto. `omp.SESSION_STARTED` → Running (solo ora il job è iniziato); `omp.START_FAILED` → Failed con il motivo, nulla annunciato o tenuto; `developer.STOP_AGENT` e `window.CLOSED` → Stopped.
- **Running**: `agent.DECLARE_NOTE` e `agent.ANNOUNCE_CARD` (ghost sulla Board fino all'arrivo della scheda o alla fine del job) restano in Running; `developer.SEND_MESSAGE` arriva senza fermare; `question-round.OPENED` (solo round di questo job) e `omp.PERMISSION_REQUESTED` → Waiting for the developer; `omp.TURN_ENDED` → Done se nessun messaggio aspetta, altrimenti Running con un nuovo turno; `omp.TURN_FAILED` → Failed; `omp.EXITED` → Failed con le ultime righe di omp; `developer.STOP_AGENT` → Stopped (turno annullato, ogni processo lanciato terminato); `window.CLOSED` → Suspended.
- **Waiting for the developer** (conta come lavoro in corso, emette `WAITING`): `question-round.CLOSED` e `developer.ANSWER_PERMISSION` → Running; un messaggio resta in attesa e non risponde mai al round o al permesso; `omp.TURN_ENDED` → Done (permesso pendente nullo); `TURN_FAILED`/`EXITED` → Failed; Stop → Stopped (permesso annullato); `window.CLOSED` → Suspended.
- **Suspended**: omp terminato, Task tenuti e ghost restano. `window.OPENED` riavvia omp e ricarica la sessione; `omp.SESSION_RESUMED` → Running con il prompt "Weaver stopped and reloaded your session. Continue your work from where it stopped."; `omp.SESSION_NOT_RESUMED` → Failed; Stop → Stopped.
- **Done**, **Failed**, **Stopped** finali; emettono `ENDED`. Nello stesso passo ogni Task tenuto, scheda annunciata e round del job è rilasciato o ritirato; una scheda consegnata si decide o si esegue solo a job finito. Un job finisce solo per un fatto certo, mai per tempo trascorso.

## Requisiti
- `requirements/tech#job`: riga di comando `<omp> [--model] [--slow] [--advisor] --config <overlay> --approval-mode <yolo|always-ask> acp`; ruoli: analysis (setup, design, import, rework, refinement, resolve), scan, execution (Run), il modello della Proposal sostituisce analysis ed execution, ruolo vuoto = default di omp. Env senza `TRAMA_*` ereditati più `TRAMA_JOB`, `TRAMA_ENDPOINT`, `TRAMA_RUN=1`, `TRAMA_TASK`; PATH con `<app local data>/bin` primo. Overlay in `overlays/<job id>.yml`, cancellato a fine job. Advisor Scanner sempre attivo nei Run (`--advisor`, `.omp/WATCHDOG.yml`), senza pre-flight né scan di fine Run. Apertura entro 30 s; capability senza `fs`/`terminal`. Form ed ask di omp rifiutati con `decline` e il testo fisso. Permessi `yolo` predefinito, `ask` alla UI senza limite. Messaggi via `_session/steering`, trattenuti durante l'attesa e spediti come `followUp`. Stop e chiusura: `session/cancel`, `session/close` 2 s, SIGTERM, SIGKILL dopo 3 s. Sub-agent da `rawOutput.details.progress[]`. omp non sopravvive mai a Weaver.
- `requirements/rules#job`: istruzione di una riga in inglese, frase per tipo di job più limite di sub-agent, context sharing, `trama question new`, lingua; mai numeri brevi né testo della richiesta.
- `requirements/data#job`: registro `jobs/<job id>.json` scritto a ogni cambio di stato, ultimi 50 job finiti per progetto, nessun testo della conversazione.
- `requirements/cli#job`: `announcement new`, `job edit` (nota ≤ 120 caratteri, ultime 6), `job show`, `job list`, `job new` (solo da General).
- `requirements/ui#job`: elenco Jobs con nomi fissi, badge, ordine, tempi, conservazione dei 30 finiti; conversazione con istruzione, messaggi, `Thinking`, righe tool, sub-agent, note, messaggi in coda, card dei permessi, Stop con conferma, righe di fine. Conversazione finita in sola lettura: `This job has ended. Continue in General.`

Decisioni: chiudere finestra o Weaver con job attivi chiede conferma, sospende e riprende la sessione ACP alla riapertura; una sessione non ripresa fa fallire il job.

## Fatto quando
- Test core: Run e resolve ammessi, gruppi di Run all con quote di sub-agent, rilascio delle schede alla fine.
- Test con agente ACP finto: ogni transizione, steering, permessi yolo/ask, form rifiutati, Stop, sospensione e ripresa.
- Nessun job finisce per timeout; `trama question new` senza risposta per 10 minuti riceve comunque le risposte.
- La vista Jobs mostra stati, note e conversazioni come da `requirements/ui#job`.
