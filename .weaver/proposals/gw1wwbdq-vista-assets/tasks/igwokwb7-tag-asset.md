---
technical: false
refs:
  - developer.SEND_MESSAGE
  - general-chat/Idle/developer.SEND_MESSAGE
  - general-chat/Loading/developer.SEND_MESSAGE
  - general-chat/Working/developer.SEND_MESSAGE
  - job/Running/developer.SEND_MESSAGE
  - job/Waiting for the developer/developer.SEND_MESSAGE
  - ask-conversation/Empty/developer.SEND_MESSAGE
  - ask-conversation/Idle/developer.SEND_MESSAGE
  - ask-conversation/Answering/developer.SEND_MESSAGE
  - ask-conversation/Waiting for permission/developer.SEND_MESSAGE
  - developer.SEND_REQUEST
  - design-request
  - job/Not started/developer.SEND_REQUEST
  - requirements/ui#assets
  - requirements/rules#assets
  - requirements/rules#job
  - requirements/tech#assets
  - requirements/quality#assets
---
# Tag @ degli asset

Quando il Task è finito, lo sviluppatore può taggare gli asset del progetto (i file regolari in `.weaver/assets/` il cui nome non inizia con `.`) scrivendo `@` seguito dal nome, in quattro punti: la casella dei messaggi della Chat, la casella di risposta della conversazione di un job nella vista Jobs, la casella di una conversazione Ask more e il testo dell'editor delle richieste aperto da `+ Work Item` (non la sua `Preview`).

**Riconoscimento** (`requirements/rules#assets`): un tag è `@` all'inizio del testo o dopo uno spazio, seguito dal nome di un asset esistente, confrontato esattamente, maiuscole comprese; se più nomi vanno bene vince il più lungo, così i nomi con spazi non richiedono virgolette; dopo il nome vengono fine testo, spazio o uno di `. , ; : ! ? ) ] } " '`. Un `@NOME` che non corrisponde a nessun asset resta testo normale e non dice niente all'agente.

**Mentre si scrive** (`requirements/ui#assets`): digitando `@` si apre un elenco di suggerimenti sotto il cursore che filtra gli asset mentre si scrive (prima quelli il cui nome inizia con il testo, al massimo 8 righe visibili); Su/Giù, Invio o Tab inseriscono `@<nome>` e uno spazio, Esc chiude; con l'elenco aperto Invio e Tab non inviano mai. Ogni tag riconosciuto è disegnato in blu primario nella casella; un tag che non corrisponde resta del colore normale. Una sola componente condivisa serve le quattro caselle: `textarea` nativa con testo trasparente e cursore visibile, e dietro uno strato specchio che disegna il testo con i tag colorati (`requirements/tech#assets`). Le caselle oggi sono `Textarea` semplici: `ui/src/components/general/general-view.tsx`, `ui/src/components/jobs/jobs-view.tsx`, `ui/src/components/ask/ask-conversation.tsx`, `ui/src/components/requests/request-pane.tsx`. L'elenco degli asset arriva dall'evento `assets-changed` del backend, costruito nel Task «Vista Assets» (voce "Asset list" di `requirements/tech#assets`); se ancora non c'è, va costruito come descritto lì.

**All'invio di un messaggio** (Chat, conversazione di un job, Ask more): i tag si riconoscono sugli asset esistenti in quel momento, mai dopo (un messaggio in attesa di consegna tiene ciò che ha avuto all'invio). Se c'è almeno un asset taggato, al testo che va a omp si aggiunge, dopo il testo dello sviluppatore, una riga vuota, la riga `Assets tagged by the developer (added by Weaver, not written by the developer):` e, una per riga, ``Read the file `.weaver/assets/<name>`.`` per ogni asset, una volta sola, nell'ordine del primo tag; nel primo messaggio di Ask more il blocco segue il messaggio dopo l'istruzione. La lettura del file è lasciata a omp. La conversazione mostra sempre solo il testo scritto dallo sviluppatore, senza il blocco: dal vivo e anche quando viene ricostruita dal replay di omp. Punti nel codice: `src-tauri/src/general/service.rs` `send_message` e la prima richiesta di Ask more (`crates/core/src/jobs/ask.rs` `first_prompt`, `shown_message`), `src-tauri/src/jobs/registry.rs` `send_message`, `src-tauri/src/jobs/session.rs` `text_prompt`, `src-tauri/src/jobs/conversation.rs` `developer_message` e `replayed_user`.

**All'invio di una richiesta di design**: la richiesta si invia come prima e il suo file resta com'è scritto; in più l'istruzione del job di design, o di setup quando il Model è vuoto, contiene subito dopo la prima frase, sulla stessa riga, una frase ``Read the file `.weaver/assets/<name>`.`` per ogni asset che la richiesta tagga al momento dell'invio (`requirements/rules#job`). Il job tiene in memoria l'elenco dei nomi fino all'invio dell'istruzione; non va scritto nel registro dei job né nel file della richiesta. Punti nel codice: `crates/core/src/job_instruction.rs` (`first_sentence`, `Job::Design` / `Job::Setup`).

**Test** come chiede `requirements/quality#assets` per i tag: funzioni pure in `weaver-core` (riconoscimento, blocco aggiunto e sua rimozione dal replay, frasi dell'istruzione) e Vitest per elenco dei suggerimenti ed evidenziazione in ognuna delle quattro caselle.
