---
created: 2026-11-02T16:23:14.892Z
updated: 2026-11-02T16:23:14.892Z
summary: I test usano il manifest Windows di Common Controls e lo smoke e2e trova la finestra del progetto per handle WebDriver, non per titolo.
tasks:
- lg83imok
- anfypzf9
cleared:
  anfypzf9:
    title: Finestra progetto nei test e2e
    refs:
    - window
    - requirements/quality#General
    technical: false
    proposal: xt9xk5c1
    label: CI verde
    finished: 2026-11-02T16:10:46.110Z
    body: |-
      Lo smoke test end-to-end (`e2e/specs/smoke.e2e.mjs`) fallisce su Linux nella CI (`ci`): lo stage `opens the fixture project in its own window and closes the welcome window` scade dopo 30 s con "no window titled weaver-fixture opened". L'app è corretta; il difetto è nel test.

      ## Causa

      `switchToWindowTitled` (`e2e/specs/smoke.e2e.mjs`, circa righe 52-66, confronto a riga 58, chiamata intorno a riga 116) cerca la finestra del progetto confrontando il titolo WebDriver con `FIXTURE_FOLDER` (riga 29). Ma il titolo WebDriver è il `document.title` della pagina, che vale `Weaver` in ogni finestra (`ui/index.html:6`). Il nome della cartella è solo il titolo nativo della finestra, impostato da `create_project_window` con `.title(title)` (`src-tauri/src/windows/tauri_shell.rs:52-65`), con valore da `folder_name(canonical)` (`src-tauri/src/windows/manager.rs:330,339`). Inoltre, mentre la finestra di benvenuto viene distrutta (`manager.rs:379` → `close_welcome` in `tauri_shell.rs:84-88`, `destroy()`), il passaggio al suo handle può dare un `no such window` transitorio. L'ultimo stage, `closes the windows and the app quits without error`, passa a vuoto quando nessuna finestra progetto è stata trovata.

      ## Cosa deve essere vero alla fine

      - Il test cattura l'handle WebDriver della finestra di benvenuto nel primo stage.
      - La finestra del progetto si trova per handle: è quella il cui handle non è della finestra di benvenuto; mai per titolo WebDriver.
      - In quella finestra il titolo nativo, letto tramite Tauri (ad esempio `window.__TAURI_INTERNALS__.invoke` del comando `title` della finestra), è uguale al nome della cartella fixture. Verificare che la capability della finestra progetto consenta quel comando; se non lo consente, va abilitato solo il permesso necessario.
      - Il `no such window` transitorio mentre la finestra di benvenuto viene distrutta è tollerato (riprovare la ricerca), senza nascondere altri errori.
      - Lo stage di chiusura fallisce se nessuna finestra progetto è stata trovata; verifica che chiudere l'ultima finestra progetto riporti la finestra di benvenuto, e che chiudere la finestra di benvenuto faccia uscire l'app senza errori.
      - Nessuna modifica all'app e nessuna modifica a `document.title`. Gli helper stanno in `e2e/lib.mjs`, la configurazione in `e2e/wdio.conf.mjs`; tenere conto anche di `secondLaunch` (circa riga 69) se usa la stessa ricerca.

      ## Verifica

      Lo step e2e passa nel workflow `ci` su Linux (xvfb + WebKitWebDriver) e su Windows (msedgedriver), con ogni stage di `smoke.e2e.mjs` eseguito e nessuno saltato. Il lavoro è completo solo quando `ci` passa su macOS, Windows e Linux, compresi eventuali altri fallimenti emersi dopo questa correzione.
  lg83imok:
    title: Manifest Windows per i test
    refs:
    - requirements/release#General
    technical: true
    proposal: xt9xk5c1
    label: CI verde
    finished: 2026-11-02T16:10:14.442Z
    body: |-
      Su `windows-latest` il workflow `ci` fallisce già allo step `The ts-rs types are up to date` (`cargo test -p weaver --lib generated_types_are_up_to_date --locked`): l'eseguibile dei test unitari di `weaver_lib` termina con `0xc0000139 STATUS_ENTRYPOINT_NOT_FOUND`. La causa: tauri-build 2.7.4 incorpora il manifest Common Controls v6 solo nel binario dell'app, mentre i binari di test linkano rfd, tao, muda e tauri-plugin-dialog, che importano simboli di comctl32 v6 (per esempio `TaskDialogIndirect`). Riferimenti upstream: tauri#13419, tauri#14580; `examples/api/src-tauri/build.rs` di Tauri usa lo stesso approccio descritto qui.

      ## Cosa deve essere vero

      - Un unico application manifest è tenuto nel repository dentro `src-tauri` (per esempio `src-tauri/windows-app-manifest.xml`). Il contenuto è quello predefinito di tauri-build: solo la dipendenza da `Microsoft.Windows.Common-Controls` versione 6.0.0.0 (processorArchitecture `*`, publicKeyToken `6595b64144ccf1df`, language `*`).
      - `src-tauri/build.rs` (`fn main()`, oggi `tauri_build::Attributes` … `try_build`) passa il manifest al linker MSVC con `cargo:rustc-link-arg=/MANIFEST:EMBED` e `cargo:rustc-link-arg=/MANIFESTINPUT:<percorso assoluto del manifest>`, solo quando il target è Windows MSVC. Il target va letto dalle variabili `CARGO_CFG_TARGET_OS` (`windows`) e `CARGO_CFG_TARGET_ENV` (`msvc`), non con `#[cfg(windows)]`, che riguarda l'host dello script di build. Usare `rustc-link-arg` e non `rustc-link-arg-tests`: quest'ultimo copre solo i test di integrazione in `tests/`, non il binario dei test unitari della lib. Aggiungere `cargo:rerun-if-changed` per il file del manifest.
      - Il manifest proprio di tauri-build è disattivato con `tauri_build::WindowsAttributes::new_without_app_manifest()` passato agli `Attributes`, così l'app porta un solo manifest.
      - In questo modo ogni eseguibile del pacchetto `weaver` (layout in `src-tauri/Cargo.toml`: `[lib] weaver_lib`, `[[bin]] weaver`), app e binari di test, incorpora lo stesso manifest.

      ## Altri errori solo Windows

      Gli step Windows successivi non sono mai stati eseguiti in CI: `cargo test --workspace --locked`, Vitest, `pnpm tauri build --no-bundle --ci`, lo smoke di `weaver-cli` (`scripts/smoke-weaver-cli.mjs`), i test di contratto della CLI e gli e2e con msedgedriver uguale alla versione di Edge installata. Anche ogni altro errore solo Windows che emerga in questi step rientra in questo Task. I punti più a rischio sono i percorsi di codice solo Windows poco coperti: i Job Object di process_group, l'implementazione Windows di `crates/cli/src/interrupts.rs`, `CREATE_NEW_CONSOLE` in `terminal.rs`. La working copy contiene una rinomina trama→weaver non ancora committata (`weaver-cli`, `BUILT_WEAVER_CLI`): la correzione vale anche lì.

      Sono fuori da questo Task, perché corretti altrove: la ricerca della finestra di progetto per titolo negli e2e e l'ordine delle risposte al ricaricamento della sessione (`session/load`).

      ## Verifica

      Il Task è finito quando il job `build-test (windows-x64)` del workflow `ci` supera tutti i suoi step e i job macOS e Linux restano verdi. Il binario dell'app deve contenere esattamente un manifest: si controlla per esempio con `mt.exe -inputresource:weaver.exe;#1 -out:con`, oppure verificando che il link non segnali risorse manifest duplicate. Come primo controllo locale o in CI: `cargo test -p weaver --lib generated_types_are_up_to_date --locked` deve passare su Windows.
---
# Correzioni CI su Windows e Linux

`src-tauri/build.rs` incorpora un solo manifest nei binari del pacchetto per i target Windows MSVC. Lo smoke e2e cattura l'handle della finestra di benvenuto, cerca la finestra del progetto per handle e legge il titolo nativo tramite Tauri.
