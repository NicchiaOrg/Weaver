---
created: 2026-11-04T09:49:20.909Z
updated: 2026-11-04T09:49:20.909Z
summary: Con Solo mode i file di Weaver di una copia di lavoro restano fuori da git tramite le esclusioni locali, scelti in Get Started, in Settings o con la riga di comando, e la vista File continua a mostrarli.
tasks:
- fpal0vpw
- g5ym05zi
- h7ts7v50
cleared:
  fpal0vpw:
    title: 'Solo mode: core e CLI'
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
    technical: false
    proposal: 06olrobr
    label: Solo mode
    finished: 2026-11-04T09:34:18.626Z
    body: |-
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
  g5ym05zi:
    title: Solo mode nell'interfaccia
    refs:
    - developer.CHANGE_SOLO_MODE
    - developer.INITIALISE_PROJECT
    - project/Not initialised/developer.INITIALISE_PROJECT
    - requirements/ui#solo-mode
    - requirements/ui#developer.CHANGE_SETTINGS
    - requirements/ui#project
    technical: false
    proposal: 06olrobr
    label: Solo mode
    finished: 2026-11-04T09:39:36.842Z
    body: |-
      Al termine, lo sviluppatore sceglie Solo mode dall'interfaccia: in Settings per la copia di lavoro di un progetto già inizializzato (`developer.CHANGE_SOLO_MODE`) e in Get Started al momento dell'inizializzazione (`developer.INITIALISE_PROJECT`).

      ## Dipendenza

      Questa Task si appoggia al modulo `crates/core/src/solo_mode.rs` di `weaver-core` e al backend Tauri creati dalla Task "Solo mode: core e CLI", che va completata prima: i comandi `set_solo_mode` e `solo_tracked_files`, il campo `ProjectStatus.solo` (stato, modo, motivo per cui è disabilitato o non escluso) e il parametro `solo` di `initialise_project`. Non va reimplementata alcuna logica di git, del blocco di esclusione o di stato: l'interfaccia mostra lo stato del progetto e invia le scelte al backend. Se un binding manca o ha forma diversa, si usa quello che quella Task ha prodotto.

      ## Controllo (requirements/ui#solo-mode)

      - Titolo `Solo mode`, controllo segmentato `Off` / `Whole folders` / `Weaver's files only`, lo stesso componente di `Theme` (radio group per le tecnologie assistive), con sotto la nota `Applies only to this working copy: other clones start with it off.`
      - Non sta nello schema delle impostazioni e non viene mai salvato in `settings.json`.
      - Accessibilità: nota, avvisi e motivo di disabilitazione sono la descrizione del controllo (`aria-describedby`); l'elenco dei file tracciati è una regione con etichetta; i cambi degli avvisi sono annunciati (`aria-live="polite"`).

      ## Settings (requirements/ui#developer.CHANGE_SETTINGS, requirements/ui#project)

      - Primo elemento del gruppo `Project`, prima di `Update skills`.
      - Salvataggio immediato alla scelta, senza conferma, anche spegnendo; cambio di modo immediato. Durante il cambio il controllo è disabilitato. Un cambio fallito (git assente, esclusioni non scrivibili) non cambia nulla: mostra sotto il controllo `Could not change Solo mode: <reason>.` nel colore di errore e ripristina il valore del progetto.
      - Cartella non inizializzata: disabilitato su `Off`, con `Choose Solo mode when you initialise the project.` sotto la nota.
      - Senza git o fuori da un repository: disabilitato su `Off` con `Solo mode needs git: git was not found.` o `Solo mode needs git: this folder is not in a git repository.` (gli altri motivi di `requirements/rules#solo-mode` come `Solo mode needs git: <reason>.`).
      - Acceso: sotto la nota, colore di avviso con `TriangleAlert`, `Weaver's files are not versioned anywhere: losing this working copy loses them.`
      - Acceso ma non escluso (riparazione all'apertura fallita; lo dice solo Settings): colore di avviso con `TriangleAlert`, `Solo mode is on but cannot exclude Weaver's files: <reason>.` quando git manca o la cartella non è più in un repository, altrimenti `Weaver's files may not be excluded from git: <reason>.` Il controllo resta utilizzabile.
      - File tracciati: con Solo mode acceso e l'elenco non vuoto, un `Alert` (variante warning) con `Git still tracks <N> of Weaver's files: their changes still go into commits.` e sotto i percorsi relativi in monospace, che scorrono oltre 8 righe. Nessuna azione per smettere di tracciarli. L'elenco si chiede a `solo_tracked_files` ogni volta che la vista Settings viene mostrata e si sostituisce col risultato di ogni cambio; durante il caricamento resta l'elenco precedente.

      ## Get Started (requirements/ui#project)

      - Il controllo sta tra l'elenco dei file dell'agente e `Initialise`; `Off` scelto all'inizio, la scelta vive solo nella schermata.
      - Con `Whole folders` o `Weaver's files only` il testo diventa `Weaver will create a .weaver folder that describes this project. With Solo mode its files stay only in this working copy and never go into a commit.` e sotto il controllo compare l'avviso di Solo mode acceso.
      - Senza git o fuori da un repository: disabilitato su `Off` con lo stesso motivo `Solo mode needs git: ...` di Settings.
      - `Initialise` e `Overwrite and initialise` inviano entrambi la scelta (`initialise_project` con `solo`). Come vuole il Model, con un modo scelto Weaver accende Solo mode prima di scrivere i suoi file; se non ci riesce il progetto non viene creato e il motivo compare nel riquadro esistente `Could not initialise: <reason>.`, con il pulsante che resta per riprovare.

      ## Punti del codice

      - `requirements/ui#solo-mode` — nuovo componente in `ui/src/components/project/` (es. `solo-mode.tsx`), riusato da Settings e Get Started; chiamate in `ui/src/lib/backend.ts`.
      - `requirements/ui#developer.CHANGE_SETTINGS` — `ui/src/components/project-content.tsx:85` (`projectSlot`: Solo mode prima di `<UpdateSkills />`), `ui/src/components/settings-view.tsx` (gruppo `Project`), stato da `ui/src/components/project/use-project-status.tsx`.
      - `project/Not initialised/developer.INITIALISE_PROJECT` — `ui/src/components/project/get-started.tsx` (testo, controllo dopo `PathList`, `Initialise`, `ConfirmDialog` di sovrascrittura, riquadro d'errore).

      ## Test (requirements/quality#solo-mode)

      In Vitest, con `vi.mock("@/lib/backend")` e query per ruolo: il controllo, le note, gli avvisi e l'elenco in Settings e in Get Started, con i percorsi da tastiera e i nomi accessibili. Aggiornare `ui/src/components/settings-view.test.tsx` (ordine del gruppo `Project`: Solo mode prima di `Update skills`; stati disabilitati, acceso, non escluso, elenco tracciato ricaricato a ogni visualizzazione, cambio fallito che ripristina il valore) e `ui/src/components/project/get-started.test.tsx` (`Off` iniziale, testo che segue la scelta, avviso, scelta inviata da `Initialise` e `Overwrite and initialise`, errore in `Could not initialise: <reason>.`, stato senza git).
  h7ts7v50:
    title: Vista File e Solo mode
    refs:
    - files
    - requirements/tech#file-view
    technical: false
    proposal: 06olrobr
    label: Solo mode
    finished: 2026-11-04T09:44:08.557Z
    body: |-
      Quando il Task è concluso, l'albero della File view continua a mostrare ciò che un blocco di Solo mode esclude, di qualunque working copy sia il blocco, e continua a lasciare fuori `.git` e tutto ciò che le altre regole di git ignorano. Il Model (`files/file-view`) dice: `developer.OPEN_FILE_VIEW` → Open: Weaver legge solo nomi e tipi delle voci della radice del progetto, mai il contenuto di un file, lasciando fuori `.git` e ciò che git ignora, tranne ciò che Solo mode tiene fuori da git. Lo stesso vale per ogni cartella espansa e per ogni nuova lettura dopo un evento del watcher.

      Cosa deve valere:
      - Il listing segue `requirements/tech#file-view`: una cartella alla volta con `read_dir`, `.git` escluso per nome, ciò che git ignora escluso con una sola chiamata `git check-ignore` per cartella listata, con `GIT_OPTIONAL_LOCKS=0`. Un percorso ignorato solo da una riga di un blocco di Solo mode viene listato.
      - Per saperlo si usa `git check-ignore -v -z --stdin`, che per ogni percorso indica file sorgente e riga della regola che lo ignora. Se la regola è una riga dentro un blocco di Solo mode del file delle esclusioni locali (formato dei marcatori `# >>> weaver solo mode: <root>` / `# <<< weaver solo mode: <root>` in `requirements/rules#solo-mode`), il percorso resta nell'albero; vale per i blocchi di ogni working copy, non solo per quello della radice corrente. Se la regola viene da qualunque altra parte (`.gitignore`, `core.excludesFile`, righe delle esclusioni fuori dai blocchi), il percorso resta fuori come oggi.
      - Il file delle esclusioni viene letto una sola volta per listing per trovare gli intervalli di righe dentro i blocchi di Solo mode, e viene trovato con l'helper di `crates/core/src/solo_mode.rs`. Quel modulo e il suo backend sono creati dal Task "Solo mode: core e CLI", che va fatto prima; questo Task li usa e non li re-implementa.
      - Resta una sola chiamata git per cartella listata (performance della File view in `requirements/quality`, e `requirements/quality#solo-mode`: mai sul thread della UI o dell'IPC, mai oltre 10 s per chiamata). Senza git, o fuori da un repository, non si ignora nulla, come oggi.
      - Il comportamento dell'albero di Import from Code non cambia: continua a lasciare fuori tutto ciò che git ignora, blocchi di Solo mode compresi.

      Punti del codice:
      - `files` — `crates/core/src/file_view/listing.rs`: listing della File view, dove si filtrano i risultati di `git check-ignore`.
      - `requirements/tech#file-view` — `crates/core/src/project_files/folder_tree.rs`: la chiamata `git check-ignore` condivisa; aggiungere la variante con `-v` per la File view lasciando invariata quella usata da Import from Code.
      - `crates/core/src/solo_mode.rs`: helper per trovare il file delle esclusioni e il formato dei marcatori (solo uso).

      Test da aggiungere, secondo `requirements/quality#solo-mode`, contro repository git temporanei: la File view lista ciò che un blocco di Solo mode ignora (anche un blocco di un'altra working copy) e continua a lasciare fuori ciò che le altre regole ignorano, incluso un percorso coperto sia dal blocco sia da `.gitignore`; l'albero di Import from Code continua a escludere ciò che il blocco ignora.
---
# Solo mode

Solo mode tiene i file di Weaver solo nella copia di lavoro: un blocco di Weaver nelle esclusioni locali di git (mai committate) esclude `.weaver/` e, a scelta, le cartelle intere dell'agente e di omp oppure solo i file installati da Weaver. Lo stato sta in `.weaver/solo/solo.json`, non nelle impostazioni.

- Core e riga di comando: modulo `solo_mode` in `weaver-core`, scelta alla creazione del progetto, riparazione silenziosa del blocco a ogni apertura, rimozione del blocco quando `.weaver` sparisce, campo `solo` di `project new`, `project edit` e `project show` con l'elenco dei file già tracciati da git.
- Interfaccia: controllo `Solo mode` in Get Started e in Settings, con avvisi sui file tracciati, sul blocco non applicato e sul fatto che i file non sono versionati altrove.
- Vista File: l'albero lista ciò che un blocco di Solo mode esclude, di qualunque copia di lavoro, e continua a lasciare fuori `.git` e le altre esclusioni di git; Import from Code non cambia.
