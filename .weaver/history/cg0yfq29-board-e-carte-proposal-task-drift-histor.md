---
created: 2026-11-02T12:31:02.881Z
updated: 2026-11-02T12:31:02.881Z
summary: "Board con Backlog e Active e le Machine delle carte: Proposal, Task, Drift, voci di History e richiesta di design, con regole, comandi CLI e interfaccia di ciascuna."
tasks:
  - 4jlsx639
  - wtf2puh1
  - zov4m67w
  - afvb4s7c
  - bdevj10k
  - qfxwz5qt
cleared:
  4jlsx639:
    title: Board
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T09:03:13.766Z
    refs:
      - work
      - developer
      - requirements/ui#work
      - requirements/rules#work
    body: |-
      Weaver mostra la Board della Feature `work` e offre nell'app le decisioni dell'Actor `developer`. La Board mostra ciò che aspetta una decisione (Proposal e Drift aperti) e i Task ancora da finire; il lavoro finito la lascia e si racconta in History. Dipende da "Scheletro del progetto" e "Shell e aspetto dell'app"; le card di ogni tipo sono descritte in "Proposal", "Task" e "Drift", i ghost vengono da "Lavori dell'agente".

      ## Colonne e intestazione (`requirements/ui#work`)
      - Due colonne di uguale larghezza: `Backlog` (Proposal aperte e Drift aperti in un unico ordine di creazione, più vecchi prima) e `Active` (Task in To do e In progress, bloccati compresi). Nessuna colonna Done, né altra colonna.
      - Ogni titolo di colonna mostra il numero delle card reali (i ghost non contano, una card di resolution conta una volta); testi vuoti `Nothing to decide` e `Nothing active`. Il raggruppamento del Backlog arriva calcolato dal backend; l'interfaccia non raggruppa.
      - Nessuna card si trascina, si sposta o si toglie. Clic sulla card fuori dai controlli, o Invio sul titolo, apre il dialogo dei dettagli. Ogni colonna scorre da sola, con il footer sempre visibile.
      - Footer: `+ Work Item` (sempre abilitato, apre una nuova richiesta, vedi "Richiesta di design") e `Run all` con i motivi disabilitati `Drifts are being resolved`, `A run is in progress`, `No Task is in To do`, `All the Tasks in To do are blocked`.
      - Intestazione: `Scan System` con `· <n> done since last scan` (Task finiti, non chiusi senza modifiche, dall'avvio dell'ultimo Scan System; nascosto a 0). Chiede conferma `Compare the whole System with the code?` (`Scan System` / `Cancel`) e confronta sempre tutto il Sistema.

      ## Ghost, dettagli, avvisi
      - Ghost in fondo alla colonna di destinazione, in ordine di annuncio: bordo tratteggiato, 70% di opacità, shimmer, tipo, titolo, fase (`Preparing`, `Writing`, `Waiting for your answers`, `Being refined`) e ultima nota del job su una riga. È un unico pulsante che apre la conversazione del job; all'arrivo della card il ghost sparisce nello stesso quadro.
      - Il dialogo dei dettagli segue la sua card senza spostare il focus e si chiude da solo quando la card lascia la Board, riportando il focus alla colonna.
      - Avvisi in cima alla Board, più recenti prima, fino alla chiusura: decisioni rifiutate all'applicazione (`Could not delete T4: the agent that filed it is still working.`) e Proposal rimandate in Draft mentre il developer le decideva.

      ## «Concerns the same thing» (`requirements/rules#work`)
      - Chiave di un riferimento: un elemento è se stesso; `requirements/<area>#<element>` è l'elemento; `requirements/<area>#General` solo se stesso; `requirements/<area>` l'Area; `specs#<field>` il campo; `specs` le Specs.
      - Contenimento: Feature ⊃ Machine; Machine ⊃ State, Event e contenuti; State ⊃ sotto-State e Transition uscenti; Actor ⊃ Event; Area ⊃ solo la sua General; `specs` ⊃ ogni campo.
      - Due riferimenti riguardano la stessa cosa se le chiavi coincidono o una contiene l'altra; due card se un riferimento dell'una lo fa con uno dell'altra; una card senza riferimenti non riguarda nulla. Una sola funzione del core serve blocco, slot, gruppi e ricerca dei Drift.

      ## Decisioni del developer
      - Ogni decisione (Proposal, Drift, Run, Delete) avviene intera o non cambia nulla, e le sue condizioni sono ricontrollate dal backend nel momento in cui si applica; un rifiuto produce un avviso e nessuna modifica.
      - Un controllo disabilitato elenca tutti i motivi che valgono, uno per riga, nell'ordine fissato; `The agent is still finishing` viene sempre primo; si riabilita da solo senza spostare il focus.
      - Scelta del developer: i Task non si spostano a mano; dall'app solo `Run` e, in To do, `Delete`. Per altro si chiede all'agente in General, che decide da riga di comando solo su richiesta esplicita del developer, contando come Event del developer.

      ## Fatto quando
      - Test del core sulla funzione «concerns the same thing» per ogni tipo di riferimento e contenimento.
      - Test Vitest: colonne, conteggi senza ghost, testi vuoti, motivi disabilitati di `Run all` in ordine, conferma di `Scan System`.
      - Una decisione rifiutata all'applicazione mostra l'avviso e non scrive nulla.
      - Un dialogo aperto si aggiorna e si chiude quando la sua card lascia la Board.
  afvb4s7c:
    title: Drift
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T12:25:52.743Z
    refs:
      - drift
      - requirements/data#drift
      - requirements/cli#drift
      - requirements/rules#drift
      - requirements/ui#drift
    body: |-
      Weaver gestisce la Machine `drift`: una differenza fra il codice e il Modello o i Requisiti che solo il developer decide. Nessun Drift si decide, si risolve o si esegue da solo. Dipende da "Scheletro del progetto" e "Core dei file del progetto"; si appoggia a "Round di domande", "Lavori dell'agente", "Proposal" e "Task".

      ## Registrazione
      - L'agente lo registra dopo aver riletto il codice: durante uno scan, durante un Run (l'advisor per codice già esistente) o da un job su richiesta del developer. Nomina riferimenti, cosa succede, cosa dovrebbe succedere e dove nel codice; offre da 2 a 5 opzioni di correzione del codice, la prima raccomandata, nessuna che cambi il Modello; nessuna opzione solo con una Proposal attaccata. Può nominare l'origine (Task) e un Drift precedente (`previous` automatico se l'origine è un fix Task).
      - Non registra di nuovo una differenza scartata finché il testo del Modello e dei Requisiti è invariato (impronta di testo salvata allo scarto).
      - Prima della fine del suo job prepara la domanda di decisione (Async Drift Resolver, sempre attivo).

      ## Stati
      - `Open`: blocca i Task che riguardano la stessa cosa. `agent.PREPARE_DRIFT_QUESTION` salva la domanda. `developer.RESOLVE_DRIFT`: con una domanda preparata valida Weaver la pone da sé come round, senza agente; altrimenti avvia un job di resolve; rifiutato mentre lavora un Run, mentre un'altra resolution è scritta o attende decisione, o mentre il job che l'ha registrato lavora. `question-round.ANSWERED` della domanda preparata: Weaver scrive e consegna da sé la resolution `Resolve: <Drift title>` con i controlli dell'agente; una domanda saltata lascia il Drift com'è; una risposta libera accanto a una scelta diversa da un fix, o una consegna rifiutata (Draft scartata), avvia un job di resolve con la risposta già data.
      - `developer.FIX_DRIFT_CODE` → `Code to be fixed`: opzione obbligatoria se ce ne sono; riferimenti scomparsi sostituiti dal contenitore più vicino (Transition → State → Machine → Feature; Event → Machine o Actor; sezione mancante tolta), rifiutato se non resta alcun elemento del Modello. Nasce il fix Task con i due lati, l'opzione, la nota e il dove nel codice.
      - `developer.ACCEPT_DRIFT_PROPOSAL` → `Proposal accepted` solo con una Proposal attaccata approvabile. `developer.DISMISS_DRIFT` → `Dismissed`, rifiutato se il Drift è in una resolution scritta o in attesa.
      - `proposal.APPROVED` di una resolution porta a `Code to be fixed`, `Proposal accepted` o `Dismissed` secondo la decisione registrata.
      - `Code to be fixed`: blocca solo l'origine. `task.FINISHED` del suo fix Task → `Fixed`; `task.CLOSED_UNCHANGED` → `Dismissed` con la nota.
      - `Fixed`, `Proposal accepted`, `Dismissed` sono finali; il file resta. Eventi `CODE_TO_BE_FIXED`, `PROPOSAL_ACCEPTED`, `DISMISSED` agiscono sulla Proposal attaccata.

      ## Domanda preparata (`requirements/rules#drift`)
      - Una domanda a scelta singola con id uguale all'identità del Drift: un'opzione per ogni fix nell'ordine più una per il comportamento attuale (`proposal` o `dismissed`); senza opzioni esattamente `proposal` (raccomandata) e `dismissed`.
      - Valida finché il Drift è aperto, il blocco si legge, l'impronta coincide con quella attuale, ogni `fix-code` nomina un'opzione esistente e un `proposal` ha una Proposal in Draft o Open.
      - Impronta: SHA-256 sui riferimenti in ordine (`# <ref>`, testo di `trama model show <ref>` e sezioni dei Requisiti), `(missing)` per i mancanti; una sola funzione del core.

      ## File, comandi, card
      - `.trama/drifts/<id>-<slug>.md` con front matter e sezioni nell'ordine di `requirements/data#drift`, compreso `## Prepared question` con un blocco `json`.
      - `trama drift new|show|list|edit|move` come in `requirements/cli#drift`; rispondere a una domanda preparata esiste solo nell'app.
      - Card compatta con bordo rosso, badge `recurring`, `resolving`, `prepared`, `Resolve` e `Dismiss Drift` con motivi disabilitati in ordine; dettagli con chip, lati affiancati, opzioni, dove nel codice con `Open in editor`, origine, Proposal attaccata, decisione `Fix the code` / `Accept Proposal` (`requirements/ui#drift`).

      ## Fatto quando
      - Test del core su ogni transizione, contenitore più vicino, validità della domanda preparata e impronta.
      - Resolve con domanda valida presenta il round senza avviare omp; la risposta produce una resolution consegnata.
      - Fix code crea il fix Task e rimette l'origine in To do in attesa.
      - Test Vitest di card e dettagli, motivi disabilitati compresi.
  bdevj10k:
    title: History
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T09:24:09.062Z
    refs:
      - history-entry
      - requirements/data#history-entry
      - requirements/cli#history-entry
      - requirements/ui#history-entry
    body: |-
      Weaver tiene la History del progetto secondo la Machine `history-entry`: ciò che un job ha fatto, raccontato dal suo agente per una macro-feature, con titolo, riassunto di una riga, testo e i Task finiti che raggruppa. Dipende da "Scheletro del progetto" e "Core dei file del progetto"; il ciclo dei Task è in "Task".

      ## Comportamento
      - L'agente scrive le voci alla fine di un Run, una per macro-feature, e può scriverne per qualunque altro job che abbia finito Task. Weaver non scrive mai una voce da sé: un Task finito e mai citato resta fuori dalla History e viene dimenticato dopo altri 20 Task finiti.
      - Stato unico `Written` (iniziale), datato al momento della scrittura o dell'ultimo aggiornamento. `agent.UPDATE_HISTORY_ENTRY` → `Written`: l'agente riscrive il testo o aggiunge Task finiti, mai ne toglie; la data diventa quella dell'aggiornamento.
      - L'Event `TASKS_CITED` è emesso quando la voce è scritta o aggiornata, per i Task citati la prima volta: ognuno passa a `Recorded in History`. Prima di cancellare il file del Task, Weaver scrive la sua copia (titolo, corpo, riferimenti, Proposal, label, `finished`) in ogni voce che lo cita; se la copia fallisce il Task resta e la Board mostra un avviso.

      ## File (`requirements/data#history-entry`)
      - `.trama/history/<id>-<slug>.md`, versionato; front matter `created`, `updated` (la data della voce), `summary` (una riga di 1–280 caratteri, solo Markdown inline), `tasks` (identità citate, senza duplicati, mai in diminuzione), `cleared` (mappa identità → copia `{title, refs, technical, proposal?, label?, finished, body}` scritta una volta e mai cambiata; scriverla non cambia `updated`). Corpo: un H1 con il titolo, poi il testo.
      - Le identità dei Task copiati restano uniche anche rispetto alle nuove.

      ## Comandi (`requirements/cli#history-entry`)
      - `trama history new {title, summary, body?, tasks}`: titolo su una riga, riassunto di 1–280 caratteri, almeno un Task, ciascuno Done e non chiuso senza modifiche; ogni problema elencato, nulla scritto.
      - `trama history edit <id> {title?, summary?, body?, tasks?}` sostituisce campi e aggiunge Task, mai ne toglie.
      - Entrambi stampano `id`, `title`, `file`, `recorded` (Task ora registrati). `trama history list {}` dal più recente; `trama history show <id>` include i Task citati.

      ## Vista History (`requirements/ui#history-entry`)
      - Voci dalla più recente per data, raggruppate per giorno locale: `Today · <n>`, `Yesterday · <n>`, poi la data lunga (`Monday, 2 November 2026 · <n>`).
      - Riga: titolo, riassunto su una riga (tagliato con `…`, testo intero come tooltip), ora `HH:MM` in Today e Yesterday, numero di Task.
      - Filtro periodo (Select): `Today`, `Last 7 days` (predefinito a ogni apertura della finestra), `Last 30 days`, `All`; ogni periodo va dalla mezzanotte locale del primo giorno a ora; una voce datata nel futuro compare solo in `All`. Vuoto: `Nothing done in this period`.
      - Clic su una voce: Dialog con titolo, data, riassunto, testo renderizzato e i Task dalle copie nell'ordine di citazione (id, titolo, badge della label, ora di fine); una riga si espande con corpo e riferimenti, in sola lettura.

      ## Fatto quando
      - Test del core: `TASKS_CITED` solo per i Task citati la prima volta, copia scritta prima della cancellazione del file, `tasks` mai in diminuzione, copia fallita che lascia il Task.
      - Test CLI: `history new` rifiuta Task non Done o chiusi senza modifiche elencando ogni problema.
      - Test Vitest della vista: raggruppamento per giorno, filtro e confini dei periodi, dialogo con i Task copiati.
  qfxwz5qt:
    title: Richiesta di design
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T11:34:25.537Z
    refs:
      - design-request
      - requirements/data#design-request
      - requirements/ui#design-request
    body: |-
      Weaver permette al developer di scrivere una richiesta di design in un editor Markdown dentro l'app (scelta del developer: niente editor esterno) e la gestisce secondo la Machine `design-request`. La richiesta di setup è lo stesso documento, scritto quando il Modello è vuoto e il developer ha scelto di partire da zero. Dipende da "Scheletro del progetto" e "Core dei file del progetto"; l'avvio dei job è in "Lavori dell'agente", il consumo alla consegna in "Proposal".

      ## Stati
      - `Not written`: `developer.NEW_REQUEST` (pulsante `+ Work Item` della Board o Start from Scratch) crea una richiesta vuota e la apre in `Writing`.
      - `Writing`: ogni modifica è salvata subito. `developer.SEND_REQUEST` → `Sent` solo se non è vuota: Weaver avvia un job di design, o di setup se il Modello è vuoto; inviare una richiesta vuota non fa nulla. `developer.DISCARD_REQUEST` → `Discarded` cancellando file e immagini. `workspace.REQUEST_DELETED` → `Discarded`: niente viene mai più inviato, anche se Weaver la mostrava ancora.
      - `Sent`: leggibile, non reinviabile. `proposal.SUBMITTED` della prima Proposal legata → `Consumed`, file e immagini cancellati. `job.ENDED` senza una Proposal legata consegnata → di nuovo `Writing`, modificabile e reinviabile. `workspace.REQUEST_DELETED` → `Discarded`.
      - `Consumed` e `Discarded` sono finali: una richiesta consumata non torna nemmeno se un editor risalva il file.
      - Chiudere l'editor o la finestra non invia mai la richiesta: parte solo con Send.

      ## File (`requirements/data#design-request`)
      - `.trama/requests/<id>.md`, Markdown senza front matter; `<id>` è l'ora locale di creazione `YYYY-MM-DDTHH-MM-SS`, con `-2`, `-3`… se il nome è preso. La cartella ha il suo `.gitignore` con `*`.
      - Immagini incollate o trascinate (PNG, JPEG, GIF, WebP) copiate in `.trama/requests/<id>.images/` e collegate con percorso relativo; quelle già nel progetto sono collegate dove sono.
      - Titolo: primo H1 fuori dai blocchi di codice, altrimenti la prima riga non vuota che non sia solo immagini, altrimenti l'id. Vuota = nulla dopo aver tolto gli spazi; una richiesta di sole immagini non è vuota.
      - `.trama/requests/.state.json` (`{sent: {<id>: <job id>}, consumed: [<id>]}`), scritto solo dall'app: un id consumato non è mai più mostrato né inviato.
      - Consumare o scartare cancella `<id>.md` e `<id>.images/` in un solo passo.
      - L'istruzione del job nomina il file della richiesta, mai il suo testo.

      ## Editor (`requirements/ui#design-request`)
      - Pannello con le richieste aperte a sinistra (titolo o `Untitled request`, ora di creazione, badge `Draft` o `Sent`, più recenti prima) e quella scelta a destra; intestazione `Setup request` o `Design request`; si apre con il focus nel testo.
      - Textarea monospazio con a capo morbido; `Preview` affiancata da circa 900 px, aggiornata entro 150 ms.
      - Autosalvataggio 500 ms dopo l'ultimo tasto e su blur, cambio vista e chiusura finestra; stato `Saving…`, `Saved` o `Could not save: <reason>.` (il prossimo cambiamento riprova). Scrivere non avvia alcun processo.
      - Cambio esterno: ricarica se nulla è in sospeso, altrimenti vince la versione di Weaver con il toast `The request changed on disk; Weaver kept your version.`; cancellazione esterna: toast `The request was deleted outside Weaver.`
      - `Send` (`SendHorizontal`, anche `Ctrl/Cmd+Enter`) disabilitato col testo vuoto, tooltip `Write the request first.`; `Discard` con conferma `Discard this request?` / `Its file and images are deleted.`
      - Inviata: vista in sola lettura con banner `Sent to the agent · <job name>` e `Open conversation`; tornata in scrittura: banner `The job ended without a Proposal. Change the request and send it again.`
      - Immagini fino a 10 MB come `image-<n>.<ext>`, altrimenti toast `Only PNG, JPEG, GIF or WebP images up to 10 MB.`

      ## Fatto quando
      - Test del core sulla Machine: invio di una richiesta vuota ignorato, consumo solo dalla prima Proposal legata, ritorno in `Writing` alla fine del job senza Proposal, file cancellato da fuori mai più inviato, id consumato ignorato anche se il file riappare.
      - Send su Modello vuoto avvia un job di setup, altrimenti di design.
      - Test Vitest dell'editor: autosalvataggio, stati di salvataggio, Send disabilitato, conferma di Discard, incolla di immagini.
      - Chiudere la finestra con una richiesta in scrittura la salva e non la invia.
  wtf2puh1:
    title: Proposal
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T10:13:53.530Z
    refs:
      - model-changes
      - proposal
      - requirements/data#proposal
      - requirements/cli#proposal
      - requirements/rules#proposal
      - requirements/ui#proposal
    body: |-
      Weaver gestisce la Machine `proposal` per intero: la cartella `.trama/proposals/<id>-<slug>/` con `proposal.md` (front matter come in `requirements/data#proposal`: `status`, `kind`, `requires`, `base`, `delete`, `created`, `request`, `reworks`, `reworkRequested`, `label`, `refining`, `job`, `drifts`), le copie complete dei file cambiati e i `tasks/<id>-<slug>.md` con soli `technical` e `refs`. Kind: `change` e `setup` portano Task, `import` e `resolution` nessuno. Si appoggia a "Scheletro del progetto", "Core dei file del progetto" e "Lavori dell'agente".

      ## Stati e transizioni
      - `Draft`: l'agente di un job la scrive (`agent.EDIT_PROPOSAL`); nessuno la decide. `agent.SUBMIT_PROPOSAL` → `Open` se ogni controllo di consegna passa (consuma la richiesta legata, vedi "Richiesta di design"); → `Approved` per una Proposal `setup` o `import` consegnata da un job che, arrivato in Running, ha trovato il Modello senza Feature né Actor, con tutti i controlli di Approve tranne `The agent is still finishing`; le Proposal dello stesso job che richiede sono già approvate così, quindi una catena di import passa intera; dopo il setup approvato parte Run all. `developer.DISCARD_PROPOSAL` → `Discarded` solo se nessun job attivo la scrive (anche se attaccata a un Drift, che resta aperto). `developer.APPROVE_PROPOSAL` → `Refining` solo per una Draft lasciata a metà da un raffinamento. `drift.CODE_TO_BE_FIXED` e `drift.DISMISSED` → `Discarded` per la Draft attaccata.
      - `Open`: `developer.APPROVE_PROPOSAL` → `Approved` solo se niente la blocca, il job che l'ha consegnata è finito e nessun Run lavora; → `Refining` (raffinamento all'Approve, sempre attivo) per una `change` non attaccata a un Drift bloccata solo da staleness: un job di raffinamento la adatta senza nuova intervista. `developer.REJECT_PROPOSAL` → `Rejected` con il motivo facoltativo; `developer.REWORK_PROPOSAL` → `Rejected` con il feedback (`## Feedback` solo se non vuoto, conferma per feedback vuoto, al massimo un rework): un job di rework la riscrive e la prima Proposal aperta con `reworks` ne prende il posto sulla Board e nei `requires` di chi la aspettava. `developer.CHOOSE_PROPOSAL_MODEL` resta `Open`. `agent.EDIT_PROPOSAL` → `Draft` anche senza differenze. `drift.PROPOSAL_ACCEPTED` → `Approved`, `drift.CODE_TO_BE_FIXED` → `Rejected`, `drift.DISMISSED` → `Discarded` solo per la Proposal attaccata, che non si decide mai da sola.
      - `Refining`: mantiene identità, numero breve, posto, label, modello scelto e chi la richiede; l'agente chiede solo se il Modello contraddice una decisione presa, o se rimuovere un cambiamento già presente (`developer.REJECT_PROPOSAL` → `Rejected`). Consegnata → `Approved` se niente la blocca (aspettando la fine di un Run), altrimenti `Open` con i motivi; `job.ENDED` senza consegna → `Draft` con `refining: true`.
      - `Approved`, `Rejected`, `Discarded` sono finali; la cartella resta com'era.

      ## Regole (`requirements/rules#proposal`)
      - Scope (`changed`, `leads into a change`, `removed, its container`) e copertura calcolati dal vivo, mai salvati; copertura da riferimenti di Task o righe `## Already present`; Task di rimozione ``Remove code of `<ref>` `` per ogni elemento rimosso più in alto.
      - Staleness con SHA-256 del contenuto normalizzato LF in `base`, con le Proposal richieste applicate sotto; i file Task e `proposal.md` non contano; solo il rebase del raffinamento riscrive `base`.
      - Motivi di rifiuto di Approve, tutti e in quest'ordine: agente che finisce, Run in corso, ogni Proposal richiesta (in scrittura, non approvata, rifiutata, scartata, mancante, ciclo), staleness per file, invalidità, scope scoperto / riferimenti inesistenti / label mancante, fix di codice non più applicabili. Consegna rifiutata per 5–7 e gli altri casi elencati, mai per staleness o `requires`.
      - Approvazione in un solo passo, tutto o niente: file sostituiti, Task in To do con le stesse identità e `created` uguale, Task di rimozione, decisioni della resolution sui Drift ancora aperti, skill dei Requisiti rigenerate (`requirements/rules#project`).
      - Label 1–30 caratteri su una riga, obbligatoria con Task; colore FNV-1a modulo 6.
      - Modello per Proposal (sempre attivo): mai nel repository, in `proposal-models/<project hash>.json` dei dati locali, tolto quando nessun Task è in To do o In progress; non rimanda in Draft.

      ## Comandi e Board
      - `trama proposal new|show|list|edit|move` come in `requirements/cli#proposal`: ogni `edit` riporta una Open in Draft; `move <id> open` consegna; `rework: true` e l'approvazione di una `change` stale richiedono l'app.
      - Card come in `requirements/ui#proposal`: badge `stale`, `refining`, `refinement interrupted`, `blocked` (Popover con ogni motivo), modello; righe file e Task; `Approve`, `Reject` con `Rework with feedback…`, controllo modello; motivi disabilitati nell'ordine fissato; card gialla della resolution con una riga per Drift; dialogo dettagli.

      ## Fatto quando
      - Test del core su ogni transizione, scope, copertura, staleness e ordine completo dei motivi.
      - Test dell'auto-approvazione di setup e di una catena di import su Modello vuoto, e del Run all che segue il setup.
      - Un'approvazione fallita a metà non lascia alcun file cambiato.
      - Approve su una `change` solo stale avvia il raffinamento; un raffinamento interrotto mostra `refinement interrupted`.
      - Il modello scelto non compare in nessun file sotto `.trama/`.
  zov4m67w:
    title: Task
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T09:26:14.455Z
    refs:
      - task
      - requirements/data#task
      - requirements/cli#task
      - requirements/rules#task
      - requirements/ui#task
    body: |-
      Weaver gestisce la Machine `task`: un'unità di lavoro per l'agente con titolo, corpo in parole di dominio e i riferimenti che riguarda (un Task tecnico non nomina elementi del Modello, gli altri almeno uno). Nasce con l'approvazione della sua Proposal, dall'agente durante uno scan o un Run, o come fix Task di un Drift. Dipende da "Scheletro del progetto" e "Core dei file del progetto"; i Run sono in "Lavori dell'agente", la regola «concerns the same thing» in "Board".

      ## Stati
      - `To do`: `agent.TAKE_TASK` → `In progress` solo per l'agente di un job attivo e solo se il Task non è bloccato, altrimenti rifiutato nominando il Drift (numero e titolo) o il fix Task; il job lo tiene (`holder`). `developer.DELETE_TASK` → `Deleted` senza traccia, rifiutato se un Drift aperto lo nomina come origine, se è il fix Task di un Drift in `Code to be fixed`, o se il job che l'ha creato lavora ancora.
      - `In progress`: lo tiene un solo job. `agent.FINISH_TASK` → `Done` e `agent.RELEASE_TASK` → `To do` solo dal job che lo tiene; `agent.CLOSE_TASK_UNCHANGED` → `Done` solo per un fix Task, con nota non vuota (`unchanged`). `job.ENDED` → `To do`: il job che lo teneva è finito senza finirlo, qualunque sia il motivo. `drift.CODE_TO_BE_FIXED` → `To do` per il Task origine di quel Drift, che aspetta il fix Task. All'apertura del progetto un `holder` sconosciuto al registro dei job, o `doing` senza `holder`, conta come job finito.
      - `Done`: lascia subito la Board ma resta disponibile (scelta del developer) finché una voce di History lo cita (`history-entry.TASKS_CITED` → `Recorded in History`, vedi "History") oppure finché nel progetto non finiscono altri 20 Task (`FINISHED` → `Forgotten`), salvo che un Drift aperto lo nomini come origine. Il file e `task-images/<id>/` sono cancellati in entrambi i casi.
      - `Deleted`, `Recorded in History`, `Forgotten` sono finali. Eventi emessi: `FINISHED` (finito dall'agente) e `CLOSED_UNCHANGED`.

      ## Regole (`requirements/rules#task`)
      - Bloccato, calcolato a ogni lettura e mai salvato: un Drift aperto lo nomina come origine o riguarda la stessa cosa, oppure è origine di un Drift in `Code to be fixed` il cui fix Task non è Done. Un Task Done non è mai bloccato; il blocco non sposta mai un Task.
      - Eseguibile: To do e non bloccato; il prossimo è il più vecchio.
      - Slot (esecuzione parallela, sempre attiva): con `runnable` e `max` N si scorrono i Task eseguibili dal più vecchio, prendendo ciascuno che non riguarda la stessa cosa di uno già preso o In progress.
      - Gruppi: Task in To do legati transitivamente dalla stessa cosa o dallo stesso percorso di file tra backtick; con `groups` ogni Task preso porta fino ad altri 2 del gruppo, al massimo 3 per sotto-agente.
      - Oblio: a ogni `FINISHED` ogni Task Done non citato conta i Task finiti dopo di lui (per `finished`, pari per identità), su file e copie in History, esclusi i chiusi senza modifiche; a 20 viene cancellato.
      - Run all raggruppa per modello effettivo (modello scelto della Proposal di design, altrimenti modello di esecuzione, confronto esatto dopo trim): un Run per gruppo, avviati insieme, che contano come un solo Run; il limite di sotto-agenti diviso in proporzione, almeno 1 ciascuno, il resto alle parti frazionarie maggiori.

      ## File e comandi
      - `.trama/tasks/<id>-<slug>.md` con `column`, `technical`, `refs`, `proposal`, `drift`, `job`, `holder`, `created`, `finished`, `unchanged` (`requirements/data#task`).
      - `trama task new|show|list|move|delete` come in `requirements/cli#task`: `move` è legato al job e solo chi tiene il Task lo sposta fuori da `doing`; `list` con `runnable`, `max`, `waiting`, `heldBack`, `proposals`, `notProposals`.

      ## Card (`requirements/ui#task`)
      - Pallino di stato (`To do`, `In progress`, `Agent working`), id, titolo, badge `technical`, `blocked` e `waiting <id>` con Popover; badge della label; bordo sinistro nel colore della label; bordo conico rotante mentre un job lo tiene (ciano statico con movimento ridotto).
      - `Run` con i motivi disabilitati in ordine; nei dettagli `Delete` solo in To do, con conferma `Delete <id> for good?` e motivi disabilitati in ordine. Nessun altro controllo: i Task non si spostano a mano.

      ## Fatto quando
      - Test del core su blocco, presa rifiutata con il bloccante, rilascio alla fine del job, slot, gruppi, divisione di Run all e oblio dopo 20 Task.
      - Un Task citato da una voce di History sparisce da `.trama/tasks/` e resta leggibile nella voce.
      - Test Vitest della card: badge, popover, motivi disabilitati di `Run` e `Delete`.
---
# Board e carte: Proposal, Task, Drift, History, richiesta di design

La Board della Feature `work` mostra ciò che aspetta una decisione (Proposal e Drift aperti) e i Task ancora da finire, in due colonne: `Backlog` e `Active`. Il lavoro finito la lascia e si racconta in History.

Le carte seguono ciascuna la propria Machine. La Proposal porta le copie dei file cambiati e i Task, con approvazione, rifiuto, rework, staleness e raffinamento. Il Task passa da To do a In progress a Done, con blocco dai Drift. Il Drift è una differenza fra codice e Modello decisa solo dal developer, con opzioni di correzione e domanda preparata. La History raccoglie le voci scritte dall'agente per macro-feature con i Task finiti. La richiesta di design si scrive in un editor Markdown nell'app e viene consumata alla consegna della Proposal.
