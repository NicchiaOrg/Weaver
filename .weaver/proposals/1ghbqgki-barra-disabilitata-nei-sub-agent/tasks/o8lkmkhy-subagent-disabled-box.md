---
refs:
  - job
  - requirements/ui#job
---

# Barra disabilitata nei sub-agent

La conversazione di un sub-agent resta di sola lettura (il developer la legge mentre il job lavora e dopo, finché il job è in elenco), ma in basso mostra sempre la barra per scrivere, disabilitata, per coerenza visiva con la conversazione di un job.

A lavoro finito vale quanto segue, come dice `requirements/ui#job` (**Sub-agent conversation**):

- In fondo alla vista del sub-agent, fissata in basso, c'è una barra con la stessa disposizione della barra di un job: casella di testo e `Send` (`ArrowUp`), entrambi disabilitati, segnaposto `Sub-agents take no messages.`, nessuno `Stop`.
- Vale per tutte e tre le viste dei sub-agent, che oggi condividono la stessa vista: sub-agent di un job (Jobs view), della Chat (General, con `Back to Chat`) e di una conversazione Ask more. Il testo è lo stesso ovunque.
- La barra è sempre la stessa barra disabilitata, qualunque sia lo stato del sub-agent o del job, della Chat o dell'Ask more a cui appartiene, anche quando sono finiti: mai la riga `This job has ended. Continue in Chat.` con `Open Chat`.
- Compare anche quando la conversazione non si può leggere: sotto l'intestazione resta solo `Conversation not available: <reason>.`, e la barra disabilitata resta in basso.
- La casella è disabilitata davvero: fuori dall'ordine di tabulazione e annunciata come non disponibile, come la casella di un job in coda. Nessun messaggio può essere inviato da questa barra.
- Restano assenti `Stop`, la scheda dei permessi, le note e la riga di fine.

Da aggiornare anche i test esistenti che oggi verificano l'assenza della barra nella vista di un sub-agent (Jobs view e Chat) e il commento della vista che dice "No message box".

Riferimenti nel codice trovati in esplorazione:

- `requirements/ui#job` — `ui/src/components/jobs/jobs-view.tsx` `SubAgentConversationView` (vista condivisa da job, Chat e Ask more) e `MessageBox` (barra del job, privata al file; richiede un job, quindi serve una variante solo visiva o una barra presentazionale condivisa)
- `general-chat` — `ui/src/components/general/general-view.tsx` mostra `SubAgentConversationView` al posto della Chat
- `ask-conversation` — `ui/src/components/jobs/jobs-view.tsx` apre i sub-agent dell'Ask more con la stessa vista
- test — `ui/src/components/jobs/jobs-view.test.tsx` e `ui/src/components/general/general-view.test.tsx` (asserzioni di assenza della barra nella vista sub-agent)
- `ui/src/components/ui/textarea.tsx` — il `Textarea` condiviso ha già lo stile `disabled:`
