---
status: approved
kind: change
base:
  model/actors/developer.md: 9287825acc241734004a2b35742d2725432b69a0de4d625d5626fe0fdb2c9194
  model/agent-work/ask-conversation.md: 0bdae25faae7f1bd914cc17a57838794c23f975807c1de0dd967b95c890fb9f4
  model/agent-work/feature.md: c97f668d0b0115a9074a0254bcee0d742d3a25c436f741149618c82b8173fddf
  requirements/rules/skills.md: facfc4f84032aa5d7fb7067efbd4538b45b4aebfcd28b9594e8c2656167a5984
  requirements/tech/ask-more.md: 537c1fe2626f4b2490ba25b235ff6191e06406d0c63b84c0b4fa03cb960c29d0
  requirements/ui/ask-more.md: bbfbf61897b5b2ed3f580450e77af4d742b6cc0f8bcadd82df88e18e7ec34d4a
  requirements/ui/views.md: 92e0b72daf88c026d817d78ea669fbcc8234da4829b83c9c572c0497bec6aa1c
created: 2026-11-03T06:42:26.897Z
request: 2026-11-03T07-38-07
label: Ask more History
job: 4tcays9b
---
# Ask more su voce History

La richiesta chiede che `Ask more` nella History si possa usare sull'intera voce, con una conversazione che riguarda la voce e la somma dei suoi Task.

## Decisioni

- `Ask more` si chiede solo sull'intera voce della History: il pulsante sul singolo Task nella parte espansa della riga sparisce, e con esso la conversazione su un solo Task della History.
- Il pulsante sta nel piede della finestra della voce, `outline`, come nelle finestre di dettaglio di Proposal, Task e Drift.
- L'istruzione porta, per ogni Task citato e nell'ordine di citazione, identità, titolo, corpo completo, riferimenti, Proposal, label e ora di fine; seguono i comandi di lettura senza ripetizioni.
- L'istruzione porta anche identità, titolo, riassunto e testo completo della voce.
- Un Task citato senza copia nella voce è letto dal suo file, come lo mostra la History; se non è leggibile è elencato con la sola identità come non disponibile. La conversazione si apre sempre quando la voce esiste.
- Nome nella vista Jobs: `Ask history <identità della voce>`.
- Testi: tooltip `Ask an agent about this History entry`, nome accessibile `Ask more about <identità della voce>`, segnaposto `Ask about the History entry…`.
