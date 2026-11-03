---
status: approved
kind: change
base:
  requirements/ui/conversations.md: d5652915c329a31e3272517c540e9caa7998006ef829742ac98cad5a951ef4c7
created: 2026-11-02T20:18:13.134Z
reworks: bcxvs5vj
label: Padding lista job
job: rx5ypg9f
---
# Spazio sopra lista job

La prima riga della lista dei job tocca il bordo superiore della vista: si aggiunge spazio sopra di essa. Rielaborazione della Proposal rifiutata `bcxvs5vj` senza feedback: stessa modifica, riscritta sui requisiti attuali.

Decisioni:
- Il padding va solo sopra la prima riga della lista dei job; le righe, la conversazione e il riquadro `Weaver` non cambiano.
- Lo spazio è di 12 px, uguale allo spazio superiore della conversazione.
- Nessuno spazio aggiunto sotto l'ultima riga.
- Lo spazio fa parte della lista e scorre via con le righe.
- Il testo `No jobs yet` della lista vuota non cambia.
- La larghezza della lista resta fuori da questa Proposal: il codice la tiene fissa a 300 px mentre il requisito la vuole ridimensionabile 240–420 px; lo sviluppatore ha scelto di non toccarla qui.
