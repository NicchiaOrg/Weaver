---
technical: false
refs:
  - proposal
  - requirements/rules#proposal
  - requirements/cli#proposal
  - requirements/ui#proposal
---
# Refine su ogni Proposal stale

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
