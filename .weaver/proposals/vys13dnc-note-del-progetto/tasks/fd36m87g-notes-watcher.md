---
refs:
  - workspace.NOTES_CHANGED
  - workspace.FILES_CHANGED
  - project/Empty Model/workspace.FILES_CHANGED
  - project/Has a Model/workspace.FILES_CHANGED
  - requirements/tech#project
  - requirements/data#General
---
# Cartella notes e watcher

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
