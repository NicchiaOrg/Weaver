---
technical: false
refs:
  - app
  - files
  - agent
  - time
  - workspace
---
# README della vista File

Il principio di Weaver cambia: Weaver non analizza mai il codice dello sviluppatore (lo fa l'agente) e legge e scrive un file del codice solo quando lo sviluppatore lo apre o lo cambia nella vista File; a vista chiusa non legge nulla del codice. La finestra di un progetto, oltre alle viste in sola lettura, ha la vista File (Feature `files`): un albero dei file del progetto e un editor CodeMirror 6 a schede, con i colori della sintassi, senza completamento, diagnostica o language server.

Questo Task porta quel cambiamento nei testi che lo descrivono, quando il resto della vista File è costruito dagli altri Task della stessa Proposal (albero e vista, schede ed editor, blocco dell'agente, conferma alla chiusura della finestra):

- `README.md`: l'introduzione dice che la finestra ha anche la vista File per piccole modifiche ai file del progetto, accanto all'editor dello sviluppatore; le tabelle dei moduli di `weaver-core` e dell'app `weaver` elencano i nuovi moduli della vista File (elenco delle cartelle, lettura e salvataggio dei file del codice, osservazione, blocco dell'agente) con la loro responsabilità; `### Where Weaver keeps files` resta vero: la vista File non scrive nulla fuori dai file che lo sviluppatore cambia, e non tiene nulla su disco.
- Ogni testo di Weaver che oggi dice che Weaver non legge mai il codice (istruzioni, skill installate, testi dell'interfaccia) si allinea al nuovo principio; cercare «never reads» e «read-only» in `crates/core/assets/` e in `ui/src/`.
- Il tempo (`time`) serve anche a liberare ciò che la vista File tiene in memoria 10 s dopo che smette di essere mostrata; mai a decidere che un agente ha smesso di lavorare.
- Il workspace osserva, oltre a `.weaver/`, solo le cartelle espanse e i file aperti della vista File; i file del codice non diventano mai `FILES_CHANGED` e non ricostruiscono la fotografia del progetto.

La riga del CHANGELOG sotto `## [Unreleased]` per la vista File la scrive il Task dell'albero; questo Task non ne aggiunge un'altra (`requirements/docs#General`).
