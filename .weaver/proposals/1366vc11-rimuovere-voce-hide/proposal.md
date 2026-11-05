---
status: rejected
kind: change
base:
  model/actors/developer.md: 751069e137e7ee8e704d9ebb814d1b12049d843fca303155a1a51e1dfbc22038
  model/app/window.md: e3c483e7efa27a61fd7774a2984970e30620894371fa2bbf2bea8de50a2cebaa
  requirements/tech/windows.md: 8833371533b37c53e407fe8c66477ae2ce9f4b22345876589fd71899cc1c3a13
  requirements/ui/look.md: ea46c74f03a91972b653f399e0f3b3b5f478c9e9c8865d01871b30d7c6239634
  requirements/ui/views.md: ee57995f67f015e2e67a118bf989a32302ad2e637a5e5c4abc685d79babdac42
created: 2026-11-04T06:26:05.678Z
request: 2026-11-04T07-18-10
reworkRequested: true
label: Hide
job: v4rykpiq
---
# Rimuovere voce Hide

La voce `Hide` in basso a sinistra della barra laterale va tolta del tutto, come chiesto dallo sviluppatore; la finestra deve però poter ancora andare in background quando la si chiude con dei job al lavoro.

Decisioni:
- La voce `Hide` sparisce dalla barra laterale senza sostituti (nessuna voce di menu, nessuna scorciatoia): una finestra va in background solo con `Keep working in background` del dialogo di chiusura, quando dei job lavorano e il File view non ha modifiche non salvate.
- Quando dal dialogo di chiusura la finestra non riesce ad andare in background, resta aperta e mostra il toast di errore `Could not hide the window.` con il motivo come descrizione (oggi l'errore finisce solo nel log).
- Scelta dell'agente, conseguenza della prima decisione: l'evento `developer.SEND_TO_BACKGROUND` esce dal Model, perché il dialogo di chiusura è già la transizione `developer.CLOSE_WINDOW` verso *In background*; il comando dell'interfaccia che la voce usava esce dal codice.
