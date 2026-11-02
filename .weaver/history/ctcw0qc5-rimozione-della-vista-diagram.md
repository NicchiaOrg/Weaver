---
created: 2026-11-02T14:59:02.868Z
updated: 2026-11-02T14:59:02.868Z
summary: L'app non ha più la vista Diagram, né nell'interfaccia né nel core, e le skill installate per l'agente misurano il dettaglio con State, Transition ed Event del Modello senza nominare il Diagram.
tasks:
- x9w2rkq7
- lzyz7p6g
cleared:
  lzyz7p6g:
    title: Skill senza Diagram
    refs:
    - requirements/rules#project
    technical: true
    proposal: 0119njgm
    label: Diagram rimosso
    finished: 2026-11-02T14:45:40.855Z
    body: |-
      L'app non ha più una vista Diagram, quindi i testi che Weaver installa per l'agente non devono più nominarla. Le skill e le definizioni dei sub-agent installate misurano il dettaglio di un confronto con gli State, le Transition e gli Event del Modello; non citano mai un Diagram. Il significato delle regole non cambia: cambia solo come sono scritte.

      ## Cosa deve valere alla fine

      - In `crates/core/assets/skills/` nessun testo installato contiene «Diagram». Le frasi da riscrivere oggi sono:
        - `trama-compare/SKILL.md:25` — «something extra that would change the Diagram» → qualcosa in più che aggiungerebbe o cambierebbe uno State, una Transition o un Event del Modello.
        - `trama-compare/SKILL.md:28` e `trama-scan/SKILL.md:64` — «Anything below Diagram level…» → sotto il livello di State, Transition ed Event del Modello.
        - `trama-compare/SKILL.md:31` e `trama-scan/SKILL.md:83` — «would drawing it change the Diagram?» → aggiungerebbe o cambierebbe uno State, una Transition o un Event del Modello?
        - `trama-scan/SKILL.md:22` — «Compare behaviour at the level of the **Diagram**: States, Transitions, Events…» → al livello di State, Transition ed Event del Modello, chi reagisce a cosa.
        - `trama-scan/SKILL.md:61` — «…a State or Transition in the Diagram» → nel Modello.
        - `trama-implement/SKILL.md:99` — «Small choices that stay below Diagram level…» → sotto il livello di State, Transition ed Event del Modello.
      - Controllare anche le definizioni dei sub-agent e le istruzioni dell'advisor installate (`.omp/agents/trama-analysis.md`, `.omp/agents/trama-implementer.md`, `.omp/WATCHDOG.yml`, sorgenti in `crates/core/assets/`): nessuna cita il Diagram.
      - I testi restano in inglese, con fine riga LF, senza prima persona; ogni definizione di sub-agent resta sotto i 6 KB. Le copie installate nei progetti si aggiornano da sole al prossimo sync, perché il loro contenuto cambia.
      - Eventuali test che confrontano il contenuto delle skill installate vanno aggiornati.
  x9w2rkq7:
    title: Rimuovere la vista Diagram
    refs:
    - app
    - requirements/ui#app
    - requirements/ui#General
    - requirements/ui#proposal
    - requirements/ui#task
    - requirements/quality#General
    - specs#frameworks
    technical: false
    proposal: 0119njgm
    label: Diagram rimosso
    finished: 2026-11-02T14:56:50.668Z
    body: |-
      La vista Diagram non esiste più nell'app: la finestra di un progetto mostra la Board, i Requirements, le Specs, la History, i Jobs, General e le Settings. Il codice della vista viene **rimosso**, non nascosto. Resta intatta tutta la logica che serve all'agente: il Modello, `weaver model show` e la vista dei riferimenti dei Task (`crates/core/src/project/model_view.rs`, usato da `crates/cli/src/commands/model.rs:78`, `crates/cli/src/commands/task.rs:295` e dall'impronta dei Drift in `crates/core/src/project/drift/fingerprint.rs:22`) non dipendono dal Diagram e non cambiano. Le carte Proposal e Task stanno in questo stesso Task perché `View on diagram` vive in un componente condiviso dalle due carte.

      ## Cosa deve valere alla fine

      - **Barra laterale:** voci in quest'ordine: Board, Requirements, Specs, History, Jobs, General, Settings (icone `SquareKanban`, `ListChecks`, `Package`, `History`, `Bot`, `MessageSquare`, `Settings`). Cambiare voce conserva focus, overlay e scroll di ogni vista. Nessun percorso (pulsante, chip, evento `show-requested`, notifica) apre più una vista Diagram.
      - **Carte:** le carte Proposal e Task e i loro dialoghi di dettaglio non hanno più `View on diagram`; resta `View requirements`, visibile solo quando il suo focus ha almeno un passo. Nella tabella delle icone resta `View requirements` = `ListChecks`.
      - **Chip dei riferimenti** (dettagli di Task e Drift): un chip apre la prima vista fra Requirements e Specs in cui il suo focus ha almeno un passo. Il focus di un elemento del Modello sui Requirements contiene le sezioni intitolate con quell'elemento o con un elemento che lo contiene, fino alla sua Feature. Un chip senza passi in nessuna delle due viste è testo semplice, non un controllo.
      - **Vista Requirements:** il titolo di ogni sezione mostra solo il titolo e `Open in editor` alla riga del titolo; il chip del riferimento che apriva l'elemento sul Diagram è tolto.
      - **Overlay:** il selettore unico per finestra serve Requirements e Specs (`Current Model`, poi ogni Proposal aperta non legata a un Drift; mai Draft, in refinement o allegate; se la Proposal smette di essere aperta si torna a `Current Model` con un avviso). I colori dei cambiamenti restano `green`/`amber`/`red` (600 chiaro, 400 scuro), sempre con l'etichetta `added`/`changed`/`removed` e il nome accessibile.
      - **Focus:** solo Requirements e Specs hanno un focus; il nome del passo è sezione, Area o campo. Esc cancella il focus (non c'è più una selezione del Diagram da cancellare prima); `Clear` conserva l'overlay.
      - **Errori:** l'`Alert` rosso `The Model has <n> errors` compare in cima a Requirements e Specs.
      - **Movimento ridotto e tastiera:** non restano regole né codice per animazioni di layout del Diagram, salti del Diagram o frecce nella selezione del Diagram.
      - **Nessuna informazione del pannello dei dettagli del Diagram** (descrizione, Transition o Event, Drift e Task di un elemento) va spostata altrove: sparisce con la vista.
      - **Specs:** cytoscape ed elkjs non sono più dipendenze (`ui/package.json:35-36`, worker di elkjs incluso).
      - **Smoke test end-to-end:** l'app parte, compare la finestra di benvenuto, il progetto di prova si apre, si disegnano la Board e i Requirements, una carta si apre, la finestra si chiude e l'app esce senza errori. La fase che disegnava il Diagram (oggi si salta da sola se manca la voce) è sostituita da quella dei Requirements. Resta il controllo già richiesto dai Requirements: con la finestra del progetto di larghezza e altezza dispari, il Dialog dei dettagli della carta aperta ha il bordo sinistro e quello superiore su pixel CSS interi.

      ## Cosa togliere

      Interfaccia (`ui/src`):
      - Barra laterale e `ViewId` — `ui/src/views.ts:3,15`; montaggio della vista in `ui/src/components/project-content.tsx:17,38-42,81-89`.
      - Vista e componenti — `ui/src/components/views/diagram-view.tsx`, `diagram-canvas.tsx`, `diagram-details.tsx`, `diagram-graph.ts`, `diagram-layout.ts`, `diagram-style.ts`, `diagram-model.ts`, `elk-client.ts`, con i loro test (`diagram-view.test.tsx`, `diagram-model.test.ts`, `diagram-style.test.ts`).
      - Stato condiviso — in `ui/src/components/views/views-state.tsx:41-63,99-116,226-236,250-264,334-343` lo stato di navigazione del Diagram (`nav`, `select`, `openLevel`, `goCrumb`, `toggleExpanded`, `viewports`, `settleNav`, `FIRST_NAV`), la chiave `diagram` di `NO_FOCUS` e il ramo Diagram di `goStep`.
      - Modello del focus — `ui/src/components/views/focus-model.ts:5,59-84,141-157`: `diagram` esce da `ViewName`, via `diagramPlan` e il Diagram da `guessedViews`/`stepCount`.
      - Esc — ramo che cancella la selezione del Diagram in `ui/src/components/views/view-frame.tsx:19-37`.
      - Pulsanti — voce `diagram` di `FOCUS_BUTTONS`/`FocusButton` in `ui/src/components/board/task-card.tsx:171-203,262-268`; `ui/src/components/board/proposal-card.tsx:409`; contratto `focusSteps`/`viewOn` in `ui/src/components/board/board-ui.tsx:36-44`.
      - Chip — `RefChip` in `ui/src/components/board/task-card.tsx:206-224` (ordine requirements → specs), usato in `task-card.tsx:247-255` e `ui/src/components/board/drift-card.tsx:164-170`; `ReferenceChip` in `ui/src/components/views/requirements-view.tsx:15-29,44` da togliere.
      - Test da aggiornare — `focus-model.test.ts:18-82`, `views-state.test.tsx:31-94`, `sidebar.test.tsx:17,30,84`, `proposal-card.test.tsx:334-343,395`, `task-card.test.tsx:218-252`, `requirements-view.test.tsx:55-67`, fixture `view-fixtures.ts:151,214-225`.

      Backend:
      - Calcolo del Diagram — `crates/core/src/project/view/diagram.rs` (livelli, `Detail`, `anchors_of` a :1077), `merge.rs` (usato solo dal Diagram), `Concerning` in `crates/core/src/project/view/mod.rs:380-436`, `ModelView` (`mod.rs:147`) e la chiamata a `diagram::build` in `mod.rs:238`; in `RefTargets` (`mod.rs:118-142`) il campo `diagram` e `TargetView::Diagram`, lasciando `relevant`, `requirements` e `specs`. Restano `eligible_overlays`, `view_errors`, `docs::requirements`/`docs::specs` e `project::concerns`, che servono altrove. Test da adattare: `crates/core/src/project/view/tests.rs:70,306-344,597-601`, `src-tauri/src/project_view.rs:224-247,284-293`.
      - IPC — `view_snapshot` (`src-tauri/src/project_view.rs:160`) resta per Requirements e Specs ma non porta più i dati del Diagram; via `ShowView::Diagram` (`src-tauri/src/windows/manager.rs:63`, `ui/src/bindings/ShowView.ts:6`) e i commenti che nominano il Diagram (`src-tauri/src/project_view.rs:1-4,157-158`, `src-tauri/src/lib.rs:23`, `crates/core/src/project/mod.rs:37`, `crates/core/src/project/view/mod.rs:1-16`, `src-tauri/src/proposal_view.rs:94`). Rigenerare i binding TypeScript.
      - e2e — `e2e/specs/smoke.e2e.mjs:2-12,127-152`: la fase del Diagram diventa quella dei Requirements.
---
# Rimozione della vista Diagram

La vista Diagram e il codice che la alimentava sono stati rimossi dall'interfaccia e dal core, con le dipendenze usate solo da essa. Le skill e le definizioni dei sub-agent installate non citano più il Diagram: il livello di dettaglio di un confronto è dato dagli State, dalle Transition e dagli Event del Modello, e un test verifica che nessun testo installato lo nomini.
