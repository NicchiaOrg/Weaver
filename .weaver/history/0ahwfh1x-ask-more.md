---
created: 2026-11-03T04:32:03.268Z
updated: 2026-11-03T07:40:41.312Z
summary: Da Proposal, Task, Drift, voci della History, domande dei round e Requisiti si apre con Ask more una conversazione con omp, visibile nella vista Jobs; sui Requisiti ce n'è uno solo, nella toolbar.
tasks:
- vlacxl3w
- w781spug
- npzfp0dk
- qsejv7fd
cleared:
  vlacxl3w:
    title: Istruzione di Ask more
    refs:
    - requirements/rules#ask-conversation
    technical: true
    proposal: 91hnzuri
    label: Ask more
    finished: 2026-11-03T03:51:59.867Z
    body: |-
      Quando il Task è finito, `weaver-core` ha una funzione pura che costruisce l'istruzione di Weaver per una conversazione di Ask more, come dice `requirements/rules#ask-conversation`, dai dati della cosa scelta letti dai file (o, per una domanda, dal round in memoria) e dalla lingua scelta dal developer.

      - Il testo fisso è in inglese, incluso nell'app (come la costante dell'istruzione di General in `crates/core/src/jobs/general.rs`), e comincia con la riga `Weaver: Ask more`. Chiede all'agente di: leggere con i comandi elencati ciò che gli serve; rispondere al messaggio del developer e spiegare; cambiare un Task, un Drift o una Proposal solo quando il developer lo chiede e solo con i comandi di `weaver` che lo permettono; non cambiare mai il Model né i Requisiti, che cambiano solo con una Proposal approvata; non scrivere mai a mano nei file sotto `.weaver/`; avviare un job di Weaver solo su richiesta esplicita del developer, come in Chat; chiedere al developer solo nelle sue risposte.
      - Segue la parte che descrive la cosa scelta, su più righe, per ciascuno dei sei tipi:
        - **Task:** identità permanente, numero breve, titolo, corpo completo, riferimenti, percorsi delle immagini e, quando ci sono, la Proposal del suo design (identità e titolo) e il Drift che corregge (identità).
        - **Proposal:** identità permanente, numero breve, titolo, stato, corpo completo, i suoi Task (identità e titolo) e, per una risoluzione, ogni Drift che risolve con la sua decisione.
        - **Drift:** identità permanente, numero breve, titolo, `What happens` e `What should happen` per intero, riferimenti e, quando ci sono, la Proposal allegata, il Task da cui nasce e il Drift che ripete.
        - **Task di una voce della History:** dalla copia nella voce: identità, titolo, corpo, riferimenti, Proposal, etichetta e ora di fine; poi identità, titolo e riassunto della voce.
        - **Domanda:** la domanda in vista per intero (testo, contesto, ogni opzione con etichetta, descrizione e dettagli, l'opzione raccomandata, se è a scelta multipla), poi testo e opzioni (etichetta e descrizione) delle altre domande del round nel loro ordine, segnando quella in vista; poi nome e identità del job che ha aperto il round o, per la domanda pronta di un Drift, l'identità del Drift. Dice che il round è ancora aperto e che l'agente non deve rispondere al posto del developer. Mai scelte, testo scritto, risposte o salti del developer: la funzione non li riceve nemmeno.
        - **Sezione dei Requisiti:** il riferimento `requirements/<area>#<sezione>`, l'Area, il frammento, il titolo e il testo completo come lo mostra la vista; con l'overlay di una Proposal, identità, numero breve e titolo della Proposal e il cambiamento che porta alla sezione: `added`, `changed` con il testo di prima e quello nuovo, `removed`, `moved from <fragment>`.
      - Dopo ogni parte, i comandi di lettura: per le schede `weaver task show <id>`, `weaver proposal show <id>` o `weaver drift show <id>` per la scheda scelta e per ogni scheda collegata, e `weaver model show <ref>` per ogni riferimento; per un Task della History `weaver history show <voce>`, `weaver proposal show <id>` solo se la Proposal esiste ancora, e `weaver model show <ref>` per ogni riferimento; per la domanda pronta di un Drift `weaver drift show <id>`; per una sezione `weaver model show requirements/<area>#<sezione>`, tranne per una sezione `added`, e con l'overlay `weaver proposal show <id>`.
      - Solo identità permanenti, tranne il numero breve della scheda scelta e quello della Proposal dell'overlay, ciascuno detto come il numero breve che vede il developer.
      - Con una lingua scelta si aggiunge solo la frase `Write the texts for the developer in <language>.` (la stessa di General: niente rimando a una skill), con la lingua ripulita come fa già `instruction()`. Nessuna delle frasi delle istruzioni dei job (limite dei sotto-agenti, Sub-agents context sharing, `weaver question new`).
      - La regola di `requirements/rules#job` sull'istruzione di una riga non vale qui: questa istruzione è su più righe e contiene testi di schede e dei Requisiti.

      Test unitari della funzione: la parte di ogni tipo con tutti i campi e con quelli facoltativi assenti; una domanda senza risposte né scelte; la sezione `added` senza `weaver model show`; la frase della lingua presente solo con una lingua scelta e nessun'altra frase aggiunta.
  w781spug:
    title: Conversazioni Ask more
    refs:
    - agent-work
    - ask-conversation
    - developer.ASK_MORE
    - developer.CLOSE_ASK
    - developer.SEND_MESSAGE
    - developer.STOP_AGENT
    - agent.START_JOB
    - job/Not started/agent.START_JOB
    - question-round/Presented/developer.ASK_MORE
    - requirements/ui#ask-conversation
    - requirements/tech#ask-conversation
    - requirements/ui#job
    - requirements/ui#proposal
    - requirements/ui#task
    - requirements/ui#drift
    - requirements/ui#app
    - requirements/ui#history-entry
    - requirements/ui#question-round
    - requirements/tech#question-round
    - requirements/cli#job
    - requirements/cli#agent
    technical: false
    proposal: 91hnzuri
    label: Ask more
    finished: 2026-11-03T04:25:29.533Z
    body: |-
      Quando il Task è finito, il developer apre con `Ask more` una conversazione nuova con omp su ciò che sta leggendo, la vede nella vista Jobs e ci parla; il primo messaggio parte con l'istruzione di Weaver costruita dalla funzione di `weaver-core` del Task "Istruzione di Ask more" (`requirements/rules#ask-conversation`).

      ## Dove sta il pulsante (`requirements/ui#ask-conversation`)
      - Finestra dei dettagli di Proposal, Task e Drift sulla Board: `Button` `outline`, icona `MessageCircleQuestion` a sinistra di `Ask more`, mai sulla scheda stessa. Proposal: dopo i controlli della scheda (anche per una risoluzione e per una Draft con refinement interrotto). Task: dopo `Run`, prima di `Delete`. Drift: dopo `Resolve` e `Dismiss Drift`. Tooltip `Ask an agent about this Proposal` / `this Task` / `this Drift`; nome accessibile `Ask more about <numero breve>`, con l'identità permanente quando la scheda non ha numero breve. I piedi di Task e Proposal oggi riusano i controlli della scheda (`ui/src/components/board/board-view.tsx:CardDetails`, `proposal-card.tsx:ProposalControls`, `task-card.tsx:TaskFooter`, `drift-card.tsx:DriftFooter`): il pulsante va solo nel piede.
      - Finestra di una voce della History (`ui/src/components/history/history-view.tsx:TaskRow`): nella parte espansa della riga di un Task, in alto a destra, `ghost` piccolo con icona e testo `Ask more`; tooltip `Ask an agent about this Task`, nome accessibile `Ask more about <identità permanente>`. La conversazione si chiama `Ask <identità permanente>` e l'istruzione si fa dalla copia nella voce.
      - Round di domande (`ui/src/components/questions/question-pane.tsx:QuestionBody`): pulsante piccolo `ghost` solo icona, in alto a destra del testo della domanda in vista, sulla sua prima riga; tooltip `Ask an agent about this question`, nome accessibile `Ask more about this question`. Non nel riepilogo, nella conferma di Skip all né nella barra ridotta. Premerlo riduce il round alla barra in fondo, senza cambiare domanda in vista, scelte né testo scritto (`question-round/Presented`).
      - Vista Requirements (`ui/src/components/views/requirements-view.tsx:SectionBlock`): in ogni intestazione di sezione mostrata, subito dopo `Open in editor`, `ghost` piccolo con icona e testo `Ask more`, anche con focus, `Only these` e sulle sezioni `added`, `changed`, `removed` dell'overlay; mai sulle intestazioni di Area o di frammento. Tooltip `Ask an agent about this Requirement`, nome accessibile `Ask more about <area>#<sezione>`. Il test `requirements-view.test.tsx` che si aspetta solo `Open in editor` va aggiornato.
      - Sempre abilitato. Premerlo porta la finestra alla vista Jobs con la conversazione nuova selezionata e il focus nel suo campo; la finestra dei dettagli, quella della voce della History e la vista Requirements (overlay, focus, `Only these`, scorrimento) restano com'erano e si ritrovano tornandoci. Testi in inglese.

      ## Apertura (`ask-conversation/Not opened`, `requirements/tech#ask-conversation`)
      - La UI manda tipo e identità della cosa scelta, mai il suo testo: identità della scheda; voce e Task per la History; riferimento della sezione e Proposal dell'overlay; identità del round e posizione della domanda.
      - Il backend legge al momento del clic: le schede dai file con `weaver-core`; la copia del Task dal file della voce; la sezione ricalcolando la vista Requirements con lo stesso overlay (testo come mostrato, testo di prima, frammento di prima, frammento: `crates/core/src/project/view/docs.rs:SectionView`, `proposal/analysis.rs:overlay`, `src-tauri/src/project_view.rs`); la domanda dal round presentato in memoria (`src-tauri/src/rounds/service.rs:Rounds`) controllandone l'identità, senza mai leggere le bozze, con il job che l'ha chiesto o, per la domanda pronta di un Drift, l'identità permanente del Drift, che il round deve tenere oltre al numero breve (`crates/core/src/question/round.rs:Origin::Drift`, `src-tauri/src/drifts.rs:Drifts::press`).
      - Non si apre nulla quando la cosa non esiste più: scheda o voce sparite, round chiuso o un altro round presentato, sezione che la vista con lo stesso overlay non mostrerebbe più o Proposal dell'overlay non più aperta.
      - Ogni clic apre una conversazione nuova, anche sulla stessa cosa; più conversazioni possono stare aperte insieme. Si apre vuota: omp non parte e nulla si manda.

      ## Conversazione (`ask-conversation`)
      - Non è un job: niente record nel registro dei job né in `general/`, niente `JobKind`; non entra in ammissione ed esclusioni (`crates/core/src/jobs/admission.rs`), nella conferma di chiusura della finestra o di uscita (`src-tauri/src/windows/jobs.rs`), nel contatore dei job in attesa né nelle notifiche. Vive in memoria, con un'identità generata all'apertura, e finisce con la finestra: non si conserva e non torna. Il backend riusa il servizio e l'attore di General (`src-tauri/src/general/`) generalizzati a più conversazioni, non la Machine dei job.
      - Primo messaggio (Empty → Working): Weaver avvia `<omp> [--model <analysis>] --approval-mode <mode> acp` (modello di analisi delle Settings, mai quello scelto per una Proposal; niente `--append-system-prompt`, `--advisor`, `--slow`, overlay) e manda un solo `session/prompt` con un blocco di testo: `Instructions from Weaver (not written by the developer):`, a capo, l'istruzione, una riga vuota, `Developer's message:`, a capo, il messaggio del developer ripulito come ogni messaggio. La vista mostra solo il messaggio del developer. I messaggi seguenti partono da soli.
      - Ambiente: `WEAVER_ASK=<identità della conversazione>` e `WEAVER_ENDPOINT`, senza `WEAVER_JOB` né `WEAVER_GENERAL`, più PATH e variabili dei certificati come per General. `weaver job new` è accettato dall'identità di una conversazione di Ask more del progetto mentre è Working, con i rifiuti della richiesta del developer (`crates/cli/src/spec.rs:Access::General`, `crates/cli/src/exec.rs`, `crates/cli/src/help.rs`, `src-tauri/src/jobs/authority.rs:GeneralAuthority`); il job avviato si mostra in quella conversazione con `Started <job name>` e `Open conversation`, il rifiuto con `Weaver did not start <kind>: <reason>.`.
      - Durante un turno: messaggi ricevuti mentre lavora; un permesso aspetta nella conversazione (sotto-stato Waiting for permission) e i messaggi aspettano dopo di lui; nessuna notifica. Fine turno, errore del turno, uscita o mancato avvio di omp → Idle con il motivo mostrato come in Chat. `Stop` (`Square`, `Stop the agent`) senza dialogo annulla solo il turno e, se omp sta partendo, lo chiude.
      - Dopo che omp si è fermato, il messaggio seguente lo riavvia e ricarica la sessione con `session/load` come Chat; nella storia ricaricata il primo messaggio mostra solo il testo dopo `Developer's message:`. Se il ricaricamento fallisce parte una sessione vuota con l'avviso di Chat (`The conversation could not be reloaded (<reason>). A new one started.`) e il messaggio porta davanti l'istruzione ricostruita dai file in quel momento, o parte da solo se la cosa non esiste più. Se nessuna sessione era mai stata aperta (omp non è partito al primo messaggio), il messaggio seguente porta l'istruzione preparata all'apertura.
      - I moduli di omp sono rifiutati con un testo che chiede di chiedere nella risposta e nomina Ask more, non General. Sotto-agenti letti, elencati e mostrati come in un job.
      - Né l'istruzione né i prompt finiscono su disco o nei log.

      ## Vista Jobs (`requirements/ui#ask-conversation`, `requirements/ui#job`, `ui/src/components/jobs/jobs-view.tsx`)
      - Riga: nome `Ask <numero breve>` (`Ask T3`, `Ask P2`, `Ask D4`; identità permanente senza numero breve e sempre per un Task della History), `Ask <area>#<sezione>` (per esempio `Ask ui#app`, `Ask rules#General`) o `Ask question`; tipo `ask` in testo secondario; distintivo `running` durante un turno, `waiting` mentre un permesso aspetta, `idle` (outline, senza icona) prima del primo messaggio e fra i turni; ora di apertura; niente tempo di lavoro né nota.
      - Ordine: le `waiting` e `running` con i job nello stesso stato; le `idle` dopo i job suspended e prima dei job finiti, dalla più recente.
      - `X` sulla riga in ogni stato (tooltip `Close`, nome accessibile `Close <name>`): a riposo chiude subito; durante un turno AlertDialog `Close <name>?` / `The agent's turn stops and the conversation ends.`, `Close` (distruttivo) e `Cancel`, focus su `Cancel`; confermato annulla il turno e chiude omp. Chiusa prima del primo messaggio non manda nulla.
      - Conversazione resa come quella di un job (messaggi, `Thinking`, righe degli strumenti, sotto-agenti con intestazione `Ask T3 › <sub-agent name>`, permessi, messaggi in coda, `Jump to latest`), senza voce dell'istruzione, note, riga di fine o riga di job finito. Vuota mostra solo il campo, con il segnaposto `Ask about the Proposal…`, `Ask about the Task…`, `Ask about the Drift…`, `Ask about the Requirement…` o `Ask about the question…`; dopo il primo messaggio `Message the agent…`. Un messaggio mandato mentre omp parte mostra `Starting omp…` sotto di sé. Avvii di job, errori, omp mancante e sessione non ricaricata come in Chat. Nessuna barra dell'uso.
      - `Back to questions` (`ArrowLeft`) in cima a una conversazione aperta da una domanda, finché quel round è aperto: riapre il round dove era; sparisce quando il round finisce, comunque finisca.

      ## Resto
      - Togliere `"ask more"` da `OUT_OF_SCOPE_PHRASES` in `crates/core/src/agent_files.rs`, ora che Weaver ha Ask more.
      - Una riga sotto `## [Unreleased]` del `CHANGELOG.md` per Ask more, senza identità di Task o Proposal.
      - Test con l'agente ACP finto: nulla parte all'apertura; il primo prompt porta istruzione e messaggio, i seguenti solo il messaggio; chiusa prima del primo messaggio omp non parte; un ricaricamento fallito rimanda l'istruzione. Test dell'endpoint: `weaver job new` accettato da una conversazione di Ask more che lavora e rifiutato da una a riposo o chiusa. Test della UI: ogni pulsante manda tipo e identità giusti, il round si riduce senza cambiare bozze, la riga ha nome, tipo, distintivi e `X` come sopra.
  npzfp0dk:
    title: Ask more su voce History
    refs:
    - agent-work
    - developer.ASK_MORE
    - requirements/rules#ask-conversation
    - requirements/tech#ask-conversation
    - requirements/ui#ask-conversation
    - requirements/ui#history-entry
    technical: false
    proposal: xn60waw9
    label: Ask more History
    finished: 2026-11-03T07:18:16.637Z
    body: |-
      `Ask more` nella History si chiede sull'intera voce, non più sul singolo Task: la conversazione riguarda la voce e tutti i Task che cita. Il pulsante sul singolo Task nella parte espansa della riga sparisce, e con esso il tipo di conversazione su un solo Task della History (nessun alias o percorso rimasto).

      Quando il lavoro è finito vale quanto segue.

      **Apertura (`ask-conversation/Not opened`, `developer.ASK_MORE`)**
      - Nella finestra di una voce della History, il piede contiene un `Button` `outline` con l'icona `MessageCircleQuestion` a sinistra del testo `Ask more`, sempre abilitato. Tooltip `Ask an agent about this History entry`; nome accessibile `Ask more about <identità della voce>`. Le righe dei Task, espanse, mostrano corpo e riferimenti in sola lettura, senza `Ask more`.
      - Premendolo, la finestra mostra la vista Jobs con la nuova conversazione selezionata e il focus nella sua casella dei messaggi; la finestra della voce resta com'era e si ritrova com'era tornando alla History. Non parte omp e non si invia nulla.
      - La UI invia solo l'identità della voce. Il backend legge la voce e le copie dei Task citati dal file della voce; un Task citato senza copia è letto dal suo file come fa la vista History. Solo una voce che non esiste più non apre nulla; un Task citato non leggibile non impedisce mai l'apertura.
      - Nome nella vista Jobs: `Ask history <identità della voce>` (per esempio `Ask history k3m9x2qa`). Segnaposto della conversazione vuota: `Ask about the History entry…` (serve un tipo di conversazione proprio, non quello del Task).

      **Istruzione (`requirements/rules#ask-conversation`)**
      - La parte che descrive la cosa scelta contiene identità, titolo, riassunto e testo completo della voce; poi ogni Task citato, nell'ordine di citazione, dalla sua copia: identità, titolo, corpo completo, riferimenti, Proposal, label e ora di fine. Un Task senza copia è letto dal suo file come lo mostra la History; uno non leggibile è elencato con la sola identità e marcato come non disponibile.
      - Comandi di lettura dopo la parte: `weaver history show <voce>`, poi `weaver proposal show <id>` una volta per ogni Proposal dei Task che esiste ancora e `weaver model show <ref>` una volta per ogni riferimento dei Task, senza ripetizioni.
      - Quando una sessione persa viene sostituita, l'istruzione è ricostruita dai file in quel momento come per gli altri tipi; se la voce non esiste più il messaggio va da solo.

      **Dove nel codice (dall'esplorazione)**
      - `src-tauri/src/ask/build.rs` — `AskTarget::History { entry, task }` (riga ~42), il ramo `History` di `build` (~148, nome `Ask {task}`, `AskKind::Task`), `history_task()` (~265): da sostituire con una variante solo sulla voce e un lettore dell'intera voce. La stessa logica copia-o-file-del-Task esiste anche in `crates/cli/src/commands/history.rs:task_json` e `src-tauri/src/history.rs:entry_of`/`task_of`.
      - `crates/core/src/jobs/ask.rs` — `Chosen`, `HistoryTaskPart`, `history_part()`, `instruction()`.
      - `src-tauri/src/general/wire.rs` — `AskKind` (nuovo valore per la voce); `ui/src/components/ask/ask-model.ts:PLACEHOLDERS`.
      - `ui/src/bindings/AskTarget.ts`, `AskKind.ts` — generati con ts-rs.
      - `ui/src/components/history/history-view.tsx` — `TaskRow` (pulsante per Task da togliere), `EntryDialog` (usa `FormDialog`, che accetta `footer`); `ui/src/components/ask/ask-button.tsx:AskMoreButton` è il pulsante `outline` dei dettagli della Board.
      - Test da aggiornare: `ui/src/components/history/history-view.test.tsx` (~280-309), `ui/src/components/ask/ask.test.tsx` (segnaposto, ~177-190), `src-tauri/src/ask/tests.rs` (~262-300, fixture `HISTORY` con un solo Task citato), test puri in `crates/core/src/jobs/ask.rs` (~832-885).
  qsejv7fd:
    title: Ask more unico sui Requirements
    refs:
    - agent-work
    - developer.ASK_MORE
    - ask-conversation
    - requirements/ui#app
    - requirements/ui#ask-conversation
    - requirements/rules#ask-conversation
    - requirements/tech#ask-conversation
    technical: false
    proposal: 4s23sjw4
    label: Ask more Requirements
    finished: 2026-11-03T07:38:33.464Z
    body: |-
      La vista Requirements passa da un `Ask more` per ogni sezione a un solo `Ask more` sui Requirements nel loro insieme. L'agente non riceve più il testo di una sezione: sa che i Requirements stanno nelle Requirement skill `trama-req-*` del progetto e carica da sé quelle adatte alla domanda. `Ask more` su card, domande e voci di History non cambia.

      Quando tutto è fatto:

      - **Toolbar della vista Requirements:** subito dopo l'overlay picker c'è un `Ask more`, un piccolo `Button` `ghost` con l'icona `MessageCircleQuestion` e il testo `Ask more`; tooltip `Ask an agent about the Requirements`, nome accessibile `Ask more about the Requirements`. È visibile sempre, anche con un focus o `Only these` attivi e con qualsiasi overlay. Le intestazioni di sezione, frammento e Area non hanno alcun `Ask more` (la sezione mostra solo titolo e `Open in editor`). La toolbar della vista Specs ha solo l'overlay picker.
      - **Premere il pulsante:** apre la conversazione nella vista Jobs, la mostra e mette il focus nella sua casella di messaggio; non invia nulla e non avvia omp. Focus, `Only these`, scroll e overlay della vista Requirements restano come erano. La riga in Jobs si chiama `Ask Requirements` e il placeholder della casella è `Ask about the Requirements…`.
      - **Apertura:** per i Requirements la conversazione si apre sempre (stato Empty). La UI manda solo l'overlay Proposal in vista, se c'è; mai sezione, Area, frammento, focus o `Only these`. Il backend controlla che la Proposal sia ancora aperta con lo stesso criterio dell'overlay picker (gli eligible overlays: aperta e non legata a una Drift); se non lo è, costruisce l'istruzione senza di essa e apre comunque. Il backend non legge testo dei Requirements né skill.
      - **Istruzione, parte Requirements:** dice che lo sviluppatore chiede dei Requirements del progetto nel loro insieme; che stanno nelle Requirement skill `trama-req-*` installate nel progetto, che contengono i Requirements correnti; e che l'agente carica da sé quelle adatte al messaggio, quante gliene servono. Non elenca skill, non nomina Area, frammento o sezione e non copia testo dei Requirements. Con un overlay ancora aperto cita identità, short number e titolo della Proposal, dice che la vista mostra i Requirements con quella Proposal applicata e che le sue modifiche si leggono con `weaver proposal show <id>`, non nelle skill; è l'unico comando di lettura elencato. Senza overlay (o con un overlay non più aperto) non è elencato alcun comando e la parte parla solo dei Requirements correnti.
      - **Altre parti dell'istruzione:** continuano a non nominare alcuna skill; l'unica eccezione ammessa è il richiamo alle Requirement skill della parte Requirements. Nessun meccanismo inietta le skill: la riga di comando di omp resta invariata e omp, avviato nella radice del progetto, trova da sé le `trama-req-*` installate.
      - **Sessione persa:** se il caricamento della sessione fallisce e la si rimpiazza, l'istruzione ricostruita dai file di quel momento cita l'overlay solo se è ancora aperto; una conversazione Ask more sui Requirements non resta mai senza istruzione.
      - **Altri Ask more:** card (Proposal, Task, Drift), domande dei round e voci di History restano come sono; in particolare la voce di History resta sull'intera voce, dal footer del suo dialogo.
      - **Pulizia:** il vecchio tipo per sezione dei Requirements sparisce ovunque: target e kind nel backend e nella UI, la parte di istruzione per sezione (`section_part`, `SectionPart`, `SectionChange`, `Chosen::Section`), la ricerca della sezione su `project_view::snapshot`, il nome `Ask {area}#{section}`, il placeholder `Ask about the Requirement…`. I binding TypeScript (`AskTarget.ts`, `AskKind.ts`, `AskRow.ts`) sono rigenerati con ts-rs, non modificati a mano. I commenti superati (doc di modulo e di `Built`, commento in `requirements-view.tsx`, commento di `BoardActions.askMore`) sono aggiornati. I test del comportamento per sezione sono sostituiti da test del nuovo comportamento (pulsante in toolbar solo in Requirements e visibile con focus/`Only these`, nessun pulsante nelle intestazioni, placeholder, apertura con overlay aperto, con overlay non più aperto e senza overlay, testo dell'istruzione). La riga del CHANGELOG che parla di "a section of the Requirements" parla dei Requirements nel loro insieme.

      Punti del codice noti dall'esplorazione:

      - `requirements/ui#app` — ui/src/components/views/requirements-view.tsx:SectionBlock
      - `requirements/ui#app` — ui/src/components/views/requirements-view.tsx:RequirementsView
      - `requirements/ui#app` — ui/src/components/views/view-frame.tsx:ViewFrame
      - `requirements/ui#app` — ui/src/components/views/overlay-picker.tsx:OverlayPicker
      - `requirements/ui#app` — ui/src/components/views/specs-view.tsx:SpecsView
      - `requirements/ui#app` — ui/src/components/board/board-ui.tsx:BoardActions
      - `requirements/ui#ask-conversation` — ui/src/components/ask/ask-model.ts:PLACEHOLDERS
      - `requirements/ui#ask-conversation` — ui/src/components/ask/use-asks.tsx:askMore
      - `requirements/ui#ask-conversation` — ui/src/components/jobs/board-actions.tsx:JobsBoardActions
      - `requirements/ui#ask-conversation` — ui/src/components/jobs/jobs-view.tsx:AskListRow
      - `requirements/ui#ask-conversation` — ui/src/components/ask/ask-conversation.tsx:AskConversation
      - `requirements/tech#ask-conversation` — src-tauri/src/ask/build.rs:AskTarget
      - `requirements/tech#ask-conversation` — src-tauri/src/ask/build.rs:requirement
      - `requirements/tech#ask-conversation` — src-tauri/src/ask/build.rs:Built
      - `requirements/tech#ask-conversation` — src-tauri/src/ask/mod.rs:open
      - `requirements/tech#ask-conversation` — src-tauri/src/general/service.rs:Rebuild
      - `requirements/tech#ask-conversation` — src-tauri/src/general/actor.rs:load_answered
      - `requirements/tech#ask-conversation` — src-tauri/src/general/wire.rs:AskKind
      - `requirements/tech#ask-conversation` — crates/core/src/project/view/mod.rs:eligible_overlays
      - `requirements/tech#ask-conversation` — ui/src/bindings/AskTarget.ts:AskTarget
      - `requirements/rules#ask-conversation` — crates/core/src/jobs/ask.rs:INSTRUCTION
      - `requirements/rules#ask-conversation` — crates/core/src/jobs/ask.rs:section_part
      - `requirements/rules#ask-conversation` — crates/core/src/jobs/ask.rs:Chosen
      - `requirements/rules#ask-conversation` — crates/core/src/project_files/requirement_skills.rs:SKILL_PREFIX
      - `ask-conversation` — src-tauri/src/ask/tests.rs:an_overlay_proposal_that_is_not_open_opens_nothing
      - `ask-conversation` — ui/src/components/views/requirements-view.test.tsx:RequirementsView
      - `agent-work` — CHANGELOG.md:Unreleased
---
# Ask more

Il pulsante Ask more apre una conversazione libera con omp su ciò che il developer sta leggendo: Proposal, Task, Drift, Task di una voce della History, domanda di un round o i Requisiti nel loro insieme. La conversazione non è un job: vive in memoria e finisce con la finestra. Il primo messaggio parte con un'istruzione di Weaver, costruita dai file al momento del clic, che descrive la cosa scelta e i comandi di lettura utili. Nella vista Requirements c'è un solo Ask more, nella toolbar dopo il selettore di overlay, e non più uno per sezione: l'agente carica da sé le Requirement skill adatte alla domanda e conosce l'overlay Proposal solo se è ancora aperta. Nella vista Jobs ogni conversazione ha la sua riga con stato running, waiting o idle e si chiude con la X. Da una domanda si torna al round con Back to questions.
