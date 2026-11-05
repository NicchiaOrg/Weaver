---
status: rejected
kind: change
base:
  requirements/tech/ask-more.md: 4ba903af9dc6d7682381e8ee4f9976f0b415b518287b1f536f69ffbb27987412
  requirements/ui/ask-more.md: 6011dbc5d8107c6c4555ef14a4b91180dd936633df81b00c0424d88e01258d62
  requirements/ui/views.md: d869c3f5c9c01e70dafb19ca462bcc9664b6a5f6812c3db030ab38daccc44c11
created: 2026-11-03T12:03:18.628Z
reworkRequested: true
label: Overlay
---
# Nascondere selettore overlay

Il selettore dell'overlay nelle barre di Requirements e Specs non è utile allo sviluppatore: va tolto. L'overlay di una Proposal resta, ma segue il focus aperto da `View requirements`.

## Decisioni

- Requirements e Specs non hanno più il selettore dell'overlay; la barra di Requirements contiene solo `Ask more`, Specs non ha più una barra.
- L'overlay è la Proposal dell'ultimo focus aperto, finché quel focus dura: `View requirements` su una Proposal che si può sovrapporre la disegna in entrambe le viste.
- Aprire un focus su un altro soggetto (un Task, un chip, una Proposal che non si può sovrapporre) in una qualsiasi delle due viste, o chiudere quel focus con `Clear` o Esc, riporta entrambe le viste a `Current Model`; un focus su un'altra Proposal sovrapponibile disegna quella.
- Se la Proposal smette di essere aperta, le viste tornano a `Current Model` con l'avviso di oggi; il focus resta.
- `Ask more` sui Requirements passa all'agente la Proposal in overlay come oggi.

## Assunzioni da verificare

- Lo sviluppatore ha chiesto la Proposal senza scegliere esplicitamente tra nascondere il selettore legando l'overlay al focus, togliere del tutto l'overlay o nasconderlo solo in Requirements: l'agente ha seguito la raccomandazione, cioè nascondere il selettore e legare l'overlay al focus.
- Il selettore sparisce anche dalla vista Specs, perché l'overlay è unico per finestra.

## Already present

- `requirements/tech#ask-conversation` — `src-tauri/src/ask/build.rs` (`requirements`) e `crates/core/src/project/view/mod.rs` (`eligible_overlays`): il backend controlla già che la Proposal in overlay sia aperta e non legata a un Drift; cambia solo la formulazione del Requirement.
