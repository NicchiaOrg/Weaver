---
status: approved
kind: change
base:
  model/actors/developer.md: e3c9bc9f14a977575bbda0f81c479b113fbf618db553d1f4c0a310e0c0e1fcd0
  model/agent-work/ask-conversation.md: e37b0e81690a14688c1f6f7841985291e7031aecd5a4a265b6fded9d825b7f7d
  model/agent-work/feature.md: 2f2ada5ad5585e94c0e3c79d8eced3ba19562523970c4e0ad9f8fea9ce1c7d42
  requirements/rules/skills.md: a9ea2f68ca4b63ac2385fbc2db265413fe0b0c83921816015e2b3c53050d3f8b
  requirements/tech/ask-more.md: 783ac81f2b5d8dc0d0483c091de4b0717a4ec39c6a706fde62999f5ceea8227b
  requirements/ui/ask-more.md: 144cc82bea62609c2446337103a7933c204a2abae30f9d3115d3e48fac95da2f
  requirements/ui/views.md: 89e5bb22317a1b4c4ea949dfa44bfcac91419585a04c445319836813bbda5d53
created: 2026-11-03T07:15:43.011Z
reworks: fghzub4m
label: Ask more Requirements
job: g1l7ogcq
---
# Ask more sui Requirements

La vista Requirements ha un solo `Ask more` per i Requirements nel loro insieme, invece di uno per sezione: l'agente sceglie da sé quali skill dei Requirement caricare in base alla domanda.

Riscrittura della Proposal respinta `fghzub4m`, senza feedback: stesso cambiamento, rifatto sul Model attuale. L'`Ask more` su una voce di History resta com'è oggi (sulla voce intera, nel piè del suo dialogo); la Proposal respinta partiva da un Model in cui era sul singolo Task.

## Decisioni

- I pulsanti `Ask more` sulle singole sezioni della vista Requirements sono rimossi.
- Un solo `Ask more` nella barra della vista Requirements, subito dopo il selettore dell'overlay; solo su Requirements, non su Specs; sempre visibile, anche con focus o `Only these`.
- L'istruzione dice che la domanda riguarda i Requirements, che sono nelle skill `trama-req-*`, e di caricare da sé quelle adatte; non elenca skill e non copia testi dei Requirements.
- Una Proposal in overlay è citata (identità, numero breve, titolo) con `weaver proposal show <id>`; se non è più aperta, la conversazione si apre sui Requirements correnti.
- Il focus e `Only these` non arrivano all'agente.
- Pulsante `ghost` piccolo con icona `MessageCircleQuestion` e testo `Ask more`.
- Testi: riga `Ask Requirements`; tooltip `Ask an agent about the Requirements`; nome accessibile `Ask more about the Requirements`; segnaposto `Ask about the Requirements…`.

## Assunzioni

- Quando una sessione persa viene sostituita, l'istruzione ricostruita cita la Proposal in overlay solo se è ancora aperta; un Ask more sui Requirements non resta mai senza istruzione.

## Already present

- `question-round/Presented/developer.ASK_MORE` — `ui/src/components/questions/question-pane.tsx:askMore`
