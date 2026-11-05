---
technical: true
refs:
  - specs#ci
  - specs#testing
---
# CI e test

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
