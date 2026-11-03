---
technical: false
refs:
  - general-chat
  - requirements/ui#general-chat
  - requirements/tech#general-chat
---
# Sub-agenti nella Chat

Nella Chat (General) di un progetto, come in un job, lo sviluppatore deve poter aprire e leggere, in sola lettura, la conversazione di ogni sub-agente avviato dall'agente.

Questo Task riusa la lettura dei transcript dei sub-agenti e il rendering della loro conversazione costruiti per i job dal Task su `job` della stessa Proposal: non si crea un secondo meccanismo, si estende quello alla Chat.

## Comportamento atteso

- Ogni riga di sub-agente sotto una riga `task` della Chat è un pulsante che apre la conversazione di quel sub-agente **al posto della chat**, nella stessa vista.
- La conversazione del sub-agente è resa come in `## job` (**Sub-agent conversation**): messaggi, `Thinking`, righe dei tool, i suoi sub-agenti come righe che aprono le loro conversazioni, segue il nuovo testo mentre il sub-agente lavora, con `Jump to latest`. Prima voce: l'incarico ricevuto, etichettato con il nome di chi l'ha avviato. Sola lettura: niente message box, niente `Stop`, niente permission card.
- Header: `Chat › <sub-agent name>` (con gli eventuali sub-agenti genitori in mezzo, come nel percorso dei job), poi lo stato del sub-agente come icona e testo.
- Un pulsante `Back to Chat` (icona `ArrowLeft`) torna alla chat **nella posizione di scroll che aveva**.
- Nel frattempo la chat prosegue: il turno dell'agente, i messaggi e gli aggiornamenti continuano ad arrivare e si vedono al ritorno.
- `Clear` (e la sua conferma) riporta anch'esso alla chat.
- Un transcript mancante o illeggibile mostra solo `Conversation not available: <reason>.` in testo secondario sotto l'header; il sub-agente resta con lo stato preso dalle altre fonti.

## Lato backend

I sub-agenti della Chat sono letti, elencati e mostrati con le stesse regole di `## job` (**Sub-agents**), con chiave il General del progetto al posto di un job:
- stato dei sub-agenti in background preso dagli aggiornamenti dei tool `task` e `wait` (`rawOutput.details.progress[]`, `rawOutput.details.jobs[]`), con il record `session_exit` del transcript solo come fallback;
- transcript letti in modo incrementale dall'ultimo offset, solo mentre servono, ogni 2 s;
- advisor di omp (file `__advisor.*`) mai elencati né mostrati;
- sub-agenti annidati a ogni profondità;
- la UI riceve snapshot più eventi di modifica, con chiave progetto, General e percorso di `task` call e `id`;
- dopo `session/load` (ripresa della conversazione conservata) i sub-agenti sono ricostruiti dalla history e i transcript riletti dall'inizio; quelli ancora `working` quando omp esce diventano `interrupted`.

## Changelog

Aggiungere una riga sotto `## [Unreleased]` di CHANGELOG.md, senza id di Task o Proposal, che dica che nella Chat si può aprire la conversazione dei sub-agenti.

## Dove nel codice

- `ui/src/components/general/general-view.tsx` — vista Chat, riusa `EntryView`; qui va lo scambio chat/conversazione del sub-agente, header e `Back to Chat`
- `ui/src/components/jobs/jobs-view.tsx` — `ToolRow`, `EntryView` e la conversazione del sub-agente da riusare
- `src-tauri/src/general/actor.rs` — controlli in background dei sub-agenti per General (righe ~80, ~609-661)
- `src-tauri/src/jobs/conversation.rs` — merge dei sub-agenti condiviso con i job
- `crates/core/src/jobs/subagents.rs` — mapping stati e percorsi dei transcript
- `CHANGELOG.md` — riga sotto `## [Unreleased]`
