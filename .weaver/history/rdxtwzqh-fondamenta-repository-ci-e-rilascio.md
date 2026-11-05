---
created: 2026-11-02T12:31:02.727Z
updated: 2026-11-02T12:31:02.727Z
summary: Repository, cargo workspace e app Tauri 2 con interfaccia vuota; workflow CI con impianto di test; workflow di rilascio su GitHub Releases con aggiornamenti dall'app.
tasks:
  - azc1e8rx
  - 7e0saact
  - xeiop1ne
cleared:
  7e0saact:
    title: CI e test
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T08:00:31.942Z
    refs:
      - specs#ci
      - specs#testing
    body: |-
      Il workflow `ci` su GitHub Actions e l'impianto di test di Weaver. Dipende da "Scheletro del progetto". Lo sviluppo avviene su macOS, dove non esiste un WebDriver per la webview: per questo i test end-to-end girano solo su GitHub Actions, su Linux e Windows.

      ## Strumenti di test (`specs#testing`)

      - `cargo test` per core, CLI e backend; Vitest con `@testing-library/react` per l'interfaccia (interazioni come le fa l'utente, percorsi da tastiera e nomi accessibili); tauri-driver con WebdriverIO per gli end-to-end dell'app impacchettata.
      - Regole dei test da rispettare in tutto il repository: logica del core come funzioni pure con orologio, identità casuali e tempi del watcher iniettati, mai attese con sleep; operazioni su file in cartelle temporanee, mai la home, le impostazioni o il portachiavi del developer; i test dei lavori usano un agente ACP finto e scriptato, mai un omp reale né la rete; la suite passa uguale con o senza variabili `TRAMA_*` o `WEAVER_*` nell'ambiente; un test saltato su un sistema operativo dice perché.
      - Test di contratto della CLI che eseguono il binario `trama` compilato, per ogni comando del registro: `--help` e `--help --json`, validazione dell'input con exit 2 e percorsi dei campi, exit 0 e 1, forma dell'output Markdown e `--json`, `--input -` con e senza BOM UTF-8. Il registro e le sue regole li realizza il Task "Riga di comando trama"; qui va l'impianto che li esegue in CI.
      - Smoke end-to-end con tauri-driver: l'app parte, compare la finestra di benvenuto, un progetto di prova si apre nella sua finestra, Board e Diagramma si disegnano, una carta si apre, la finestra si chiude e l'app esce senza errori.

      ## Workflow `ci` (`specs#ci`)

      - File `.github/workflows/ci.yml`. Trigger: push su `dev`, pull request verso `dev` o `main`, push su `main`. `main` e `dev` sono protetti: `main` cambia solo da `dev` con una pull request i cui controlli `ci` sono passati.
      - Matrice `fail-fast: false`, ogni voce richiesta: `macos-latest` (arm64), `windows-latest` (x64), `ubuntu-22.04` (x64). Su Linux si installano prima `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `patchelf`.
      - Toolchain fissate: Rust in `rust-toolchain.toml` (versione esatta, con `rustfmt` e `clippy`), Node in `.nvmrc` e `engines` di `ui/package.json`, pnpm con `packageManager`. Installazione dai lockfile senza aggiornarli (`cargo --locked`, `pnpm install --frozen-lockfile`).
      - Passi per piattaforma: installazione, build del frontend, `cargo test --workspace --locked`, Vitest, build dell'app in release con `trama` incluso. Il controllo dei tipi TypeScript generati con `ts-rs` fallisce su un file non aggiornato; `ci` fallisce anche quando la versione di `src-tauri/tauri.conf.json` differisce da quelle di `src-tauri/Cargo.toml`, `crates/core/Cargo.toml`, `crates/cli/Cargo.toml` e `ui/package.json`.
      - Smoke su ogni piattaforma con il `trama` appena compilato e una cartella temporanea nuova come `--root`: `trama --help`, `trama project new --input '{}'`, `trama model check`; ciascuno deve uscire con 0.
      - End-to-end con tauri-driver solo su Linux (`webkit2gtk-driver` sotto `xvfb-run`) e Windows (`msedgedriver` della versione di Edge installata sul runner), sempre contro la build release, mai contro un dev server.
      - Timeout: 25 minuti per i job di build e test, 15 per gli altri. Cache (registro Cargo e `target`, store dei pacchetti JS) con chiave su sistema operativo, versione della toolchain e hash del lockfile.

      ## Fatto quando

      - Un push su `dev` o una pull request esegue `ci` sulle tre piattaforme e tutti i passi sopra passano.
      - Lo smoke di `trama` e gli end-to-end (Linux e Windows) fanno parte del workflow e un loro fallimento fa fallire `ci`.
      - Una versione disallineata fra i cinque file o un file `ts-rs` non aggiornato fanno fallire `ci`.
      - Esiste almeno un test Vitest con Testing Library, un test di contratto della CLI e lo smoke end-to-end descritto.
  azc1e8rx:
    title: Scheletro del progetto
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T07:46:47.203Z
    refs:
      - specs#languages
      - specs#frameworks
      - specs#repository
      - specs#data
      - specs#notes
      - requirements/tech
      - requirements/tech#General
    body: |-
      La base su cui poggiano tutti gli altri Task: il repository, il cargo workspace, l'app Tauri 2 e l'interfaccia vuota ma funzionante. Questo Task copre l'Area `requirements/tech` solo come base dell'architettura (`requirements/tech#General`); le sezioni legate alle singole Machine (watcher, round di domande, finestre, notifiche, omp e ACP) sono realizzate dai Task che le riguardano.

      ## Repository

      - Repository GitHub `nicchia-code/weaver`, con i rami `dev` (dove arriva il lavoro) e `main` (ciò che è rilasciato).
      - `README.md` come pagina d'ingresso: cos'è Weaver, come si installa, e la nota delle Specs (`specs#notes`): le build non sono firmate né notarizzate per ora; firma e notarizzazione su macOS e firma su Windows arriveranno con una Proposal dedicata. Le istruzioni per aprire le build non firmate le completa il Task "Rilascio e aggiornamenti".

      ## Cargo workspace e linguaggi (`specs#languages`, `specs#frameworks`)

      - Rust stable, edition 2021; TypeScript 5 in modalità `strict`, moduli ES, `noUncheckedIndexedAccess`, verificato con `tsc --noEmit`.
      - `crates/core`: libreria `trama-core`, senza dipendenza da Tauri né runtime async, solo I/O `std` sincrono. Contiene i tipi del protocollo CLI↔app e deriva dall'identificatore Tauri `dev.nicchia.weaver` le cartelle di config, dati locali e runtime, così la CLI le trova anche ad app chiusa.
      - `crates/cli`: binario `trama` (clap, serde), qui solo l'ossatura con il client del socket.
      - `src-tauri`: crate dell'app `weaver` su Tokio; il lavoro su disco gira in `spawn_blocking`, mai sul main thread né sul thread IPC.
      - `ui/`: Vite + React 19 + TypeScript, pnpm, shadcn/ui (base zinc, primario blu), Radix UI, Tailwind CSS 4, lucide-react, class-variance-authority, clsx, tailwind-merge.
      - `trama` incluso come sidecar Tauri `externalBin`; a ogni avvio Weaver lo copia in `<app local data>/bin/` quando il contenuto differisce (percorso stabile anche sotto AppImage); quella cartella contiene solo `trama`.

      ## Tauri, IPC e dati (`specs#data`)

      - Plugin `single-instance` (registrato per primo), `window-state`, `notification`, `dialog`, `opener`, `updater`, chiamati da Rust. Nessun plugin `deep-link`, `shell`, `fs` o `http`. La capability della webview concede solo i comandi di Weaver e i permessi core di eventi e finestre; la CSP non ammette origini remote.
      - Ossatura backend ↔ UI: le azioni sono comandi Tauri che restituiscono un risultato o un errore tipizzato, gli aggiornamenti sono eventi del backend (per esempio `project-changed` con id del progetto, revisione e porzioni cambiate). I tipi TypeScript di comandi, eventi e payload sono generati dai tipi Rust con `ts-rs`; un controllo fallisce se il file generato non è aggiornato. La UI non tiene stato di dominio.
      - Dati solo in file: `.trama/` nel progetto, cartella di configurazione del sistema per impostazioni e stato locale, portachiavi del sistema come unica sede ammessa per segreti (nessuno nella prima versione). Nessun database.

      ## Canale CLI ↔ app

      - Crate `interprocess`, nessuna porta TCP o HTTP. Unix: socket `weaver-<8 casuali>.sock` in una cartella `0700` (`$XDG_RUNTIME_DIR/weaver/`, altrimenti `<app local data>/run/`, oppure `/tmp/weaver-<uid>/` se il percorso supererebbe 100 byte). Windows: named pipe `\\.\pipe\weaver-<16 casuali>`.
      - A ogni avvio l'app scrive in modo atomico `<app local data>/runtime.json` (modo `0600`) con `{endpoint, secret, pid, version}`. Il segreto: 32 byte dal CSPRNG, in hex, nuovo a ogni avvio, mai in variabili d'ambiente, argomenti, log o `.trama/`; confrontato a tempo costante, uno sbagliato o assente chiude la connessione senza risposta.
      - Messaggi JSON separati da newline, al massimo 1 MiB, una richiesta per connessione, ciascuna con una versione del protocollo; con versione diversa la CLI si comporta come se nessuna app rispondesse.

      ## Scritture e processi

      - Una coda seriale di mutazioni per progetto. Ogni mutazione tiene un lock consultivo del sistema (`fd-lock`: `flock` / `LockFileEx`) su `<app local data>/locks/<project hash>.lock`, lo stesso che prende la CLI quando scrive da sé; `<project hash>` sono i primi 16 caratteri hex dello SHA-256 della radice canonica. Sotto il lock la mutazione rilegge da disco, valida il risultato e poi scrive; un rifiuto non scrive nulla. Lock non ottenuto entro 10 s: errore "project busy".
      - Ogni processo figlio parte senza shell, con percorso del programma e array di argomenti; nessun testo dell'utente interpolato in una riga di comando; su Windows `CREATE_NO_WINDOW`. Una sola funzione di avvio condivisa.

      ## Fatto quando

      - `cargo build --workspace` e `pnpm build` in `ui/` riescono; l'app si avvia e mostra una finestra con l'interfaccia di base.
      - Test: la CLI ad app chiusa trova le stesse cartelle dell'app; un client con segreto sbagliato non riceve risposta; un messaggio oltre 1 MiB o di altra versione è rifiutato; due mutazioni concorrenti (app e CLI) sullo stesso progetto si serializzano e un lock non ottenuto in 10 s dà "project busy".
      - Il controllo dei tipi generati con `ts-rs` fallisce su un file non aggiornato.
      - Il README esiste con la nota sulle build non firmate.
  xeiop1ne:
    title: Rilascio e aggiornamenti
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T08:42:44.629Z
    refs:
      - specs#deployment
      - requirements/release#General
    body: |-
      Il workflow `release` su GitHub Releases e gli aggiornamenti di Weaver dall'app. Dipende da "Scheletro del progetto" e da "CI e test", perché il rilascio parte solo dopo un `ci` riuscito su `main`. Decisione dell'intervista: per ora build non firmate, la soluzione più veloce; firma e notarizzazione su macOS e firma su Windows arriveranno con una Proposal dedicata.

      ## Workflow `release` (`requirements/release#General`)

      - File `.github/workflows/release.yml`, avviato da `workflow_run` di `ci` completato sul ramo `main` con esito `success`; lavora sullo `head_sha` di quella esecuzione. Mai per `dev` o pull request, mai su un commit i cui test sono falliti.
      - Primo job (15 min): legge `version` da `src-tauri/tauri.conf.json` e cerca `refs/tags/v<version>` su `origin`. Se il tag esiste il workflow finisce con successo dopo un `::notice::` che dice che il rilascio di `v<version>` è saltato perché il tag esiste: il push su `main` ha fatto solo i test. Altrimenti crea una GitHub Release in bozza `Weaver v<version>` per il tag `v<version>` su `head_sha` (la bozza non crea il tag).
      - Job di build (25 min ciascuno) con `tauri-apps/tauri-action`, che caricano sulla bozza i file con i nomi che Tauri produce per il prodotto `Weaver`: `macos-latest` target `universal-apple-darwin` → `.dmg` e `.app.tar.gz` + `.sig`; `windows-latest` target `x86_64-pc-windows-msvc` → installer NSIS `-setup.exe` + `.sig` (niente MSI); `ubuntu-22.04` target `x86_64-unknown-linux-gnu` → `.AppImage` + `.sig` e `.deb`.
      - `trama` compilato per lo stesso target dell'app e incluso come sidecar; su macOS le build arm64 e x64 unite con `lipo` in un binario universale.
      - `latest.json` dell'updater (piattaforme `darwin-aarch64`, `darwin-x86_64`, `windows-x86_64`, `linux-x86_64`) allegato alla release.
      - Un job finale pubblica la bozza, e così crea il tag `v<version>` su `head_sha`, solo se tutti i job di build sono riusciti; se uno è fallito o annullato la bozza è cancellata e nessun tag nasce. Il workflow non sposta, cancella né ricrea mai un tag e non modifica mai una release pubblicata.
      - Note di rilascio: per ogni piattaforma il nome esatto del file da scaricare e i passi per aprire una build non firmata.

      ## Firma

      - Artefatti dell'updater firmati con la chiave dell'updater di Tauri, necessaria anche senza firma del codice: segreti GitHub Actions `TAURI_SIGNING_PRIVATE_KEY` e `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`; chiave pubblica in `tauri.conf.json` (`plugins.updater.pubkey`); `bundle.createUpdaterArtifacts` a `true`. La chiave privata va conservata anche fuori da GitHub: perderla ferma gli aggiornamenti di ogni copia installata.
      - Nessuna firma del codice né notarizzazione per ora. Il README spiega come aprire le build non firmate: macOS, clic destro → **Open** o System Settings → Privacy & Security → **Open Anyway**; Windows SmartScreen, **More info** → **Run anyway**; Linux, `chmod +x` dell'AppImage.

      ## Aggiornamenti dall'app

      - Plugin updater di Tauri con endpoint `https://github.com/nicchia-code/weaver/releases/latest/download/latest.json`.
      - Controllo all'avvio e poi al massimo una volta ogni 24 ore mentre l'app è aperta; l'istante dell'ultimo controllo sta nelle impostazioni. Un controllo fallito è scritto nel log e ritentato alla scadenza successiva, mai mostrato come errore.
      - Un aggiornamento trovato mostra versione e note e si installa solo quando il developer sceglie **Install and restart**; **Later** lo nasconde fino al controllo successivo. Install and restart passa dalla stessa strada dell'uscita da Weaver, compresa la sua conferma quando ci sono lavori in corso.
      - Un aggiornamento la cui firma non si verifica è rifiutato e scritto nel log.

      ## Fatto quando

      - Un push su `main` con una versione nuova, dopo un `ci` riuscito, produce una release pubblicata con `.dmg`, installer NSIS, AppImage, deb, firme e `latest.json`, e il tag `v<version>`.
      - Un push su `main` con una versione già taggata termina con il `::notice::` e non costruisce nulla.
      - Un job di build fallito non lascia né release né tag.
      - Test del backend con un updater finto: controllo a 24 ore, errore non mostrato, Later, firma non valida rifiutata.
---
# Fondamenta: repository, CI e rilascio

Base su cui poggia tutto il resto di Weaver.

Lo scheletro del progetto crea il cargo workspace, l'app Tauri 2 e un'interfaccia vuota ma funzionante, con le scelte di linguaggi, framework e dati delle Specs.

Il workflow `ci` su GitHub Actions esegue l'impianto di test: cargo test per core, CLI e backend, Vitest con Testing Library per l'interfaccia, tauri-driver con WebdriverIO per gli end-to-end su Linux e Windows.

Il workflow `release` parte solo dopo un `ci` riuscito su `main` e pubblica su GitHub Releases le build, per ora non firmate, con gli aggiornamenti dall'app.
