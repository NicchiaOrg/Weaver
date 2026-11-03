---
status: approved
kind: change
base:
  requirements/ui/look.md: 3c18efab7081118a239a17d97f64a0285a7718ce2880d04985f666eb3fd701d0
created: 2026-11-03T11:46:48.270Z
request: 2026-11-03T12-45-07
label: Hide
job: angup3ea
---
# Etichetta "Hide"

La voce della barra laterale che nasconde la finestra di un progetto si chiama ora `Hide` invece di `Run in background`, più breve e coerente con l'icona `EyeOff`.

Decisioni:
- La voce della barra laterale si chiama `Hide` (anche nome accessibile e tooltip con la barra stretta); icona `EyeOff` e comportamento invariati.
- Il toast di errore quando la finestra non si riesce a nascondere diventa `Could not hide the window.`, con il motivo come descrizione.
- Il pulsante `Keep working in background` della conferma di chiusura resta com'è.
- Model invariato: l'Event `developer.SEND_TO_BACKGROUND` e lo State `window/In background` mantengono i loro nomi.
