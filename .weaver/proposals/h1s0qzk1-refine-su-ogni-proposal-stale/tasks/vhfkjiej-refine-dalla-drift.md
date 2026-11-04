---
technical: false
refs:
  - drift
  - developer.REFINE_DRIFT_PROPOSAL
  - requirements/ui#drift
---
# Refine dalla Drift

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
