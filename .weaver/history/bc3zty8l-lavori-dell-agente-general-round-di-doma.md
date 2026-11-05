---
created: 2026-11-02T12:31:02.933Z
updated: 2026-11-02T12:31:02.933Z
summary: Job affidati a omp via ACP, chat General libera, round di domande al developer, controllo e installazione di omp e notifiche di sistema quando tocca al developer.
tasks:
  - qxv8fksb
  - cdoi70sr
  - uyj9ftam
  - mlyua7vr
  - jodwoiri
cleared:
  cdoi70sr:
    title: Chat General
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T11:41:00.534Z
    refs:
      - general-chat
      - requirements/tech#general-chat
      - requirements/data#general-chat
      - requirements/ui#general-chat
    body: |-
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
  jodwoiri:
    title: Notifiche di sistema
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T12:16:35.974Z
    refs:
      - notification
      - requirements/tech#notification
      - requirements/ui#notification
    body: |-
      Dipende da "Scheletro del progetto", "Lavori dell'agente" (eventi `job.ENDED`, `job.WAITING`), "Round di domande" e "Finestre e avvio" (porta la finestra in primo piano).

      Una `notification` dice al developer che tocca a lui: un job di un progetto è finito, si è fermato in modo anomalo, o aspetta le sue risposte o il suo permesso. Per scelta del developer è mostrata subito e solo quando la finestra di quel progetto non è in primo piano; altrimenti nulla. Nomina progetto, job e cosa è successo, mai ciò che l'agente ha scritto, chiesto o fatto. I job fermati dal developer e General non notificano mai. Una notifica non cambia nulla nel progetto, nei job o nei round. Non esiste un'impostazione: decidono i permessi del sistema.

      ## Machine `notification`
      - **Not shown** (iniziale): `job.ENDED` → Shown quando il job è finito (Done) o fallito (Failed) e la finestra non è in primo piano; `job.WAITING` → Shown quando la finestra non è in primo piano.
      - **Shown**: `desktop.NOTIFICATION_CLICKED` → Clicked; `question-round.CLOSED` → Withdrawn (il round annunciato si è chiuso); `developer.ANSWER_PERMISSION` → Withdrawn (il permesso annunciato ha avuto risposta); `job.ENDED` → Withdrawn quando annunciava un'attesa di un job ora finito.
      - **Clicked** (finale): il developer è portato al progetto; emette `CLICKED`, che la Machine `window` usa per portare avanti la finestra. **Withdrawn** (finale): non è più vera e lascia l'elenco del sistema.

      ## Requisiti
      - `requirements/tech#notification`: `tauri-plugin-notification` ≥ 2.8, chiamato solo da Rust; i clic arrivano da `Notification::on_action`. Test del primo piano al momento dell'invio: finestra del progetto visibile, non minimizzata e `is_focused()`, seguita da `WindowEvent::Focused`; un'altra finestra di Weaver a fuoco non conta. Ogni notifica ha un id `i32` da un contatore dell'app, mappato a project hash e destinazione (job, round o permesso); un `tap` su un id mappato mostra, de-minimizza e mette a fuoco la finestra e dice all'interfaccia cosa mostrare; clic su id non mappati o progetti chiusi ignorati. Ritiro: su Linux `remove_active`; su macOS e Windows si toglie solo la mappatura, così un clic successivo porta solo avanti la finestra. Tutte le mappature cadono alla chiusura della finestra. Su Windows un clic dall'Action Center avvia Weaver e single-instance mette a fuoco l'ultima finestra.
      - `requirements/ui#notification`: testi esatti — Finished `<project> – <job> finished` / `Weaver is waiting for you.`; Several finished `<project> – <n> jobs finished` / `Weaver is waiting for you.`; Failed `<project> – <job> stopped` / `The agent stopped before finishing its work.`; Question round `<project> – <job> asks a question` / `The agent is waiting for your answers.`; Permission `<project> – <job> asks permission` / `The agent is waiting for your approval.` `<project>` è il nome della cartella, `<job>` il nome fisso del job. Raggruppamento: un job che finisce mentre è mostrata una notifica di fine dello stesso progetto la sostituisce con `<n> jobs finished`, al massimo una per progetto; gli altri tipi uno per evento. Id di progetto + job + tipo per sostituire o ritirare. Destinazione del clic: finito/fallito → conversazione del job (più job → vista Jobs); domanda → overlay del round; permesso → conversazione scorsa alla card. Nome e icona dell'app, suono predefinito del sistema.

      ## Fatto quando
      - Test core puri sulla decisione di notificare (primo piano, job fermato dal developer, General esclusa) e sul raggruppamento.
      - Con finestra in primo piano nessuna notifica; in secondo piano compaiono i testi esatti senza contenuti dell'agente.
      - Il clic apre la vista giusta; round chiuso, permesso risposto o job finito ritirano la notifica d'attesa.
  mlyua7vr:
    title: Controllo di omp
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T09:31:49.416Z
    refs:
      - omp-setup
      - time
      - specs#infrastructure
      - requirements/tech#omp
      - requirements/tech#omp-setup
      - requirements/ui#omp-setup
    body: |-
      Realizza la Machine `omp-setup`, unica per tutta l'app: dice se omp è pronto. Nessun lavoro e nessuna General parte finché non lo è; un lavoro creato intanto aspetta. Dipende da "Scheletro del progetto" e legge `ompPath` da "Impostazioni".

      ## Machine `omp-setup`
      - `Checking` (iniziale, all'avvio e quando qualcosa può aver cambiato omp): `omp.READY` → `Ready`, `omp.MISSING` → `Missing`, `omp.OUTDATED` → `Updating` (esegue l'update di omp), `omp.NO_PROVIDER` → `Not connected`.
      - `Missing`: `developer.INSTALL_OMP` → `Installing` aprendo il terminale di sistema con l'installer ufficiale; `developer.CHANGE_SETTINGS` → `Checking` solo se il percorso di omp è cambiato.
      - `Installing`: `time.RECHECK_DUE` ricontrolla; `MISSING` resta; `READY`, `OUTDATED`, `NO_PROVIDER` come in `Checking`; `INSTALL_OMP` riapre l'installer; `CHANGE_SETTINGS` come sopra.
      - `Updating`: i lavori creati aspettano. `omp.UPDATE_ENDED` → `Checking`; `omp.UPDATE_FAILED` → `Ready` se omp risponde ancora con la versione minima e un modello (fallimento mostrato, non blocca), altrimenti → `Missing` con l'installer offerto e il fallimento.
      - `Not connected`: elenca i fornitori; `developer.LOG_IN_PROVIDER` → `Logging in` aprendo `omp login <provider>` nel terminale; `CHANGE_SETTINGS` come sopra.
      - `Logging in`: Weaver non legge mai ciò che il developer scrive nel terminale. `RECHECK_DUE` ricontrolla i modelli; `NO_PROVIDER` resta; `READY` → `Ready`; `MISSING` → `Missing`; `LOG_IN_PROVIDER` riapre il login.
      - `Ready`: mostra versione, modelli e fornitori connessi. `LOG_IN_PROVIDER` → `Logging in`; `developer.LOG_OUT_PROVIDER` → `Checking` (omp dimentica solo le credenziali che ha salvato; se il fornitore resta connesso Weaver dice che altre credenziali dell'ambiente lo connettono); `CHANGE_SETTINGS` come sopra; `time.UPDATE_CHECK_DUE` chiede in background se esiste una versione nuova, senza mai far aspettare un lavoro; `omp.UPDATE_AVAILABLE` → `Updating` prima del prossimo lavoro, quelli in corso continuano; `MISSING` → `Missing`.
      - Un risultato calcolato per un percorso di omp che non è più quello corrente è ignorato.

      ## Requisiti (`requirements/tech#omp`, `requirements/tech#omp-setup`)
      - Ricerca: `ompPath` (se impostato, da solo), poi PATH, poi le cartelle dell'installer per sistema. PATH di login letto una volta all'avvio su macOS e Linux (`$SHELL -l -c`, 5 s); su Windows dal registro a ogni ricerca.
      - `MIN_OMP_VERSION = 18.15.0` nel core, semver; versione illeggibile = vecchia. Comandi senza shell con limiti: `--version` 15 s, `models --json` 15 s (nessun modello = `NO_PROVIDER`), `update --check` 30 s, `update` 5 min, `auth-broker logout <provider>` 30 s. Motivi di fallimento dall'ultima riga di stderr, con suggerimenti per certificati e proxy.
      - Terminale di sistema (scelta del developer): Terminal via `osascript` su macOS, PowerShell 5.1 con nuova console su Windows, primo emulatore disponibile su Linux; senza terminale si mostra il comando da copiare. Una sola funzione testata fa il quoting; si inseriscono solo il percorso assoluto di omp e un id fornitore della lista fissa. Installer `curl -fsSL https://omp.sh/install | sh` o `irm https://omp.sh/install.ps1 | iex`; Weaver non scrive mai `ompPath`.
      - Ricontrollo ogni 3 s, uno alla volta, solo in `Installing` e `Logging in`. Controllo aggiornamenti al massimo una volta all'ora per tutta l'app, un solo `omp update` alla volta.
      - Fornitori in ordine: `openai-codex`, `anthropic`, `github-copilot`, `google-gemini-cli`, `google-antigravity`, `cursor`, `xai-oauth`; connesso se un modello lo ha come `provider`. Cursor solo tramite login, nessun campo per chiavi API, nessun segreto salvato in v1.

      ## Interfaccia (`requirements/ui#omp-setup`)
      Un unico box di stato (`aria-live="polite"`) in Settings, Get Started e finestra di benvenuto, con icona, testo e azione per ogni State come da tabella del Requisito; righe `Providers` con `Log in`/`Log in again` e `Log out` con conferma `Log out of <name>?`; errore `Could not open a terminal: <reason>.` con il comando e `Copy`.

      ## Fatto quando
      - Test unitari della Machine con eventi e orologio iniettati (nessun sleep, nessun omp reale): ogni transizione, i ricontrolli che si fermano uscendo dallo State, il risultato di un percorso vecchio scartato.
      - Test del quoting per i tre sistemi e della ricerca di omp su cartelle temporanee.
      - Test UI del box per ogni State e del dialog di logout, tastiera inclusa.
  qxv8fksb:
    title: Lavori dell'agente
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T11:07:34.615Z
    refs:
      - agent-work
      - job
      - omp
      - requirements/tech#job
      - requirements/data#job
      - requirements/cli#job
      - requirements/rules#job
      - requirements/ui#job
    body: |-
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
  uyj9ftam:
    title: Round di domande
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T11:58:01.295Z
    refs:
      - question-round
      - requirements/tech#question-round
      - requirements/cli#question-round
      - requirements/rules#question-round
      - requirements/ui#question-round
    body: |-
      Dipende da "Scheletro del progetto", "Riga di comando trama" (canale socket con l'app) e "Lavori dell'agente"; la domanda preparata di un Drift viene da "Drift".

      Un `question-round` è un gruppo di decisioni che l'agente di un job chiede al developer insieme: ogni domanda ha opzioni, una raccomandazione, scelta singola o multipla, un contesto facoltativo e sempre spazio per una risposta libera. È anche il modo in cui Weaver stesso pone la domanda preparata di un Drift, senza agente. Un round appartiene al suo job e al suo progetto, non compare mai nella finestra di un altro progetto; né Weaver né l'agente fissano un limite di tempo. Domande e risposte non sono mai scritte nel progetto.

      ## Machine `question-round`
      - **Not asked**: `agent.ASK_QUESTIONS` → Unavailable se chi chiama non è l'agente di un job di Weaver che lavora o Weaver non gira; → Queued se un altro round del progetto è presentato; → Presented altrimenti, con la finestra del progetto in primo piano. `developer.RESOLVE_DRIFT` porta la domanda preparata in Queued o Presented allo stesso modo.
      - **Queued**: aspetta i round precedenti, il più vecchio prima, senza sostituire quello presentato. `CLOSED` → Presented quando è il più vecchio in attesa; `agent.STOP_WAITING` → Abandoned; `job.ENDED` e `window.CLOSED` → Interrupted.
      - **Presented** (emette `OPENED`): risposte confermate e non confermate sono tenute da Weaver, così il round resta com'era quando il developer si sposta, lo riduce o ci torna. `developer.SEND_ANSWERS` → Answered solo con ogni domanda risposta o saltata; `developer.SKIP_ALL_QUESTIONS` → Answered saltando quelle senza risposta confermata; `agent.STOP_WAITING` → Abandoned; `job.ENDED` e `window.CLOSED` → Interrupted. Ogni fine emette `CLOSED`; l'invio emette `ANSWERED`.
      - **Answered**: l'agente riceve ogni risposta, le saltate marcate come tali; per ognuna usa la raccomandazione e la elenca come decisione da verificare. Una domanda preparata di Drift saltata lascia il Drift aperto.
      - **Abandoned**: nessuna risposta consegnata. **Interrupted**: risposte scartate; l'agente, se c'è ancora, ne è informato e ferma il lavoro che aspettava senza usare raccomandazioni. **Unavailable**: l'agente sa che Weaver non può chiedere, non deve chiedere in altro modo e deve fermarsi e dirlo al developer.

      ## Requisiti
      - `requirements/cli#question-round`: `trama question new --input -` stampa solo JSON; valida tutto prima di contattare l'app, poi blocca senza limite di tempo. Esiti mai confusi: `answered` (exit 0), `unavailable` con il testo "Weaver cannot show questions for this command. Do not ask in any other way: stop and tell the developer." (exit 0), `invalid-input` (exit 2, nomina domanda e campo), `interrupted` e `cancelled` (exit 1). `--help` chiede testi autosufficienti, niente prima persona, niente opzione "Other", esecuzione senza limite di tempo.
      - `requirements/tech#question-round`: invio di `{secret, job, request}` a `TRAMA_ENDPOINT`, nessun timeout da nessuna parte; instradamento solo con `TRAMA_JOB` (sub-agent e worktree arrivano al progetto del job); l'app accetta round solo da job Running o Waiting for the developer; una coda per progetto creata all'apertura. SIGINT, SIGTERM o stdout chiuso chiudono la connessione: è `agent.STOP_WAITING`. Stato e bozze solo in memoria del backend (bozze di testo debounced a 300 ms), mai su disco né nei log.
      - `requirements/rules#question-round`: validazione intera con percorso del campo (`questions[1].options[0].label`): almeno 1 domanda, id unici, almeno 2 opzioni con label e description non vuote, label uniche, `recommended` nell'intervallo, `context`/`details` non vuoti. Un round presentato per progetto, gli altri FIFO per istante di invio, domande preparate incluse; i progetti non si aspettano. Il backend rifiuta l'invio con domande mancanti; una saltata non porta nulla.
      - `requirements/ui#question-round`: overlay modale su ogni vista (Esc e clic fuori inerti), intestazione `Questions from the agent · <job name>` o `Question about <Drift id> · <Drift title>`, `+<k> more round(s) waiting`, indice con stati `Answered`/`Skipped`/`Missing`, opzioni con tasti `1`–`9` e `Recommended` preselezionata solo alla prima visita, risposta libera, `Back`/`Skip all`/`Skip`/`Next`/`Review`, conferma Skip all, riepilogo con `Send to agent`, barra minimizzata.

      ## Fatto quando
      - Test core su validazione, coda FIFO per progetto e risultato con skip.
      - Test di `trama question new` per ogni esito ed exit code, incluso un round lasciato senza risposta a lungo.
      - Chiusura della finestra o fine del job interrompono i round; Ctrl+C li abbandona.
      - Test Vitest dell'overlay con tastiera, riduzione e ritorno senza perdere le bozze.
---
# Lavori dell'agente, General, round di domande, controllo di omp e notifiche

Il `job` è un lavoro che Weaver affida a omp (setup, design, import, scan, Run, resolve, rework, refinement) con un processo omp, una sessione ACP e una conversazione; i permessi passano dal developer e i job si sospendono e riprendono.

General è la conversazione libera con omp di un progetto e, su richiesta esplicita, avvia lavori con `trama job new`.

Il round di domande raccoglie le decisioni che l'agente chiede al developer, con opzioni, raccomandazione e risposta libera, ed è usato anche per la domanda preparata di un Drift.

La Machine `omp-setup` verifica che omp sia pronto: lo cerca, lo installa, lo aggiorna o chiede il collegamento a un provider, e nessun lavoro parte finché non lo è.

Le notifiche di sistema avvisano che un job è finito o aspetta il developer, solo quando la finestra del progetto non è in primo piano.
