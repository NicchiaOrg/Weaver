---
created: 2026-11-04T08:28:18.498Z
updated: 2026-11-04T08:28:18.498Z
summary: Ogni progetto ha una vista Notes con un foglio Markdown locale e le sue immagini, salvato in .weaver/notes e seguito dal watcher senza toccare il resto del progetto.
tasks:
- d2y0b486
- fd36m87g
- jso6s7hc
- g5yryamx
cleared:
  d2y0b486:
    title: Vista Notes
    refs:
    - notes
    - notes-view
    - requirements/ui#notes-view
    - requirements/ui#General
    - specs#frameworks
    - developer.OPEN_NOTES_VIEW
    - developer.EDIT_NOTES
    - developer.TICK_NOTES_ITEM
    technical: false
    proposal: vys13dnc
    label: Notes
    finished: 2026-11-04T08:25:51.981Z
    body: |-
      Ogni progetto ha le note dello sviluppatore: un solo foglio Markdown, `.weaver/notes/notes.md`, con le immagini accanto, solo locale e mai versionato. La vista Notes di una finestra di progetto lo mostra formattato mentre lo sviluppatore scrive, come Notion: una riga che inizia con Markdown, come `# ` per un titolo o `[ ] ` per una voce di checklist, diventa subito quell'elemento, e il Markdown non si vede mai. Le checklist vengono prima di tutto: una voce si spunta con un clic e resta al suo posto, barrata. Ogni modifica si salva da sola, senza comando di salvataggio. Il file resta Markdown GFM che altri programmi possono cambiare: ciò che il foglio non sa mostrare formattato resta come testo e viene riscritto invariato. Le note sono solo dello sviluppatore: Weaver non le nomina né le manda mai all'agente. I comandi del backend (lettura e scrittura del foglio, copia, lettura e cestino delle immagini, richiesta del testo in attesa alla chiusura) e l'evento `notes-changed` (`workspace.NOTES_CHANGED`) sono in altri Task; questo Task è la vista. Quando è fatto deve valere quanto segue.

      **Voce e struttura (`requirements/ui#General`).**
      - `Notes` è una nuova vista: `ViewId` `"notes"` in `ui/src/views.ts`, voce `Notes` nel gruppo in alto della barra laterale subito dopo Assets, con l'icona lucide `NotebookPen`. L'ordine in alto diventa Board, Chat, Jobs, Files, Assets, Notes, Requirements, Specs, History; il test d'ordine in `ui/src/components/sidebar.test.tsx:54-57` cambia di conseguenza (nove voci in alto, `Notes` con `lucide-notebook-pen`).
      - Come ogni vista tranne Settings, mostra Get Started finché il progetto non ha `.weaver/` (`ui/src/components/project/project-gate.tsx`, `ui/src/components/project-content.tsx`).
      - La vista inizia con la riga d'intestazione comune a tutte le viste, titolo `Notes`.
      - L'icona di un'immagine che non si può mostrare è `ImageOff`.
      - Le regole del Markdown renderizzato (`ui/src/components/markdown.tsx`) non valgono per l'editor delle note, che segue `requirements/ui#notes-view`, titoli e checkbox cliccabili compresi. Fra i componenti del tema, l'editor della vista Notes è TipTap (come CodeMirror lo è per il File view).

      **Editor (`specs#frameworks`: TipTap 3).**
      - TipTap 3 in React (`@tiptap/react`): `@tiptap/starter-kit` senza il suo `Underline` e con `Link` che non apre nulla al clic (`openOnClick: false`, `autolink: true`, `linkOnPaste: true`, protocolli `http` e `https`), `TaskList` e `TaskItem` da `@tiptap/extension-list` con `nested: true`, `@tiptap/extension-image` come blocco, `@tiptap/extension-placeholder`, e `@tiptap/markdown`, che legge il file con `contentType: 'markdown'` e lo scrive con `editor.getMarkdown()`.
      - Questi moduli si caricano con un solo `import()` dinamico la prima volta che la vista Notes si apre in una finestra, come fa `mermaid` in `ui/src/components/mermaid-block.tsx`; il resto della UI non li importa mai.
      - Contenuto non supportato: ogni token di blocco del file che nessuna estensione riconosce (HTML grezzo, tabelle, note a piè di pagina, front matter, marcatori di conflitto…) diventa un nodo di blocco `rawMarkdown` che tiene il suo testo sorgente come testo semplice e modificabile e lo riscrive byte per byte. La sintassi inline sconosciuta resta testo. La prima scrittura può comunque cambiare come sono scritti gli elementi supportati (marcatori di elenco, righe vuote, escape).
      - Elementi supportati, scritti come GFM: paragrafi; titoli di livello 1-3; voci puntate (`-`), numerate (`1.`), di checklist (`- [ ] `, `- [x] `), tutte annidabili per indentazione; grassetto, corsivo, barrato, codice in linea, blocchi di codice delimitati; citazioni; link; divisori (`---`); immagini (`![](<name>)`).

      **Stati (`notes-view`).** La vista Notes di una finestra di progetto e il foglio che mostra. Scegliere un'altra vista conserva quello che lo sviluppatore stava facendo nel foglio, ma Weaver smette di seguire le note finché non torna.
      - *Closed* (iniziale): la vista non è mostrata; Weaver non legge nulla delle note.
      - *Open*: la vista è mostrata; segue da sola le note su disco, e ogni modifica dello sviluppatore viene scritta da sola.

      **`developer.OPEN_NOTES_VIEW`** (Closed → Open). Lo sviluppatore mostra la vista Notes. Weaver legge il foglio com'è su disco e lo mostra formattato, con le sue immagini; senza foglio, il foglio vuoto dice cosa ci si può scrivere. Quando il foglio su disco è ancora quello che la vista mostrava quando lo sviluppatore l'ha lasciata (stesso hash), la vista torna com'era, con il cursore, lo scroll e le modifiche annullabili; altrimenti il contenuto è sostituito da quello su disco. Lo stato è per finestra di progetto, solo in memoria della UI: l'editor con la cronologia di annullamento, cursore e scroll, l'hash del testo mostrato, le immagini tolte e la cache delle immagini; nulla di esso va su disco oltre al foglio e alle immagini.

      **Aspetto.**
      - Intestazione: a destra solo lo stato del salvataggio in testo attenuato: `Saving…` mentre una scrittura è in corso, `Saved` dopo, oppure `Could not save: <reason>.` in `red` con `CircleX` finché la scrittura successiva non riesce. Nulla prima della prima modifica della sessione della finestra.
      - Foglio: il resto della vista scorre come una pagina; il testo sta in una colonna larga al massimo 760 px, centrata, con 48 px sopra, 24 px ai lati e 96 px sotto. Tutta la colonna riceve i clic: un clic sotto l'ultimo blocco mette il cursore alla fine. L'editor è un `textbox` con `aria-multiline` e nome accessibile `Notes`.
      - Vuoto: mentre il foglio non ha testo, la prima riga mostra in testo attenuato `Write notes for this project. Start a line with [ ] for a checklist.`; non fa parte del testo e sparisce col primo carattere.
      - WYSIWYG: i marcatori Markdown non si vedono mai. Titoli: livello 1 a 24 px, livello 2 a 20 px, livello 3 a 16 px, tutti semibold, con spazio sopra; paragrafi alla dimensione base con interlinea 1.6. Codice in linea e blocchi di codice nel font monospazio su sfondo `muted`; i blocchi senza colori di sintassi e con scorrimento orizzontale nel proprio riquadro. Citazioni con una linea sinistra `border` di 3 px; divisori come linea `border` di 1 px.

      **`developer.EDIT_NOTES`** (Open → Open). Lo sviluppatore cambia il foglio: scrive, formatta, incolla testo, sposta o toglie testo e immagini, annulla e ripete. La modifica si vede subito ed è scritta nel foglio un momento dopo l'ultima, creando prima la cartella delle note se non esiste. Quando la scrittura fallisce, lo sviluppatore ne vede il motivo nell'intestazione, il testo resta com'è e la modifica successiva riprova.
      - Salvataggio automatico 500 ms dopo l'ultima modifica, e subito quando l'editor perde il focus, quando la vista viene lasciata o la finestra si chiude; prima che la finestra si chiuda o l'app esca, la vista risponde alla richiesta del backend con il testo in attesa, come `ui/src/components/requests/request-pane.tsx` / `use-requests.tsx` fanno con `setFlusher` e `requests-flush` (stesso `AUTOSAVE_MS = 500`). Al massimo una scrittura alla volta; i testi arrivati nel frattempo si fondono in una scrittura dell'ultimo. Digitare non avvia processi né lavoro su disco prima della pausa e non aspetta mai una scrittura.
      - Scorciatoie Markdown, digitate all'inizio di una riga o di un paragrafo vuoto seguite da uno spazio, trasformano la riga e spariscono: `#`, `##`, `###` un titolo; `-`, `*` o `+` una voce puntata; `1.` una voce numerata; `[]`, `[ ]` una voce di checklist non spuntata e `[x]` una spuntata (anche dopo `-`); `>` una citazione. ```` ``` ```` seguito da Invio o spazio avvia un blocco di codice; `---` seguito da Invio un divisore. In linea, `**testo**`, `*testo*` o `_testo_`, `~~testo~~` e `` `testo` `` diventano grassetto, corsivo, barrato e codice appena si digita il marcatore di chiusura. `Cmd+B`, `Cmd+I`, `Cmd+Shift+S`, `Cmd+E` (`Ctrl` altrove) fanno lo stesso sulla selezione. `Backspace` all'inizio di un titolo, citazione o voce la riporta a paragrafo. Niente menu `/`, niente barra degli strumenti, niente maniglie né trascinamento di blocchi.
      - Elenchi: Invio in una voce crea sotto una voce dello stesso tipo, non spuntata per una checklist; Invio in una voce vuota chiude l'elenco. Tab annida la voce sotto quella sopra, Shift+Tab la porta fuori di un livello. Annulla e ripeti: `Cmd+Z` e `Cmd+Shift+Z` (`Ctrl+Z`, `Ctrl+Y` o `Ctrl+Shift+Z` altrove).
      - Link: nel blu primario, sottolineati. Un clic mette il cursore nel link; `Cmd`+clic (`Ctrl`+clic altrove) lo apre come i link del Markdown di `requirements/ui#General`, solo `http` e `https`, con il suo toast d'errore. Il tooltip di un link mostra l'indirizzo e `Cmd+click to open` (`Ctrl+click to open`). Un indirizzo `http` o `https` digitato e seguito da uno spazio, o incollato da solo, diventa un link.
      - Incollare testo: il testo formattato tiene gli elementi supportati e lascia cadere il resto; il testo Markdown incollato viene letto come Markdown.

      **`developer.TICK_NOTES_ITEM`** (Open → Open). Lo sviluppatore spunta o toglie la spunta a una voce di checklist. La voce viene spuntata, il testo barrato e attenuato, o torna testo normale; resta dov'è, nient'altro nel foglio cambia, e il foglio viene scritto subito.
      - Ogni voce inizia con una checkbox di 16 px con l'aspetto della `Checkbox` di shadcn, allineata alla prima riga del testo. Un clic spunta o toglie la spunta senza spostare cursore né scroll; `Cmd+Enter` (`Ctrl+Enter` altrove) fa lo stesso sulla voce che contiene il cursore. Una voce spuntata tiene il suo posto, con il testo barrato in `muted-foreground`. La checkbox ha ruolo `checkbox`, `aria-checked` e come nome accessibile il testo della voce.
      - Clic e `Cmd/Ctrl+Enter` sono una sola transazione ProseMirror che cambia solo l'attributo `checked` di quel `taskItem`, così il file scritto differisce solo per il marcatore di quella voce.

      **Immagini nel foglio.**
      - Un nodo immagine conserva la destinazione Markdown come scritta. Solo una destinazione che è esattamente il nome di un file in `.weaver/notes/` viene mostrata: la UI chiede al backend una volta per nome l'URL `data:` e lo tiene in memoria finché un `notes-changed` non nomina quell'immagine o lo stato della vista non viene scartato. Ogni altra destinazione (altro percorso, `http`, `https`, `data:`) non viene mai letta né scaricata e mostra il segnaposto; anche l'evento `error` dell'`<img>` mostra il segnaposto.
      - Le immagini sono a grandezza naturale, mai ingrandite, mai più larghe della colonna, come blocco a sé. Un clic ne seleziona una, con un contorno `ring` di 2 px; `Backspace` o `Delete` la toglie. Il segnaposto è largo quanto la colonna, alto 48 px, su `muted` con bordo `border` tratteggiato: `ImageOff` e il nome dell'immagine, o il suo indirizzo, in testo attenuato.
      - Aggiungere (`developer.ADD_NOTES_IMAGES`, nel Task del backend): `Cmd+V` (`Ctrl+V` altrove) con immagini negli appunti, prese da `ClipboardEvent.clipboardData.files` e mandate come byte con il nome del file quando c'è; oppure file trascinati dal Finder o da Explorer sul foglio, da `getCurrentWebview().onDragDropEvent` ascoltato solo mentre la vista Notes è mostrata, come fa la vista Assets (`ui/src/components/assets/`), mandando i percorsi assoluti. Mentre i file sono trascinati sopra, una linea primaria di 2 px mostra dove andranno. Le immagini aggiunte si inseriscono, scritte `![](<name>)`, al cursore o nel punto di rilascio, nell'ordine dato. Toast d'errore, uno per file rifiutato: `"<name>" is larger than 10 MB: notes images can be at most 10 MB.` e `"<name>" is not a PNG, JPEG, GIF or WebP image.`; una copia fallita: `Could not add "<name>": <reason>.` I file che non sono immagini non mostrano nulla.

      **Modifica esterna (`workspace.NOTES_CHANGED`, evento `notes-changed`).** Quando nessuna modifica dello sviluppatore aspetta di essere scritta, il foglio mostra il file com'è ora, vuoto se è stato cancellato: la UI rilegge il foglio e sostituisce il contenuto in una sola transazione segnata come non dello sviluppatore, senza toast, tenendo il cursore dove il testo intorno esiste ancora. Altrimenti la versione del foglio viene scritta subito sopra il file e compare il toast di avviso `The notes changed on disk; Weaver kept your version.` Le immagini cambiate su disco si mostrano come sono ora, quelle mancanti come segnaposto.

      **Lasciare la vista e chiudere la finestra.** Per la visita la UI tiene i nomi delle immagini che escono dal documento per transazioni dello sviluppatore, annulla e ripeti compresi (non per una ricarica da disco). Su `developer.LEAVE_NOTES_VIEW` e prima che la finestra si chiuda (`window.CLOSED`), dopo aver scritto il testo in attesa, manda al backend quelle che il Markdown scritto non nomina più, perché vadano nel cestino; poi l'insieme riparte vuoto. Un'immagine rifiutata dal cestino resta, con il toast d'errore `Could not move "<name>" to the trash: <reason>` (`Could not move <n> images to the trash: <reason>` per più immagini). Le immagini mai tolte restano anche quando il foglio non le mostra.

      **Log.** Mai il testo del foglio, i byte di un'immagine o il suo URL `data:`.

      **Test** (Vitest, con i loro percorsi da tastiera): caricare e poi salvare restituisce lo stesso testo per ogni elemento supportato scritto come lo scrive l'editor; blocchi non supportati (HTML grezzo, una tabella, marcatori di conflitto, front matter) riscritti byte per byte; ogni scorciatoia Markdown; Invio che continua una checklist non spuntata e la chiude su una voce vuota; Tab e Shift+Tab; spunta con clic e con `Cmd/Ctrl+Enter` che cambia solo quel marcatore; tempi del salvataggio automatico e fusione delle scritture con timer finti; modifica esterna con e senza scrittura in attesa; segnaposto per un'immagine mancante, un percorso altrove e un indirizzo; immagini tolte raccolte con annulla e ripeti, non con una ricarica, e mandate nel cestino solo quando non sono più nominate; l'ordine della barra laterale con Notes dopo Assets.
  fd36m87g:
    title: Cartella notes e watcher
    refs:
    - workspace.NOTES_CHANGED
    - workspace.FILES_CHANGED
    - project/Empty Model/workspace.FILES_CHANGED
    - project/Has a Model/workspace.FILES_CHANGED
    - requirements/tech#project
    - requirements/data#General
    technical: false
    proposal: vys13dnc
    label: Notes
    finished: 2026-11-04T08:12:36.524Z
    body: |-
      Weaver ha una nuova cartella solo locale `.weaver/notes/`, che contiene il foglio delle note dello sviluppatore (`notes.md`) e le immagini incollate in esso. Quando questo Task è fatto deve valere quanto segue.

      **Layout (`requirements/data#General`).**
      - `notes/` è nel layout di `.weaver/` tra le cartelle solo locali, accanto a `requests/`, `short-numbers/` e `task-images/` (in `crates/core/src/project/layout.rs`: `LOCAL_ONLY_DIRS`, mai in `VERSIONED_ENTRIES`).
      - Ha un proprio `.gitignore` con `*`, scritto quando la cartella viene creata e riscritto ogni volta che manca, come le altre cartelle solo locali (`ensure_local_folder`).
      - Viene creata solo alla prima scrittura del foglio o di un'immagine, mai all'inizializzazione del progetto.
      - I suoi file non sono mai letti come file di Model, Requirements, Specs o card e non entrano mai nel quadro del progetto. Solo la vista Notes scrive nella cartella.

      **Watcher (`requirements/tech#project`).**
      - Nessun nuovo watcher: il watcher del progetto vede già `.weaver/notes/`.
      - I suoi eventi lì non arrivano mai al quadro del progetto, alla sua ricostruzione, a `project-changed` né a `workspace.FILES_CHANGED`. Oggi `crates/core/src/project/image.rs:157` salta solo i percorsi nascosti e gli asset prima di segnare `touched`: va escluso anche `notes/`. I file delle note non contano mai come `workspace.FILES_CHANGED`, così come quelli del codice e gli asset.
      - Gli eventi sotto `notes/` vanno solo alla vista Notes, raggruppati 150 ms dopo l'ultimo. Ogni gruppo produce, verso la finestra del progetto, un evento `notes-changed` che è `workspace.NOTES_CHANGED`. L'evento porta:
        - l'id del progetto;
        - un numero di generazione;
        - se il foglio è cambiato, con il suo nuovo hash SHA-256 (nessuno se il file manca);
        - i nomi delle immagini cambiate.
      - Un cambiamento del foglio il cui hash è uguale all'ultimo che Weaver ha scritto o letto non viene inviato: è una scrittura di Weaver stesso.
      - Il modo in cui la vista reagisce a `workspace.NOTES_CHANGED` è nel Task della vista `notes-view`; questo Task fornisce l'evento.

      **Test** (contro cartelle temporanee, in `weaver-core` o `src-tauri` come per gli asset):
      - la cartella e il suo `.gitignore` creati alla prima scrittura e riscritti se mancano;
      - `classify` che non tratta mai un file sotto `notes/` come file del progetto;
      - gli eventi sotto `notes/` che non toccano mai il quadro del progetto;
      - una scrittura di Weaver stesso che non viene inviata come `notes-changed`.
  g5yryamx:
    title: README e CHANGELOG Notes
    refs:
    - app
    technical: false
    proposal: vys13dnc
    label: Notes
    finished: 2026-11-04T08:02:20.363Z
    body: |-
      Weaver ora ha le note del progetto: la vista `Notes`, nella barra laterale dopo Assets. È un unico foglio Markdown per progetto, scritto in un editor WYSIWYG alla Notion e salvato da solo. Ha le scorciatoie Markdown durante la scrittura (`# ` per un titolo, `[ ] ` per una voce di checklist) e checklist con voci che, spuntate, restano al loro posto barrate. Le immagini si incollano o si trascinano dal Finder o da Explorer. Il foglio e le sue immagini stanno in `.weaver/notes/`, una cartella solo locale con un `.gitignore` `*`: non sono mai versionati e l'agente non li vede mai. Quando questo Task è fatto, la documentazione del repository lo dice, come chiede `requirements/docs#General`.

      - `CHANGELOG.md`, sotto `## [Unreleased]` → `### Added`: una riga, in inglese e senza id di Task o Proposal, che dice all'utente cosa c'è di nuovo. Deve nominare:
        - la vista `Notes` dopo Assets;
        - il foglio unico in stile Notion, con scorciatoie Markdown e checklist;
        - le immagini incollate o trascinate (PNG, JPEG, GIF o WebP fino a 10 MB);
        - il salvataggio automatico;
        - che le note restano solo su questo computer, in `.weaver/notes/`, non versionate e mai mostrate all'agente.
      - `README.md`:
        - il paragrafo iniziale nomina la vista Notes accanto al File view;
        - `### Where Weaver keeps files` ha un paragrafo su `.weaver/notes/`, accanto a quello su `.weaver/assets/`. Il paragrafo dice che la cartella contiene `notes.md` e le immagini del foglio; che è solo locale (`.gitignore` `*`), quindi un altro clone parte senza note; che viene creata alla prima nota o immagine; che è scritta solo dalla vista Notes; che git ne ignora i file, quindi il File view mostra la cartella vuota; che l'agente non riceve mai le note.
      - La tabella dei moduli di `weaver-core` (`### weaver-core modules`) e quella dei moduli dell'app (`### weaver (app) modules`) elencano i moduli nuovi delle note, con una riga ciascuno, come fanno per gli asset.
  jso6s7hc:
    title: Backend delle note
    refs:
    - requirements/tech#notes-view
    - requirements/rules#notes-view
    - requirements/quality#notes-view
    - developer.ADD_NOTES_IMAGES
    - developer.LEAVE_NOTES_VIEW
    technical: false
    proposal: vys13dnc
    label: Notes
    finished: 2026-11-04T08:13:49.408Z
    body: |-
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
---
# Vista Notes

La finestra di progetto ha la vista Notes: un foglio Markdown per progetto, modificabile con salvataggio automatico, con immagini incollate o trascinate e voci spuntabili. La cartella .weaver/notes è solo locale e viene creata alla prima scrittura. Il watcher avvisa la vista con un evento dedicato, senza mai raggiungere l'immagine del progetto, e l'agente non riceve mai le note. README e CHANGELOG descrivono la funzione.
