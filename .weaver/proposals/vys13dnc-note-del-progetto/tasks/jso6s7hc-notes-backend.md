---
technical: false
refs:
  - requirements/tech#notes-view
  - requirements/rules#notes-view
  - requirements/quality#notes-view
  - developer.ADD_NOTES_IMAGES
  - developer.LEAVE_NOTES_VIEW
---
# Backend delle note

Le note del progetto sono un solo foglio Markdown, `.weaver/notes/notes.md`, con le sue immagini come file normali direttamente accanto, nella stessa cartella `.weaver/notes/`; non ci sono altre pagine né cartelle. La cartella è solo locale (mai versionata, con un `.gitignore` che contiene `*`): il suo posto nel layout di `.weaver/`, il suo `.gitignore` e l'evento `notes-changed` (`workspace.NOTES_CHANGED`) dal watcher del progetto sono in un altro Task; questo Task fornisce le operazioni sul foglio e sulle immagini e i comandi Tauri che la vista Notes usa. Quando è fatto deve valere quanto segue.

**Dove sta il codice.**
- Lettura e scrittura del foglio, copia delle immagini, scelta dei loro nomi, lettura per la visualizzazione e spostamento nel cestino vivono in `weaver-core` (I/O `std` sincrono, funzioni pure dove possibile), in un nuovo modulo delle note accanto a `crates/core/src/assets/`.
- `src-tauri` le espone come comandi Tauri eseguiti in `spawn_blocking`, come fa `src-tauri/src/assets/commands.rs`. La UI non tocca mai un file e non riceve il plugin `fs`.
- I comandi ricevono nomi di immagine, mai percorsi; il backend costruisce il percorso dopo aver controllato il nome e rifiuta ogni nome che contiene un separatore, che inizia con `.` o che è uguale a `notes.md`.
- La cartella `.weaver/notes/`, con il suo `.gitignore`, viene creata (con `ensure_local_folder` di `crates/core/src/project/layout.rs`) dalla prima scrittura del foglio o di un'immagine, mai prima.

**Lettura del foglio.** Un comando restituisce il testo del foglio, oppure nessun testo quando il file manca, insieme allo SHA-256 dei suoi byte (`sha256_bytes` di `crates/core/src/file_tab/read.rs`). Un file CRLF viene letto come LF.

**Scrittura del foglio.**
- Un comando riceve il testo intero e lo scrive in modo atomico con lo scrittore condiviso (`crates/core/src/atomic.rs`), in UTF-8 con LF, e restituisce il nuovo hash. Quando la scrittura fallisce restituisce il motivo, che la vista mostra.
- Per ogni progetto il backend ricorda l'hash dell'ultimo foglio che ha scritto o letto, così che il watcher non mandi come `notes-changed` una scrittura di Weaver stesso.
- Al massimo una scrittura per finestra alla volta; i testi che arrivano nel frattempo si fondono in un'unica scrittura successiva dell'ultimo. Scrivere non blocca mai la digitazione né gli altri comandi della finestra.
- Prima che una finestra di progetto si chiuda o che l'app esca (`window.CLOSED`), il backend chiede alla UI di quella finestra il testo del foglio ancora in attesa, come `requests-flush` fa per l'editor delle richieste (`src-tauri/src/requests.rs`, `src-tauri/src/windows/unsaved.rs`, con lo stesso limite `FLUSH_LIMIT`), con un nuovo evento e la sua risposta, e lo scrive. Nella stessa risposta arrivano le immagini da mettere nel cestino (sotto), che il backend sposta dopo aver scritto il testo.

**Visualizzazione delle immagini.** Un comando riceve un nome e restituisce `data:<mime>;base64,<…>` da una sola lettura del file in `.weaver/notes/`, con il tipo MIME ricavato dalla firma come per le immagini del File view (`crates/core/src/file_tab/read.rs`, `ImageKind` di `crates/core/src/project/requests/image.rs`). Solo un file PNG, JPEG, GIF o WebP dentro `.weaver/notes/` viene letto; un nome che non c'è o che non è un'immagine di quei tipi restituisce un errore, e la vista mostra il segnaposto. Weaver non legge mai un file fuori da quella cartella per il foglio e non scarica mai un indirizzo.

**`developer.ADD_NOTES_IMAGES`.** Lo sviluppatore incolla immagini nel foglio o vi trascina file dal Finder o da Explorer. Ogni immagine viene copiata nella cartella delle note, creata prima se non esiste, con il nome del suo file quando ne ha uno e altrimenti con data e ora, con un numero aggiunto quando quel nome è preso; gli originali restano dove sono. Un'immagine più grande del limite, o di un tipo che il foglio non può mostrare, viene rifiutata con il motivo e non copiata; le altre vengono aggiunte comunque. I file che non sono immagini vengono lasciati fuori, e quando nessuno è un'immagine non succede nulla. In concreto:
- Un comando di copia riceve le immagini incollate, ognuna come byte con il nome del file quando ce l'ha; un comando (o lo stesso) riceve i percorsi assoluti dei file trascinati. La UI li ottiene rispettivamente da `ClipboardEvent.clipboardData.files` e da `getCurrentWebview().onDragDropEvent`.
- Le sorgenti si leggono con `symlink_metadata`, senza mai seguire un link. Sono accettati PNG, JPEG, GIF e WebP riconosciuti dalla firma, al massimo 10 MB ciascuno (10 × 1024 × 1024 byte, compreso). Un file con l'estensione di un tipo di immagine (quelle di `requirements/ui#asset-view`) che non è accettato, o che supera il limite, è rifiutato; una cartella, un link simbolico o qualunque altro file è lasciato fuori senza messaggio. Ogni file è deciso da solo: un rifiuto non ferma gli altri.
- Nome: quello del file stesso quando l'incolla o il trascinamento ne dà uno ed è un nome valido (le regole dei nomi degli asset, `validate_name` in `crates/core/src/assets/mod.rs`); altrimenti `pasted-YYYY-MM-DD-HHMMSS.<ext>`, ora locale, con l'estensione dalla firma (`png`, `jpg`, `gif`, `webp`). Quando il nome è preso, confrontato come lo confronta il file system (`same_name`), si aggiunge `-2`, `-3`… prima dell'estensione.
- Ogni file è creato con `create_new` e non sostituisce mai nulla; gli originali sono copiati, mai spostati né modificati. Una copia non blocca mai la UI né gli altri comandi della finestra.
- Il risultato dice, per ogni file e nell'ordine dato, il nome assegnato, oppure il motivo del rifiuto (oltre 10 MB, tipo non accettato, copia fallita con la sua ragione), oppure che è stato lasciato fuori. La vista inserisce poi le immagini aggiunte, scritte `![](<name>)`, e mostra i rifiuti.

**`developer.LEAVE_NOTES_VIEW`.** Un'altra vista della finestra prende il posto della vista Notes, scelta dallo sviluppatore o da un link o una notifica su cui ha cliccato. Il cambiamento ancora in attesa viene scritto subito; ogni immagine che lo sviluppatore ha tolto dal foglio durante questa visita, e che il foglio non mostra più da nessuna parte, va nel cestino del sistema. Un'immagine che il cestino rifiuta resta, e lo sviluppatore viene informato del motivo. Le immagini che non ha mai tolto restano, anche quando il foglio non le mostra. Lo stesso vale prima che la finestra si chiuda. Per questo:
- Un comando riceve i nomi delle immagini da mettere nel cestino e le sposta una alla volta con il crate `trash`, come `ops::delete` / `system_trash` degli asset (`crates/core/src/assets/ops.rs`, `src-tauri/src/assets/mod.rs`); il risultato elenca i nomi rifiutati dal cestino con i loro motivi.
- Nessun altro file della cartella viene mai cancellato da Weaver, anche quando il foglio non lo nomina.

**L'agente.** Le istruzioni di Weaver, le skill installate e i messaggi all'agente non nominano mai `.weaver/notes/` né i suoi file, e all'agente non vengono mai mandate le note.

**Log.** Mai il testo del foglio, i byte di un'immagine o il suo URL `data:`; nomi e percorsi sono ammessi.

**Test** (`cargo test`):
- In `weaver-core`, come funzioni pure: i nomi delle immagini (nome proprio, nome proprio non valido, data e ora, nomi presi con `-2` e con maiuscole diverse), i tipi accettati per firma, il limite di 10 MB esattamente a 10 MB e un byte sopra, i file rifiutati rispetto a quelli lasciati fuori.
- Contro cartelle temporanee: la cartella e il suo `.gitignore` creati dalla prima scrittura del foglio e da quella di un'immagine, la lettura di un foglio che manca, la scrittura atomica, CRLF letto come LF, copie che non sostituiscono mai, link e cartelle lasciati fuori, nomi rifiutati con un separatore, un `.` iniziale o `notes.md`, il cestino con un rifiuto fra più immagini.
