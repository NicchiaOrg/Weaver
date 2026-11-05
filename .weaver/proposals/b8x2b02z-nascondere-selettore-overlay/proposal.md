---
status: approved
kind: change
base:
  requirements/tech/ask-more.md: 4ba903af9dc6d7682381e8ee4f9976f0b415b518287b1f536f69ffbb27987412
  requirements/ui/ask-more.md: 6011dbc5d8107c6c4555ef14a4b91180dd936633df81b00c0424d88e01258d62
  requirements/ui/views.md: dbaacbce1e437b81729fbf5d86331a69e9ecd598b28a6ff83aa54f720d41b25a
created: 2026-11-03T13:15:07.504Z
reworks: qy5pbrd4
label: Overlay
job: 2sm74oan
---
# Nascondere selettore overlay

Il selettore dell'overlay nelle intestazioni di Requirements e Specs non è utile allo sviluppatore: va tolto. L'overlay di una Proposal resta, ma segue il focus aperto da `View requirements`.

Riscrive la Proposal respinta qy5pbrd4, senza feedback, sui Requirements attuali: le viste hanno ora un'intestazione comune (`requirements/ui#General`, View header), quindi i controlli stanno nell'intestazione invece che in una barra.

## Decisioni

- Requirements e Specs non hanno più il selettore dell'overlay; l'intestazione di Requirements contiene a destra solo `Ask more`, quella di Specs non ha controlli.
- L'overlay è la Proposal dell'ultimo focus aperto, finché quel focus dura: `View requirements` su una Proposal che si può sovrapporre la disegna in entrambe le viste.
- Aprire un focus su un altro soggetto (un Task, un chip, una Proposal che non si può sovrapporre) in una qualsiasi delle due viste, o chiudere quel focus con `Clear` o Esc, riporta entrambe le viste a `Current Model`; un focus su un'altra Proposal sovrapponibile disegna quella.
- Se la Proposal smette di essere aperta, le viste tornano a `Current Model` con l'avviso di oggi; il focus resta.
- `Ask more` sui Requirements passa all'agente la Proposal in overlay come oggi.

## Assunzioni da verificare

- Le decisioni sono quelle della Proposal respinta, che non ha feedback: l'agente le ha riprese senza un nuovo giro di domande.
- Il selettore sparisce anche dalla vista Specs, perché l'overlay è unico per finestra.
- L'intestazione di Specs resta, con il solo titolo, come per le altre viste senza controlli.

## Already present

- `requirements/tech#ask-conversation` — `src-tauri/src/ask/build.rs` (`requirements`) e `crates/core/src/project/view/mod.rs` (`eligible_overlays`): il backend controlla già che la Proposal in overlay sia aperta e non legata a un Drift; cambia solo la formulazione del Requirement.
