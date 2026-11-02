---
technical: false
refs: ["window", "requirements/quality#General"]
---

# Finestra progetto nei test e2e

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
