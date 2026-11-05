---
technical: false
refs:
  - project-setup
  - project
  - workspace
  - requirements/tech#project
  - requirements/rules#project
  - requirements/cli#project
  - requirements/ui#project
---
# Progetto e skill installate

Realizza la Machine `project` e i file che Weaver installa per l'agente. Dipende da "Scheletro del progetto", "Core dei file del progetto" e "Riga di comando trama"; i testi installati vengono da "Testi delle skill".

## Machine `project`
- `Not initialised`: nessuna `.trama`; ogni azione che richiede un progetto dice di inizializzarlo, mai cartelle genitore. `developer.INITIALISE_PROJECT` → `Empty Model`: crea `.trama` con un Modello vuoto (System col nome della cartella), le Aree iniziali `ui`, `tech`, `data`, `rules`, `quality` (solo `area.md`, titoli e descrizioni di `requirements/rules#project`) e ogni file per l'agente. Se file da installare esistono con contenuto diverso, non crea nulla finché il developer non conferma la sovrascrittura; un fallimento non lascia nulla. `workspace.PROJECT_FOLDER_APPEARED` → `Empty Model` o `Has a Model` secondo il contenuto.
- `Empty Model` (nessuna Feature né Actor; Drift e Requisiti non contano): `proposal.APPROVED` e `workspace.FILES_CHANGED` portano a `Has a Model` se ora c'è una Feature o un Actor, altrimenti restano e rigenerano le skill dei Requisiti (per `FILES_CHANGED` solo se i Requisiti sono cambiati). `developer.UPDATE_SKILLS` riscrive i file diversi, crea le Aree iniziali se mancano i Requisiti e riporta sempre cosa ha cambiato o l'errore. `workspace.PROJECT_FOLDER_REMOVED` → `Not initialised`.
- `Has a Model`: all'apertura della finestra confronta in background ogni file installato e riscrive quelli diversi, anche modificati a mano; parla solo se ha cambiato qualcosa e mostra un fallimento a ogni apertura finché non sparisce. `proposal.APPROVED` rigenera le skill dei Requisiti; `FILES_CHANGED` resta o torna a `Empty Model` se non c'è più Feature né Actor; `UPDATE_SKILLS` come sopra; `PROJECT_FOLDER_REMOVED` → `Not initialised`, i lavori in corso continuano.

## File installati (`requirements/rules#project`, `requirements/tech#project`)
- Un solo elenco nel core, incorporato con `include_dir` (app e CLI scrivono byte identici): le sette skill `trama-*` in `.agents/skills/`, `.omp/agents/trama-analysis.md`, `.omp/agents/trama-implementer.md`, `.omp/WATCHDOG.yml` e le skill generate `trama-req-*`.
- Confronto puro (CRLF→LF, byte per byte), senza versioni né rete; si scrive atomicamente solo ciò che manca o differisce, nient'altro si tocca tranne le `trama-req-*` che il modo corrente non produce. Un controllo per progetto alla volta, le richieste intanto si fondono; mai ritarda la finestra.
- Skill dei Requisiti: budget = caratteri di `trama-req-<area>-<fragment>` più descrizione, sommati sui fragment con contenuto; fino a 4.000 incluso modo fragment, oltre modo index (una skill per Area), tutte le Aree insieme. Rigenerate da una sola funzione del core all'approvazione (app o CLI), all'inizializzazione, a Update skills, al controllo d'apertura e 1 s dopo l'ultima modifica sotto `.trama/requirements/`. Un unico risultato strutturato (file scritti e rimossi, modo, budget, cambio di modo) per app e CLI.
- Watcher `notify` per progetto: radice non ricorsiva per `.trama` che compare o sparisce, `.trama/` ricorsiva; le scritture proprie (hash registrato) non sono `FILES_CHANGED`; la cancellazione in `requests/` è `workspace.REQUEST_DELETED`.

## Interfaccia (`requirements/ui#project`)
- Get Started in ogni vista tranne Settings, con l'elenco dei file installati e `Initialise`; dialog `Some agent files already exist` con `Overwrite and initialise`/`Cancel`; errore `Could not initialise: <reason>.`.
- In `Empty Model` due card sopra la Board: `Start from Scratch` apre una richiesta di setup (vedi "Richiesta di design"); `Import from Code` apre l'albero delle cartelle con caselle, `<N> excluded`, `Start import` (disabilitato se nulla è incluso) che avvia il lavoro di import (vedi "Lavori dell'agente"). Le esclusioni valgono solo per quell'import, mai salvate. Avviso su Start from Scratch quando la cartella ha altri nomi senza punto; le card si nascondono mentre lavora un setup o un import o c'è una Proposal aperta.
- Update skills in Settings (gruppo `Project`) col report; toast all'apertura se ha riscritto file, banner con `Retry` se fallisce.

## Comandi (`requirements/cli#project`)
`project new {force?}` rifiuta se `.trama/` esiste o se file installati differiscono (tutti elencati) senza `force: true`; `project edit {}` è Update skills e stampa `changed`, `areas`, `generated`; `project show {}` stampa la Board (`toDecide`, `inWriting`, `todo`, `doing`).

## Fatto quando
- Test su cartelle temporanee: inizializzazione completa, conflitto senza conferma che non crea nulla, fallimento senza residui.
- Test del budget a 4.000 e 4.001 caratteri, del cambio di modo e della rimozione delle `trama-req-*` non più prodotte.
- Cancellare e ricreare `.trama` porta la finestra a Get Started e indietro; un file installato modificato a mano viene riscritto all'apertura con il toast.
- Test UI di Get Started, dialog di conflitto e albero di Import from Code con tastiera.
