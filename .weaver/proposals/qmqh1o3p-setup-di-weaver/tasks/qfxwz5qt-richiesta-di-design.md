---
technical: false
refs:
  - design-request
  - requirements/data#design-request
  - requirements/ui#design-request
---
# Richiesta di design

Weaver permette al developer di scrivere una richiesta di design in un editor Markdown dentro l'app (scelta del developer: niente editor esterno) e la gestisce secondo la Machine `design-request`. La richiesta di setup è lo stesso documento, scritto quando il Modello è vuoto e il developer ha scelto di partire da zero. Dipende da "Scheletro del progetto" e "Core dei file del progetto"; l'avvio dei job è in "Lavori dell'agente", il consumo alla consegna in "Proposal".

## Stati
- `Not written`: `developer.NEW_REQUEST` (pulsante `+ Work Item` della Board o Start from Scratch) crea una richiesta vuota e la apre in `Writing`.
- `Writing`: ogni modifica è salvata subito. `developer.SEND_REQUEST` → `Sent` solo se non è vuota: Weaver avvia un job di design, o di setup se il Modello è vuoto; inviare una richiesta vuota non fa nulla. `developer.DISCARD_REQUEST` → `Discarded` cancellando file e immagini. `workspace.REQUEST_DELETED` → `Discarded`: niente viene mai più inviato, anche se Weaver la mostrava ancora.
- `Sent`: leggibile, non reinviabile. `proposal.SUBMITTED` della prima Proposal legata → `Consumed`, file e immagini cancellati. `job.ENDED` senza una Proposal legata consegnata → di nuovo `Writing`, modificabile e reinviabile. `workspace.REQUEST_DELETED` → `Discarded`.
- `Consumed` e `Discarded` sono finali: una richiesta consumata non torna nemmeno se un editor risalva il file.
- Chiudere l'editor o la finestra non invia mai la richiesta: parte solo con Send.

## File (`requirements/data#design-request`)
- `.trama/requests/<id>.md`, Markdown senza front matter; `<id>` è l'ora locale di creazione `YYYY-MM-DDTHH-MM-SS`, con `-2`, `-3`… se il nome è preso. La cartella ha il suo `.gitignore` con `*`.
- Immagini incollate o trascinate (PNG, JPEG, GIF, WebP) copiate in `.trama/requests/<id>.images/` e collegate con percorso relativo; quelle già nel progetto sono collegate dove sono.
- Titolo: primo H1 fuori dai blocchi di codice, altrimenti la prima riga non vuota che non sia solo immagini, altrimenti l'id. Vuota = nulla dopo aver tolto gli spazi; una richiesta di sole immagini non è vuota.
- `.trama/requests/.state.json` (`{sent: {<id>: <job id>}, consumed: [<id>]}`), scritto solo dall'app: un id consumato non è mai più mostrato né inviato.
- Consumare o scartare cancella `<id>.md` e `<id>.images/` in un solo passo.
- L'istruzione del job nomina il file della richiesta, mai il suo testo.

## Editor (`requirements/ui#design-request`)
- Pannello con le richieste aperte a sinistra (titolo o `Untitled request`, ora di creazione, badge `Draft` o `Sent`, più recenti prima) e quella scelta a destra; intestazione `Setup request` o `Design request`; si apre con il focus nel testo.
- Textarea monospazio con a capo morbido; `Preview` affiancata da circa 900 px, aggiornata entro 150 ms.
- Autosalvataggio 500 ms dopo l'ultimo tasto e su blur, cambio vista e chiusura finestra; stato `Saving…`, `Saved` o `Could not save: <reason>.` (il prossimo cambiamento riprova). Scrivere non avvia alcun processo.
- Cambio esterno: ricarica se nulla è in sospeso, altrimenti vince la versione di Weaver con il toast `The request changed on disk; Weaver kept your version.`; cancellazione esterna: toast `The request was deleted outside Weaver.`
- `Send` (`SendHorizontal`, anche `Ctrl/Cmd+Enter`) disabilitato col testo vuoto, tooltip `Write the request first.`; `Discard` con conferma `Discard this request?` / `Its file and images are deleted.`
- Inviata: vista in sola lettura con banner `Sent to the agent · <job name>` e `Open conversation`; tornata in scrittura: banner `The job ended without a Proposal. Change the request and send it again.`
- Immagini fino a 10 MB come `image-<n>.<ext>`, altrimenti toast `Only PNG, JPEG, GIF or WebP images up to 10 MB.`

## Fatto quando
- Test del core sulla Machine: invio di una richiesta vuota ignorato, consumo solo dalla prima Proposal legata, ritorno in `Writing` alla fine del job senza Proposal, file cancellato da fuori mai più inviato, id consumato ignorato anche se il file riappare.
- Send su Modello vuoto avvia un job di setup, altrimenti di design.
- Test Vitest dell'editor: autosalvataggio, stati di salvataggio, Send disabilitato, conferma di Discard, incolla di immagini.
- Chiudere la finestra con una richiesta in scrittura la salva e non la invia.
