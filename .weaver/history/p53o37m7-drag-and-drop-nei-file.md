---
created: 2026-11-03T23:35:55.809Z
updated: 2026-11-04T07:02:37.657Z
summary: Nella vista File le righe si spostano trascinandole con i pointer events e i file del Finder o di Explorer rilasciati sull'albero vengono copiati nel progetto, tutto o niente.
tasks:
- 2dhhe7nt
- 7t5j92vb
- v1v89jif
- sxx57v25
cleared:
  2dhhe7nt:
    title: Drag and drop nei File
    refs:
    - files
    - file-view/Open/developer.MOVE_ENTRY
    - file-view/Open/developer.DROP_EXTERNAL_ENTRIES
    - developer.DROP_EXTERNAL_ENTRIES
    - requirements/ui#file-view
    - requirements/tech#files
    - requirements/tech#file-view
    - requirements/rules#file-view
    - requirements/quality#file-view
    - requirements/docs#General
    technical: false
    proposal: 0udrt06o
    label: Files drag and drop
    finished: 2026-11-03T23:32:48.475Z
    body: |-
      ## Il bug e la sua causa

      Nella File view trascinare una riga dell'albero su una cartella non sposta nulla nell'app, anche se il Model lo prevede (`developer.MOVE_ENTRY`, "by dragging it"). L'albero usa l'API HTML5 drag and drop (`draggable`, `dragstart`, `dragover`, `drop`), ma il drag-drop handler nativo di Tauri è attivo in ogni finestra (default) e intercetta il trascinamento: la pagina non riceve mai `dragover`/`drop`. I test Vitest simulano eventi HTML5 in jsdom e passano comunque. `Move to…` funziona e resta com'è.

      ## Cosa deve essere vero alla fine

      ### Trascinamento delle righe (`developer.MOVE_ENTRY`)

      - L'albero non usa più l'API HTML5 drag and drop: niente `draggable`, niente handler `dragstart`/`dragover`/`drop`/`dragleave`. Usa pointer events: `pointerdown` col tasto primario su una riga, `setPointerCapture`, il drag parte solo dopo 4 px di movimento; una pressione che si muove meno resta un click (apre il file o espande la cartella come prima).
      - Durante il drag un'etichetta a posizione fissa con `pointer-events: none`, con icona e nome della riga, segue il puntatore. Il target si trova con `document.elementFromPoint` a ogni `pointermove`: la cartella sotto il puntatore è evidenziata (l'albero stesso per lo spazio vuoto, cioè la root); dove lo spostamento sarebbe rifiutato non si evidenzia nulla (stesse regole di `canDrop`: la cartella stessa o una sua sottocartella, o dentro `.weaver/`).
      - Al `pointerup` su un target evidenziato l'entry si sposta tramite il comando esistente `files_move`; i tab sull'entry o su file al suo interno seguono il nuovo path e tengono le modifiche non salvate; i rifiuti sono quelli di un rename, più cartella stessa/sottocartella e target dentro `.weaver/`, con il toast di errore col motivo. Rilasciare altrove, `Esc`, `pointercancel` o la perdita della capture terminano il drag senza cambiare nulla.
      - **Aiuti al trascinamento**, validi sia per le righe sia per i drop dall'esterno: una cartella chiusa sotto il puntatore per 1 s si espande; col puntatore entro 24 px dal bordo superiore o inferiore dell'albero, l'albero scorre in quella direzione, più veloce più ci si avvicina al bordo.

      ### Handler nativo di Tauri

      - Il drag-drop handler nativo di Tauri resta attivo in ogni finestra (progetto e welcome): mai `disable_drag_drop_handler()`, mai `dragDropEnabled: false` in `tauri.conf.json`. È ciò che riporta i path dei file trascinati, qui e nel request editor; il request editor mantiene il suo listener (`onFilesDropped`), che agisce solo mentre è mostrato.

      ### Drop dall'esterno (`developer.DROP_EXTERNAL_ENTRIES`)

      Lo sviluppatore trascina file o cartelle dal Finder o da Explorer e li rilascia su una cartella dell'albero, o sul suo spazio vuoto per la root del progetto.

      - La File view ascolta `getCurrentWebview().onDragDropEvent` solo mentre è mostrata. `enter` e `over` danno il puntatore in pixel fisici: diviso per `devicePixelRatio` e testato con `document.elementFromPoint` contro l'albero, evidenzia la cartella sotto il puntatore (o l'albero per la root) come il drag di una riga; valgono gli stessi aiuti (espansione dopo 1 s, scroll ai bordi). `leave`, o un drop fuori dall'albero, toglie l'evidenziazione; un drop fuori dall'albero non fa nulla nella File view. `drop` dentro l'albero invia i path e la cartella target al comando di copia.
      - File e cartelle vengono **copiati** nella cartella target, le cartelle con tutto il contenuto, i link simbolici come link; gli originali restano dove sono.
      - Durante la copia un toast di caricamento dice `Copying <n> items…` (`Copying 1 item…`) fino alla fine, senza annullamento; il resto della view resta usabile. Lasciare la view o chiudere la finestra non ferma la copia, che va fino in fondo.
      - A copia riuscita il toast si chiude e, se la view è ancora mostrata, la cartella target si espande se era chiusa e la prima entry copiata, nell'ordine dell'albero, è selezionata e portata in vista.
      - **Tutto o niente:** il drop è rifiutato per intero, col motivo e senza copiare nulla, quando un'altra copia rilasciata nella stessa finestra è ancora in corso, quando una qualunque entry rilasciata sta già dentro il progetto (path canonico dentro la root canonica), quando il suo nome non è valido, quando nella cartella target esiste già un'entry con quel nome (confronto come il file system: case-insensitive su macOS e Windows), quando due entry rilasciate hanno lo stesso nome (stesso confronto), o quando la cartella target sta dentro `.weaver/`. I rifiuti usano il toast di errore delle azioni rifiutate, per esempio `"<name>" is already in the project: drag it inside the tree to move it.`, `Two dropped items are both named "<name>".`, `Another copy is still running in this window.`, `A file or folder named "<name>" already exists here.`
      - **Fallimento a metà:** ciò che questo drop aveva già copiato viene rimosso, il progetto torna com'era, e se la finestra è ancora aperta il toast diventa l'errore `Could not copy "<name>": <reason>. Nothing was copied.`

      ### Backend della copia (`requirements/tech#files`, `requirements/tech#file-view`)

      - La copia vive in `weaver-core` (I/O `std` sincrono); `src-tauri` la espone come un nuovo comando Tauri, eseguito in `spawn_blocking`, registrato in `lib.rs` e con il permesso in `build.rs` come gli altri `files_*`. La UI non tocca file e non riceve il plugin `fs`.
      - **Eccezione ai path:** gli unici path assoluti accettati sono le sorgenti di un drop dall'esterno, così come Tauri le riporta; la loro forma canonica (`dunce`, case-insensitive su Windows) deve stare fuori dalla root canonica del progetto. La cartella target segue le regole normali (relativa alla root, separatori `/`, niente `..`, niente `.git`, dentro la root, non dentro `.weaver/`, non attraverso un link che esce dalla root).
      - Una finestra ha al più una copia in corso; il comando rifiuta la seconda col motivo. Ogni rifiuto di `requirements/rules#file-view` è controllato per ogni entry rilasciata prima di scrivere qualsiasi cosa, e ricontrollato nel backend all'esecuzione; un comando rifiutato non cambia nulla.
      - Le entry si leggono con `symlink_metadata`, senza mai seguire un link: un file si crea con `create_new` e se ne copiano i byte mantenendo i permessi; una cartella con `create_dir`, poi le sue entry una a una, in profondità; un link simbolico si ricrea con lo stesso target (`std::os::unix::fs::symlink`; su Windows `symlink_file` o `symlink_dir`, il cui rifiuto, ad esempio per mancanza di privilegi, fa fallire la copia). Non si sostituisce mai nulla.
      - Il comando registra ogni entry di primo livello creata; se un passo fallisce le rimuove definitivamente (non nel cestino: sono copie del drop) e restituisce il nome e il motivo del fallimento. La copia va fino in fondo anche se la view non è più mostrata o la finestra si chiude; il risultato allora non va da nessuna parte, e la protezione "una copia per finestra" sparisce con la finestra. I watch delle cartelle riportano le nuove entry come per ogni cambiamento esterno.
      - **Prestazioni e affidabilità:** leggere per intero una cartella rilasciata, per copiarla, è l'unica eccezione al divieto di letture ricorsive; la copia non blocca mai la UI né gli altri comandi della view; non sostituisce mai un'entry, non cambia mai gli originali, non lascia mai una parte del drop nel progetto.

      ### Test (`requirements/quality#file-view`)

      - Rust, su cartelle temporanee: copia di file, cartelle annidate, link simbolici mantenuti come link, permessi; ogni rifiuto (già nel progetto, nome preso incluso il solo cambio di maiuscole, due entry rilasciate con lo stesso nome, `.weaver/`, seconda copia mentre una è in corso) e la rimozione di quanto copiato dopo un fallimento iniettato a metà.
      - Un test Rust verifica che le finestre progetto e welcome siano costruite con il drag-drop handler di Tauri attivo e che `tauri.conf.json` non lo disattivi.
      - Vitest: i test di drag HTML5 in `files-view.test.tsx` sono sostituiti; nessun test dell'albero lancia eventi HTML5 di drag. Trascinamento righe con pointer events (soglia, evidenziazione, target rifiutati, `Esc`, `pointercancel`, espansione dopo 1 s con fake timers, scroll ai bordi); drop dall'esterno emettendo gli eventi drag-drop di Tauri (`enter`, `over`, `drop`, `leave`) tramite un webview mockato, con il toast di avanzamento, i toast di rifiuto e di fallimento, e l'espansione e selezione dopo il successo.

      ### CHANGELOG (`requirements/docs#General`)

      Sotto `## [Unreleased]` di `CHANGELOG.md`, oggi vuoto (la File view è uscita nella 0.1.0), aggiungere i gruppi che servono: una riga in `### Added` per i file e le cartelle trascinati dal Finder o da Explorer sull'albero di Files, copiati nel progetto; una riga in `### Fixed` per lo spostamento trascinando le righe nell'albero di Files, che ora funziona nell'app.

      ## Punti del codice

      - Albero: `ui/src/components/files/file-tree.tsx` (stato `dragging`/`dropTarget` ~152, `dropFolderOf`/`onDragOver`/`onDrop` ~298-328, handler sul `role=tree` ~341-344, riga ~366 e `draggable`/`onDragStart` ~478-490).
      - `canDrop`: `ui/src/components/files/tree-model.ts:127`.
      - `useFileTree.move` → `files_move`: `ui/src/components/files/use-file-tree.ts:176`.
      - `onFilesDropped` (`onDragDropEvent`, solo drop): `ui/src/lib/backend.ts:371`; usato dal request editor in `ui/src/components/requests/request-pane.tsx:302`.
      - Comando `files_move`: `src-tauri/src/file_view/commands.rs:130`, registrato in `src-tauri/src/lib.rs:216`, permesso in `src-tauri/build.rs:94`; servizio `move_into` in `src-tauri/src/file_view/mod.rs:375`; core in `crates/core/src/file_view/ops.rs:119` e `:185`.
      - Builder delle finestre: `src-tauri/src/windows/tauri_shell.rs:93` (progetto) e `:110` (welcome); `src-tauri/tauri.conf.json` (`"windows": []`).
      - Test: `ui/src/components/files/files-view.test.tsx` (~499-608, drag HTML5 da sostituire), `ui/src/components/files/tree-model.test.ts`.
      - `CHANGELOG.md`: `## [Unreleased]` riga 7, ancora senza gruppi; le sezioni `### Added` e `### Fixed` sotto sono della 0.1.0 e non si toccano.
  7t5j92vb:
    title: 'Fix the code: Copia: rimozione fallita lascia resti'
    refs:
    - requirements/quality#file-view
    - requirements/rules#file-view
    - file-view/Open/developer.DROP_EXTERNAL_ENTRIES
    technical: false
    finished: 2026-11-04T05:48:49.115Z
    body: |-
      ## What happens

      Quando una copia da un drop dall'esterno fallisce a metà e anche la rimozione di ciò che era già stato copiato fallisce (ad esempio per permessi), nel progetto resta una parte del drop. Il toast non dice più `Nothing was copied.` ma elenca le entry rimaste con il motivo. Questo ramo non ha test.

      ## What should happen

      La copia è tutto o niente: non lascia mai una parte del drop nel progetto, e un fallimento a metà rimuove tutto ciò che il drop aveva copiato. Il Model e i Requirement non prevedono il caso in cui la rimozione stessa fallisca, quindi non dicono cosa mostrare né come evitare i resti.

      ## Chosen option

      ### Copia in cartella di staging

      Il drop viene copiato in una cartella temporanea dentro il progetto e poi spostato con rename atomici nella destinazione. Un fallimento a metà tocca solo lo staging; resta il rischio che la pulizia dello staging fallisca, ma l'albero visibile non contiene mai parti del drop. Costa più I/O e una cartella temporanea.

      ## Where in the code

      `crates/core/src/file_view/copy_in.rs`: `CopyInError::Failed` (campo `not_removed`) e il ramo di rimozione dopo il fallimento; messaggio nel toast della File view.
  v1v89jif:
    title: 'Fix the code: Copia: cartella di staging visibile nell''albero'
    refs:
    - requirements/tech#file-view
    - requirements/quality#file-view
    - requirements/rules#file-view
    technical: true
    finished: 2026-11-04T05:58:02.804Z
    body: |-
      ## What happens

      La copia di un drop dall'esterno ora scrive prima in una cartella temporanea (`.copy-in.<random>.tmp`) dentro la cartella di destinazione e poi sposta le entry con rename. Il watcher la ignora, ma l'albero la elenca: il listing mostra ogni nome che inizia con un punto, quindi finché dura la copia chi espande la cartella vede la cartella di staging. Se la pulizia dello staging (o delle entry già spostate) fallisce, la cartella con parte del drop resta nel progetto e l'errore la elenca invece di dire `Nothing was copied.`. Il passo di rinomina in blocco non è nemmeno quello descritto per la copia.

      ## What should happen

      I Requirement dicono che una copia non lascia mai parte di un drop nel progetto, che i nomi con il punto sono tutti elencati, e descrivono la copia diretta con registrazione delle entry di primo livello create. Non dicono nulla su una cartella di staging: né se può comparire nel listing, né come si chiama, né cosa mostrare quando la sua rimozione fallisce. La scelta di staging fatta dallo sviluppatore non si concilia con queste regole senza una decisione sui Requirement.

      ## Chosen option

      ### Staging in una cartella nascosta di Weaver

      Lo staging va in una cartella già ignorata (ad esempio sotto `.weaver/`), non elencata e non osservata. Costa un nuovo uso di `.weaver/` che oggi è scritta solo dalle decisioni, e un rename che può attraversare volumi se `.weaver/` è un mount.

      ## Where in the code

      `crates/core/src/file_view/copy_in.rs` (`copy_in_with`, staging e ramo di rimozione); `crates/core/src/file_view/listing.rs` (`list`, nomi con il punto elencati)
  sxx57v25:
    title: 'Fix the code: Staging copia in .weaver'
    refs:
    - requirements/rules#file-view
    - requirements/quality#file-view
    - requirements/tech#file-view
    technical: true
    finished: 2026-11-04T06:33:10.963Z
    body: |-
      ## What happens

      La copia di un drop dall'esterno scrive lo staging in `.weaver/short-numbers/`, la cartella locale che i Requirement descrivono come **cache degli short number**, ricostruibile in qualsiasi momento. Dentro il progetto finiscono quindi i file del drop, che non sono cache. Se la pulizia fallisce, parte del drop resta in quella cartella, nascosta ma ancora su disco. Senza git nulla è ignorato, e chi espande quella cartella durante la copia vede lo staging. I test della copia non guardano più dentro `.weaver`, e che l'ignore tenga lo staging fuori dall'albero non è verificato.

      ## What should happen

      I Requirement dicono che le cartelle locali di `.weaver/` hanno ciascuna il proprio scopo (`short-numbers/` è solo la cache degli short number) e che nient'altro vi è ignorato, che una copia non lascia mai parte di un drop nel progetto, che copiare qualcosa dentro `.weaver/` è rifiutato, e descrivono la copia diretta con registrazione delle entry di primo livello create. Nessuno prevede uno staging dentro `.weaver/` né dice come riportarne la pulizia fallita.

      ## Chosen option

      ### Copia diretta senza staging

      Il drop si copia direttamente nella destinazione, come descrivono i Requirement, registrando le entry di primo livello create. Un fallimento a metà rimuove solo ciò che è stato registrato; se la rimozione fallisce l'errore elenca le entry rimaste. Nessuna cartella nuova; la destinazione mostra per un attimo le entry parziali.

      ## Where in the code

      `crates/core/src/file_view/copy_in.rs` (`make_staging`, `copy_and_place`, ramo di rimozione e helper `tree()` dei test); `.weaver/requirements/data/project-files.md` (cartelle locali)
---
# Drag and drop nei File

Il trascinamento nell'albero della vista File ora funziona nell'app, dove l'handler nativo di Tauri intercettava i drag HTML5.

- **Spostare le righe:** l'albero usa i pointer events (soglia di 4 px, etichetta che segue il puntatore, cartella di destinazione evidenziata, `Esc` o annullamento senza effetti); lo spostamento passa per lo stesso comando di `Move to…`, con i tab che seguono il nuovo path.
- **Aiuti al trascinamento:** una cartella chiusa sotto il puntatore per 1 s si espande e l'albero scorre vicino ai bordi.
- **Drop dall'esterno:** file e cartelle trascinati dal Finder o da Explorer sull'albero (o sulla root) sono copiati nel progetto, con cartelle complete e link simbolici come link; gli originali restano dove sono. Un toast indica la copia in corso; a fine copia la cartella si espande e la prima entry copiata è selezionata.
- **Tutto o niente:** il drop è rifiutato per intero, col motivo, se un'entry è già nel progetto, ha un nome non valido o già preso, se due entry hanno lo stesso nome, se la destinazione è in `.weaver/` o se un'altra copia è in corso; un errore a metà rimuove quanto già copiato.
- **Handler nativo:** il drag-drop handler di Tauri resta attivo in ogni finestra, come serve anche al request editor.
