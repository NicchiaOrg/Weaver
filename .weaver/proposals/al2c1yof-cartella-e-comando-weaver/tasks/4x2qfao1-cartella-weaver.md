---
refs:
  - project
  - project-setup
  - workspace.PROJECT_FOLDER_REMOVED
  - workspace.PROJECT_FOLDER_APPEARED
  - requirements/data
  - requirements/ui#project
  - requirements/ui#proposal
  - requirements/quality#General
  - specs#data
---

# Cartella .weaver

La cartella di progetto diventa `.weaver` al posto di `.trama`, ovunque nel codice, nei test e nella UI. Al termine, Weaver e la CLI leggono e scrivono il progetto solo sotto `.weaver/`, con lo stesso layout di prima: `model/`, `requirements/<area>/`, `specs.yaml`, `proposals/`, `tasks/`, `drifts/`, `history/` e le cartelle solo locali `requests/`, `short-numbers/assigned`, `task-images/<task id>/`, ognuna col proprio `.gitignore` contenente `*`. Nient'altro in `.weaver/` è ignorato.

## Cosa deve essere vero

- **`project/Not initialised`**: la cartella non ha `.weaver`, anche quando ha una `.trama` di una versione precedente di Weaver. Ogni azione che richiede un progetto dice di inizializzarlo prima, nessuna fallisce in silenzio. Weaver non cerca mai un progetto nelle cartelle superiori.
- **`developer.INITIALISE_PROJECT`** (→ Empty Model) crea la cartella `.weaver` con un Model vuoto; un fallimento non lascia nulla. `project-setup`: una cartella senza progetto offre di inizializzarla e Weaver crea `.weaver` con un Model vuoto.
- **`.trama` non significa nulla**: non viene mai letta, scritta, spostata o cancellata, e non compare alcun avviso. Una cartella che ha sia `.weaver` sia `.trama` è il progetto in `.weaver`, senza avvisi. Lo sviluppatore sposta a mano un progetto vecchio. Non va scritta nessuna migrazione.
- **Watcher**: solo la comparsa o la scomparsa di `.weaver` produce `workspace.PROJECT_FOLDER_APPEARED` ("A `.weaver` folder appeared in a folder that was not a project") e `workspace.PROJECT_FOLDER_REMOVED` ("The `.weaver` folder of the project disappeared"). Gli eventi su `.trama` sono ignorati. PROJECT_FOLDER_APPEARED porta a Empty Model o Has a Model, secondo che la nuova `.weaver` contenga o no una Feature o un Actor. PROJECT_FOLDER_REMOVED riporta a Not initialised: la finestra torna a offrire l'inizializzazione e i job già al lavoro continuano.
- **UI, Get Started** (`requirements/ui#project`): il testo diventa esattamente `Weaver will create a .weaver folder that describes this project. It holds no secrets and is meant to be committed, like the agent files it installs.` L'avviso dell'Empty Model compare quando la cartella contiene nomi che non iniziano con un punto oltre a `.weaver`. `folder_has_files` salta già i nomi col punto, quindi `.weaver` e `.trama` sono entrambe ignorate: va solo confermato, senza cambiare il codice.
- **UI, Proposal** (`requirements/ui#proposal`): i percorsi nella riga Files sono relativi a `.weaver/`.
- **Requirements e Specs** (`requirements/data`, `requirements/quality#General`, `specs#data`): tutti i percorsi sono `.weaver/...`, per esempio `.weaver/requests/<id>.md`, `.weaver/requests/<id>.images/`, `.weaver/requests/.state.json`. Il codice deve rispettarli. Il costo di costruire l'immagine del progetto resta lineare nel numero di file sotto `.weaver/`.
- I dati dell'app (lock, registro dei job, progetti recenti) contengono solo le radici dei progetti: non c'è nulla da migrare.

## Punti noti del codice (indicazioni, da rileggere)

- `crates/core/src/project/layout.rs:29-63`: `pub const TRAMA_DIR: &str = ".trama"`, `trama_dir()`, `absolute`/`rel_key`. Le cartelle solo locali sono a :40-45 e :101-108. Rinominare con nomi Weaver (es. `WEAVER_DIR`, `weaver_dir`). Rinominare anche `has_trama` in `crates/core/src/project_files/mod.rs:37-65` e `watching_trama` nel watcher.
- Usi della costante: `crates/cli/src/exec.rs:84-94,202` (messaggio NOT_INITIALISED); `crates/cli/src/commands/project.rs:72-88` (rifiuto Exists e output "Created `.weaver/`"); `crates/core/src/project/view/lines.rs:189`; `view/mod.rs:219`; `project_files/sync.rs:25,219`; `project_files/initialise.rs:66-69`; `requests/store.rs:515` (componenti del percorso); `crates/core/src/project/image.rs:122-135`; `src-tauri/src/project_setup.rs:195-212`; `src-tauri/src/windows/watch.rs:145-215`; `src-tauri/src/requests.rs:380-387`.
- `.trama` scritto a mano in testi che non sono test:
  - `crates/core/src/requests/store.rs:50` (errore su `.state.json`);
  - `ui/src/components/project/get-started.tsx:57`, il suo doc comment a :30 e il test `get-started.test.tsx:63`;
  - i commenti in `ui/src/components/project/project-gate.tsx:7` e `ui/src/lib/backend.ts:121`;
  - i binding TS generati `ui/src/bindings/ProposalFiles.ts:4` e `ViewError.ts:8`, da rigenerare a partire da `src-tauri/src/proposal_view.rs:49` e dalla view del core;
  - circa 45 doc comment Rust.
- Nessuna modifica all'albero di import in `crates/core/src/project_files/folder_tree.rs:34,63`: salta già tutti i nomi col punto.
- Test: più di 200 occorrenze di `root.join(".trama")` o `".trama/…"`.
  - Rust: circa 20 file del core, 8 della CLI e 15 di `src-tauri`. Nei test Rust conviene usare la costante o la funzione invece della stringa.
  - Test e fixture UI: `view-fixtures.ts:135-210`, `diagram-view.test.tsx`, `requirements-view.test.tsx:74,78`, `specs-view.test.tsx:77`, `open-in-editor.test.tsx:22-34`.
  - Altro: `tests/cli-contract/contract.test.mjs` (circa 12 percorsi) e il commento in `e2e/lib.mjs:72`.

## Test da aggiungere

- Una cartella con solo `.trama` è Not initialised: la CLI esce con codice 1 e l'errore not-initialised, e l'app mostra Get Started.
- `project new` e Initialise, in una cartella con solo `.trama`, creano `.weaver` e lasciano `.trama` intatta.
- Una cartella con sia `.weaver` sia `.trama` usa `.weaver`, senza avvisi.
- Nel watcher, solo `.weaver` produce PROJECT_FOLDER_APPEARED/REMOVED e gli eventi su `.trama` sono ignorati.

## Fuori scopo (lo coprono altre Task della stessa Proposal)

- Il nome del comando `weaver`. Il rename dei crate in `weaver-core`/`weaver-cli`, il binario `weaver-cli`, il sidecar, la copia `weaver` in `<app local data>/bin/` e la rimozione della vecchia `trama`. Le variabili d'ambiente `WEAVER_*`. I testi di help e usage della CLI, comprese le loro menzioni di `.trama`, e la rigenerazione di `crates/core/assets/skills/trama-design/COMMANDS.md`. Il testo fisso del rifiuto dei form di omp e i messaggi di log del watcher: questa Task cambia solo cosa il watcher osserva. README, descrizioni Cargo, «Trama»→«Weaver» nei doc comment e i workflow di CI.
- Le stringhe delle istruzioni dei job, il preambolo di General, i testi delle skill e degli agenti installati (tranne COMMANDS.md) e i testi delle regole dei Requirement.
- La cartella `.trama` di questo repository e le copie installate in `.agents/skills` e `.omp` non vanno spostate né modificate: lo sviluppatore le sposta a mano in seguito con `git mv .trama .weaver`.
