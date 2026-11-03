---
status: approved
kind: change
base:
  requirements/ui/board.md: a91a535e7c0d617147a0f590ccaa46640e32947dbca0516ba6cc7b3bc625fe34
  requirements/ui/look.md: 1d0a94d4c6284720dcf1ee716e11d3e42f01be52595a6df0a6b86fde182efabc
created: 2026-11-03T06:51:08.666Z
request: 2026-11-03T07-48-17
label: Controllo modello
job: htfirt4i
---
# Controllo modello a destra

Il controllo che cambia il modello di una Proposal passa in fondo alla riga dei controlli della card, allineato al bordo destro, e usa l'icona delle impostazioni.

## Decisioni
- Card: il controllo del modello è l'ultimo della riga dei controlli, dopo `View requirements`, spinto al bordo destro della card anche quando la riga va a capo; gli altri controlli restano a sinistra.
- Dialogo dei dettagli: stesso ordine della card, quindi il controllo del modello è l'ultimo dei controlli della card e `Ask more` resta l'ultimo elemento a destra.
- Icona del controllo: `Settings` (ingranaggio), la stessa della voce Settings della barra laterale.
- Badge del modello scelto: resta con l'icona `Cpu`.
