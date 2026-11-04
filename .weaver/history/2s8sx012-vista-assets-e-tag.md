---
created: 2026-11-04T07:02:37.813Z
updated: 2026-11-04T07:02:37.813Z
summary: Una nuova vista Assets raccoglie i file del progetto in .weaver/assets, con rilascio, rinomina, cestino e trascinamento fuori; nei messaggi si citano con il tag @.
tasks:
- bdf8el0v
- igwokwb7
cleared:
  bdf8el0v:
    title: Vista Assets
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
    technical: false
    proposal: gw1wwbdq
    label: Assets
    finished: 2026-11-04T06:55:05.945Z
    body: |-
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
  igwokwb7:
    title: Tag @ degli asset
    refs:
    - developer.SEND_MESSAGE
    - general-chat/Idle/developer.SEND_MESSAGE
    - general-chat/Loading/developer.SEND_MESSAGE
    - general-chat/Working/developer.SEND_MESSAGE
    - job/Running/developer.SEND_MESSAGE
    - job/Waiting for the developer/developer.SEND_MESSAGE
    - ask-conversation/Empty/developer.SEND_MESSAGE
    - ask-conversation/Idle/developer.SEND_MESSAGE
    - ask-conversation/Answering/developer.SEND_MESSAGE
    - ask-conversation/Waiting for permission/developer.SEND_MESSAGE
    - developer.SEND_REQUEST
    - design-request
    - job/Not started/developer.SEND_REQUEST
    - requirements/ui#assets
    - requirements/rules#assets
    - requirements/rules#job
    - requirements/tech#assets
    - requirements/quality#assets
    technical: false
    proposal: gw1wwbdq
    label: Assets
    finished: 2026-11-04T06:55:24.538Z
    body: |-
      Quando il Task è finito, lo sviluppatore può taggare gli asset del progetto (i file regolari in `.weaver/assets/` il cui nome non inizia con `.`) scrivendo `@` seguito dal nome, in quattro punti: la casella dei messaggi della Chat, la casella di risposta della conversazione di un job nella vista Jobs, la casella di una conversazione Ask more e il testo dell'editor delle richieste aperto da `+ Work Item` (non la sua `Preview`).

      **Riconoscimento** (`requirements/rules#assets`): un tag è `@` all'inizio del testo o dopo uno spazio, seguito dal nome di un asset esistente, confrontato esattamente, maiuscole comprese; se più nomi vanno bene vince il più lungo, così i nomi con spazi non richiedono virgolette; dopo il nome vengono fine testo, spazio o uno di `. , ; : ! ? ) ] } " '`. Un `@NOME` che non corrisponde a nessun asset resta testo normale e non dice niente all'agente.

      **Mentre si scrive** (`requirements/ui#assets`): digitando `@` si apre un elenco di suggerimenti sotto il cursore che filtra gli asset mentre si scrive (prima quelli il cui nome inizia con il testo, al massimo 8 righe visibili); Su/Giù, Invio o Tab inseriscono `@<nome>` e uno spazio, Esc chiude; con l'elenco aperto Invio e Tab non inviano mai. Ogni tag riconosciuto è disegnato in blu primario nella casella; un tag che non corrisponde resta del colore normale. Una sola componente condivisa serve le quattro caselle: `textarea` nativa con testo trasparente e cursore visibile, e dietro uno strato specchio che disegna il testo con i tag colorati (`requirements/tech#assets`). Le caselle oggi sono `Textarea` semplici: `ui/src/components/general/general-view.tsx`, `ui/src/components/jobs/jobs-view.tsx`, `ui/src/components/ask/ask-conversation.tsx`, `ui/src/components/requests/request-pane.tsx`. L'elenco degli asset arriva dall'evento `assets-changed` del backend, costruito nel Task «Vista Assets» (voce "Asset list" di `requirements/tech#assets`); se ancora non c'è, va costruito come descritto lì.

      **All'invio di un messaggio** (Chat, conversazione di un job, Ask more): i tag si riconoscono sugli asset esistenti in quel momento, mai dopo (un messaggio in attesa di consegna tiene ciò che ha avuto all'invio). Se c'è almeno un asset taggato, al testo che va a omp si aggiunge, dopo il testo dello sviluppatore, una riga vuota, la riga `Assets tagged by the developer (added by Weaver, not written by the developer):` e, una per riga, ``Read the file `.weaver/assets/<name>`.`` per ogni asset, una volta sola, nell'ordine del primo tag; nel primo messaggio di Ask more il blocco segue il messaggio dopo l'istruzione. La lettura del file è lasciata a omp. La conversazione mostra sempre solo il testo scritto dallo sviluppatore, senza il blocco: dal vivo e anche quando viene ricostruita dal replay di omp. Punti nel codice: `src-tauri/src/general/service.rs` `send_message` e la prima richiesta di Ask more (`crates/core/src/jobs/ask.rs` `first_prompt`, `shown_message`), `src-tauri/src/jobs/registry.rs` `send_message`, `src-tauri/src/jobs/session.rs` `text_prompt`, `src-tauri/src/jobs/conversation.rs` `developer_message` e `replayed_user`.

      **All'invio di una richiesta di design**: la richiesta si invia come prima e il suo file resta com'è scritto; in più l'istruzione del job di design, o di setup quando il Model è vuoto, contiene subito dopo la prima frase, sulla stessa riga, una frase ``Read the file `.weaver/assets/<name>`.`` per ogni asset che la richiesta tagga al momento dell'invio (`requirements/rules#job`). Il job tiene in memoria l'elenco dei nomi fino all'invio dell'istruzione; non va scritto nel registro dei job né nel file della richiesta. Punti nel codice: `crates/core/src/job_instruction.rs` (`first_sentence`, `Job::Design` / `Job::Setup`).

      **Test** come chiede `requirements/quality#assets` per i tag: funzioni pure in `weaver-core` (riconoscimento, blocco aggiunto e sua rimozione dal replay, frasi dell'istruzione) e Vitest per elenco dei suggerimenti ed evidenziazione in ognuna delle quattro caselle.
---
# Vista Assets e tag @

La vista Assets mostra gli asset come tessere ordinate per nome, con selezione multipla, menu contestuale, copia dei file rilasciati dall'esterno tutto o niente e trascinamento verso altre app come copia. Nelle caselle dei messaggi (Chat, risposte dei job, Ask more e richiesta di design) il tag @ suggerisce gli asset, li evidenzia e li passa all'agente senza cambiare il testo che lo sviluppatore ha scritto.
