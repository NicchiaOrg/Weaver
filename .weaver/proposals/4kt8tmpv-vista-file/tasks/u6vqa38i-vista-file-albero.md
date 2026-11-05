---
technical: false
refs:
  - file-view
  - developer.OPEN_FILE_VIEW
  - developer.LEAVE_FILE_VIEW
  - developer.EXPAND_FOLDER
  - developer.COLLAPSE_FOLDER
  - developer.CREATE_ENTRY
  - developer.RENAME_ENTRY
  - developer.MOVE_ENTRY
  - developer.DELETE_ENTRY
  - workspace.CODE_FOLDER_CHANGED
  - time.FILE_VIEW_RELEASE_DUE
  - requirements/ui#file-view
  - requirements/ui#General
  - requirements/tech#files
  - requirements/tech#file-view
  - requirements/tech#General
  - requirements/tech#project
  - requirements/rules#file-view
  - requirements/quality#file-view
---
# Vista File e albero

Una nuova vista `Files` nella finestra di progetto mostra l'albero dei file del progetto e, a destra, le schede dell'editor (le schede, l'editor e il lock dell'agente sono altri Task; qui servono solo l'area vuota e i punti d'aggancio: aprire un file dall'albero, chiudere o spostare le schede su un percorso, sapere se una scheda ha modifiche non salvate o un lock dell'agente).

## Voce e layout
- Voce `Files` nella sidebar dopo Jobs, icona `FolderTree`; ordine del gruppo in alto: Board, Chat, Jobs, Files, Requirements, Specs, History; il gruppo in basso (Usage, Settings) non cambia. Come le altre viste tranne Settings, mostra Get Started finché il progetto non ha `.weaver/`.
- A sinistra l'albero, a destra schede ed editor; senza schede la destra dice `Open a file from the tree.` in testo muted.
- A differenza delle altre viste, scegliere un'altra voce chiude la vista File (non ne conserva lo stato).

## Stati (backend, per finestra di progetto, solo in memoria)
- **Closed** (iniziale): Weaver non legge nessuna cartella del codice e non osserva nulla del codice.
- `OPEN_FILE_VIEW` da Closed → Open: legge solo nomi e tipi delle voci della radice (mai il contenuto di un file), escludendo `.git` e ciò che git ignora, e osserva solo la radice. Da Releasing → Open: l'albero torna com'era senza rileggere nulla.
- In Open, `EXPAND_FOLDER` legge solo quella cartella (come la radice) e la osserva da sola, senza le sottocartelle; `COLLAPSE_FOLDER` smette di osservare quella cartella e ogni cartella espansa al suo interno, e ne dimentica le voci. Ogni cartella parte chiusa.
- `CODE_FOLDER_CHANGED` (una voce apparsa, sparita o rinominata in una cartella osservata, da chiunque, Weaver incluso): in Open e in Releasing Weaver rilegge quella cartella.
- `LEAVE_FILE_VIEW` (un'altra vista scelta dalla sidebar, da un link o da una notifica): se ci sono modifiche non salvate un `AlertDialog` chiede `Save changes to <n> files?` elencando i percorsi, con `Save all`, `Discard all` (destructive) e `Cancel` (ultimo, focus iniziale); Cancel lascia la vista File e non cambia nulla. Poi ogni scheda si chiude → Releasing. Se con `Save all` un file non si salva, gli altri vengono salvati, l'altra vista non viene mostrata e la vista File resta sulla scheda di quel file, che dice perché (resta Open).
- **Releasing**: vista non mostrata, nessuna scheda; Weaver tiene albero e osservazioni per 10 s, misurati con un clock iniettato. `FILE_VIEW_RELEASE_DUE` → Closed: osservazioni e listing scartati. Anche la chiusura della finestra scarta tutto subito, da Open o Releasing.

## Albero (UI)
- Righe con icona (`Folder`, `FolderOpen`, `File`) e nome, circa 16 px di rientro per livello come l'albero di Import from Code; prima le cartelle poi i file, ciascuno per nome.
- ARIA `tree`: Su/Giù tra righe, Destra espande o va al primo figlio, Sinistra chiude o va al genitore, Enter apre un file in scheda o alterna una cartella; il click fa lo stesso.
- Le voci di `.weaver/` appaiono come le altre ma le azioni che le cambierebbero sono disabilitate (`aria-disabled`, motivo in tooltip e descrizione accessibile) con il motivo `.weaver is written by Weaver: read-only here.`
- Un file aperto in una scheda con lock dell'agente mostra `Lock` con nome accessibile e tooltip `The agent is editing this file` (lo stato del lock arriva dal Task del lock).

## Azioni
- shadcn `ContextMenu` (da aggiungere in `ui/src/components/ui/`) su ogni riga e sullo spazio vuoto dell'albero per la radice, aperto con click destro, tasto Menu o `Shift+F10`: `New file`, `New folder` (solo cartelle e radice), `Rename`, `Move to…`, `Delete`; icone `FilePlus`, `FolderPlus`, `Pencil`, `FolderInput`, `Trash2`.
- `F2` rinomina la riga selezionata; `Delete` (`Cmd+Backspace` su macOS) la elimina.
- `New file`, `New folder`, `Rename` mostrano un campo di testo nella riga col nome selezionato (senza estensione quando si rinomina un file); Enter conferma, Esc o uscire dal campo annulla.
- **CREATE_ENTRY**: file vuoto (`create_new`) o cartella (`create_dir`), mai sostituendo una voce esistente; appare nell'albero.
- **RENAME_ENTRY** e **MOVE_ENTRY**: un solo `rename` sullo stesso volume che non sostituisce mai una voce (controllo prima, più `renameat2(RENAME_NOREPLACE)` su Linux, `renamex_np(RENAME_EXCL)` su macOS, `MoveFileExW` senza `MOVEFILE_REPLACE_EXISTING` su Windows); lo spostamento tra volumi è rifiutato col motivo. Le schede aperte sulla voce o su file al suo interno seguono il nuovo percorso e tengono le modifiche non salvate. Spostare: trascinando una riga su una cartella o sullo spazio vuoto della radice (cartella sotto il puntatore evidenziata; rilascio dove è rifiutato non fa nulla), oppure con `Move to…`, un `Dialog` con le cartelle del progetto ad albero, lette una cartella alla volta all'espansione, e `Move` / `Cancel`.
- **DELETE_ENTRY**: `AlertDialog` `Move "<name>" to the trash?` più `<n> open files have unsaved changes that will be lost.` quando schede su di essa hanno modifiche non salvate; `Move to Trash` (destructive) e `Cancel`. Va nel cestino di sistema con il crate `trash` (da aggiungere, e in `specs#trash`); se il cestino rifiuta o non c'è, niente viene eliminato. Le schede sulla voce o su file al suo interno si chiudono.
- **Rifiuti** (controllati di nuovo nel backend all'esecuzione; un comando rifiutato non cambia nulla; la UI mostra un toast di errore col motivo, es. `A file or folder named "<name>" already exists here.`, `The agent is editing this file.`, `Could not move "<name>" to the trash: <reason>`):
  - nome non valido: vuoto, `.` o `..`, contiene `/`, `\` o NUL, oppure su Windows contiene `<>:"|?*`, finisce con spazio o punto, o è un nome riservato (`CON`, `NUL`, `COM1`…);
  - nome già presente nella cartella di destinazione, confrontato come fa il file system (senza distinzione di maiuscole su macOS e Windows); un rename che cambia solo le maiuscole è permesso;
  - creare dentro `.weaver/`; rinominare, spostare o eliminare `.weaver/` o qualcosa al suo interno; spostare qualcosa dentro `.weaver/`;
  - spostare una cartella in se stessa o in una sua sottocartella;
  - rinominare, spostare o eliminare un file con lock dell'agente, o una cartella che ne contiene uno.

## Backend
- Listing, lettura, scrittura, creazione, rename, spostamento ed eliminazione vivono in `weaver-core` (I/O `std` sincrono); `src-tauri` li espone come comandi Tauri in `spawn_blocking`. La UI non tocca mai un file e non riceve il plugin `fs`.
- **Percorsi**: relativi alla radice con `/`. Rifiutati: assoluti, parti `..`, qualsiasi percorso dentro `.git`, e percorsi la cui forma canonica (`dunce`, senza maiuscole su Windows) esce dalla radice canonica. Un link simbolico con destinazione fuori dalla radice è elencato ma non si può aprire, scrivere, rinominarci dentro o spostarci dentro.
- **Listing**: una cartella alla volta con `read_dir`, solo nomi e tipi (file, cartella, link simbolico), mai ricorsione, indice o ricerca, mai aprire un file. `.git` escluso per nome; gli ignorati da git esclusi con un `git check-ignore -z --stdin` per cartella con `GIT_OPTIONAL_LOCKS=0`; senza git o fuori da un repository nulla è ignorato. Gli altri nomi col punto sono elencati (diversamente da `folder_tree::list`, che li salta tutti). Cartelle prima, poi file, per nome senza distinzione di maiuscole.
- **Osservazione**: un watcher `notify` per finestra per la vista File, separato da quello del progetto: una watch non ricorsiva per cartella espansa, radice inclusa, e una per file aperto in scheda. Eventi raggruppati per cartella, 150 ms dopo l'ultimo; un gruppo rilegge la cartella. File temporanei delle scritture atomiche ignorati. Nessuno di questi eventi raggiunge la picture, il suo batcher o `workspace.FILES_CHANGED`. Su Linux un `ENOSPC` di inotify passa al poll watcher come per il progetto.
- **Stato**: per finestra, solo in memoria: stato della vista, cartelle espanse e loro voci, schede (percorso, hash letto, fine riga, BOM, motivo di sola lettura) e lock. Niente su disco, nella picture o in `project-changed`. La UI rende ciò che il backend invia e tiene solo i testi degli editor e la loro cronologia di annullamento.
- **Eventi** al UI quando una cartella elencata cambia, quando il file di una scheda cambia, sparisce o ricompare, e quando un lock è preso o rilasciato; ognuno porta l'id di progetto della finestra e un numero di generazione. Tipi generati con `ts-rs`.
- Il watcher del progetto resta com'è: radice non ricorsiva solo per `.weaver`, `.weaver/` ricorsivo, nessun codice; i file del codice non entrano mai nella picture e non causano rebuild.

## Qualità
- A vista chiusa nessuna cartella del codice letta e niente osservato fuori da `.weaver/` oltre alla watch della radice per `.weaver`. Mostrare la vista legge solo la radice; espandere solo quella cartella; espandere avvia al massimo un `git check-ignore`, nient'altro della vista avvia processi.
- 10 s dopo che la vista smette di essere mostrata non resta in memoria nessuna watch o listing del codice; testato con clock iniettato.
- Test Rust su cartelle temporanee con temporizzazione del watcher iniettata: listing (`.git` escluso, ignorati, nomi col punto, senza git), watch aggiunte e tolte su espansione, chiusura cartella, rilascio e chiusura finestra, tutti i rifiuti, rename/move che non sostituiscono mai, eliminazione nel cestino e suo rifiuto. Test Vitest: albero, context menu, `F2`, `Delete`, trascinamento, `Move to…` e dialogo di uscita, con percorsi da tastiera e nomi accessibili; aggiornare `ORDER` in `sidebar.test.tsx` a Board, Chat, Jobs, Files, Requirements, Specs, History, Settings.

## CHANGELOG
Aggiungere sotto `## [Unreleased]` una riga per l'intera vista File (albero, editor in schede con colori della sintassi, lock dell'agente), come chiede `requirements/docs`.

## Da dove partire
- Sidebar e viste: `ui/src/views.ts` (`VIEWS` con `group: "top"`, `ViewId`; la Chat ha id `general`; `USAGE_ENTRY` non è una vista), `ui/src/components/sidebar.tsx`, `ui/src/components/sidebar.test.tsx` (`ORDER`).
- Instradamento viste: `src-tauri/src/windows/manager.rs` (`ShowView`) → `ui/src/bindings/ShowView.ts`; `ui/src/components/project-window.tsx` (`ProjectWindow`: le viste restano montate con `hidden`, `show-requested`; qui va intercettata l'uscita dalla vista File col dialogo, e la vista File va smontata invece di restare nascosta); `ui/src/components/project-content.tsx` (`ProjectShell`) e `ui/src/components/project/project-gate.tsx` (`ProjectGate`); `ui/src/components/view-shown.ts` (`useViewShown`).
- Albero precedente: `crates/core/src/project_files/folder_tree.rs` (`list`, che salta i nomi col punto) → `src-tauri/src/project_setup.rs` (`project_folder`) → `ui/src/lib/backend.ts` (`project_folder`); UI `ui/src/components/project/import-panel.tsx`.
- Watcher: `src-tauri/src/windows/watch.rs` (`NotifyWatch::start`, `run`, `route`, `Batcher`, `WatchGuard`); durata in `src-tauri/src/windows/manager.rs` (`watchers`, `open_reserved`, chiusura, uscita).
- Picture (da non toccare con il codice): `src-tauri/src/pictures.rs`.
- Lavoro su disco: `src-tauri/src/blocking.rs` (`disk`, `disk_named`); file temporanei: `crates/core/src/atomic.rs` (`is_temp_name`).
- Nuovi comandi: `src-tauri/src/commands.rs`, `CommandName`, `generate_handler!` in `lib.rs`, `COMMANDS` in `build.rs`, `capabilities/default.json`, tipi in `bindings.rs`.
- Dialoghi e toast: `ui/src/components/ui/alert-dialog.tsx`, `ui/src/components/dialogs.tsx`, `ui/src/components/ui/sonner.tsx`; `ui/src/components/ui/context-menu.tsx` non esiste ancora.
