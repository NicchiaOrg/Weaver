---
technical: false
refs:
  - file-tab
  - developer.OPEN_FILE
  - developer.CLOSE_TAB
  - developer.EDIT_FILE
  - requirements/ui#file-tab
  - specs#frameworks
---

# Schede ed editor CodeMirror

Le schede della vista File con l'editor CodeMirror 6, secondo la macchina `file-tab` (stati Closed, Open con Saved, Modified, Locked, Deleted, Read-only, Binary). Lettura, salvataggio ed eventi del disco vengono dal backend delle schede.

## Aprire e chiudere

- `developer.OPEN_FILE`: un file già aperto mostra solo la sua scheda (al massimo una scheda per file). Al massimo 10 schede: all'undicesima si chiude la scheda senza modifiche non salvate mostrata meno di recente e il nuovo file si apre al suo posto; se tutte hanno modifiche, nulla si apre e appare il toast di avviso `Every tab has unsaved changes: save or close one to open another file.`
- All'apertura: binario → pannello `Binary file` con `Open in editor`, senza testo; oltre 2 MB → riga `This file is larger than 2 MB: read-only, without colours.`, sola lettura, senza colori; in `.weaver/` → riga `Files in .weaver are written by Weaver: read-only here.`; un file non binario né di sola lettura che l'agente sta già modificando si apre Locked; altrimenti Saved, con i colori del suo tipo o testo semplice se il tipo è sconosciuto.
- `developer.CLOSE_TAB`: con modifiche non salvate, dialogo `Save changes to <name>?` con `Save`, `Discard` (rosso) e `Cancel`, `Cancel` col focus iniziale; un salvataggio fallito o rifiutato lascia la scheda aperta. La scheda si chiude anche lasciando la vista File (salvo un Save all fallito), eliminando il file o una sua cartella (le modifiche si perdono) e alla chiusura della finestra; ogni chiusura emette `file-tab.CLOSED` e libera subito editor, testo e osservazione.
- `developer.EDIT_FILE`: Saved → Modified; in Deleted resta Deleted. Nessun lavoro su disco mentre si scrive.

## Barra delle schede e scorciatoie

- Barra sopra l'editor, una `tab` ARIA in stile shadcn per file in ordine di apertura: nome del file (percorso nel tooltip, monospace), un punto con nome accessibile `Unsaved changes` se ci sono modifiche, `Lock` mentre il blocco è tenuto, `TriangleAlert` mentre una barra attende, e `X` solo icona (`Close`). Sinistra/Destra si spostano tra le schede.
- Solo con la vista File in primo piano: `Cmd+S`/`Ctrl+S` salva la scheda attiva (nulla se non modificabile); `Cmd+W`/`Ctrl+W` chiude la scheda attiva invece della finestra; `Ctrl+Tab` e `Ctrl+Shift+Tab` mostrano la scheda successiva e precedente. Non c'è un livello di scorciatoie globale: oggi `Cmd+W` è Close Window nel menu macOS di `src-tauri/src/windows/menu.rs:build`, che non ha la voce Save.
- Barra strumenti: percorso relativo al progetto (monospace) e `Open in editor`, che apre il file alla riga del cursore (`ui/src/components/open-in-editor.tsx`, comando `open_in_editor` con `line`).

## Editor

- CodeMirror 6 (nessun `@codemirror/*` ancora in `ui/package.json`), font monospace, tema chiaro o scuro risolto da Weaver che cambia subito con esso (`ui/src/components/theme-provider.tsx:useResolvedTheme`); colori della sintassi conformi al contrasto di `requirements/quality#General` in entrambi i temi. Numeri di riga, annulla/ripeti, cerca e sostituisci (`Cmd+F`/`Ctrl+F`), parentesi corrispondenti, piegatura, più cursori; niente completamento, diagnostica o language server.
- `@codemirror/language-data` sceglie il linguaggio per nome del file, poi per estensione; ogni linguaggio si carica con un import dinamico al primo file del suo tipo (precedente: `ui/src/components/mermaid-block.tsx`). Nessun WASM; la CSP di `src-tauri/tauri.conf.json` resta com'è.

## Barre e dialoghi

- Barre `Alert` in cima all'editor, finché risolte: Modified con file cambiato su disco → `This file changed on disk. Saving will ask what to keep.`; sblocco con modifiche → `The agent changed this file while it had unsaved changes.` con `Take the agent's version` (rosso, rilegge dal disco), `Keep mine` (sovrascrive) e `Cancel` (toglie la barra, tiene le modifiche; il prossimo salvataggio passa per il controllo); eliminato → `The agent deleted this file.` dopo un blocco, `This file was deleted on disk.` altrimenti, con `Save again` se la scheda è modificabile. Un Deleted che ricompare torna Saved senza modifiche, altrimenti Modified con la barra del cambiamento.
- Salvataggio che trova il file cambiato: `AlertDialog` `This file changed on disk since it was opened.` con `Reload from disk` (rosso), `Overwrite` (rosso) e `Cancel` col focus iniziale.
- Scrittura fallita: toast di errore `Could not save <name>.` col motivo come descrizione; la scheda tiene il testo.
- Locked: testo non modificabile, le modifiche non salvate restano; allo sblocco si rilegge (anche al nuovo percorso se omp ha detto dove l'agente ha spostato il file) o appare la barra dello sblocco. Read-only si rilegge ai cambiamenti e mostra la barra di eliminazione senza poter salvare. Binary dice se il file è stato eliminato e smette quando ricompare.
- Precedente di rilettura nell'interfaccia: `ui/src/components/requests/request-pane.tsx`.

## Test

In Vitest: schede, barre, dialoghi e scorciatoie (`Cmd/Ctrl+S`, `Cmd/Ctrl+W`, `Ctrl+Tab`) con i percorsi da tastiera e i nomi accessibili.
