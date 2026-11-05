---
refs:
  - project-setup
  - solo-mode
  - requirements/rules#solo-mode
  - requirements/data#solo-mode
  - requirements/data#General
  - requirements/tech#solo-mode
  - requirements/cli#project
  - requirements/quality#solo-mode
  - requirements/docs#solo-mode
---
# Solo mode: core e CLI

Questa Task costruisce tutto Solo mode tranne la UI (Settings e Get Started) e l'elenco della vista File, che appartengono alle altre due Task di questa Proposal e si appoggiano su quanto fatto qui. Va completata per prima.

Al termine, con Solo mode acceso i file di Weaver di una copia di lavoro restano fuori da git grazie a un blocco di Weaver nelle esclusioni locali di git (`git rev-parse --git-path info/exclude`), che git non committa mai. Lo stato è della copia di lavoro: un altro clone o un worktree collegato parte spento.

## Modulo `crates/core/src/solo_mode.rs`

Nuovo modulo di `weaver-core`, l'unico che legge lo stato, trova il file delle esclusioni e il prefisso, scrive e toglie il blocco ed elenca i file tracciati. CLI e backend Tauri lo chiamano; la UI non esegue mai git (`requirements/tech#solo-mode`).

- Stato in `.weaver/solo/solo.json`: `{"mode": "folders"}` o `{"mode": "files"}`, UTF-8, LF, scrittura atomica. Nessun file = spento. Un file illeggibile o con un altro valore conta come spento ed è riportato come diagnostica con percorso e motivo; viene riscritto solo dal cambio successivo. Spegnere cancella l'intera cartella `solo/`. Solo mode non è un'impostazione: mai in `settings.json` né nel suo schema (`requirements/data#solo-mode`).
- `solo/` si aggiunge alle cartelle locali: `LOCAL_ONLY_DIRS` in `crates/core/src/project/layout.rs`, con il proprio `.gitignore` `*` scritto alla creazione e riscritto quando manca (`requirements/data#General`); aggiornare il relativo test.
- Modi `off`, `folders` (predefinito quando si accende senza scegliere), `files`; gli stessi valori in app, riga di comando e `solo.json`.
- Blocco, prefisso, scrittura, rimozione, blocco atteso, ordine dei passi nei cambi, file tracciati e testi `<reason>` (`git was not found`, `this folder is not in a git repository`, `git did not answer within 10 s`, `git failed: <first line of its error output>`, `could not write <path>: <system error>`) esattamente come in `requirements/rules#solo-mode`: righe di inizio/fine `# >>> weaver solo mode: <root>` / `# <<< weaver solo mode: <root>` con la radice canonica e separatori `/`; righe `folders`: `/<p>.weaver/`, `/<p>.agents/skills/`, `/<p>.omp/`; righe `files`: `/<p>.weaver/`, `/<p>.agents/skills/weaver-*/`, `/<p>.omp/agents/weaver-*.md`, `/<p>.omp/WATCHDOG.yml`; caratteri speciali del prefisso escapati; ogni altro byte del file (blocchi di altre copie compresi) resta intatto; riscrivere lo stesso blocco non scrive nulla; prima occorrenza sostituita, duplicati rimossi, riga di inizio orfana rimossa; file o `info/` mancanti creati; rimozione che lascia un file vuoto se resta vuoto.
- Chiamate git: solo `git rev-parse --show-prefix --git-path info/exclude` (una per operazione) e `git ls-files -z --cached --ignored --exclude=<line>` per ogni riga del blocco, eseguita nella radice del repository. Avvio con `crates/core/src/process.rs` `command` (niente shell, niente console su Windows), cwd la radice del progetto, `GIT_OPTIONAL_LOCKS=0`, git cercato sul PATH. Avvio non trovato → `git was not found`; `rev-parse` con exit 128 → `this folder is not in a git repository`.
- Ogni chiamata git ha un limite di 10 s e viene terminata oltre, tramite il runner a tempo condiviso con omp: spostare `run_command` da `crates/core/src/omp/run.rs` fuori dal modulo omp (per esempio accanto a `crates/core/src/process.rs`) e far usare a omp e a Solo mode lo stesso runner. Il limite deve essere iniettabile per i test.
- File tracciati: i file che git traccia e che le righe del blocco includono, percorsi relativi alla radice del progetto con `/`, ordinati; mai altri file tracciati ignorati. Accendere riesce anche con file già tracciati; Weaver non offre modo di smettere di tracciarli e non tocca mai l'indice di git.
- Un cambio che fallisce non cambia nulla e restituisce il motivo. Accendere: blocco, poi `solo.json`; spegnere: rimozione del blocco, poi della cartella `solo/`; cambio di modo: riscrittura del blocco, poi `solo.json`. Cambi immediati, senza conferma.

## Inizializzazione

`initialise` in `crates/core/src/project_files/initialise.rs` riceve la scelta di Solo mode (`project/Not initialised/developer.INITIALISE_PROJECT`, `solo-mode/Off/developer.INITIALISE_PROJECT`):

- dopo i controlli di esistenza e di conflitto dei file dell'agente, e prima che `plan.apply()` scriva qualsiasi file dell'agente o `.weaver/`, scrive il blocco; `solo/solo.json` si scrive con il resto di `.weaver/`;
- fuori da un repository git, senza git o con il blocco non scrivibile: progetto non creato, nessun file, errore con il motivo (nuova variante di `InitialiseError` o equivalente);
- un fallimento successivo (rollback di `Plan` in `crates/core/src/project_files/plan.rs`) toglie anche il blocco: nessun blocco resta dietro un'inizializzazione fallita;
- con `off` git non viene chiamato.

## Backend Tauri (`src-tauri/src/project_setup.rs`)

- `ProjectStatus` porta `solo`: `mode`, `unavailable` (motivo per cui Solo mode non si può cambiare, assente quando si può) e `problem` (perché il blocco non è stato messo a posto all'ultima apertura, assente altrimenti), inviato con `project-state`; rigenerare il binding ts-rs `ui/src/bindings/ProjectStatus.ts`.
- Comando `set_solo_mode {projectId, mode}`: nella coda di mutazione del progetto sotto il lock del progetto, come le altre scritture; restituisce il nuovo stato con i file tracciati, oppure l'errore.
- Comando `solo_tracked_files {projectId}`: calcola i file tracciati solo su richiesta (Settings mostrato); mai altrimenti.
- `initialise_project(project_id, force, solo)`: nuovo parametro `solo` passato a `initialise`.
- Registrazione dei comandi in `src-tauri/src/commands.rs` e `src-tauri/src/lib.rs`.
- Riparazione all'apertura (`solo-mode/On/Excluded` e `Not excluded`, `window.OPENED`): all'apertura di una finestra su un progetto inizializzato, sia Empty Model sia Has a Model (oggi `opened()` richiede `OpeningCheck` solo con `HasModel`), gira nel lavoro in background del progetto, coalescita come il controllo d'apertura dei file installati (`crates/core/src/project_files/schedule.rs`, `run_work`) e nello stesso giro quando servono entrambi. Scrive solo quando il blocco manca o differisce, in silenzio. Il suo esito imposta o azzera solo `solo.problem`; non cambia mai l'esito del controllo d'apertura, il suo toast o il suo banner. Senza git o fuori dal repository Solo mode resta acceso e si riprova alla prossima apertura.
- `.weaver` che sparisce (`solo-mode/On/workspace.PROJECT_FOLDER_REMOVED`, watcher in `src-tauri/src/windows/watch.rs`): prima di tutto il resto, toglie il blocco di questa copia trovato solo dai marcatori; un fallimento si registra nel log e non si mostra nulla. Se la cartella sparisce a Weaver chiuso il blocco resta.
- Nessuna chiamata git sul thread della UI o dell'IPC (pool bloccante `src-tauri/src/blocking.rs`).

## Riga di comando (`crates/cli/src/commands/project.rs`)

Come in `requirements/cli#project` e `requirements/tech#solo-mode`:

- `project new {force?, solo?}`: `solo` `off` (predefinito), `folders`, `files`; con un valore acceso e Solo mode non attivabile, rifiuto exit 1 con il motivo e nulla creato; stampa anche `solo`.
- `project edit {solo?}`: con `solo` cambia solo Solo mode (via app in esecuzione, o da sé sotto il lock del progetto) e stampa `solo`; senza, resta Update skills.
- `project show {}`: aggiunge `solo` accanto alla Board.
- Uscita `solo` come `{mode, unavailable?, problem?, tracked}`, `tracked` vuoto quando spento; `project show` e `project edit {solo}` lo calcolano alla chiamata, `problem` dice perché, con Solo mode acceso, il blocco non è al suo posto; la riga di comando non ripara mai.
- Aggiornare il test unitario `project_show_prints_the_board_of_an_empty_project`; rigenerare `COMMANDS.md` con `cargo run -p weaver-cli --example write_command_reference` (test `the_command_reference_of_the_skills_is_the_one_the_registry_renders`); togliere `"solo"` da `OUT_OF_SCOPE_WORDS` in `crates/core/src/agent_files.rs`.
- Test di contratto in `tests/cli-contract/contract.test.mjs` (con `tests/cli-contract/support.mjs`) per il campo `solo` di `project new`, `project edit` e `project show`: help, validazione con exit 2, exit 0 e 1, forme Markdown e `--json`.

## Riferimenti nel codice

- `solo-mode` — `crates/core/src/solo_mode.rs` (nuovo)
- `requirements/data#General` — `crates/core/src/project/layout.rs`
- `requirements/tech#solo-mode` — `crates/core/src/process.rs`, `crates/core/src/omp/run.rs`
- `project-setup` — `crates/core/src/project_files/initialise.rs`, `crates/core/src/project_files/plan.rs`, `crates/core/src/project_files/schedule.rs`
- `solo-mode` — `src-tauri/src/project_setup.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/windows/watch.rs`
- `requirements/cli#project` — `crates/cli/src/commands/project.rs`, `crates/core/src/agent_files.rs`, `tests/cli-contract/contract.test.mjs`

## Test (`requirements/quality#solo-mode`)

Contro repository git temporanei con git reale, mai la home dello sviluppatore:

- blocco per entrambi i modi alla radice e in una sottocartella, prefisso con caratteri speciali, worktree collegato che scrive nel file del repository principale;
- altre righe intatte byte per byte, con e senza newline finale; stesso blocco scritto due volte senza toccare il file; blocchi di due copie indipendenti; blocchi duplicati e rotti; rimozione che lascia il resto;
- git mancante (PATH vuoto), cartella fuori da un repository, chiamata git oltre il limite (limite iniettato);
- inizializzazione con ciascun modo: blocco scritto prima di ogni altro file, rifiuto fuori da git senza nulla creato, fallimento successivo che toglie il blocco;
- riparazione all'apertura con Model vuoto e con Model, silenziosa e senza cambiare l'esito del controllo d'apertura; `.weaver` che sparisce toglie il blocco;
- file tracciati per ciascun modo, con i file del team in `.agents/skills/` esclusi in modo `files`;
- campo `solo` della CLI in `project new`, `project edit` e `project show`.

I test della UI (Vitest di Settings e Get Started) e quelli dell'elenco della vista File appartengono alle altre Task.

## Documentazione (`requirements/docs#solo-mode`)

- `README.md`: sezione `## Solo mode` dopo `## Install` e `## Behind a corporate proxy`: cosa fa (i file di Weaver restano solo in questa copia e non entrano in un commit, tramite le esclusioni locali di git); che appartiene alla copia di lavoro e dove si sceglie (Get Started, Settings → Project, `solo` di `weaver project new` e `project edit`); i due modi e quando usare il secondo; che i file già tracciati non vengono nascosti e Settings li elenca; che richiede git e che, acceso, perdere la copia di lavoro fa perdere i file di Weaver.
- `CHANGELOG.md`: una riga sotto `## [Unreleased]`, `Added`, che descrive Solo mode.
