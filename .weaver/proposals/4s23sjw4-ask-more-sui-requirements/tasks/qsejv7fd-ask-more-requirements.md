---
technical: false
refs:
  - agent-work
  - developer.ASK_MORE
  - ask-conversation
  - requirements/ui#app
  - requirements/ui#ask-conversation
  - requirements/rules#ask-conversation
  - requirements/tech#ask-conversation
---
# Ask more unico sui Requirements

La vista Requirements passa da un `Ask more` per ogni sezione a un solo `Ask more` sui Requirements nel loro insieme. L'agente non riceve più il testo di una sezione: sa che i Requirements stanno nelle Requirement skill `trama-req-*` del progetto e carica da sé quelle adatte alla domanda. `Ask more` su card, domande e voci di History non cambia.

Quando tutto è fatto:

- **Toolbar della vista Requirements:** subito dopo l'overlay picker c'è un `Ask more`, un piccolo `Button` `ghost` con l'icona `MessageCircleQuestion` e il testo `Ask more`; tooltip `Ask an agent about the Requirements`, nome accessibile `Ask more about the Requirements`. È visibile sempre, anche con un focus o `Only these` attivi e con qualsiasi overlay. Le intestazioni di sezione, frammento e Area non hanno alcun `Ask more` (la sezione mostra solo titolo e `Open in editor`). La toolbar della vista Specs ha solo l'overlay picker.
- **Premere il pulsante:** apre la conversazione nella vista Jobs, la mostra e mette il focus nella sua casella di messaggio; non invia nulla e non avvia omp. Focus, `Only these`, scroll e overlay della vista Requirements restano come erano. La riga in Jobs si chiama `Ask Requirements` e il placeholder della casella è `Ask about the Requirements…`.
- **Apertura:** per i Requirements la conversazione si apre sempre (stato Empty). La UI manda solo l'overlay Proposal in vista, se c'è; mai sezione, Area, frammento, focus o `Only these`. Il backend controlla che la Proposal sia ancora aperta con lo stesso criterio dell'overlay picker (gli eligible overlays: aperta e non legata a una Drift); se non lo è, costruisce l'istruzione senza di essa e apre comunque. Il backend non legge testo dei Requirements né skill.
- **Istruzione, parte Requirements:** dice che lo sviluppatore chiede dei Requirements del progetto nel loro insieme; che stanno nelle Requirement skill `trama-req-*` installate nel progetto, che contengono i Requirements correnti; e che l'agente carica da sé quelle adatte al messaggio, quante gliene servono. Non elenca skill, non nomina Area, frammento o sezione e non copia testo dei Requirements. Con un overlay ancora aperto cita identità, short number e titolo della Proposal, dice che la vista mostra i Requirements con quella Proposal applicata e che le sue modifiche si leggono con `weaver proposal show <id>`, non nelle skill; è l'unico comando di lettura elencato. Senza overlay (o con un overlay non più aperto) non è elencato alcun comando e la parte parla solo dei Requirements correnti.
- **Altre parti dell'istruzione:** continuano a non nominare alcuna skill; l'unica eccezione ammessa è il richiamo alle Requirement skill della parte Requirements. Nessun meccanismo inietta le skill: la riga di comando di omp resta invariata e omp, avviato nella radice del progetto, trova da sé le `trama-req-*` installate.
- **Sessione persa:** se il caricamento della sessione fallisce e la si rimpiazza, l'istruzione ricostruita dai file di quel momento cita l'overlay solo se è ancora aperto; una conversazione Ask more sui Requirements non resta mai senza istruzione.
- **Altri Ask more:** card (Proposal, Task, Drift), domande dei round e voci di History restano come sono; in particolare la voce di History resta sull'intera voce, dal footer del suo dialogo.
- **Pulizia:** il vecchio tipo per sezione dei Requirements sparisce ovunque: target e kind nel backend e nella UI, la parte di istruzione per sezione (`section_part`, `SectionPart`, `SectionChange`, `Chosen::Section`), la ricerca della sezione su `project_view::snapshot`, il nome `Ask {area}#{section}`, il placeholder `Ask about the Requirement…`. I binding TypeScript (`AskTarget.ts`, `AskKind.ts`, `AskRow.ts`) sono rigenerati con ts-rs, non modificati a mano. I commenti superati (doc di modulo e di `Built`, commento in `requirements-view.tsx`, commento di `BoardActions.askMore`) sono aggiornati. I test del comportamento per sezione sono sostituiti da test del nuovo comportamento (pulsante in toolbar solo in Requirements e visibile con focus/`Only these`, nessun pulsante nelle intestazioni, placeholder, apertura con overlay aperto, con overlay non più aperto e senza overlay, testo dell'istruzione). La riga del CHANGELOG che parla di "a section of the Requirements" parla dei Requirements nel loro insieme.

Punti del codice noti dall'esplorazione:

- `requirements/ui#app` — ui/src/components/views/requirements-view.tsx:SectionBlock
- `requirements/ui#app` — ui/src/components/views/requirements-view.tsx:RequirementsView
- `requirements/ui#app` — ui/src/components/views/view-frame.tsx:ViewFrame
- `requirements/ui#app` — ui/src/components/views/overlay-picker.tsx:OverlayPicker
- `requirements/ui#app` — ui/src/components/views/specs-view.tsx:SpecsView
- `requirements/ui#app` — ui/src/components/board/board-ui.tsx:BoardActions
- `requirements/ui#ask-conversation` — ui/src/components/ask/ask-model.ts:PLACEHOLDERS
- `requirements/ui#ask-conversation` — ui/src/components/ask/use-asks.tsx:askMore
- `requirements/ui#ask-conversation` — ui/src/components/jobs/board-actions.tsx:JobsBoardActions
- `requirements/ui#ask-conversation` — ui/src/components/jobs/jobs-view.tsx:AskListRow
- `requirements/ui#ask-conversation` — ui/src/components/ask/ask-conversation.tsx:AskConversation
- `requirements/tech#ask-conversation` — src-tauri/src/ask/build.rs:AskTarget
- `requirements/tech#ask-conversation` — src-tauri/src/ask/build.rs:requirement
- `requirements/tech#ask-conversation` — src-tauri/src/ask/build.rs:Built
- `requirements/tech#ask-conversation` — src-tauri/src/ask/mod.rs:open
- `requirements/tech#ask-conversation` — src-tauri/src/general/service.rs:Rebuild
- `requirements/tech#ask-conversation` — src-tauri/src/general/actor.rs:load_answered
- `requirements/tech#ask-conversation` — src-tauri/src/general/wire.rs:AskKind
- `requirements/tech#ask-conversation` — crates/core/src/project/view/mod.rs:eligible_overlays
- `requirements/tech#ask-conversation` — ui/src/bindings/AskTarget.ts:AskTarget
- `requirements/rules#ask-conversation` — crates/core/src/jobs/ask.rs:INSTRUCTION
- `requirements/rules#ask-conversation` — crates/core/src/jobs/ask.rs:section_part
- `requirements/rules#ask-conversation` — crates/core/src/jobs/ask.rs:Chosen
- `requirements/rules#ask-conversation` — crates/core/src/project_files/requirement_skills.rs:SKILL_PREFIX
- `ask-conversation` — src-tauri/src/ask/tests.rs:an_overlay_proposal_that_is_not_open_opens_nothing
- `ask-conversation` — ui/src/components/views/requirements-view.test.tsx:RequirementsView
- `agent-work` — CHANGELOG.md:Unreleased
