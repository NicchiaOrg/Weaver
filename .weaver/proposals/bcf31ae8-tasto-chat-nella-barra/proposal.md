---
status: approved
kind: change
base:
  requirements/ui/conversations.md: 1a5bced9b4a056b71a6f67116e9f9e1a5f01b7409606bd55c1be85e36da9e3ef
  requirements/ui/look.md: 8b61371159736364ccbf548ea924b8646517d27e5816ce8b9294aaf7821520ac
created: 2026-11-02T15:37:38.372Z
request: 2026-11-02T16-31-44
label: Chat
job: jj8nrq0c
---
# Tasto Chat nella barra

La voce `General` della barra laterale diventa `Chat` e sale al secondo posto, dopo Board, come chiesto nella richiesta scritta.

Decisioni:
- Il nome `Chat` sostituisce `General` in tutti i testi dell'interfaccia: voce della barra (e il suo tooltip in modalità compatta), titolo della vista, conferma `Clear Chat?`, riquadro di un job finito `This job has ended. Continue in Chat.` con `Open Chat`, rifiuto `Chat is not open for this project`.
- I testi per l'agente (istruzione di General, riga di comando, skill), il Modello, i Requisiti di `tech`, `data` e `cli` e gli identificatori interni restano `General`.
- Nuovo ordine della barra: Board, Chat, Jobs, Requirements, Specs, History, Settings (anche Jobs sale, accanto a Chat).
- L'icona `MessageSquare` resta invariata.
