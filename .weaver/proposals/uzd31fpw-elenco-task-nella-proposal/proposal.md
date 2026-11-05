---
status: approved
kind: change
base:
  requirements/ui/board.md: 939e72492cf0a56200ef3e3080054d12003df684b0bf21036f97a0b6b8f5b6bf
created: 2026-11-02T15:45:35.932Z
request: 2026-11-02T16-39-50
label: Proposal Tasks list
job: q97zb1pp
---
# Elenco Task nella Proposal

La finestra dei dettagli di una Proposal elenca i Task che crea con i riferimenti come chip monospace e il badge `technical`: l'elenco è affollato e i nomi si leggono male. La richiesta chiede di togliere i tag e mostrare meglio i nomi.

## Decisioni

- Cambia solo la finestra dei dettagli della Proposal; la finestra delle voci di History resta com'è.
- Nell'elenco dei Task niente riferimenti come chip: restano solo i nomi.
- Nell'elenco niente badge `technical`.
- Ogni Task di `Tasks it creates` è una riga cliccabile con il solo titolo.
- Un clic su una riga apre una sotto-finestra sopra quella della Proposal, con il titolo del Task e il suo testo reso in Markdown, senza riferimenti né badge; Esc o `Close` la chiudono e il focus torna alla riga.
- I Task di rimozione del codice restano nella sezione `Code-removal Tasks`, con lo stesso stile di elenco, come righe semplici non cliccabili; il riferimento nel titolo è mostrato come codice invece che con gli apici inversi.

## Ipotesi dell'agente

- Un Task con testo vuoto mostra `No description.` nella sotto-finestra, come già fa il testo vuoto di una Proposal.
- Il Model non cambia: la modifica riguarda solo la sezione `requirements/ui#proposal`.
