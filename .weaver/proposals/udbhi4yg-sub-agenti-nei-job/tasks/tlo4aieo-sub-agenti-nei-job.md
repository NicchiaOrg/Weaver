---
technical: false
refs:
  - job
  - requirements/ui#job
  - requirements/tech#job
---
# Sub-agenti nei Job

A Task completato, selezionando un job nella vista Jobs (vale per tutti i job, non solo `All tasks`) sotto la sua riga si apre l'elenco dei suoi sub-agenti. Cliccandone uno, a destra compare la sua conversazione in sola lettura, sia mentre il job lavora sia dopo la fine, finché il job resta in elenco.

## Backend

- **Identità e ordine:** un sub-agente è identificato dalla sua chiamata `task` e dal suo `id`. I sub-agenti diretti di un job si leggono da `rawOutput.details.progress[]` delle sue chiamate `task`: `id`, nome (il primo presente fra `name`, `agent`, `description`, `id`) e stato. L'ordine di lancio è quello delle chiamate `task` e, dentro ciascuna, l'ordine di omp.
- **Mappa degli stati**, uguale per ogni fonte: `pending` e `running` → working; `completed` → succeeded; `failed` → failed; `aborted` → interrupted.
- **Sub-agenti in background** (omp `async.enabled`): la chiamata `task` termina mentre sono ancora pending, quindi lo stato va preso da ogni aggiornamento successivo degli strumenti di omp che li nomina: il `progress` di `task` e `rawOutput.details.jobs[]` (`id`, `status`) della chiamata dello strumento `wait`. Il record `session_exit` del transcript resta solo come ripiego (`kind: normal` → succeeded, un errore → failed, altro → interrupted). Oggi un sub-agente finito resta `working` per circa 7 minuti, perché omp scrive `session_exit` solo al dispose: con questa modifica non deve più succedere.
- **Transcript:** `<file della sessione principale senza .jsonl>/<id>.jsonl` sotto la cartella delle sessioni di omp; per un sub-agente annidato `<transcript del genitore senza .jsonl>/<id>.jsonl`. L'`id` deve rispettare `^[A-Za-z0-9_.-]+$` senza `.` iniziale e il percorso reale deve restare nella cartella. I file il cui nome inizia con `__advisor.` sono gli advisor di omp: mai elencati né mostrati.
- **Lettura incrementale:** ogni transcript si legge dall'ultimo offset letto, mai di nuovo per intero; un'ultima riga incompleta si rilegge al passaggio successivo. Un transcript si legge solo mentre è mostrata la conversazione del suo sub-agente, o è mostrato l'elenco dei sub-agenti del suo job, o il suo stato aspetta ancora il ripiego `session_exit`; ogni 2 s finché possono arrivare record.
- **Record → voci:** il primo `message` con `role: user` è l'incarico; `assistant` `content[]` dà `text` (messaggio dell'agente), `thinking` (`Thinking`) e `toolCall` (`id`, `name`, `arguments`, `intent`: una riga di strumento); un `toolResult` (`toolCallId`, `isError`) chiude la riga corrispondente. Gli altri record si ignorano.
- **Annidati:** i sub-agenti avviati da un sub-agente, e il loro stato, si ricavano dai risultati `task` e `wait` del transcript del genitore, con le stesse regole del job, a qualsiasi profondità.
- **Errori:** file mancante, illeggibile o con una riga completa malformata → quella conversazione diventa `Conversation not available: <reason>`; il sub-agente resta in elenco e mantiene lo stato dalle altre fonti.
- **Verso la UI:** la conversazione di un sub-agente arriva come snapshot più eventi di cambiamento, come per un job, con chiave progetto, job (o General) e percorso di coppie chiamata `task` + `id` dal job fino al sub-agente.
- **Ripresa:** dopo `session/load` (resume o reload) i sub-agenti si ricostruiscono dalla storia rigiocata e i transcript si rileggono dall'inizio.
- **Fine:** ogni sub-agente ancora working quando omp esce o il job finisce diventa interrupted, annidati compresi.
- Il lettore dei sub-agenti va scritto condiviso e indipendente dal job: un altro Task su `general-chat` lo riusa per General (chiave General al posto del job), quindi niente logica legata solo all'attore dei job.
- Questi dettagli interni di omp vanno riverificati a ogni aumento di `MIN_OMP_VERSION`.

## UI

- **Elenco sub-agenti:** sotto la riga del job selezionato, e solo sotto quella, un elenco indentato dei suoi sub-agenti; selezionare un altro job lo chiude e apre quello del nuovo. Un job senza sub-agenti non mostra elenco. Sub-agenti diretti in ordine di lancio; sotto ciascuno, indentati di un passo in più, quelli che ha avviato. Gli advisor non compaiono mai.
- **Riga:** una linea con `Bot`, nome di omp troncato con `…` (nome intero nel tooltip), stato come icona e testo (`working`, `succeeded`, `failed`, `interrupted`). Fra fratelli con lo stesso nome, ciascuno aggiunge l'ora di avvio `HH:MM` in testo secondario.
- La riga è un pulsante: mostra a destra la conversazione del sub-agente e riceve `aria-current`; la riga del job resta segnata come genitore, e selezionarla mostra di nuovo la conversazione del job.
- **Righe sotto `task`:** nelle conversazioni ogni linea di sub-agente sotto la riga `task` diventa un pulsante che apre la stessa conversazione e seleziona il sub-agente nell'elenco; lo stato mostra icona e testo.
- **Conversazione del sub-agente:** resa come quella di un job (messaggi, `Thinking`, righe di strumento, i suoi sub-agenti come linee cliccabili), segue il nuovo testo mentre lavora, con `Jump to latest`. Intestazione `<job name> › <sub-agent name>` con ogni genitore in mezzo, poi lo stato come icona e testo. Prima voce: l'incarico ricevuto, come messaggio etichettato col nome di chi l'ha avviato (il job o il sub-agente genitore). Sola lettura: niente message box, niente `Stop`, niente permission card, note o riga di fine. Se il transcript manca o non si legge, sotto l'intestazione solo `Conversation not available: <reason>.` in testo secondario.
- **Dopo la fine:** elenco, stati e conversazioni restano finché il job finito è in elenco, anche dopo il reload della sessione; rimuovere il job rimuove i suoi sub-agenti.
- Aggiungere una riga sotto `## [Unreleased]` di CHANGELOG.md che descrive i sub-agenti visibili nei job, senza id di Task o Proposal.

## Test

`crates/fake-acp` oggi non produce `progress`, dettagli `wait` né file di transcript dei sub-agenti: servono fixture (sub-agenti diretti, in background, annidati, advisor, transcript mancante o malformato, righe incomplete) per coprire le regole sopra.

## Dove nel codice

- `crates/core/src/jobs/subagents.rs` — mappa stati, `from_raw_output`, `find_main_file`, `agent_folder`, `agent_file`, `exit_state`
- `src-tauri/src/jobs/conversation.rs` — `Tool.sub_agents`, `looks_like_task_call`, `background_agents`, `set_sub_agent`, `finish`
- `src-tauri/src/jobs/actor.rs` — `BACKGROUND_EVERY`, controllo che legge solo `session_exit`
- `src-tauri/src/jobs/host.rs` — `sessions_root`
- `src-tauri/src/jobs/wire.rs`, `src-tauri/src/jobs/commands.rs` — `jobs_snapshot`, `job_conversation`, nuovi comandi/eventi
- `ui/src/bindings/SubAgent.ts`, `SubAgentState.ts`, `Tool.ts`, `JobRow.ts`
- `ui/src/components/jobs/jobs-view.tsx` — `JobsView`, `Row`, `ToolRow`, `EntryView`, `Conversation`
- `ui/src/components/jobs/use-jobs.tsx` — `JobsProvider`, `useConversation`
- `crates/fake-acp/src/main.rs` — fixture dei sub-agenti
- `CHANGELOG.md` — `## [Unreleased]`
