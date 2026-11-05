---
technical: false
refs:
  - assets
  - developer.OPEN_ASSETS_VIEW
  - developer.LEAVE_ASSETS_VIEW
  - developer.DROP_ASSETS
  - developer.RENAME_ASSET
  - developer.DELETE_ASSETS
  - developer.DRAG_ASSETS_OUT
  - workspace.ASSETS_CHANGED
  - workspace.FILES_CHANGED
  - project/Empty Model/workspace.FILES_CHANGED
  - project/Has a Model/workspace.FILES_CHANGED
  - requirements/ui#asset-view
  - requirements/ui#General
  - requirements/rules#asset-view
  - requirements/tech#asset-view
  - requirements/tech#project
  - requirements/data#General
  - requirements/docs#General
  - specs#frameworks
---
# Vista Assets

Quando il Task è finito, ogni finestra di progetto ha nella barra laterale la voce `Assets`, dopo Files, con l'icona `Paperclip`, e la vista Assets (Machine `asset-view`) si comporta come dice il Model.

**Cartella.** Gli asset sono i file regolari direttamente in `.weaver/assets/` il cui nome non inizia con `.`; la cartella è piatta, senza sottocartelle, tracciata da git (nessun `.gitignore`, non va tra le cartelle solo locali) e nasce solo quando entra il primo asset, mai all'inizializzazione. Nella vista Files `.weaver/assets/` resta in sola lettura come il resto di `.weaver/`.

**Elenco degli asset (backend).** Per ogni progetto aperto il backend tiene in memoria nome e dimensione di ogni asset, letti con un solo `read_dir` e i metadati che restituisce, senza mai aprire un file; li rilegge all'apertura del progetto e dopo ogni gruppo di eventi del watcher del progetto sotto `.weaver/assets/` (150 ms dopo l'ultimo), e manda a ogni finestra del progetto l'evento `assets-changed` con id del progetto, numero di generazione ed elenco intero; un comando restituisce l'elenco corrente. Questo è `workspace.ASSETS_CHANGED`, e vale come le voci "Asset list" e "Watching" di `requirements/tech#assets`. Gli eventi sotto `.weaver/assets/` non raggiungono mai il quadro del progetto, la sua ricostruzione, `project-changed` né `workspace.FILES_CHANGED` (oggi costano una ricostruzione dei dati derivati: `crates/core/src/project/image.rs` `refresh`, `src-tauri/src/windows/watch.rs`). Lo stesso elenco serve anche ai tag `@` di un altro Task, quindi va esposto in una forma riusabile.

**Comportamento della vista**, come nel Model:
- All'apertura mostra gli asset come sono ora, ognuno con l'icona del suo tipo, e segna quelli oltre il limite di dimensione; niente è selezionato; senza asset dice come aggiungerne. Mentre è aperta si aggiorna da sola a ogni `workspace.ASSETS_CHANGED`, segni compresi; un asset sparito esce dalla selezione. Scegliendo un'altra vista la selezione si dimentica; una copia in corso va avanti fino alla fine.
- Trascinare file dal Finder o da Esplora risorse sulla vista li copia in `.weaver/assets/` (creata prima se manca); gli originali restano dove sono. Durante la copia lo sviluppatore vede che è in corso e può usare la vista; lasciare la vista o chiudere la finestra non la ferma. Alla fine gli asset copiati sono selezionati. Ogni file copiato oltre il limite fa comparire un avviso che lo nomina e resta segnato. La caduta è rifiutata per intero, con il motivo e senza copiare niente, quando contiene una cartella o un collegamento simbolico, un nome non valido, un nome già usato da un asset, due file con lo stesso nome, o quando un'altra copia della stessa finestra è ancora in corso. Un file da un'altra parte del progetto si può copiare; una caduta fatta solo di asset del progetto stesso, trascinati di nuovo sulla propria vista, non fa nulla.
- Rinominare un asset ne tiene il contenuto sotto il nuovo nome e lo lascia selezionato; rifiutato, con il motivo, quando il nome non è valido o un altro asset ha già quel nome (un cambio di sole maiuscole è permesso).
- Cancellare, dopo conferma, manda ogni asset selezionato nel cestino di sistema; quelli che il cestino rifiuta restano e lo sviluppatore sa quanti e perché.
- Trascinare asset fuori da Weaver verso un'altra app (Finder, Esplora risorse, editor, browser) dà a quell'app delle copie: quello trascinato, o tutti i selezionati se è tra questi; gli asset restano come sono, qualunque cosa chieda l'app. Il doppio clic non fa nulla; nessuna anteprima.

**Regole** (`requirements/rules#asset-view`): limite di dimensione oltre 10 MB (10 × 1024 × 1024 byte), solo avviso e segno, mai un rifiuto; nomi con le regole della vista Files più il rifiuto dei nomi con il punto iniziale; nomi confrontati come li confronta il file system.

**Interfaccia** (`requirements/ui#asset-view`, `requirements/ui#General`): griglia di caselle in ordine di nome, icone lucide per estensione secondo la tabella, segno ambra `TriangleAlert` sugli asset oltre il limite, tooltip con nome e dimensione, selezione multipla (Cmd/Ctrl+clic, Shift+clic, frecce, Cmd/Ctrl+A), menu contestuale con solo `Rename` e `Delete`, `F2` e `Delete`, conferma `Move to Trash`, toast e testi esattamente come scritti lì; la barra laterale e l'elenco delle viste con intestazione includono Assets.

**Tecnica** (`requirements/tech#asset-view`, `specs#frameworks`): comandi che prendono nomi di asset, mai percorsi; ricezione delle cadute con `onDragDropEvent` solo mentre la vista è mostrata, come la vista Files (`ui/src/lib/backend.ts`) e l'editor delle richieste; copia, rinomina (stessa primitiva senza sostituzione di `crates/core/src/file_view/rename.rs`) e cestino (`trash`, come `crates/core/src/file_view/ops.rs`) in `weaver-core`, esposti da `src-tauri` in `spawn_blocking`; le operazioni di `crates/core/src/file_view/` rifiutano `.weaver/` e non vanno allentate: servono operazioni proprie degli asset, riusando le parti comuni (`names.rs`, schema tutto-o-niente di `copy_in.rs`). Il trascinamento verso altre app usa il crate `drag` di CrabNebula (2.x) chiamato dal backend con `DragMode::Copy`, mai `tauri-plugin-drag`; va aggiunto agli Specs come nuova dipendenza.

**Test** come chiede `requirements/quality#assets` per la vista (cartelle temporanee per elenco, copia e rifiuti, rinomina, cestino; Vitest per griglia, selezione, menu, tastiera, segno, testo vuoto).

**Documentazione** (`requirements/docs#General`): una riga sotto `## [Unreleased]` in `CHANGELOG.md` e, nel README (`### Where Weaver keeps files`), la cartella `.weaver/assets/` tracciata da git e cosa fa la vista Assets.
