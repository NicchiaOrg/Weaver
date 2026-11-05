---
status: approved
kind: change
base:
  model/agent-work/general-chat.md: ac78acb5ad3765f3df1d9fda332b5f3fba5e8f0ba61c7c04b77cafb3bef6cc6c
  model/agent-work/job.md: d3454da282dde1cd9eeff3a96c498cd57c2600aa3ad3477612df363a737de620
  requirements/tech/acp.md: b2a596e721781825cb487add43a0e905c9a72d7e22158fe913932dcb123b7230
  requirements/ui/conversations.md: 8baa8263d2fc548f2d78a4210804084ef7cff840ae90cf827f607e71b1473a9c
  requirements/ui/rounds.md: new
created: 2026-11-02T21:11:26.056Z
request: 2026-11-02T21-58-15
label: Sub-agenti visibili
job: snh57ldf
---
# Sub-agenti nei Job

Con Run all il lavoro finisce in un solo Job (`All tasks`) e i sub-agenti a cui l'agente affida i Task sono solo righe con nome e stato: lo sviluppatore non vede cosa stanno facendo. La Proposal rende consultabili le conversazioni dei sub-agenti in tutti i Job e in Chat.

## Decisioni

- Sotto il Job selezionato, nella lista dei Job, si apre l'elenco dei suoi sub-agenti; cliccandone uno, a destra compare la sua conversazione in sola lettura. Vale per tutti i Job.
- Sono mostrati i sub-agenti diretti e, rientrati sotto ciascuno, i loro sub-agenti annidati; gli advisor di omp sono nascosti.
- Lo stato di un sub-agente in background viene dagli aggiornamenti che omp riporta negli strumenti `task` e `wait`; il record di chiusura della trascrizione resta come riserva.
- L'elenco si apre quando il Job viene selezionato e si chiude quando se ne seleziona un altro.
- La riga di un sub-agente mostra icona `Bot`, nome e stato (icona e testo).
- Ordine di avvio, anche tra più chiamate `task`; nomi uguali distinti dall'ora di avvio.
- Conversazione del sub-agente in sola lettura: intestazione con nome del Job › nome del sub-agente e stato, nessuna casella dei messaggi né `Stop`.
- Le righe dei sub-agenti nelle conversazioni (sotto la chiamata `task`) diventano cliccabili e aprono la stessa conversazione.
- A Job finito, elenco e conversazioni restano consultabili finché il Job resta nella lista, anche dopo una ripresa della sessione.
- Trascrizione assente o illeggibile: il sub-agente resta nell'elenco e la sua conversazione mostra `Conversation not available: <reason>.`.
- In Chat (General) le righe dei sub-agenti aprono la loro conversazione in sola lettura al posto della chat, con un pulsante per tornare.

## Scelte dell'agente da verificare

- La prima voce della conversazione di un sub-agente è l'incarico ricevuto, etichettato con il nome di chi lo ha avviato (il Job o il sub-agente genitore).
- In Chat l'intestazione è `Chat › <sub-agent name>` e il pulsante per tornare è `Back to Chat` (`ArrowLeft`); anche `Clear` riporta alla chat.
- Le trascrizioni si leggono a incrementi, ogni 2 s, solo mentre servono (conversazione o elenco visibile, oppure stato in attesa del record di chiusura), come il controllo in background di oggi.
- Il fragment `ui/conversations` supererebbe i 12 KB: le sezioni `question-round` e `notification` passano, senza cambiare testo, nel nuovo fragment `ui/rounds` ("Question rounds and notifications").
