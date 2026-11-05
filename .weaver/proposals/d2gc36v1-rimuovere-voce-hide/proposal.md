---
status: approved
kind: change
base:
  model/actors/developer.md: 8b008ee1059ce932e4faa9f37c5a6f62fa8f364d52224a2544d58cf2cce5a121
  model/app/window.md: e3c483e7efa27a61fd7774a2984970e30620894371fa2bbf2bea8de50a2cebaa
  requirements/tech/windows.md: 8833371533b37c53e407fe8c66477ae2ce9f4b22345876589fd71899cc1c3a13
  requirements/ui/look.md: a1d07cd3239adffdad75678e0591946ac73f2dbdb0f57455062c83270aba19e8
  requirements/ui/views.md: ee57995f67f015e2e67a118bf989a32302ad2e637a5e5c4abc685d79babdac42
created: 2026-11-04T06:31:55.450Z
reworks: 1366vc11
label: Hide
job: j89d99gt
---
# Rimuovere voce Hide

La voce `Hide` in basso a sinistra della barra laterale va tolta del tutto, come chiesto dallo sviluppatore; la finestra deve però poter ancora andare in background quando la si chiude con dei job al lavoro. Rifacimento della Proposal respinta `1366vc11`, che non aveva feedback: stessa modifica, riscritta sul Model attuale (che nel frattempo ha aggiunto la vista Assets e i tag degli asset).

Decisioni:
- La voce `Hide` sparisce dalla barra laterale senza sostituti (nessuna voce di menu, nessuna scorciatoia): una finestra va in background solo con `Keep working in background` del dialogo di chiusura, quando dei job lavorano e il File view non ha modifiche non salvate.
- Quando dal dialogo di chiusura la finestra non riesce ad andare in background, resta aperta e mostra il toast di errore `Could not hide the window.` con il motivo come descrizione (oggi l'errore finisce solo nel log).
- Scelta dell'agente, conseguenza della prima decisione: l'evento `developer.SEND_TO_BACKGROUND` esce dal Model, perché il dialogo di chiusura è già la transizione `developer.CLOSE_WINDOW` verso *In background*; il comando dell'interfaccia che la voce usava esce dal codice.
