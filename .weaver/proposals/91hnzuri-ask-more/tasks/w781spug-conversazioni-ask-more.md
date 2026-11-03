---
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
---
# Conversazioni Ask more

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
