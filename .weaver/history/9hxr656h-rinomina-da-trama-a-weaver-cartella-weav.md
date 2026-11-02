---
created: 2026-11-02T14:15:43.492Z
updated: 2026-11-02T14:15:43.492Z
summary: Il progetto vive in .weaver al posto di .trama e l'agente raggiunge Weaver con il comando weaver. Skill, sotto-agenti, istruzioni dei job, preambolo di General e regole dei Requirement usano i nuovi nomi. La cartella .trama è ignorata.
tasks:
- 4x2qfao1
- 9vrusxd6
- gv29qggl
cleared:
  4x2qfao1:
    title: Cartella .weaver
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
    technical: false
    proposal: al2c1yof
    label: .weaver e weaver
    finished: 2026-11-02T14:04:57.010Z
    body: |-
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
  9vrusxd6:
    title: Comando weaver
    refs:
    - agent
    - requirements/cli
    - requirements/tech
    - requirements/release
    - specs#languages
    - specs#frameworks
    technical: false
    proposal: al2c1yof
    label: .weaver e weaver
    finished: 2026-11-02T14:11:55.068Z
    body: |-
      Il comando che l'agente usa per raggiungere Weaver si chiama `weaver` e non più `trama`. Le skill installate (`trama-setup`, `trama-design`, `trama-import`, `trama-scan`, `trama-compare`, `trama-implement`, `trama-resolve-drifts`, le `trama-req-*` generate) e i sub-agent di omp (`trama-analysis`, `trama-implementer`) mantengono il nome.

      L'Actor `agent` raggiunge Weaver solo attraverso il comando `weaver`, che conosce il job a cui appartiene; ciò che riporta lì sono i suoi Event. Solo un agente di un job avviato da Weaver può tenere un Task, annunciare una card o porre un giro di domande; un comando eseguito fuori da un job del genere viene rifiutato per queste azioni.

      ## Cosa deve essere vero alla fine

      - **Crate e binari.** `crates/core` è la libreria `weaver-core` (prima `trama-core`); `crates/cli` è la crate `weaver-cli` (prima `trama-cli`) e produce il binario `weaver-cli`, mai `weaver`, che è il nome dell'eseguibile dell'app (crate `src-tauri`, binario `weaver`): due binari `weaver` nello stesso workspace Cargo o nella stessa cartella del pacchetto non possono convivere. Tutti gli `use trama_core::…`, i `Cargo.toml` del workspace, `Cargo.lock` e i riferimenti alle crate passano ai nuovi nomi.
      - **Nome mostrato.** Help, usage, esempi e messaggi del comando lo chiamano sempre `weaver`, qualunque sia il nome del file da cui gira (`weaver-cli` nella build e nel pacchetto). Questo vale anche per `crates/core/assets/skills/trama-design/COMMANDS.md`, generato dal registro dei comandi: va rigenerato, e il test che lo confronta con il registro (`registry.rs:123` circa) deve passare.
      - **Sidecar e cartella `bin`.** `weaver-cli` è il sidecar Tauri `externalBin`, incluso nel pacchetto con quel nome accanto all'eseguibile `weaver` dell'app (`src-tauri/tauri.conf.json:31` oggi `binaries/trama`; `src-tauri/build.rs:78-150` costruisce il sidecar). A ogni avvio Weaver lo copia in `<app local data>/bin/` come `weaver` (`weaver.exe` su Windows) quando il contenuto è diverso, e toglie da quella cartella il file `trama` (`trama.exe`) lasciato dalle versioni precedenti. Quella cartella contiene solo `weaver` e va per prima nel PATH di ogni omp che Weaver avvia (`src-tauri/src/sidecar.rs`, il passo di avvio in `src-tauri/src/lib.rs:218-227`, `crates/core/src/paths.rs:96-98`). Nessun alias `trama`.
      - **Variabili d'ambiente.** Weaver passa al comando, nei job e in General, `WEAVER_ENDPOINT`, `WEAVER_JOB`, `WEAVER_GENERAL`, `WEAVER_RUN`, `WEAVER_TASK` al posto di `TRAMA_*`, con lo stesso significato; le variabili `WEAVER_*` ereditate dall'ambiente di Weaver vengono tolte prima di avviare omp. Il lato app e il lato comando cambiano insieme: `crates/cli/src/cli.rs:37-43`, `crates/core/src/jobs/launch.rs:105-115`, `crates/core/src/jobs/general.rs:451-454`, `crates/core/src/jobs/record.rs:94-101`, `src-tauri/src/jobs/actor.rs:245`, il commento di `crates/fake-acp/src/main.rs:16` e i test che le usano (`crates/cli/tests/question_round.rs`, `src-tauri/src/general/tests.rs`, `src-tauri/src/jobs/tests.rs`, `src-tauri/src/jobs/testing.rs`). I record dei job scritti da versioni precedenti non vengono migrati. La suite passa uguale con o senza qualunque variabile `WEAVER_*` nell'ambiente.
      - **Testi del comando.** Gli help e gli schemi che nominano la cartella o il comando dicono `.weaver` e `weaver`: `crates/cli/src/help.rs:11` (`--root <dir>` usa `<dir>/.weaver`), `crates/cli/src/commands/project.rs:19-26`, `commands/proposal.rs:91,112` (immagini copiate in `.weaver/task-images/<task>/`), `commands/history.rs:20`, `commands/model.rs:68`, il commento di `crates/cli/src/spec.rs:202`.
      - **Testi fissi dell'integrazione omp.** La risposta fissa a ogni richiesta di form o di ask di omp in un job è: "In Weaver jobs, ask the developer only with `weaver question new` (bash timeout 0). Do not ask in any other way."
      - **Watcher.** I messaggi di errore e di log del watcher dicono `.weaver/` (`src-tauri/src/windows/watch.rs:150,196`, oggi `"cannot watch .trama/: …"`). Il watcher guarda la radice del progetto solo per vedere `.weaver` comparire o sparire e `.weaver/` ricorsivamente finché esiste; una cartella `.trama` che compare, cambia o sparisce viene ignorata.
      - **Release.** I workflow CI e release costruiscono e includono `weaver-cli`; lo smoke test usa il binario appena costruito: `weaver-cli --help`, `weaver-cli project new --input '{}'`, `weaver-cli model check` su una cartella temporanea come `--root`, ognuno con exit 0; su macOS i build arm64 e x64 di `weaver-cli` vengono uniti con `lipo`. I test di contratto del CLI (`tests/cli-contract/contract.test.mjs`) eseguono il binario `weaver-cli`.
      - **Nome «Trama» nei testi.** Dove «Trama» indica il prodotto diventa «Weaver»: `README.md` (righe 3, 28, 46, 99 circa, compresi «Trama projects», i comandi `trama` e la cartella `.trama`), la `description` di `src-tauri/Cargo.toml` («the desktop app of Trama») e di `crates/cli/Cargo.toml`, i commenti di `crates/core/src/project/layout.rs` e `crates/core/src/agent_files.rs`. Nessun file `.trama` del repository di Weaver stesso, né le sue copie installate in `.agents/skills` e `.omp`, vengono spostati o modificati: lo sviluppatore sposta `.trama` a mano.

      ## Fuori da questo Task

      Altri Task della stessa Proposal fanno: la costante della cartella del progetto e tutti i percorsi `.trama` nel codice, nei test e nell'interfaccia (compresi il riconoscimento di `.weaver`, il fatto che `.trama` non conti e il testo di Get Started); le stringhe delle istruzioni dei job, il preambolo di General e i testi delle skill e degli agent installati, tranne `COMMANDS.md`, che rigenera questo Task.
  gv29qggl:
    title: Istruzioni e skill installate
    refs:
    - job
    - requirements/rules
    technical: false
    proposal: al2c1yof
    label: .weaver e weaver
    finished: 2026-11-02T14:13:01.603Z
    body: "La cartella del progetto diventa `.weaver` al posto di `.trama` e il comando diventa `weaver` al posto di `trama`. Questo Task porta il cambio nei testi che Weaver manda agli agenti o installa nei progetti: le istruzioni dei job, il preambolo di General, i testi delle skill e dei sub-agent installati e i testi delle regole dei Requirement.\n\n## Cosa deve essere vero alla fine\n\n**Istruzioni dei job.** In `job/Queued`, Weaver avvia omp nel progetto con la command line `weaver` sul path e il job già noto, poi manda l'istruzione. Le transizioni da Not started verso Queued (SEND_REQUEST, IMPORT_CODE, SCAN_SYSTEM, RUN_TASK, RUN_ALL, proposal.APPROVED, RESOLVE_DRIFT, question-round.ANSWERED, REWORK_PROPOSAL, APPROVE_PROPOSAL, agent.START_JOB) non cambiano, ma i job ricevono i nuovi testi. La tabella delle istruzioni in rules#job diventa:\n\n- Setup: ``Use the trama-setup skill on the setup request in `.weaver/requests/<request>.md`.``\n- Design: ``Use the trama-design skill on the design request in `.weaver/requests/<request>.md`.``\n- Run all: ``Use the trama-implement skill on the runnable Tasks: take them from `weaver task list --input '{\"runnable\":true,\"max\":<N>,\"groups\":true}'`, one batch at a time, until it says there is none.``\n- frase aggiunta 1: `Run at most <N> sub-agents at once.`, dove N è anche il `max` di `weaver task list`;\n- frase aggiunta 3: ``Ask the developer only with `weaver question new`, as the skill says, never with your own question tool; run it with no time limit and wait until it ends.``\n- Le altre righe della tabella non cambiano.\n\n**Preambolo di General.** Nomina `.weaver` e il comando `weaver` invece di `.trama` e `trama`.\n\n**Skill e sub-agent installati.** Nessun testo installato menziona `.trama` o il comando `trama`. Restano solo i nomi delle skill e dei sub-agent `trama-*`. In dettaglio:\n\n- Ogni comando `trama <verb>` diventa `weaver <verb>`.\n- Ogni percorso `.trama/...` diventa `.weaver/...`.\n- La parola «Trama» diventa «Weaver» dove indica il prodotto.\n- Le description delle skill che dicono \"a project with a `.trama/` folder\" passano a `.weaver/`. Gli agenti scelgono la skill da questa frase, quindi va cambiata ovunque.\n- La regola greenfield di `trama-scan` (\"only `.trama/`\") diventa \"only `.weaver/`\".\n- I nomi restano invariati: `trama-setup`, `trama-design` (con `FORMAT.md` e `COMMANDS.md`), `trama-import`, `trama-scan`, `trama-compare`, `trama-implement`, `trama-resolve-drifts`, i sub-agent `.omp/agents/trama-analysis.md` e `.omp/agents/trama-implementer.md`, `.omp/WATCHDOG.yml` e le skill generate `trama-req-*`.\n\n**Testi delle regole (requirements/rules).** Rispecchiano la versione del Model:\n\n- La prosa dell'area dice: \"Every rule here is computed in the Rust core, the same for the app and the `weaver` command line, never in the interface.\"\n- rules#General: le identità sono uniche tra `.weaver/tasks/` e i `tasks/` di ogni Proposal; la cache dei numeri brevi sta \"under `.weaver/`\".\n- rules#task: \"Slots: `weaver task list` with `runnable` and `max` N…\".\n- rules#drift: il fingerprint usa il testo che stampa `weaver model show <ref>`; \"`weaver drift list` with `refs` marks Dismissed Drifts…\".\n- rules#project: i file installati sono incorporati \"in the binary that the app and the `weaver` command line share\". La rigenerazione avviene \"1 s after the last change under `.weaver/requirements/`\". \"`weaver model check` warns … each fragment over 12,288 bytes…\".\n\nOvunque nel codice ci siano testi derivati da queste regole (messaggi, avvisi, commenti), vanno allineati.\n\n## Indizi nel codice (da ricontrollare)\n\n- `crates/core/src/job_instruction.rs`: intorno a :194 e :199 le istruzioni Setup e Design scrivono `.trama/requests/{request}.md`. I test intorno a :309 e :313 vanno aggiornati. Nello stesso file ci sono anche la stringa di Run all (`trama task list ...`) e \"Ask the developer only with `trama question new`\", da verificare.\n- `src-tauri/src/jobs/general.rs` intorno a :480: il preambolo di General nomina `.trama`/`trama`.\n- Asset installati, incorporati con `include_dir` da `crates/core/assets` (`crates/core/src/agent_files.rs:40`). Menzioni di `.trama` contate durante l'esplorazione:\n  - `skills/trama-design/SKILL.md`: 9\n  - `skills/trama-design/FORMAT.md`: 3\n  - `skills/trama-setup/SKILL.md`: 5\n  - `skills/trama-scan/SKILL.md`: 4, inclusa la regola greenfield\n  - `skills/trama-import/SKILL.md`: 3\n  - `skills/trama-implement/SKILL.md`: 3\n  - `skills/trama-resolve-drifts/SKILL.md`: 3\n  - `skills/trama-compare/SKILL.md`: 1\n  - `omp/agents/trama-implementer.md`: 2\n  \n  Vanno cercati anche i comandi `trama <verb>` e la parola «Trama».\n- `crates/core/src/project_files/requirement_skills.rs`: le skill generate non contengono `.trama`. `SKILL_PREFIX` `trama-req-` (circa :26) resta.\n\n## Fuori dallo scope di questo Task\n\n- `crates/core/assets/skills/trama-design/COMMANDS.md`: è generato dal registry della CLI e lo rigenera un altro Task della Proposal. Non va modificato a mano.\n- La costante della cartella, i percorsi `.trama` in codice, test e UI, il fatto che `.trama` venga ignorata, il testo di Get Started e la macchina del progetto: li gestisce un altro Task della Proposal.\n- Altre parti che gestisce un altro Task della Proposal:\n  - rinomina dei crate (`weaver-core`, `weaver-cli`);\n  - binario `weaver-cli`, sidecar e copia `weaver` in `<app local data>/bin/` con rimozione della vecchia copia `trama`;\n  - variabili d'ambiente `WEAVER_ENDPOINT`, `WEAVER_JOB`, `WEAVER_GENERAL`, `WEAVER_RUN`, `WEAVER_TASK`;\n  - help e usage della CLI;\n  - testo fisso di rifiuto dei form di omp;\n  - messaggi di log del watcher;\n  - README, descrizioni Cargo e doc comment;\n  - workflow CI.\n- Le copie installate nella repository di Weaver (`.agents/skills/*`, `.omp/agents/*`) e la cartella `.trama` della repository non vanno toccate né spostate. Lo sviluppatore le sposta a mano in seguito.\n\n## Fatto quando\n\n- Ogni istruzione che Weaver manda nomina `.weaver/requests/<request>.md` e comandi `weaver`.\n- Nessun testo degli asset installati menziona `.trama` o il comando `trama`, tranne i nomi `trama-*` delle skill e dei sub-agent.\n- I test delle istruzioni dei job sono aggiornati.\n- Quando anche il Task che mette `weaver` sul PATH e imposta le variabili `WEAVER_*` è completato, il controllo finale è che l'agente di un job riesca a eseguire `weaver job show`."
---
# Rinomina da Trama a Weaver: cartella .weaver, comando weaver e testi installati

Il prodotto passa dal nome Trama a Weaver, in tre parti che procedono insieme.

La cartella del progetto diventa `.weaver`, con lo stesso layout di prima. Weaver e la CLI leggono e scrivono solo lì. Una cartella con la sola `.trama` risulta `Not initialised`, e Weaver non la legge, non la sposta e non la cancella, senza avvisi né migrazione. Se esistono sia `.weaver` sia `.trama`, vale `.weaver`. L'inizializzazione crea `.weaver` con un Modello vuoto e il watcher segnala solo la comparsa o la scomparsa di `.weaver`. Get Started, i percorsi della riga Files delle Proposal e i test sono aggiornati di conseguenza.

Il comando usato dall'agente si chiama `weaver`. Le crate diventano `weaver-core` e `weaver-cli`, e il binario `weaver-cli` è il sidecar Tauri. A ogni avvio Weaver lo copia in `<app local data>/bin/` come `weaver` e rimuove il vecchio `trama`. Le variabili d'ambiente passano a `WEAVER_*`. Help, usage e `COMMANDS.md` rigenerato dicono sempre `weaver`. Cambiano anche il testo fisso di rifiuto dei form di omp, i messaggi del watcher, i workflow CI e di rilascio, e README e descrizioni Cargo. Il solo nome `trama` che resta è quello delle skill e dei sotto-agenti.

I testi che Weaver manda agli agenti o installa nei progetti nominano `.weaver` e `weaver`. Riguardano le istruzioni dei job, il preambolo di General, le skill e i sotto-agenti installati e i testi delle regole dei Requirement. I nomi `trama-*` di skill e sotto-agenti restano invariati.
