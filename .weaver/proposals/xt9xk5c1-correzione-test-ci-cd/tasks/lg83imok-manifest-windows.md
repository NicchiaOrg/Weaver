---
technical: true
refs: ["requirements/release#General"]
---

# Manifest Windows per i test

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
