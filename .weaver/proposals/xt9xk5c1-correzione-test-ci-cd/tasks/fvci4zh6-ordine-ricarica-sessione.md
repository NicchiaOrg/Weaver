---
technical: false
refs: ["job", "general-chat", "requirements/tech#job", "requirements/tech#general-chat"]
---
# Ordine ricarica sessione

## Problema

Su Linux (run `ci` 41448848343) falliscono in modo intermittente:

- `jobs::tests::closing_the_window_suspends_the_job_and_opening_it_reloads_the_session`: l'assert in `src-tauri/src/jobs/tests.rs` (~715) non trova `Entry::Agent{"After the reload", complete: true}`; il testo diventa "replayedAfter the reload".
- `general::tests::after_a_restart_of_weaver_the_first_message_reloads_the_session_and_the_history_comes_back`: va in timeout in `until` (`src-tauri/src/jobs/testing.rs` ~233-243) perché "Answer" e "Newer answer" vengono fusi.

È un bug del prodotto, non solo del test. In `load_session` (`src-tauri/src/jobs/session.rs` ~151) la risposta a `session/load` passa per una one-shot (`rpc.request`), e il task di apertura invia `Msg::Opened`. Le notifiche ripetute invece passano reader → canale `incoming` → task forwarder → `Msg::Incoming`. Sono due task indipendenti, quindi `Opened` può superare le ultime righe ripetute. `opened()` imposta `loading = false`, così le righe arrivate tardi finiscono nella conversazione live, dove `append_agent` (`src-tauri/src/jobs/conversation.rs` ~521) attacca la risposta successiva alla voce rimasta aperta. Il commento in `load_session` dice che l'ordine è garantito, ma non è vero.

## Cosa deve essere vero alla fine

- `session/load` è inviato con lo stesso invio ordinato già usato da `session/prompt` (`prompt` in `session.rs`, `send_ordered` in `src-tauri/src/jobs/rpc.rs` ~169, instradamento in `route` ~239-242). Il caricamento termina quando la sua risposta viene gestita, sempre dopo ogni `session/update` letto prima di essa.
- Nessun aggiornamento ripetuto raggiunge la conversazione live. Il primo messaggio dell'agente dopo il caricamento apre una nuova voce.
- Vale sia per i job (ripresa di un job sospeso quando la finestra si riapre: `spawned()`, `opened()` e `incoming()` con `if self.loading` in `src-tauri/src/jobs/actor.rs`) sia per General (`spawned()`, `opened()` e `incoming()` ~444-450 in `src-tauri/src/general/actor.rs`).
- La cronologia ripetuta continua ad arrivare alla UI in un unico blocco alla fine del caricamento (`adopt_replay` in `src-tauri/src/jobs/registry.rs` ~816).
- Il limite unico di 30 s su spawn, `initialize` e `session/new` / `session/load` resta valido anche con il caricamento ordinato.
- In General, se il caricamento fallisce (`OpenError::Failed`), si avvia comunque una sessione vuota (`new_session`) e un avviso lo comunica, come oggi.
- Un nuovo test di regressione deterministico a livello RPC, accanto ai test esistenti in `rpc.rs` (~360-415), verifica che la risposta a una richiesta ordinata venga gestita dopo le notifiche lette prima di essa. Niente sleep nei test.

Il fake agent (`crates/fake-acp/src/main.rs` ~149-156) ripete la cronologia e poi risponde; non va modificato per nascondere il problema.

## Verifica

- I due test citati passano in modo affidabile anche ripetuti molte volte sotto carico, ad esempio in un ciclo con la CPU occupata.
- Il nuovo test RPC passa.
- `cargo test --workspace --locked` passa nel workflow `ci` su macOS, Windows e Linux.
