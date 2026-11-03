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

Nella vista Requirements l'Ask more non è più per sezione: c'è un solo `Ask more` per i Requirements nel loro insieme, e l'agente carica da sé le skill dei Requirement adatte alla domanda.

Quando il Task è finito deve valere quanto segue.

**Vista Requirements (`requirements/ui#app`, `requirements/ui#ask-conversation`)**
- Nessuna intestazione di sezione, di frammento o di Area ha più un `Ask more`; l'intestazione di sezione mostra solo titolo e `Open in editor`.
- La barra della vista Requirements mostra, subito dopo il selettore dell'overlay, un `Button` `ghost` piccolo con icona `MessageCircleQuestion` e testo `Ask more`, sempre abilitato e visibile anche con un focus, con `Only these` e con qualsiasi overlay. Tooltip `Ask an agent about the Requirements`; nome accessibile `Ask more about the Requirements`. La vista Specs, che condivide la stessa barra, non ha questo pulsante.
- Premuto, la finestra mostra la vista Jobs con la nuova conversazione selezionata e il focus nella sua casella del messaggio; la vista Requirements (overlay, focus, `Only these`, posizione di scorrimento) resta com'era.
- La riga nella vista Jobs si chiama `Ask Requirements`; da vuota la conversazione ha il segnaposto `Ask about the Requirements…`.

**Apertura e istruzione (`ask-conversation`, `developer.ASK_MORE`, `requirements/tech#ask-conversation`, `requirements/rules#ask-conversation`)**
- `developer.ASK_MORE` sui Requirements porta sempre a Empty: i Requirements esistono sempre. La UI invia solo la Proposal in overlay in vista, se c'è, mai una sezione, un'Area o il focus.
- Il backend controlla che la Proposal in overlay sia ancora aperta, come fa il selettore dell'overlay; se non lo è, l'istruzione viene costruita senza di essa e la conversazione si apre comunque sui Requirements correnti.
- La parte **Requirements** dell'istruzione dice che lo sviluppatore chiede dei Requirements del progetto nel loro insieme; che stanno nelle skill dei Requirement `trama-req-*` installate nel progetto, che contengono i Requirements correnti; e che l'agente carica da sé quelle adatte al messaggio dello sviluppatore, quante ne servono. Non elenca skill, non nomina Area, frammento o sezione e non copia testo dei Requirements; il focus e `Only these` non arrivano mai all'agente.
- Con una Proposal in overlay ancora aperta, la parte ne riporta identità, numero breve e titolo, dice che la vista mostra i Requirements con quella Proposal applicata e che le sue modifiche si leggono con `weaver proposal show <id>`, non nelle skill; questo è l'unico comando di lettura elencato. Senza overlay non è elencato alcun comando.
- La regola che l'istruzione di Ask more non nomina skill vale ancora per tutte le altre parti; solo la parte Requirements nomina le skill `trama-req-*`. Le skill non vengono iniettate in alcun modo: omp, avviato nella radice del progetto, le trova da solo come in ogni job; la riga di comando di omp per Ask more non cambia.
- Quando il ricaricamento della sessione fallisce, il messaggio che apre la sessione nuova porta l'istruzione ricostruita dai file in quel momento, con la Proposal in overlay solo se è ancora aperta; per i Requirements l'istruzione c'è sempre.
- Le conversazioni Ask more su Proposal, Task, Drift, Task di una voce di History e domande non cambiano.

**Pulizia**
- Il vecchio tipo per sezione sparisce del tutto: niente riferimento di sezione nella richiesta di apertura, niente ricerca della sezione nella vista ricalcolata, niente parte d'istruzione con testo, Area, frammento o modifica della sezione, e i test relativi vengono sostituiti da test sul nuovo comportamento (pulsante unico nella barra e nessuno nelle intestazioni; istruzione senza testo dei Requirements e con le skill `trama-req-*`; overlay aperto citato, overlay chiuso omesso con apertura comunque riuscita).

Punti del codice noti dall'esplorazione:
- `requirements/ui#app` — `ui/src/components/views/requirements-view.tsx:SectionBlock` (pulsante per sezione da rimuovere, chiamata `askMore` con `{kind:"requirement", reference, overlay}`), `ui/src/components/views/view-frame.tsx:ViewFrame` (prop `toolbar` mai usata da Requirements e Specs), `ui/src/components/views/overlay-picker.tsx:OverlayPicker`
- `requirements/ui#ask-conversation` — `ui/src/components/ask/ask-model.ts:PLACEHOLDERS`, `ui/src/components/ask/use-asks.tsx:askMore`, `ui/src/components/jobs/board-actions.tsx:JobsBoardActions`
- `requirements/tech#ask-conversation` — `src-tauri/src/ask/build.rs:AskTarget::Requirement` e `requirement` (nome `Ask {area}#{section}`, ricalcolo con `project_view::snapshot`), `src-tauri/src/ask/mod.rs:open` (closure `Rebuild`), `src-tauri/src/general/actor.rs:load_answered`, `src-tauri/src/general/wire.rs:AskKind`, binding `ui/src/bindings/AskTarget.ts` e `AskKind.ts`
- `requirements/rules#ask-conversation` — `crates/core/src/jobs/ask.rs:INSTRUCTION`, `section_part`, `SectionPart`, `Overlay`, `SectionChange`; il test del testo fisso verifica oggi che non contenga `skill`
- Test — `ui/src/components/views/requirements-view.test.tsx`, `ui/src/components/ask/ask.test.tsx` (segnaposto), `src-tauri/src/ask/tests.rs` (tre test del tipo Requirement), test di `section_part` in `crates/core/src/jobs/ask.rs`
