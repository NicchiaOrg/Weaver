---
technical: false
refs:
  - agent-work
  - developer.ASK_MORE
  - requirements/rules#ask-conversation
  - requirements/tech#ask-conversation
  - requirements/ui#ask-conversation
  - requirements/ui#history-entry
---
# Ask more su voce History

`Ask more` nella History si chiede sull'intera voce, non più sul singolo Task: la conversazione riguarda la voce e tutti i Task che cita. Il pulsante sul singolo Task nella parte espansa della riga sparisce, e con esso il tipo di conversazione su un solo Task della History (nessun alias o percorso rimasto).

Quando il lavoro è finito vale quanto segue.

**Apertura (`ask-conversation/Not opened`, `developer.ASK_MORE`)**
- Nella finestra di una voce della History, il piede contiene un `Button` `outline` con l'icona `MessageCircleQuestion` a sinistra del testo `Ask more`, sempre abilitato. Tooltip `Ask an agent about this History entry`; nome accessibile `Ask more about <identità della voce>`. Le righe dei Task, espanse, mostrano corpo e riferimenti in sola lettura, senza `Ask more`.
- Premendolo, la finestra mostra la vista Jobs con la nuova conversazione selezionata e il focus nella sua casella dei messaggi; la finestra della voce resta com'era e si ritrova com'era tornando alla History. Non parte omp e non si invia nulla.
- La UI invia solo l'identità della voce. Il backend legge la voce e le copie dei Task citati dal file della voce; un Task citato senza copia è letto dal suo file come fa la vista History. Solo una voce che non esiste più non apre nulla; un Task citato non leggibile non impedisce mai l'apertura.
- Nome nella vista Jobs: `Ask history <identità della voce>` (per esempio `Ask history k3m9x2qa`). Segnaposto della conversazione vuota: `Ask about the History entry…` (serve un tipo di conversazione proprio, non quello del Task).

**Istruzione (`requirements/rules#ask-conversation`)**
- La parte che descrive la cosa scelta contiene identità, titolo, riassunto e testo completo della voce; poi ogni Task citato, nell'ordine di citazione, dalla sua copia: identità, titolo, corpo completo, riferimenti, Proposal, label e ora di fine. Un Task senza copia è letto dal suo file come lo mostra la History; uno non leggibile è elencato con la sola identità e marcato come non disponibile.
- Comandi di lettura dopo la parte: `weaver history show <voce>`, poi `weaver proposal show <id>` una volta per ogni Proposal dei Task che esiste ancora e `weaver model show <ref>` una volta per ogni riferimento dei Task, senza ripetizioni.
- Quando una sessione persa viene sostituita, l'istruzione è ricostruita dai file in quel momento come per gli altri tipi; se la voce non esiste più il messaggio va da solo.

**Dove nel codice (dall'esplorazione)**
- `src-tauri/src/ask/build.rs` — `AskTarget::History { entry, task }` (riga ~42), il ramo `History` di `build` (~148, nome `Ask {task}`, `AskKind::Task`), `history_task()` (~265): da sostituire con una variante solo sulla voce e un lettore dell'intera voce. La stessa logica copia-o-file-del-Task esiste anche in `crates/cli/src/commands/history.rs:task_json` e `src-tauri/src/history.rs:entry_of`/`task_of`.
- `crates/core/src/jobs/ask.rs` — `Chosen`, `HistoryTaskPart`, `history_part()`, `instruction()`.
- `src-tauri/src/general/wire.rs` — `AskKind` (nuovo valore per la voce); `ui/src/components/ask/ask-model.ts:PLACEHOLDERS`.
- `ui/src/bindings/AskTarget.ts`, `AskKind.ts` — generati con ts-rs.
- `ui/src/components/history/history-view.tsx` — `TaskRow` (pulsante per Task da togliere), `EntryDialog` (usa `FormDialog`, che accetta `footer`); `ui/src/components/ask/ask-button.tsx:AskMoreButton` è il pulsante `outline` dei dettagli della Board.
- Test da aggiornare: `ui/src/components/history/history-view.test.tsx` (~280-309), `ui/src/components/ask/ask.test.tsx` (segnaposto, ~177-190), `src-tauri/src/ask/tests.rs` (~262-300, fixture `HISTORY` con un solo Task citato), test puri in `crates/core/src/jobs/ask.rs` (~832-885).
