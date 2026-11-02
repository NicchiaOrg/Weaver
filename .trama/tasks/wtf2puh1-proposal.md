---
column: done
technical: false
refs:
  - model-changes
  - proposal
  - requirements/data#proposal
  - requirements/cli#proposal
  - requirements/rules#proposal
  - requirements/ui#proposal
drift: null
proposal: qmqh1o3p
agent: false
created: 2026-11-02T07:29:32.875Z
finished: 2026-11-02T10:13:53.530Z
---
# Proposal

Weaver gestisce la Machine `proposal` per intero: la cartella `.trama/proposals/<id>-<slug>/` con `proposal.md` (front matter come in `requirements/data#proposal`: `status`, `kind`, `requires`, `base`, `delete`, `created`, `request`, `reworks`, `reworkRequested`, `label`, `refining`, `job`, `drifts`), le copie complete dei file cambiati e i `tasks/<id>-<slug>.md` con soli `technical` e `refs`. Kind: `change` e `setup` portano Task, `import` e `resolution` nessuno. Si appoggia a "Scheletro del progetto", "Core dei file del progetto" e "Lavori dell'agente".

## Stati e transizioni
- `Draft`: l'agente di un job la scrive (`agent.EDIT_PROPOSAL`); nessuno la decide. `agent.SUBMIT_PROPOSAL` → `Open` se ogni controllo di consegna passa (consuma la richiesta legata, vedi "Richiesta di design"); → `Approved` per una Proposal `setup` o `import` consegnata da un job che, arrivato in Running, ha trovato il Modello senza Feature né Actor, con tutti i controlli di Approve tranne `The agent is still finishing`; le Proposal dello stesso job che richiede sono già approvate così, quindi una catena di import passa intera; dopo il setup approvato parte Run all. `developer.DISCARD_PROPOSAL` → `Discarded` solo se nessun job attivo la scrive (anche se attaccata a un Drift, che resta aperto). `developer.APPROVE_PROPOSAL` → `Refining` solo per una Draft lasciata a metà da un raffinamento. `drift.CODE_TO_BE_FIXED` e `drift.DISMISSED` → `Discarded` per la Draft attaccata.
- `Open`: `developer.APPROVE_PROPOSAL` → `Approved` solo se niente la blocca, il job che l'ha consegnata è finito e nessun Run lavora; → `Refining` (raffinamento all'Approve, sempre attivo) per una `change` non attaccata a un Drift bloccata solo da staleness: un job di raffinamento la adatta senza nuova intervista. `developer.REJECT_PROPOSAL` → `Rejected` con il motivo facoltativo; `developer.REWORK_PROPOSAL` → `Rejected` con il feedback (`## Feedback` solo se non vuoto, conferma per feedback vuoto, al massimo un rework): un job di rework la riscrive e la prima Proposal aperta con `reworks` ne prende il posto sulla Board e nei `requires` di chi la aspettava. `developer.CHOOSE_PROPOSAL_MODEL` resta `Open`. `agent.EDIT_PROPOSAL` → `Draft` anche senza differenze. `drift.PROPOSAL_ACCEPTED` → `Approved`, `drift.CODE_TO_BE_FIXED` → `Rejected`, `drift.DISMISSED` → `Discarded` solo per la Proposal attaccata, che non si decide mai da sola.
- `Refining`: mantiene identità, numero breve, posto, label, modello scelto e chi la richiede; l'agente chiede solo se il Modello contraddice una decisione presa, o se rimuovere un cambiamento già presente (`developer.REJECT_PROPOSAL` → `Rejected`). Consegnata → `Approved` se niente la blocca (aspettando la fine di un Run), altrimenti `Open` con i motivi; `job.ENDED` senza consegna → `Draft` con `refining: true`.
- `Approved`, `Rejected`, `Discarded` sono finali; la cartella resta com'era.

## Regole (`requirements/rules#proposal`)
- Scope (`changed`, `leads into a change`, `removed, its container`) e copertura calcolati dal vivo, mai salvati; copertura da riferimenti di Task o righe `## Already present`; Task di rimozione ``Remove code of `<ref>` `` per ogni elemento rimosso più in alto.
- Staleness con SHA-256 del contenuto normalizzato LF in `base`, con le Proposal richieste applicate sotto; i file Task e `proposal.md` non contano; solo il rebase del raffinamento riscrive `base`.
- Motivi di rifiuto di Approve, tutti e in quest'ordine: agente che finisce, Run in corso, ogni Proposal richiesta (in scrittura, non approvata, rifiutata, scartata, mancante, ciclo), staleness per file, invalidità, scope scoperto / riferimenti inesistenti / label mancante, fix di codice non più applicabili. Consegna rifiutata per 5–7 e gli altri casi elencati, mai per staleness o `requires`.
- Approvazione in un solo passo, tutto o niente: file sostituiti, Task in To do con le stesse identità e `created` uguale, Task di rimozione, decisioni della resolution sui Drift ancora aperti, skill dei Requisiti rigenerate (`requirements/rules#project`).
- Label 1–30 caratteri su una riga, obbligatoria con Task; colore FNV-1a modulo 6.
- Modello per Proposal (sempre attivo): mai nel repository, in `proposal-models/<project hash>.json` dei dati locali, tolto quando nessun Task è in To do o In progress; non rimanda in Draft.

## Comandi e Board
- `trama proposal new|show|list|edit|move` come in `requirements/cli#proposal`: ogni `edit` riporta una Open in Draft; `move <id> open` consegna; `rework: true` e l'approvazione di una `change` stale richiedono l'app.
- Card come in `requirements/ui#proposal`: badge `stale`, `refining`, `refinement interrupted`, `blocked` (Popover con ogni motivo), modello; righe file e Task; `Approve`, `Reject` con `Rework with feedback…`, controllo modello; motivi disabilitati nell'ordine fissato; card gialla della resolution con una riga per Drift; dialogo dettagli.

## Fatto quando
- Test del core su ogni transizione, scope, copertura, staleness e ordine completo dei motivi.
- Test dell'auto-approvazione di setup e di una catena di import su Modello vuoto, e del Run all che segue il setup.
- Un'approvazione fallita a metà non lascia alcun file cambiato.
- Approve su una `change` solo stale avvia il raffinamento; un raffinamento interrotto mostra `refinement interrupted`.
- Il modello scelto non compare in nessun file sotto `.trama/`.
