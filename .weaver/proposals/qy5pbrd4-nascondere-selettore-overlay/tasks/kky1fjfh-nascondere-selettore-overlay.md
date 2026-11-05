---
technical: false
refs:
  - app
  - requirements/ui#app
  - requirements/ui#ask-conversation
---
# Nascondere selettore overlay

Il selettore dell'overlay non esiste più in nessuna vista: la barra della vista Requirements contiene solo il pulsante `Ask more` sui Requirements nel loro insieme (con tooltip, nome accessibile e comportamento di oggi), e la vista Specs non ha più una barra; la sua barra del focus, quando c'è un focus, sta in cima alla vista. Il componente del selettore e il codice che serviva solo a lui sono tolti.

L'overlay di una Proposal resta, unico per finestra e condiviso da Requirements e Specs, ma segue il focus:

- `View requirements` su una Proposal che si può sovrapporre (aperta e non legata a un Drift; mai una Draft, una Proposal in refinement o allegata) disegna quella Proposal in entrambe le viste, applicata sopra le Proposal che richiede, con le etichette e i colori `added`/`changed`/`removed` di oggi. `View requirements` su una Proposal che non si può sovrapporre apre il focus sul Modello corrente.
- L'overlay è la Proposal dell'ultimo focus aperto, finché quel focus dura. Aprire un focus su un altro soggetto (un Task, un chip, un'altra Proposal) in una qualsiasi delle due viste riporta entrambe le viste al Modello corrente (o all'altra Proposal, se è quella aperta e si può sovrapporre).
- Chiudere con `Clear` o con Esc il focus che ha disegnato la Proposal chiude anche l'overlay: entrambe le viste tornano al Modello corrente. Prima `Clear` conservava l'overlay; ora non più.
- Se la Proposal disegnata smette di essere aperta, le viste tornano al Modello corrente e compare l'avviso di oggi (`Proposal <n> is no longer open, so the views show the Current Model.`); il focus resta, ricalcolato sul Modello corrente.
- Premere `Ask more` sui Requirements passa all'agente la Proposal in overlay in vista, se c'è, come oggi; premere `Ask more` non cambia overlay, focus, `Only these` né scorrimento.
- Cambiare vista dalla barra laterale conserva overlay e focus come oggi.

Il codice di oggi: il selettore è `ui/src/components/views/overlay-picker.tsx`, montato in `ui/src/components/views/view-frame.tsx`; l'overlay e il focus vivono in `ui/src/components/views/views-state.tsx` (`focusOn` già imposta l'overlay per una Proposal sovrapponibile, ma non lo toglie per gli altri soggetti né alla chiusura del focus). I test esistenti della UI che riguardano il selettore vanno tolti o adattati al nuovo comportamento.
