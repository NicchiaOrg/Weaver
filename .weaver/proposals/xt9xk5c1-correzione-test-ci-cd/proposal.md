---
status: approved
kind: change
base:
  requirements/quality/main.md: 90a2c9db966a40f178eb8aa0bdaf0d76fe68c15f74b7a3cfb9746e17bea79191
  requirements/release/pipeline.md: 6dc02b461bc93fe673e0972c47b1e99ffafed5b829ea7075c1119cadbe8e0ae8
  requirements/tech/omp.md: fce66d57a0e883f00c1b552d0a8bb70d937d263d4127d4291bf3e87dce1e1f4e
created: 2026-11-02T15:54:17.954Z
request: 2026-11-02T16-48-28
label: CI verde
job: zt29jcks
---
# Correzione test CI/CD

Il workflow `ci` fallisce su Windows e Linux (run 41448904669 su `dev` e 41448848343 su `main`) per tre cause indipendenti; macOS e il job `versions` passano. La Proposal fissa le correzioni nei Requirements e le assegna ai Task, finché `ci` passa su tutte e tre le piattaforme.

## Cause

- **Windows:** il binario dei test unitari di `weaver_lib` non parte (`STATUS_ENTRYPOINT_NOT_FOUND`, 0xc0000139) perché `tauri-build` incorpora il manifest Common Controls v6 solo nell'eseguibile della app; i passi Windows successivi non sono mai stati eseguiti.
- **Linux, test Rust instabili:** dopo `session/load` la risposta può superare gli ultimi aggiornamenti della cronologia ripetuta, che finiscono nella conversazione viva e si fondono con il primo messaggio nuovo dell'agente. È un difetto della app, per i job e per General.
- **Linux, end-to-end:** il test cerca la finestra del progetto tramite il titolo WebDriver, che è `document.title` (sempre `Weaver`), non il titolo nativo della finestra; la app è corretta.

## Decisioni

- Windows: un manifest dell'applicazione tenuto nel repository, incorporato dal linker in ogni eseguibile del pacchetto `weaver` (app e binari dei test); il manifest di `tauri-build` è disattivato.
- Assunzione: il contenuto del manifest è identico al default di `tauri-build` (solo la dipendenza Common Controls v6), quindi la app si comporta come oggi.
- Ricarica della sessione: `session/load` viene inviato con l'invio ordinato già usato per i prompt, così la sua risposta arriva sempre dopo la cronologia ripetuta; vale per i job e per General, con un test di regressione deterministico al livello RPC.
- End-to-end: la finestra del progetto è individuata per handle (quello diverso dalla finestra di benvenuto) e il suo titolo nativo è verificato via Tauri uguale al nome della cartella.
- End-to-end: l'ultimo test fallisce se la finestra del progetto non è stata aperta e verifica che chiuderla riporti la finestra di benvenuto e che la app esca senza errori.
- Il lavoro è finito quando `ci` passa su macOS, Windows e Linux, compreso ogni altro guasto che emerge dopo queste correzioni.
- Le correzioni diventano regole nei Requirements `release`, `tech` e `quality`.
