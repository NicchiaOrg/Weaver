---
technical: false
refs:
  - general-chat
  - requirements/tech#general-chat
  - requirements/ui#general-chat
---

# Caricamento anticipato Chat

All'apertura della finestra del progetto, una conversazione Chat (General) conservata torna subito, prima che lo sviluppatore scriva qualcosa e senza inviare nulla all'agente. Senza conversazione conservata omp parte solo con il primo messaggio, come oggi. Weaver continua a non conservare il testo della conversazione: la cronologia arriva da omp tramite `session/load`.

## Cosa deve valere a lavoro finito

### Machine `general-chat`

**Idle (iniziale)**
- `window.OPENED` → Reloading: solo quando una conversazione è conservata e omp è pronto; Weaver avvia omp e la ricarica, senza inviare alcun prompt (come per i job sospesi).
- `window.OPENED` → Waiting for omp: solo quando una conversazione è conservata e omp non è pronto.
- Senza conversazione conservata `window.OPENED` non avvia omp. Il caricamento parte all'apertura della finestra, non all'apertura della vista Chat.
- `developer.SEND_MESSAGE` → Working: Weaver avvia omp quando non è in esecuzione, ricaricando la conversazione conservata quando non è ancora tornata, e invia il messaggio.
- `developer.CLEAR_GENERAL` → Idle.

**Loading** (nuovo stato padre di Reloading e Waiting for omp): la finestra si è appena aperta e la conversazione conservata sta tornando da omp; nel frattempo nulla viene inviato all'agente.
- `developer.SEND_MESSAGE` → Loading: il messaggio compare subito (con `Starting omp…`) e resta in attesa; la macchina resta in Loading, NON passa a Working. Viene inviato quando la conversazione è tornata, sotto di essa. Più messaggi partono in ordine. La casella messaggio è sempre abilitata.
- `developer.CLEAR_GENERAL` → Idle: il caricamento si interrompe, quell'omp viene terminato, la voce conservata viene cancellata e la conversazione riparte vuota.
- `window.CLOSED` → Idle: omp viene terminato; la conversazione resta conservata.

**Reloading (iniziale di Loading)**
- `omp.SESSION_RESUMED` → Idle quando nessun messaggio è in attesa; → Working quando ci sono messaggi in attesa: la cronologia va sopra di essi e i messaggi vengono inviati.
- `omp.SESSION_NOT_RESUMED` → Idle / Working allo stesso modo: subito conversazione vuota con l'avviso; omp resta pronto; gli eventuali messaggi in attesa vengono inviati.
- `omp.START_FAILED` → Idle e `omp.EXITED` → Idle (anche al superamento del limite di 30 s del caricamento): si mostrano `omp stopped: <reason>.` e `Your next message starts omp again.`; il messaggio successivo riavvia omp e ricarica la conversazione conservata.

**Waiting for omp**
- `omp.READY` → Reloading: appena omp-setup è Ready, Weaver avvia omp e ricarica la conversazione conservata da solo, senza attendere un messaggio.

**Working** (invariato salvo):
- `developer.STOP_AGENT` → Idle: il turno viene annullato; quando omp sta ancora partendo viene terminato, e il messaggio successivo lo riavvia e ricarica la conversazione conservata.
- `developer.CLEAR_GENERAL` → Idle: il turno si ferma e la conversazione riparte vuota.

### `requirements/tech#general-chat` (omp)
- Processo: un omp per il General di ciascun progetto, mantenuto fino alla chiusura della finestra, che lo termina come in `## job`.
  - Con una sessione conservata parte all'apertura della finestra, oppure appena omp-setup diventa Ready se non lo era; non viene mai avviato per una finestra senza sessione conservata.
  - Altrimenti, e dopo che si è fermato, viene avviato dal messaggio successivo.
- Sessione: un avvio successivo ricarica la sessione con `session/load`, entro il limite di 30 s di `## job`, inviato in ordine con gli aggiornamenti riprodotti come in `## job` (Resume). Se il caricamento fallisce parte una sessione vuota e un avviso lo dice.
  - L'avvio all'apertura della finestra non invia prompt. La cronologia riprodotta si mostra in un solo blocco quando il caricamento risponde; i messaggi inviati nel frattempo sono trattenuti, messi sotto la cronologia e poi inviati come prompt in ordine.
  - Clear chiude la sessione e cancella la voce; durante il caricamento all'apertura termina anche quell'omp.

### `requirements/ui#general-chat` (conversations)
- Caricamento: mentre la conversazione conservata si carica, testo secondario centrato `Loading conversation…` al posto del testo vuoto, sopra gli eventuali messaggi inviati nel frattempo, finché la cronologia non compare.
- In Waiting for omp con omp mancante si aggiungono subito la riga `omp is not installed.` e `Install omp`; con omp solo in verifica, in aggiornamento o non connesso si mostra solo `Loading conversation…`.
- `Clear`: disabilitato quando la conversazione è vuota; una conversazione conservata in caricamento non è vuota, quindi `Clear` è abilitato durante il caricamento. Il dialogo AlertDialog mostra `Clear Chat?` / `The conversation is deleted and a new one starts.` anche durante il caricamento, anche con un messaggio in attesa; solo durante un turno usa il testo `The agent's turn stops, ...`.

## Già presente (solo da verificare)
- `Clear` durante un turno e `Stop` mentre omp parte (che lo termina) sono già nel codice: il Model si allinea soltanto, nessun lavoro oltre a verificarli.
- L'ordinamento di `session/load` con gli aggiornamenti riprodotti appartiene alla Task `fvci4zh6`; i testi `Chat` dell'interfaccia alle Task `f2ws7caj`, `h6spvmt3`, `lyktflgx`. Questa Task non li rifà, ma il caricamento all'apertura deve passare per lo stesso percorso di `session/load`, così da beneficiare di quell'ordinamento.

## Dove intervenire (indicazioni)
- `crates/core/src/jobs/general.rs`: `GeneralMachine` (`handle`, `send`, `opened`, `clear`, `stop`, `window_closed`, `exited`; enum `State {Idle, Working}`, `Omp {Off, Starting, Open}`, `Event`, `Effect`). Oggi solo `send` emette `Effect::StartOmp`; `opened()` con `held` vuoto torna a Idle senza prompt; il ramo `(Idle, _)` di `send` trattiene già un messaggio mentre omp è Starting. Servono uno stato di caricamento (Loading con Reloading / Waiting for omp), un evento di apertura finestra con sessione conservata e la gestione di omp pronto.
- `src-tauri/src/general/service.rs`: `General::open` (~217) legge solo il record, `slot.loaded = record.is_none()`, e la doc dice "first message does it": deve avviare omp quando c'è un record. Inoltre `send_message`, `clear`, `carry_out` (`Effect::Clear`, che durante il caricamento deve terminare omp), `start_actor`, `launch_facts`, `session_opened` (con `!slot.loaded` mette le voci correnti dopo la cronologia, `mark_sent`, `reset = true`), `Slot::state`.
- `src-tauri/src/general/mod.rs`: `GeneralWindows::window_opened` (avvia `general.open`); precedente da seguire `src-tauri/src/jobs/registry.rs` `Jobs::window_opened` (~1116).
- `src-tauri/src/general/actor.rs`: `begin` (attende `host.omp_ready()` senza limite: qui avviene l'attesa di omp-setup Ready e la segnalazione di omp mancante), `launch` (azzera `opened_at`), `spawned` (`report_omp_missing`, `fail_opening`, `load_session` → ripiego `new_session` con avviso `fell_back`), `opened`, `incoming` (cronologia trattenuta in `self.replay` mentre `loading`), `exited`.
- `src-tauri/src/general/wire.rs`: `GeneralState {project_id, working, starting, kept}`; la doc di `kept` è superata e manca un flag di caricamento.
- `ui/src/components/general/general-view.tsx`: `EMPTY`, `empty = entries.length === 0 && !state?.kept`, `waiting`, `missing`; il testo del dialogo Clear oggi dipende da `working`, ed è sbagliato durante il caricamento con un messaggio in attesa. `ui/src/components/general/use-general.tsx` (`useGeneral`).

## Test
- `crates/core/src/jobs/general.rs`: aggiornare `the_first_message_starts_omp_and_goes_out_with_the_first_prompt` e aggiungere test per apertura con/senza conversazione conservata, omp non pronto → pronto, messaggi in attesa durante il caricamento, ripresa riuscita/non riuscita, avvio fallito/uscita, Clear e chiusura finestra durante il caricamento.
- `src-tauri/src/general/tests.rs`: `after_a_restart_of_weaver_the_first_message_reloads_the_session_and_the_history_comes_back` verifica il comportamento vecchio e va riscritto (la cronologia torna all'apertura, senza messaggi); adattare `when_the_kept_session_cannot_be_loaded_an_empty_one_starts_with_a_notice`; aggiungere un test con un messaggio inviato durante il caricamento (cronologia sopra, poi il prompt) e uno per Clear durante il caricamento.
- `crates/fake-acp/src/main.rs`: ha `replay`/`loadError` per `session/load` ma nessun ritardo o blocco del caricamento; aggiungerlo per il test dei messaggi durante il caricamento.
- `ui/src/components/general/general-view.test.tsx`: sostituire "keeps Clear usable while a kept session waits for the first message" e coprire `Loading conversation…`, omp mancante, testo del dialogo Clear durante il caricamento con un messaggio in attesa.
