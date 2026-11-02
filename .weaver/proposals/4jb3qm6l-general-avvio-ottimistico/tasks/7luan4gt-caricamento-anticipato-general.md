---
refs:
  - general-chat
  - requirements/tech#general-chat
  - requirements/ui#general-chat
technical: false
---

# Caricamento anticipato General

Quando la finestra del progetto si apre e una conversazione General è conservata, Weaver avvia omp e ricarica la sessione con `session/load` subito, senza attendere il primo messaggio e senza inviare alcun prompt. Weaver continua a non conservare il testo della conversazione. Senza conversazione conservata omp parte solo al primo messaggio, come oggi.

## Cosa deve valere

- Il caricamento parte all'apertura della finestra (come la ripresa dei job sospesi), non all'apertura della vista General. Un solo omp per General di progetto, mantenuto fino alla chiusura della finestra.
- Durante il caricamento la vista mostra, al posto del testo vuoto, il testo secondario centrato `Loading conversation…`, sopra ogni messaggio inviato nel frattempo, finché la cronologia non compare.
- Il replay è bufferizzato e mostrato in un unico blocco alla risposta di `session/load`, entro il limite di 30 s di `## job`.
- Un messaggio inviato durante il caricamento compare subito con `Starting omp…` sotto; resta in attesa; la cronologia va sopra di esso e, a caricamento finito, i messaggi in attesa sono inviati in ordine. La casella del messaggio resta sempre abilitata.
- Sessione non ricaricabile: subito conversazione vuota con avviso; omp resta pronto e gli eventuali messaggi in attesa vengono inviati.
- omp che non parte, termina o supera il limite durante il caricamento: `omp stopped: <reason>.` e `Your next message starts omp again.`; il messaggio successivo riavvia omp e ricarica la conversazione conservata.
- omp mancante all'apertura: subito `omp is not installed.` con `Install omp`; appena omp diventa pronto (`omp.READY`, omp-setup Ready) Weaver avvia omp e ricarica da solo, senza attendere un messaggio. Con omp solo in verifica, in aggiornamento o non connesso si mostra solo `Loading conversation…`.
- `Clear`: una conversazione conservata in caricamento non è vuota, quindi `Clear` è abilitato; il dialogo usa il testo normale `The conversation is deleted and a new one starts.`. Confermato durante il caricamento, interrompe il caricamento, termina quell'omp, cancella la voce e la conversazione riparte vuota.
- Chiudere la finestra durante il caricamento termina omp e conserva la conversazione.

## Dove intervenire (indicazioni)

- `crates/core/src/jobs/general.rs`: `GeneralMachine` (`send`, `opened`, enum `Omp`) — aggiungere l'apertura finestra con avvio senza prompt, gli stati Loading/Reloading/Waiting for omp, i messaggi in attesa nel braccio `(Working, Starting)`; aggiornare il test `the_first_message_starts_omp…`.
- `src-tauri/src/general/service.rs`: `General::open` (oggi legge solo il record, `slot.loaded = record.is_none()`), `start_actor`, `General::send_message`, `General::session_opened` (unione dei messaggi in coda dopo il replay quando `!slot.loaded`, `reset`), `carry_out` (`Effect::Clear`), `Slot::state`.
- `src-tauri/src/general/mod.rs`: `GeneralWindows::window_opened` — avviare il caricamento quando c'è una sessione conservata; precedente in `src-tauri/src/jobs/registry.rs: window_opened`.
- `src-tauri/src/general/actor.rs`: `begin` (attesa di omp pronto), `launch`, `spawned` (`load_session`/fallback `new_session`), `opened`, cattura del replay in `incoming`, `report_omp_missing`.
- `src-tauri/src/general/wire.rs`: `GeneralState` (`kept`, eventuale stato di caricamento).
- `ui/src/components/general/general-view.tsx` (`EMPTY`, `empty`, `waiting`, `missing`) e `ui/src/components/general/use-general.tsx` (`useGeneral`).
- Test: `src-tauri/src/general/tests.rs` (`after_a_restart_of_weaver_the_first_message_reloads_the_session…`, che oggi verifica che nulla appaia prima del primo messaggio), `ui/src/components/general/general-view.test.tsx`; ACP finto in `crates/fake-acp/src/main.rs`.
