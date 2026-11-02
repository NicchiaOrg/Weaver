---
column: done
technical: false
refs:
  - drift
  - requirements/data#drift
  - requirements/cli#drift
  - requirements/rules#drift
  - requirements/ui#drift
drift: null
proposal: qmqh1o3p
agent: false
created: 2026-11-02T07:29:32.875Z
finished: 2026-11-02T12:25:52.743Z
---
# Drift

Weaver gestisce la Machine `drift`: una differenza fra il codice e il Modello o i Requisiti che solo il developer decide. Nessun Drift si decide, si risolve o si esegue da solo. Dipende da "Scheletro del progetto" e "Core dei file del progetto"; si appoggia a "Round di domande", "Lavori dell'agente", "Proposal" e "Task".

## Registrazione
- L'agente lo registra dopo aver riletto il codice: durante uno scan, durante un Run (l'advisor per codice già esistente) o da un job su richiesta del developer. Nomina riferimenti, cosa succede, cosa dovrebbe succedere e dove nel codice; offre da 2 a 5 opzioni di correzione del codice, la prima raccomandata, nessuna che cambi il Modello; nessuna opzione solo con una Proposal attaccata. Può nominare l'origine (Task) e un Drift precedente (`previous` automatico se l'origine è un fix Task).
- Non registra di nuovo una differenza scartata finché il testo del Modello e dei Requisiti è invariato (impronta di testo salvata allo scarto).
- Prima della fine del suo job prepara la domanda di decisione (Async Drift Resolver, sempre attivo).

## Stati
- `Open`: blocca i Task che riguardano la stessa cosa. `agent.PREPARE_DRIFT_QUESTION` salva la domanda. `developer.RESOLVE_DRIFT`: con una domanda preparata valida Weaver la pone da sé come round, senza agente; altrimenti avvia un job di resolve; rifiutato mentre lavora un Run, mentre un'altra resolution è scritta o attende decisione, o mentre il job che l'ha registrato lavora. `question-round.ANSWERED` della domanda preparata: Weaver scrive e consegna da sé la resolution `Resolve: <Drift title>` con i controlli dell'agente; una domanda saltata lascia il Drift com'è; una risposta libera accanto a una scelta diversa da un fix, o una consegna rifiutata (Draft scartata), avvia un job di resolve con la risposta già data.
- `developer.FIX_DRIFT_CODE` → `Code to be fixed`: opzione obbligatoria se ce ne sono; riferimenti scomparsi sostituiti dal contenitore più vicino (Transition → State → Machine → Feature; Event → Machine o Actor; sezione mancante tolta), rifiutato se non resta alcun elemento del Modello. Nasce il fix Task con i due lati, l'opzione, la nota e il dove nel codice.
- `developer.ACCEPT_DRIFT_PROPOSAL` → `Proposal accepted` solo con una Proposal attaccata approvabile. `developer.DISMISS_DRIFT` → `Dismissed`, rifiutato se il Drift è in una resolution scritta o in attesa.
- `proposal.APPROVED` di una resolution porta a `Code to be fixed`, `Proposal accepted` o `Dismissed` secondo la decisione registrata.
- `Code to be fixed`: blocca solo l'origine. `task.FINISHED` del suo fix Task → `Fixed`; `task.CLOSED_UNCHANGED` → `Dismissed` con la nota.
- `Fixed`, `Proposal accepted`, `Dismissed` sono finali; il file resta. Eventi `CODE_TO_BE_FIXED`, `PROPOSAL_ACCEPTED`, `DISMISSED` agiscono sulla Proposal attaccata.

## Domanda preparata (`requirements/rules#drift`)
- Una domanda a scelta singola con id uguale all'identità del Drift: un'opzione per ogni fix nell'ordine più una per il comportamento attuale (`proposal` o `dismissed`); senza opzioni esattamente `proposal` (raccomandata) e `dismissed`.
- Valida finché il Drift è aperto, il blocco si legge, l'impronta coincide con quella attuale, ogni `fix-code` nomina un'opzione esistente e un `proposal` ha una Proposal in Draft o Open.
- Impronta: SHA-256 sui riferimenti in ordine (`# <ref>`, testo di `trama model show <ref>` e sezioni dei Requisiti), `(missing)` per i mancanti; una sola funzione del core.

## File, comandi, card
- `.trama/drifts/<id>-<slug>.md` con front matter e sezioni nell'ordine di `requirements/data#drift`, compreso `## Prepared question` con un blocco `json`.
- `trama drift new|show|list|edit|move` come in `requirements/cli#drift`; rispondere a una domanda preparata esiste solo nell'app.
- Card compatta con bordo rosso, badge `recurring`, `resolving`, `prepared`, `Resolve` e `Dismiss Drift` con motivi disabilitati in ordine; dettagli con chip, lati affiancati, opzioni, dove nel codice con `Open in editor`, origine, Proposal attaccata, decisione `Fix the code` / `Accept Proposal` (`requirements/ui#drift`).

## Fatto quando
- Test del core su ogni transizione, contenitore più vicino, validità della domanda preparata e impronta.
- Resolve con domanda valida presenta il round senza avviare omp; la risposta produce una resolution consegnata.
- Fix code crea il fix Task e rimette l'origine in To do in attesa.
- Test Vitest di card e dettagli, motivi disabilitati compresi.
