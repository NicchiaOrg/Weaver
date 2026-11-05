---
technical: false
refs:
  - job/Not started
  - requirements/rules#job
---

# Refinement con skill per kind

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
