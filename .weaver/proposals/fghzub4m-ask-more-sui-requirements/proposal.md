---
status: rejected
kind: change
base:
  model/actors/developer.md: 9287825acc241734004a2b35742d2725432b69a0de4d625d5626fe0fdb2c9194
  model/agent-work/ask-conversation.md: 0bdae25faae7f1bd914cc17a57838794c23f975807c1de0dd967b95c890fb9f4
  model/agent-work/feature.md: c97f668d0b0115a9074a0254bcee0d742d3a25c436f741149618c82b8173fddf
  requirements/rules/skills.md: facfc4f84032aa5d7fb7067efbd4538b45b4aebfcd28b9594e8c2656167a5984
  requirements/tech/ask-more.md: 537c1fe2626f4b2490ba25b235ff6191e06406d0c63b84c0b4fa03cb960c29d0
  requirements/ui/ask-more.md: bbfbf61897b5b2ed3f580450e77af4d742b6cc0f8bcadd82df88e18e7ec34d4a
  requirements/ui/views.md: 92e0b72daf88c026d817d78ea669fbcc8234da4829b83c9c572c0497bec6aa1c
created: 2026-11-03T07:08:59.183Z
request: 2026-11-03T07-58-08
reworkRequested: true
label: Ask more Requirements
job: szf6cv3p
---
# Ask more sui Requirements

# Ask more sui Requirements

La vista Requirements ha un solo `Ask more` per i Requirements nel loro insieme, invece di uno per sezione: l'agente sceglie da sé quali skill dei Requirement caricare in base alla domanda.

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
