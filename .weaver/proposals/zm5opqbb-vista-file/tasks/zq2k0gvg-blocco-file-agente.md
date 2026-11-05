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

Quando l'agente di un job, di General o di una conversazione Ask more del progetto sta modificando un file aperto in una scheda della File view, la scheda diventa di sola lettura e il file appare segnato nell'albero. È solo un'indicazione: la protezione resta il controllo al salvataggio della scheda.

## Sorgente e strumenti che contano

- Il backend legge le notifiche ACP `session/update` dal vivo di ogni job, di General e di ogni conversazione Ask more (che gira sull'attore di General). Le update rigiocate mentre una sessione si carica (`self.replay`) non contano mai, così come le trascrizioni dei sub-agenti e lo strumento `task`.
- Contano solo `tool_call` e `tool_call_update` con `kind` `edit`, `delete` o `move`; `execute`, `other` e gli altri mai. omp riporta come `edit` modifiche, scritture, cancellazioni e spostamenti dentro un edit.
- Inizio (`omp.FILE_TOOL_STARTED`): uno strumento conta dal suo `tool_call`. Con `locations` conta per quei percorsi; senza conta per ogni file aperto in una scheda finché non finisce (blocco prudente: nella modalità di edit predefinita di omp, `hashline` e anche `apply_patch`, il `tool_call` iniziale non ha `locations`, che arrivano solo nell'ultimo `tool_call_update`). `locations` arrivate in update successive di uno strumento in corso aggiungono i loro percorsi.
- Fine (`omp.FILE_TOOL_ENDED`): un `tool_call_update` con status `completed` o `failed` termina lo strumento. Ogni strumento di una conversazione termina inoltre, una volta sola e al primo di questi eventi: fine o fallimento del turno, uscita di omp o chiusura della connessione, fine del job (Done, Failed, Stopped), pulizia di General, chiusura della conversazione Ask more.
- Spostamenti: su un edit completato le `locations` finali contengono la destinazione; se `rawOutput.details.sourcePath` nomina come sorgente un file aperto in una scheda, il rilascio del blocco porta con sé la destinazione (evento `RELEASED`, usato dalla scheda per seguire il file). Questi campi sono interni di omp: vanno ricontrollati a ogni aumento di `MIN_OMP_VERSION`, come i campi dei sub-agenti di `requirements/tech#job`; aggiungere il promemoria accanto a quello esistente.
- Percorsi: `locations[].path` sono assoluti e non risolti dai symlink. Si confrontano con i file aperti in forma canonica; un percorso che non esiste più si confronta come forma canonica del genitore più il nome. I percorsi fuori dalla radice canonica del progetto, worktree git compresi, si ignorano. Deve funzionare anche su Windows.
- Memoria: gli strumenti in corso di ogni conversazione restano in memoria nel backend anche a File view chiusa, senza lavoro su disco, così un file aperto durante uno strumento si apre già bloccato.

## Stati del blocco (per ogni file)

- Non tracciato: il file non è aperto in una scheda. All'apertura (`developer.OPEN_FILE`) va in Bloccato se uno strumento che conta per quel file è già in corso, altrimenti in Libero. La chiusura della scheda (`file-tab.CLOSED`) lo riporta a Non tracciato.
- Libero: nessuno strumento conta; un `FILE_TOOL_STARTED` che nomina il file o non nomina file lo porta a Bloccato ed emette `LOCKED`.
- Bloccato: almeno uno strumento conta. Un nuovo strumento conta anch'esso. A ogni fine (strumento, turno finito o fallito, uscita, fine del job, pulizia di General, chiusura Ask more) torna Libero ed emette `RELEASED` solo se nessuno strumento, anche di un'altra conversazione, conta ancora per il file; altrimenti resta Bloccato.

## Interfaccia

Lo stato arriva dal backend; l'interfaccia non lo ricava mai dalle conversazioni. Finché il blocco è tenuto l'editor è di sola lettura e una riga sopra il testo, con l'icona `Lock`, dice `The agent is editing this file.`; la scheda e la riga dell'albero portano `Lock`. La riga è annunciata alle tecnologie assistive quando appare e quando sparisce.

## Affidabilità e test

Un blocco non sopravvive mai alla fine del suo strumento, al turno della sua conversazione, al suo processo omp, al suo job o alla sua conversazione. Test con l'agente ACP finto scriptato: strumenti con e senza `locations`, più strumenti su un file, strumenti di due conversazioni, fine per `completed`, `failed`, fine turno, uscita, fine job, General pulito e Ask more chiuso, update rigiocate durante il caricamento, uno spostamento con destinazione, una cancellazione, percorsi fuori dalla radice e su Windows.

## Da dove partire

- `src-tauri/src/jobs/actor.rs` e `src-tauri/src/general/actor.rs`: `Actor::incoming` (update dal vivo vs `self.replay`), `Actor::turn_done`, `Actor::exited` / `Msg::Exited`.
- `src-tauri/src/jobs/conversation.rs`: `apply_update`, `tool_call`, `tool_update`, `tool_kind` (oggi fonde `edit|delete|move` in `ToolKind::Edit`), `status_of`; `locations` oggi non è mai letto, e `ui/src/bindings/Tool.ts` non le espone.
- `src-tauri/src/jobs/registry.rs`: `Jobs::dispatch_in` (`Effect::Ended`, `Effect::ShutDown`); macchina in `crates/core/src/jobs/machine.rs`.
- `src-tauri/src/general/service.rs`: `General::dispatch` (pulizia di General, `ask_close` in `src-tauri/src/ask/commands.rs`).
- `crates/core/src/paths.rs`: `canonical_root`; `crates/core/src/omp/version.rs`: `MIN_OMP_VERSION`.
- Agente finto: `src-tauri/src/jobs/testing.rs`: `fake_agent`.
