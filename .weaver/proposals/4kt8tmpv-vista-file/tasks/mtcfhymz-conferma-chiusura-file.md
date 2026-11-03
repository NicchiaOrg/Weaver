---
technical: false
refs:
  - window/Open/developer.CLOSE_WINDOW
  - window/Open/developer.QUIT_APP
  - developer.CLOSE_WINDOW
  - developer.QUIT_APP
  - requirements/tech#window
---
# Conferma chiusura con modifiche

Chiudere la finestra di un progetto o uscire da Weaver deve tenere conto, oltre che dei job al lavoro, delle modifiche non salvate della vista File. Le sezioni `## window` e `## notification` passano senza cambiare testo da `requirements/tech/architecture.md` al nuovo frammento `requirements/tech/windows.md`; cambiano solo i punti della conferma di chiusura.

Quando tutto è fatto:

- La UI di ogni finestra comunica al backend, a ogni cambiamento, se la sua vista File ha modifiche non salvate e in quali file. Lo stato non salvato e il salvataggio (`developer.SAVE_FILE`) sono costruiti dai Task file-view e file-tab di questa Proposal: questo Task li consuma.
- `WindowEvent::CloseRequested` è impedito mentre lavorano job di quel progetto o la sua vista File ha modifiche non salvate; `RunEvent::ExitRequested` mentre lavorano job di qualunque progetto o una qualunque vista File ha modifiche non salvate.
- La conferma resta un dialogo nativo di `tauri-plugin-dialog` (funziona anche con la webview bloccata), unito a quello dei job: dice quanti job lavorano ed elenca i file con modifiche non salvate. Con modifiche non salvate i pulsanti sono `Save all`, `Discard all` e `Cancel` (`MessageDialogButtons::YesNoCancelCustom`, presente in tauri-plugin-dialog 2.8.4); con soli job al lavoro chiede di confermare o annullare, come oggi.
- All'uscita, ogni finestra con modifiche non salvate chiede a turno, e il suo dialogo conta anche i job di tutti i progetti; `Cancel` in uno qualunque ferma l'uscita e non si chiude nient'altro.
- `Save all`: il backend chiede alla UI di quella finestra i testi delle sue schede, come fa `requests-flush` per l'editor delle richieste, e aspetta al massimo lo stesso limite (`FLUSH_LIMIT`); poi li salva come `developer.SAVE_FILE`. Se la UI non risponde in tempo o un file non si può salvare, gli altri file sono salvati, la finestra resta aperta, la UI mostra la scheda di quel file che dice perché, nulla viene sospeso e (all'uscita) Weaver non esce.
- `Discard all` scarta le modifiche senza salvarle.
- Dopo la conferma e il salvataggio i job sono sospesi (omp terminato come in `requirements/tech#job`), poi la finestra si chiude o l'app esce. Chiudendo la finestra il progetto non si riapre al prossimo avvio; uscendo, si riapre.

Punti di partenza: `src-tauri/src/windows/mod.rs` (prevenzione, nel gestore di `RunEvent`/`WindowEvent`, di `ExitRequested`/`CloseRequested`); `src-tauri/src/windows/manager.rs` (`close_requested`, `close_window`, `quit`, `close_prompt`, `quit_prompt`, `ConfirmPrompt`, oggi a due soli pulsanti); `src-tauri/src/windows/tauri_shell.rs` (`confirm` con `OkCancelCustom`, `flush_before_close`); `src-tauri/src/requests.rs:flush_window` (con `FLUSH_LIMIT`, oggi 2 secondi) e `ui/src/components/requests/use-requests.tsx` (risposta `requests_flushed`) come precedente del flush.
