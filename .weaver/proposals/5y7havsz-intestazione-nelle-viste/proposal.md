---
status: approved
kind: change
base:
  requirements/ui/board.md: d17b8955af5f53906c76d82e29cc4a550266e23d30ed9c70ca7a0c11ea72a015
  requirements/ui/conversations.md: 7b1d443c091a0a6c8b06c409a97fda14dc8f478cf28a60b9220b0e7ccdd928fb
  requirements/ui/files.md: 774a155a5e1690f2ed1407f87457281a674472c25fbf22746a8a5c09801b82d2
  requirements/ui/look.md: 4380581256a49fe69c060a6edc8f3597c1447e5eb9bd79b4c5d96dfa85dc87a5
  requirements/ui/setup.md: f89e740cffc824c71d84d73958a343c42c91557db5e2f16f3adef0e4cad4dca4
  requirements/ui/views.md: d869c3f5c9c01e70dafb19ca462bcc9664b6a5f6812c3db030ab38daccc44c11
created: 2026-11-03T12:09:11.021Z
request: 2026-11-03T13-01-53
label: View header
job: wcmn3vi8
---
# Intestazione nelle viste

Chat ha in alto un'intestazione con il titolo `Chat`; le altre viste no. La Proposal dà a ogni vista la stessa intestazione con il suo nome, al posto dello spazio vuoto o della barra senza titolo.

Decisioni:
- Tutte le viste ricevono l'intestazione: Board, Chat, Jobs, Files, Requirements, Specs, History e Settings.
- Dove c'è già una barra di controlli (Board, History, Requirements, Specs) il titolo entra nella stessa riga a sinistra e i controlli esistenti vanno a destra, come `Clear` in Chat.
- In Jobs e Files l'intestazione occupa tutta la larghezza, sopra entrambi i pannelli; le barre interne (conversazione di un sub-agent, schede) restano sotto.
- Tutte le intestazioni hanno la stessa altezza fissa, quella di Chat, anche senza controlli.
- L'intestazione si vede anche durante il caricamento e in caso di errore, ma non al posto di Get Started.
- Il titolo è solo testo, senza icona.
- Il titolo visibile diventa l'intestazione principale della vista e sostituisce il titolo nascosto per gli screen reader.
- L'intestazione resta ferma in alto mentre il contenuto scorre, anche in Settings.
- In Settings i titoli dei gruppi scendono a 14 px, per non superare il titolo della vista.
