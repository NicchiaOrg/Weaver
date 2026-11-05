---
refs:
  - job
  - requirements/ui#job
---

# Job terminato: rimando a Chat

Quando una conversazione di un job è terminata resta in sola lettura e, al posto del campo messaggio, mostra il testo `This job has ended. Continue in Chat.` con il pulsante `Open Chat` (prima `This job has ended. Continue in General.` e `Open General`).

`Open Chat` apre la vista Chat (la conversazione libera del progetto), esattamente come faceva `Open General`. L'identificatore interno della vista resta `general`: cambia solo il testo mostrato all'utente, non gli identificatori, i comandi o i testi destinati all'agente.

I test che verificano i vecchi testi vanno aggiornati ai nuovi, `This job has ended. Continue in Chat.` e `Open Chat`.
