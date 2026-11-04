---
created: 2026-11-04T08:28:18.780Z
updated: 2026-11-04T08:28:18.780Z
summary: Una Proposal stale di qualsiasi tipo si rielabora con Refine, anche dalla Drift a cui è allegata, con una skill di refinement per ogni kind.
tasks:
- 3qhmz2cs
- vhfkjiej
- x8qn3p0h
cleared:
  3qhmz2cs:
    title: Refinement con skill per kind
    refs:
    - job/Not started
    - requirements/rules#job
    technical: false
    proposal: h1s0qzk1
    label: Refine ovunque
    finished: 2026-11-04T08:03:36.222Z
    body: |-
      La refinement di una Proposal stale non usa più sempre `weaver-design`: la skill segue il kind della Proposal come per la rework, e ogni skill ha una sezione per rifinire una Proposal stale del proprio kind.

      ## Cosa deve essere vero

      ### Istruzione (`crates/core/src/job_instruction.rs`)
      - `Job::Refinement` porta il kind: `Refinement { proposal, kind }`, con lo stesso tipo della rework (rinominare/generalizzare `ReworkKind` in un tipo condiviso, es. `ProposalKind`, migrando ogni chiamante; nessun alias).
      - La scelta della skill in `first_sentence` è una funzione condivisa tra `Job::Rework` e `Job::Refinement`:
        - change → `weaver-design`;
        - setup → `weaver-setup` mentre il Model è vuoto, altrimenti `weaver-design` (il kind resta `setup`);
        - import → `weaver-import`;
        - resolution → `weaver-resolve-drifts`, solo sui Drift ancora aperti; lista vuota → `InstructionError::NoDrifts` (aggiornare il messaggio dell'errore, oggi parla solo di rework).
      - Frase: `Use the <skill> skill to refine the stale Proposal <id> on the current Model and Requirements.`; per `weaver-resolve-drifts` la frase finisce con ``, on the Drifts `<d1>`, `<d2>`.`` (es. `... on the current Model and Requirements, on the Drifts `abcdef08`, `abcdef09`.`).
      - Test: in `cases()` sostituire il caso `Job::Refinement { proposal: id(10) }` con un caso per ogni kind (change, setup vuoto, setup non vuoto, import, resolution con due Drift); aggiungere che una refinement di resolution senza Drift aperti dà `NoDrifts`. In `the_skills_named_are_installed_and_describe_the_sentences_that_name_their_sections` aggiungere gli assert sulle nuove sezioni di `weaver-setup`, `weaver-import` e `weaver-resolve-drifts` (titoli sotto).

      ### App (`src-tauri/src/jobs/starts.rs`)
      - Il calcolo di kind e Drift aperti oggi dentro `start_rework` diventa un helper condiviso, usato da `start_rework` e da `start_refinement`.
      - `start_refinement` costruisce `Job::Refinement { proposal, kind }` come `start_rework`. Per una resolution senza Drift ancora aperti non parte alcun job.
      - Interfaccia con il percorso di approvazione (`src-tauri/src/proposals.rs` `approve`/`after_approval`, `src-tauri/src/jobs/bridges.rs`, `plan_approve` in `crates/core/src/project/proposal/transitions.rs`, curati da un altro Task): una refinement di una resolution nessuno dei cui Drift è ancora aperto deve essere rifiutata *prima* di scrivere qualunque cosa (`None of its Drifts is still open`), così nessuna Proposal resta `Refining` senza job. Se `start_refinement` trova comunque zero Drift aperti, restituisce un `Refusal` (non un job) e non scrive nulla.
      - Una refinement avviata da un Drift (`developer.REFINE_DRIFT_PROPOSAL`, sulla Proposal attaccata a quel Drift) usa la stessa `start_refinement` e quindi la stessa istruzione; aggiornare il commento di `start_refinement` (oggi cita solo `developer.APPROVE_PROPOSAL`).

      ### Skill (`crates/core/assets/skills/`)
      - `weaver-design/SKILL.md`, `## Refining a stale Proposal`:
        - vale per ogni Proposal che riceve: change, e setup su Model non vuoto;
        - non più sempre "Weaver approves it at the end": una Proposal attaccata a un Drift torna sempre Open all'hand-over e lo sviluppatore la accetta di nuovo dal Drift; una bloccata da una Proposal richiesta non ancora approvata torna Open; altrimenti Weaver la approva (o attende la fine di una Run). L'agente non approva mai;
        - il rebase (`{"rebase":true}`) ora toglie le requires rotte (Proposal richieste rifiutate, scartate, mancanti o in un ciclo; in un ciclo solo la voce propria) e le elenca nell'output; l'agente adatta il cambiamento senza di esse e, quando serviva qualcosa di una Proposal tolta, chiede allo sviluppatore — nello stesso unico round di `weaver question new`;
        - i rifiuti dell'hand-over (Model non valido, scope non coperto, refs dei Task mancanti, label mancante, fix del codice non applicabili) vanno corretti dall'agente.
      - Nuove sezioni, con le stesse regole della refinement di design (niente lettura del codice, nessun annuncio, decisioni nel body già prese, al massimo un round e solo su una decisione contraddetta dal Model attuale, `Remove`/`Keep` quando l'intero cambiamento è già nel Model, rebase con requires tolte, hand-over e rifiuti come sopra, Draft `refining` lasciata a metà che riprende):
        - `weaver-setup/SKILL.md`: `## Refining a stale setup Proposal` (Model vuoto);
        - `weaver-import/SKILL.md`: `## Refining a stale import Proposal`;
        - `weaver-resolve-drifts/SKILL.md`: `## Refining a stale resolution`: solo sui Drift aperti nominati nell'istruzione; le decisioni già prese per quei Drift restano; le righe dei Drift chiusi le ha già tolte il rebase (che le elenca); i file del Model delle decisioni `proposal` si riscrivono sul Model attuale. Nella `## Steps` la frase "one of three forms" diventa quattro forme, con la quarta: rifinire una Proposal stale sui Drift nominati (seguire "Refining a stale resolution").
      - Le sezioni dicono che si applicano solo quando l'istruzione dice di rifinire la Proposal stale `<id>`.

      ## Fatto quando
      `cargo test` di `job_instruction` e dei job dell'app passa con i nuovi casi; le skill installate contengono le sezioni nuove.
  vhfkjiej:
    title: Refine dalla Drift
    refs:
    - drift
    - developer.REFINE_DRIFT_PROPOSAL
    - requirements/ui#drift
    technical: false
    proposal: h1s0qzk1
    label: Refine ovunque
    finished: 2026-11-04T08:15:20.976Z
    body: |-
      ## Cosa deve essere vero alla fine

      ### Nuovo Event `developer.REFINE_DRIFT_PROPOSAL`
      Il developer chiede all'agente di adattare la Proposal stale attaccata a una Drift Open al Model e ai Requirements correnti, o di continuarne una refinement lasciata a metà, **senza decidere la Drift**. Nella Machine `drift` (stato Open): `developer.REFINE_DRIFT_PROPOSAL` → Open, solo quando la Proposal attaccata è stale (e Open), oppure è un Draft lasciato a metà da una refinement (`data::is_interrupted`). Un job di refinement adatta quella Proposal; la Drift resta Open e attende di nuovo la decisione del developer una volta che la Proposal è consegnata. Lato Proposal (Machine `proposal`, gestito da un'altra Task): la Proposal stale attaccata passa Open → Refining; il Draft interrotto attaccato passa Draft → Refining e un nuovo job di refinement lo continua; alla consegna una Proposal raffinata attaccata a una Drift torna sempre Open (mai Approved, mai in attesa di una Run), e il developer la accetta di nuovo dalla Drift. Stesse condizioni del `Refine` della Proposal: ad esempio una resolution stale nessuna delle cui Drift è ancora aperta è rifiutata con `None of its Drifts is still open` prima di scrivere qualsiasi cosa (nulla resta Refining senza job); una resolution senza file non è mai stale.

      ### Dettagli della Drift (`requirements/ui#drift`)
      - Con una Proposal attaccata **stale e Open**, `Refine` sostituisce `Accept Proposal`, tooltip `Refine on the current Model`, disabilitato **solo** da `The agent is still finishing` (bottone giallo disabilitato con quell'unico motivo mentre l'agente che l'ha consegnata sta ancora finendo); nessun altro motivo di blocco lo disabilita, abilitato anche durante una Run.
      - Con una Proposal attaccata **Draft lasciata a metà da una refinement**: `Refine` (tooltip `Continue refining`) e `Discard` (destructive, confermato come sulla card della Proposal) sostituiscono `Accept Proposal`. `Discard` usa il discard esistente della Proposal attaccata (`plan_discard` con `Decider::Drift`).
      - Il click su `Refine` scrive Refining sulla Proposal attaccata (via `refining_text`, flag `refining: true`) e avvia un job di refinement riusando `start_refinement` del bridge dei job; la Drift non cambia.
      - Mentre la Proposal attaccata è **Refining**: `Accept Proposal`, `Fix the code`, `Resolve` e `Dismiss Drift` sono disabilitati con `Being refined`. Ordine dei motivi: `Resolve`: `The agent is still finishing`, `A run is in progress`, `Being refined`, `This Drift is being resolved`, `Another resolution is being written`, `Proposal <id> resolving Drifts awaits a decision`. `Dismiss Drift`: `The agent is still finishing`, `Being refined`, `This Drift is in a resolution`. `The agent is still finishing` è sempre primo su ogni controllo di decisione.
      - Le decisioni del core rifiutano lo stesso: `developer.RESOLVE_DRIFT`, `developer.FIX_DRIFT_CODE`, `developer.ACCEPT_DRIFT_PROPOSAL` e `developer.DISMISS_DRIFT` sono rifiutati mentre la Proposal attaccata è Refining.
      - I dettagli mostrano la Proposal attaccata con titolo, stato (`Draft`, `Refinement interrupted`, `Open` o `Refining`), badge `stale` quando è stale, body renderizzato e file.

      ### Resolution in Refining o interrotta
      - Una resolution in Refining conta come una in scrittura: badge `resolving` (tooltip `An agent is resolving this Drift`) sulle Drift che elenca, `Another resolution is being written` su `Resolve` delle altre Drift.
      - Una resolution con refinement interrotta (Draft con flag refining) conta come una in attesa di decisione: `Proposal <id> resolving Drifts awaits a decision` (non `Another resolution is being written`).
      - In entrambi i casi le sue Drift sono `in a resolution`: `Dismiss Drift` disabilitato con `This Drift is in a resolution`, e `weaver drift list {unprepared}` le esclude.

      ## Dove nel codice
      - `crates/core/src/project/drift/decide.rs`: `enum Decision` — nuova variante (es. `RefineProposal`, verb `refine the Proposal of`) che pianifica tramite `proposal::transitions` (la stessa pianificazione Refine/Continue di `plan_approve` con `Decider::Drift`, che la Task lato Proposal smette di rifiutare sugli attaccati) e non scrive la Drift; in alternativa un comando Tauri dedicato. `plan_accept`, `plan_dismiss` e il piano di fix rifiutano con la Proposal attaccata Refining; `plan_accept` continua a rifiutare tutto ciò che non è `Approval::Approve`.
      - `crates/core/src/project/drift/reasons.rs`: `resolutions()` includere `Status::Refining`; distinguere "in scrittura" (Draft non interrotta, Refining) da "in attesa di decisione" (Open, Draft interrotta) in `resolve_reasons`, `is_resolving`; aggiungere `Being refined` in `resolve_reasons` e `dismiss_reasons` nell'ordine sopra (costante condivisa con `REASON_BEING_REFINED` di `src-tauri/src/proposal_view.rs`, preferibilmente spostata nel core).
      - `crates/core/src/project/board.rs`: `drifts_resolving` conta anche resolution Refining (non le Draft interrotte); verificare `compute` (attaccate nascoste) e `proposal_state`.
      - `crates/core/src/project/drift/prepared.rs`: `live_attached` — valutare se includere Refining dove serve per i controlli.
      - `src-tauri/src/drift_view.rs`: `attached()` smette di nascondere Refining; `AttachedProposal.state` diventa `Draft`/`Refinement interrupted`/`Open`/`Refining`; nuovi campi per stale, per il controllo `Refine` (tooltip, motivi) e `Discard`; `accept_reasons` e i motivi di `Fix the code` contengono `Being refined` quando Refining. Rivedere `REASON_PROPOSAL_IS_DRAFT`.
      - `src-tauri/src/drifts.rs`: nuovo comando (es. `refine_drift_proposal`) accanto a `accept_drift_proposal`/`decide_drift`, che dopo la scrittura chiama `start_refinement` (`src-tauri/src/jobs/bridges.rs` `JobsProposals::start_refinement`, `src-tauri/src/jobs/starts.rs` `start_refinement`), come fa `after_approval` in `src-tauri/src/proposals.rs`; registrarlo nell'handler Tauri e nei binding generati.
      - UI: `ui/src/components/board/drift-card.tsx` `DecisionForm` (sostituzione di `Accept Proposal` con `Refine`/`Discard`, `Fix the code` disabilitato), sezione della Proposal attaccata (stato e badge `stale`), `DriftFooter`; `ui/src/components/board/board-state.tsx` nuova azione accanto a `acceptDriftProposal`, `Discard` con la stessa conferma della card della Proposal.

      ## Test da riscrivere
      - `crates/core/src/project/drift/tests.rs`: `accepting_needs_a_proposal_that_could_be_approved` (l.1305) e `a_stale_attached_proposal_cannot_be_carried_and_a_missing_one_neither` (l.1843) fissano la vecchia regola (Proposal attaccata stale non raffinabile): riscriverli per la nuova; aggiungere test per Refine/Continue dalla Drift, i rifiuti con `Being refined` e le resolution Refining/interrotte.
      - `src-tauri/src/drift_view.rs` e `src-tauri/src/drifts.rs` (test dello stato `Draft`/`Open` e di `resolutions`).
      - `ui/src/components/board/drift-card.test.tsx` (l.199, l.221): `Accept Proposal` con Proposal stale/interrotta/Refining.
  x8qn3p0h:
    title: Refine su ogni Proposal stale
    refs:
    - proposal
    - requirements/rules#proposal
    - requirements/cli#proposal
    - requirements/ui#proposal
    technical: false
    proposal: h1s0qzk1
    label: Refine ovunque
    finished: 2026-11-04T08:12:37.749Z
    body: |-
      Quando il lavoro è finito, la refinement non è più limitata alle `change` non attaccate e bloccate solo dalla staleness: vale per ogni Proposal Open stale di qualsiasi kind (change, setup, import, resolution), attaccata a un Drift o no, qualunque altra cosa la blocchi, anche durante un Run.

      ## Regola di refinement (core)
      - `crates/core/src/project/proposal/analysis.rs` `Analysis::refinable`: vero quando la Proposal è stale (`!self.stale.is_empty()`), qualunque sia `kind`, `attached` e gli altri motivi 2–7. Falso solo per il motivo 1 (`The agent is still finishing`) e, per una resolution, quando nessuno dei Drift elencati è ancora open. Serve un modo per esporre il motivo del rifiuto (nuovo testo `None of its Drifts is still open`), così che view e `plan_approve` lo usino. Una resolution senza file non ha `base`, non è mai stale e mai refinabile (verificare che resti così).
      - `crates/core/src/project/proposal/transitions.rs` `plan_approve`:
        - Open stale → `Approval::Refine` per ogni kind; il rifiuto `it is attached to a Drift: it is decided through its Drift` per `Decider::Developer` resta per l'approvazione vera (Approved) e per la Refine dal Board, ma il Drift (Task separata, Event `developer.REFINE_DRIFT_PROPOSAL`) deve poter ottenere `Approval::Refine`/`Approval::Continue` sulla Proposal attaccata con le stesse condizioni: aggiungere il percorso/decider necessario (p.es. un `Decider` o una funzione di plan dedicata riusata dal lato Drift). Il lato Drift (`decide.rs`, `drift_view.rs`, `drifts.rs`, `drift-card.tsx`) è di un'altra Task: qui va solo fornita l'interfaccia.
        - Draft con `refining: true` (refinement interrotta) → `Approval::Continue` con le stesse condizioni: rifiutata per `The agent is still finishing` e, per una resolution senza Drift ancora open, `None of its Drifts is still open`.
        - I rifiuti avvengono prima di scrivere qualsiasi cosa: nessuna Proposal resta Refining senza job. Anche `start_refinement` (`src-tauri/src/jobs/starts.rs`) deve essere rifiutata prima di scrivere quando una resolution non ha più Drift open (l'istruzione del job, che per la resolution nomina i Drift ancora open, e la scelta della skill per kind sono di un'altra Task; qui conta solo che la verifica avvenga prima di `refining_text`/scrittura in `src-tauri/src/proposals.rs` `approve`).
      - `plan_submit` (hand-over da Refining, `refined`):
        - attaccata a un Drift → sempre Open (rimuovendo `refining`), mai Approved né WaitsForRun; lo sviluppatore la riaccetta dal Drift;
        - altrimenti Approved se nulla blocca, `WaitsForRun` se blocca solo il motivo 2, Open con i motivi quando ancora blocca una Proposal richiesta (still being written / not approved yet).
        - I rifiuti di hand-over (Model invalido, scope non coperto, ref di Task mancanti, label mancante, code fix non applicabili; motivi 5–7 e gli altri `handover_only`) restano anche per la refinement: li corregge l'agente. Mai per staleness o Proposal richieste.
        - Un setup raffinato su un Model non vuoto mantiene kind `setup`: la sua approvazione avvia ancora Run all (`plan.start_run_all`, `after_approval`).
      - `plan_refinement_ended` (`job.ENDED` di refinement senza hand-over): Refining → Draft con `refining: true`, per ogni kind e anche se attaccata; resta a metà finché lo sviluppatore la raffina di nuovo (dal Drift se attaccata) o la scarta. Aggiornare il commento.

      ## Rebase (`proposal edit {rebase:true}`)
      `crates/core/src/project/proposal/create.rs` `plan_edit` (blocco `if request.rebase`): togliere i rifiuti `only a change is refined` e `a Proposal attached to a Drift is not refined`. In ordine:
      1. togliere da `requires` ogni voce rifiutata (eccetto una rifiutata con Rework il cui rework non è ancora stato consegnato: conta come still being written), scartata o mancante, e ogni voce di questa Proposal che forma un ciclo (solo la propria; le altre Proposal del ciclo non si toccano);
      2. per una resolution, togliere le righe `drifts` il cui Drift non è più open (le decisioni sui Drift ancora open restano);
      3. poi ricalcolare `base` (`analysis::current_base`) con le Proposal richieste rimaste applicate sotto;
      4. impostare `refining: true`.
      L'output del comando (`crates/cli/src/commands/proposal.rs`, esito di edit) elenca ogni requisito e riga tolti. Se di una Proposal tolta serviva qualcosa, chiede l'agente (istruzione/skill: altra Task).

      ## CLI
      - `crates/cli/src/commands/proposal.rs`: help di `edit` (`rebase: true`, any kind, attached to a Drift or not, drops broken `requires` and, in a resolution, the rows of Drifts no longer open, then records `base` again and sets `refining`; it prints what it dropped) e di `move` (`rework: true`, and approving a stale Proposal, need the running app) come in `requirements/cli#proposal`; messaggio needs-the-app di approve valido per ogni kind.
      - Rigenerare `crates/core/assets/skills/weaver-design/COMMANDS.md` dall'help.

      ## App
      - `src-tauri/src/proposal_view.rs` `controls()`:
        - Open stale (qualsiasi kind, attaccata o no): `refines: true`, motivi disabilitanti solo `The agent is still finishing` e, per una resolution senza Drift ancora open, `None of its Drifts is still open`; gli altri motivi restano nel badge `blocked`.
        - Tooltip: `TOOLTIP_REFINE` `Refine on the current Model, then approve`, oppure un nuovo `Refine on the current Model` quando la Proposal è attaccata a un Drift o una Proposal richiesta è still being written o not approved yet; analogamente `TOOLTIP_CONTINUE` `Continue refining, then approve` oppure nuovo `Continue refining` per la Draft interrotta (disabilitata come sulla Open).
        - Gli altri disabled reasons di `Approve` restano in ordine: `The agent is still finishing`, `A run is in progress`, `Being refined`, poi ogni motivo bloccante; `Reject`/`Rework with feedback…` disabilitati solo da finishing e `Being refined`.
      - `src-tauri/src/proposals.rs` `approve`/`after_approval`/`approve_from_cli`: Refine/Continue per ogni kind avviano `start_refinement`; testi di `approve_from_cli` non specifici della `change`.
      - UI: `ui/src/components/board/proposal-card.tsx` `ProposalControls` mostra già `Refine` su `view.approve.refines` con tooltip e motivi dalla view; nessun cambio previsto salvo che serva per i nuovi tooltip/motivi.

      ## Test da riscrivere
      - `crates/core/src/project/proposal/tests.rs`: `a_change_that_only_staleness_blocks_is_refined_also_during_a_run`, `staleness_with_anything_else_is_not_refined_and_a_drift_attached_one_never_is` (diventa: stale con altri motivi e attaccata → refinable; rifiuti solo finishing / nessun Drift open), `a_refined_proposal_handed_over_is_approved_waits_for_a_run_or_goes_back_to_open` (aggiungere attaccata → Open, setup mantiene kind), `drifts_belong_to_a_resolution_and_rebase_to_a_change_not_attached_to_a_drift` (rebase su ogni kind/attaccata, requires e righe tolti ed elencati, ciclo solo propria voce, Rework non consegnato resta), `approve_is_refused_with_every_reason_in_order_and_writes_nothing`.
      - `src-tauri/src/proposal_view.rs` `staleness_alone_enables_approve_with_its_tooltip_also_during_a_run` (più casi: altri motivi, attaccata, tooltip brevi, `None of its Drifts is still open`).
      - `src-tauri/src/proposals.rs` `approving_a_stale_change_writes_refining_and_starts_the_refinement` (ogni kind; resolution senza Drift open rifiutata senza scrivere).
---
# Refine di ogni Proposal

Il job di refinement usa la skill del kind della Proposal (design, setup, import, resolve). Ogni Proposal stale mostra Refine al posto di Approve, la rebase elenca ciò che scarta, e la Drift offre Refine e Discard per la Proposal allegata, con la ripresa di un refinement interrotto.
