---
status: approved
kind: change
base:
  requirements/ui/board.md: 69e79b08476943542074713edb4a0b92fb677cbb708d11ac21803ced9983b2c9
  requirements/ui/look.md: a487c92d51a33b061c7e10497543fcebf8b2863908b7a101e5082e31cf9f721f
created: 2026-11-02T16:00:50.078Z
request: 2026-11-02T16-56-44
label: Refine button
job: 8qxeqd4s
---
# Pulsante Refine giallo

# Pulsante Refine giallo

Quando l'unico blocco di una Proposal è la staleness, il clic sul pulsante di approvazione non approva subito: avvia un job dell'agente che affina la Proposal sul Model attuale. Oggi il pulsante resta `Approve` blu e lo dice solo il tooltip. Questa Proposal lo fa diventare `Refine`, giallo e con un'icona che indica il lavoro dell'agente, così si capisce che parte un'operazione AI.

Decisioni dell'intervista:

- Il testo del pulsante è `Refine`, anche come nome accessibile.
- L'affinamento resta com'è: se alla fine non resta nessun altro blocco, la Proposal viene approvata da sola. Il tooltip `Refine on the current Model, then approve` resta.
- Lo stile è giallo pieno con testo scuro: sfondo `yellow-500` nel tema chiaro e `yellow-400` nel tema scuro, testo `zinc-950`, hover un tono più scuro.
- L'icona è `Sparkles` al posto di `Check`.
- Il pulsante della Draft con affinamento interrotto diventa lo stesso `Refine` giallo; il tooltip `Continue refining, then approve` resta.
- Una Proposal stale con altri blocchi mostra `Approve` disabilitato, con tutti i motivi nel tooltip.
- Nessuna conferma prima di avviare il job.
- Il Model non cambia: cambiano solo l'aspetto e il testo del controllo.
