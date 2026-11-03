---
status: rejected
kind: change
base:
  requirements/ui/conversations.md: 4b7fe46e55e7c0a46b1800ca9951093cce44fd87359095677420300314758d82
created: 2026-11-02T19:43:41.827Z
request: 2026-11-02T20-39-37
reworkRequested: true
label: Padding lista job
job: zbv09h5q
---
# Spazio sopra lista job

La prima riga della lista dei job tocca il bordo superiore della vista: si aggiunge spazio sopra di essa.

Decisioni:
- Il padding va solo sopra la prima riga della lista dei job; le righe, la conversazione e il riquadro `Weaver` non cambiano.
- Lo spazio è di 12 px, uguale al margine superiore della conversazione.
- Nessuno spazio aggiunto sotto l'ultima riga.
- Lo spazio fa parte della lista e scorre via con le righe.
