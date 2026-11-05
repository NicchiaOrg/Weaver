---
status: approved
kind: change
base:
  requirements/ui/conversations.md: 9914dc16bcdf09c63ad603edd0629b2857ef7aba259392ab1ee496354f837f68
created: 2026-11-03T07:53:58.160Z
request: 2026-11-03T08-41-34
label: Sub-agent message box
job: e3vuqhgl
---
# Barra disabilitata nei sub-agent

La conversazione di un sub-agent è di sola lettura e oggi non ha nessuna barra in basso, a differenza delle conversazioni di job, Chat e Ask more. Per coerenza visiva mostra comunque la barra per scrivere, disabilitata.

Decisioni:
- Vale in tutte e tre le viste dei sub-agent: job, Chat (General) e Ask more.
- Testo dedicato come segnaposto: `Sub-agents take no messages.`, uguale ovunque.
- Casella e `Send` disabilitati; nessuno `Stop`.
- Sempre la stessa barra disabilitata, anche quando il job, la Chat o l'Ask more a cui appartiene il sub-agent è finito (nessuna riga `This job has ended…`).
- La barra compare anche quando la conversazione non si può leggere (`Conversation not available: <reason>.`).
- Disabilitata davvero: fuori dall'ordine di tabulazione e annunciata come non disponibile, come la casella di un job in coda.

Il Model resta invariato: le conversazioni dei sub-agent restano di sola lettura.
