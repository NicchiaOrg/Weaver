---
technical: false
refs:
  - file-lock
  - omp.FILE_TOOL_STARTED
  - omp.FILE_TOOL_ENDED
  - requirements/ui#file-lock
  - requirements/tech#file-lock
  - requirements/rules#file-lock
  - requirements/quality#file-lock
---

# Blocco file dell'agente

Quando l'agente di un job, di General o di una conversazione Ask more del progetto sta cambiando un file aperto in una scheda della File view, la scheda diventa di sola lettura e l'albero segna il file. È solo un'indicazione: la protezione resta il controllo al salvataggio della scheda. Contano solo gli strumenti che cambiano file (modifiche, cancellazioni e spostamenti) mentre sono in corso; gli strumenti dei sub-agenti, i comandi che l'agente esegue, gli altri programmi e i percorsi fuori dal progetto non contano mai.

## Sorgente e strumenti che contano

- Sorgente: le notifiche ACP `session/update` dal vivo di ogni job, di General e di ogni conversazione Ask more del progetto. Le update rigiocate mentre una sessione si carica non contano mai. Le trascrizioni dei sub-agenti e lo strumento `task` non contano mai.
- Contano solo `tool_call` e `tool_call_update` con `kind` `edit`, `delete` o `move`. omp riporta come `edit` le sue modifiche, le scritture e le cancellazioni e gli spostamenti dentro un edit; `execute`, `other` e gli altri non contano mai.
- Inizio (`omp.FILE_TOOL_STARTED`): uno strumento conta dal suo `tool_call`. Con `locations` conta per quei percorsi; senza `locations` conta per ogni file aperto in una scheda, finché non finisce. È il blocco prudente deciso: nella modalità di edit predefinita di omp (`hashline`) il `tool_call` iniziale non ha `locations`. `locations` arrivate in update successive di uno strumento in corso aggiungono i loro percorsi.
- Fine (`omp.FILE_TOOL_ENDED`): un `tool_call_update` con status `completed` o `failed` termina lo strumento. Ogni strumento di una conversazione termina inoltre quando il suo turno finisce o fallisce, quando il suo omp esce o la sua connessione si chiude, quando il suo job finisce (Done, Failed, Stopped), quando General viene pulito e quando una conversazione Ask more si chiude. Uno strumento smette di contare una volta sola, al primo di questi eventi. La pulizia di General e la chiusura di Ask more contano a sé perché lì omp viene fermato senza un'uscita segnalata.
- Spostamenti: su un edit completato le `locations` finali contengono la destinazione; quando `rawOutput.details` del risultato nomina come sorgente (`sourcePath`) un file aperto in una scheda, il blocco rilasciato porta con sé la destinazione. Questi campi sono interni di omp: vanno ricontrollati a ogni aumento di `MIN_OMP_VERSION`, come i campi dei sub-agenti di `requirements/tech#job`; aggiungere il promemoria nel codice accanto a quello esistente.
- Percorsi: `locations[].path` sono assoluti. Si confrontano con i file aperti in forma canonica; un percorso che non esiste più si confronta come forma canonica del genitore più il suo nome. I percorsi fuori dalla radice canonica del progetto, worktree git compresi, si ignorano. Il confronto deve funzionare anche su Windows.
- Memoria: gli strumenti in corso di ogni conversazione restano nella memoria del backend anche a File view chiusa, senza lavoro su disco, così un file aperto durante uno strumento si apre già bloccato.

## Stati del blocco (per ogni file)

- Not tracked (iniziale): il file non è aperto in una scheda. All'apertura (`developer.OPEN_FILE`) va in Held se uno strumento che conta per quel file è già in corso, altrimenti in Free.
- Tracked: il file è aperto in una scheda; la chiusura della scheda (`file-tab.CLOSED`) lo riporta a Not tracked. Contiene Free e Held.
- Free (iniziale dentro Tracked): nessuno strumento conta per il file. Un `omp.FILE_TOOL_STARTED` che nomina il file, o che non nomina nessun file, lo porta a Held.
- Held: almeno uno strumento in corso conta per il file. Un nuovo `omp.FILE_TOOL_STARTED` che nomina il file o non nomina file conta anch'esso e resta Held. `omp.FILE_TOOL_ENDED` porta a Free se era l'ultimo strumento che contava per il file, altrimenti resta Held. Con `omp.TURN_ENDED`, `omp.TURN_FAILED` e `omp.EXITED` smettono di contare gli strumenti di quella conversazione; con `job.ENDED` quelli di quel job; con `developer.CLEAR_GENERAL` quelli di General; con `developer.CLOSE_ASK` quelli di quella conversazione Ask more. In tutti questi casi va in Free se nessuno strumento di un'altra conversazione conta ancora per il file, altrimenti resta Held.
- Eventi: `LOCKED` quando il blocco di un file aperto in una scheda passa da Free a Held; `RELEASED` quando passa da Held a Free, con il posto in cui l'agente ha spostato il file quando omp lo dice. La scheda (`file-tab`, altro Task) li usa per diventare Locked o Read-only, rileggere il file e seguirlo alla nuova posizione.

## Interfaccia

Lo stato del blocco arriva dal backend; l'interfaccia non lo ricava mai dalle conversazioni. Finché il blocco del file di una scheda è tenuto, il suo editor è di sola lettura e una riga sopra il testo, con l'icona `Lock` (`lucide-react`), dice `The agent is editing this file.`; la scheda e la riga dell'albero portano `Lock`. L'albero segna ogni file aperto in una scheda e bloccato, anche quando la sua scheda non è quella in vista. La riga è annunciata alle tecnologie assistive quando appare e quando sparisce (come le regioni `aria-live="polite"` già in uso).

## Affidabilità e test

Un blocco non sopravvive mai alla fine del suo strumento, al turno della sua conversazione, al suo processo omp, al suo job o alla sua conversazione. Test con l'agente ACP finto scriptato: strumenti con e senza `locations`, più strumenti su un file, strumenti di due conversazioni, fine per `completed`, `failed`, fine turno, uscita, fine job, General pulito e Ask more chiuso, update rigiocate durante il caricamento di una sessione, uno spostamento con la sua destinazione, una cancellazione, percorsi fuori dalla radice e su Windows.

## Da dove partire

- Update dal vivo e rigiocate: `src-tauri/src/jobs/actor.rs` e `src-tauri/src/general/actor.rs`, `Actor::incoming`: con `self.loading` le update vanno in `self.replay` (poi `Jobs::adopt_replay` / `General::session_opened`), altrimenti in `apply_update` della conversazione viva. Il blocco va alimentato solo dal ramo dal vivo, non da dentro `Conversation::apply_update`, che serve anche alla conversazione rigiocata e ai test dei sub-agenti. Le conversazioni Ask more girano sullo stesso attore di General (`Conversation::ask()` quando c'è `rebuild`) e stanno negli slot di `General` con `slot.ask`.
- Fine turno e uscita: `Actor::turn_done` (`TurnEnd::Ended` / `TurnEnd::Failed`) e `Actor::exited` / `Msg::Exited` nei due attori.
- Lettura degli strumenti: `src-tauri/src/jobs/conversation.rs`, `tool_of`, `Conversation::tool_call`, `Conversation::tool_update`, `tool_kind` (oggi fonde `edit|delete|move` in `ToolKind::Edit`, quindi il `kind` originale va letto dalla update), `status_of`, `looks_like_task_call` (strumento `task`). `locations` oggi non viene mai letto e `ui/src/bindings/Tool.ts` non le espone.
- Sub-agenti: le loro trascrizioni entrano in conversazioni proprie tramite `src-tauri/src/subagents/records.rs` (`apply`, `Conversation::tool_started` / `tool_finished`, che mappa anch'esso `edit|write|delete|move` in `ToolKind::Edit`): questa via non deve mai alimentare il blocco.
- Fine del job: `src-tauri/src/jobs/registry.rs`, `Jobs::dispatch_in`, `Effect::Ended` (Done, Stopped, Failed) e `Effect::ShutDown`; Machine in `crates/core/src/jobs/machine.rs`.
- Pulizia di General e chiusura Ask more: `src-tauri/src/general/service.rs`, `General::carry_out` (`Effect::Clear`, `Effect::ShutDown`, `Effect::Stopped`), `General::clear`, `General::close_ask` / `General::end_ask` (chiamato da `ask_close` in `src-tauri/src/ask/commands.rs`); Machine di General in `crates/core/src/jobs/general.rs`.
- Percorsi: `crates/core/src/paths.rs`, `canonical_root` (usa `dunce::canonicalize`, adatto a Windows).
- Promemoria su omp: `crates/core/src/jobs/subagents.rs` (commento del modulo sugli interni di omp da riverificare a ogni aumento di `MIN_OMP_VERSION`); costante in `crates/core/src/omp/version.rs`.
- Test: `src-tauri/src/jobs/testing.rs`, `fake_agent`, con lo script di `crates/fake-acp/src/main.rs` (passi `{"update": {...}}`, `replay`, `exit`, `hold`, `end`); test di General in `src-tauri/src/general/tests.rs`.
