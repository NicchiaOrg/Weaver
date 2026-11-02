---
created: 2026-11-02T16:23:14.830Z
updated: 2026-11-02T16:23:14.830Z
summary: 'Il ricaricamento della sessione di job e Chat invia session/load in modo ordinato: la cronologia ripetuta arriva in un blocco e le risposte successive aprono una nuova voce; la Chat parte subito se c''è una conversazione conservata.'
tasks:
- fvci4zh6
- sf8aqsg0
cleared:
  fvci4zh6:
    title: Ordine ricarica sessione
    refs:
    - job
    - general-chat
    - requirements/tech#job
    - requirements/tech#general-chat
    technical: false
    proposal: xt9xk5c1
    label: CI verde
    finished: 2026-11-02T16:12:19.562Z
    body: |-
      ## Problema

      Su Linux (run `ci` 41448848343) falliscono in modo intermittente:

      - `jobs::tests::closing_the_window_suspends_the_job_and_opening_it_reloads_the_session`: l'assert in `src-tauri/src/jobs/tests.rs` (~715) non trova `Entry::Agent{"After the reload", complete: true}`; il testo diventa "replayedAfter the reload".
      - `general::tests::after_a_restart_of_weaver_the_first_message_reloads_the_session_and_the_history_comes_back`: va in timeout in `until` (`src-tauri/src/jobs/testing.rs` ~233-243) perché "Answer" e "Newer answer" vengono fusi.

      È un bug del prodotto, non solo del test. In `load_session` (`src-tauri/src/jobs/session.rs` ~151) la risposta a `session/load` passa per una one-shot (`rpc.request`), e il task di apertura invia `Msg::Opened`. Le notifiche ripetute invece passano reader → canale `incoming` → task forwarder → `Msg::Incoming`. Sono due task indipendenti, quindi `Opened` può superare le ultime righe ripetute. `opened()` imposta `loading = false`, così le righe arrivate tardi finiscono nella conversazione live, dove `append_agent` (`src-tauri/src/jobs/conversation.rs` ~521) attacca la risposta successiva alla voce rimasta aperta. Il commento in `load_session` dice che l'ordine è garantito, ma non è vero.

      ## Cosa deve essere vero alla fine

      - `session/load` è inviato con lo stesso invio ordinato già usato da `session/prompt` (`prompt` in `session.rs`, `send_ordered` in `src-tauri/src/jobs/rpc.rs` ~169, instradamento in `route` ~239-242). Il caricamento termina quando la sua risposta viene gestita, sempre dopo ogni `session/update` letto prima di essa.
      - Nessun aggiornamento ripetuto raggiunge la conversazione live. Il primo messaggio dell'agente dopo il caricamento apre una nuova voce.
      - Vale sia per i job (ripresa di un job sospeso quando la finestra si riapre: `spawned()`, `opened()` e `incoming()` con `if self.loading` in `src-tauri/src/jobs/actor.rs`) sia per General (`spawned()`, `opened()` e `incoming()` ~444-450 in `src-tauri/src/general/actor.rs`).
      - La cronologia ripetuta continua ad arrivare alla UI in un unico blocco alla fine del caricamento (`adopt_replay` in `src-tauri/src/jobs/registry.rs` ~816).
      - Il limite unico di 30 s su spawn, `initialize` e `session/new` / `session/load` resta valido anche con il caricamento ordinato.
      - In General, se il caricamento fallisce (`OpenError::Failed`), si avvia comunque una sessione vuota (`new_session`) e un avviso lo comunica, come oggi.
      - Un nuovo test di regressione deterministico a livello RPC, accanto ai test esistenti in `rpc.rs` (~360-415), verifica che la risposta a una richiesta ordinata venga gestita dopo le notifiche lette prima di essa. Niente sleep nei test.

      Il fake agent (`crates/fake-acp/src/main.rs` ~149-156) ripete la cronologia e poi risponde; non va modificato per nascondere il problema.

      ## Verifica

      - I due test citati passano in modo affidabile anche ripetuti molte volte sotto carico, ad esempio in un ciclo con la CPU occupata.
      - Il nuovo test RPC passa.
      - `cargo test --workspace --locked` passa nel workflow `ci` su macOS, Windows e Linux.
  sf8aqsg0:
    title: Caricamento anticipato Chat
    refs:
    - general-chat
    - requirements/tech#general-chat
    - requirements/ui#general-chat
    technical: false
    proposal: maxvaxhl
    label: General ottimistico
    finished: 2026-11-02T16:19:40.749Z
    body: |-
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
---
# Ripresa ordinata della sessione

`session/load` usa lo stesso invio ordinato di `session/prompt`, così nessun aggiornamento ripetuto raggiunge la conversazione live e i test intermittenti su Linux diventano affidabili. La Chat con una conversazione conservata si avvia all'apertura della finestra e la ricarica senza inviare nulla all'agente.
