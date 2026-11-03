---
created: 2026-11-03T13:24:34.120Z
updated: 2026-11-03T13:24:34.120Z
summary: Ogni vista della finestra inizia con la stessa intestazione (titolo e controlli a destra); il selettore dell'overlay è tolto e l'overlay di una Proposal segue il focus.
tasks:
- l4x0lic0
- qth6aykt
cleared:
  l4x0lic0:
    title: Intestazione in ogni vista
    refs:
    - requirements/ui#General
    - general-chat
    - requirements/ui#general-chat
    - job
    - requirements/ui#job
    - file-view
    - requirements/ui#file-view
    - work
    - requirements/ui#work
    - app
    - requirements/ui#app
    - history-entry
    - requirements/ui#history-entry
    - developer.CHANGE_SETTINGS
    - requirements/ui#developer.CHANGE_SETTINGS
    technical: false
    proposal: 5y7havsz
    label: View header
    finished: 2026-11-03T13:20:40.006Z
    body: |-
      Ogni vista della finestra di progetto (Board, Chat, Jobs, Files, Requirements, Specs, History, Settings) inizia con la stessa intestazione che oggi ha solo Chat, al posto dello spazio vuoto o della barra senza titolo. Un unico componente condiviso, ricavato dall'intestazione di Chat, la disegna in tutte le viste; Chat lo usa a sua volta.

      Quando il lavoro è finito vale quanto segue (`requirements/ui#General`, "View header"):

      - Una riga a tutta larghezza della vista: a sinistra il nome della voce della barra laterale come titolo, solo testo e senza icona, 14 px semibold; a destra i controlli propri della vista, se ne ha; lo spazio libero sta in mezzo. Un bordo inferiore la separa dal contenuto, con padding laterale di 16 px e controlli `size="sm"` centrati in verticale.
      - Altezza fissa uguale per tutte le viste, 56 px più il bordo inferiore di 1 px, anche senza controlli: passando da una vista all'altra il bordo non si sposta.
      - Il titolo visibile è l'intestazione principale della vista per le tecnologie assistive: la regione della vista è etichettata da quel titolo, e il titolo nascosto (`sr-only`) che oggi ripete il nome della vista viene tolto.
      - L'intestazione resta ferma in alto mentre il contenuto sotto scorre.
      - Si vede in ogni stato della vista (caricamento, errore di caricamento, vuoto), ma non al posto di Get Started: dove una vista mostra Get Started perché il progetto non ha ancora `.weaver/`, Get Started occupa tutta la vista, intestazione compresa.

      Per vista:

      - **Chat** (`general-chat`): titolo `Chat`, `Clear` a destra come oggi. Quando Chat mostra la conversazione di un sub-agent, l'intestazione `Chat › <sub-agent>` con `Back to Chat` prende il posto dell'intestazione della vista, nella stessa riga e alla stessa altezza.
      - **Board** (`work`): titolo `Board`; `Scan System` e il testo `· <n> done since last scan` passano a destra nella stessa riga, senza una riga in più. Notices e avvisi di History restano sotto l'intestazione, sopra le colonne.
      - **History** (`history-entry`): titolo `History`; il filtro `Period` passa a destra nella stessa riga.
      - **Requirements** e **Specs** (`app`): titolo `Requirements` o `Specs`; a destra il selettore della Proposal sovrapposta e, solo in Requirements, `Ask more`, nella stessa riga. La barra di focus resta a tutta larghezza sotto l'intestazione; l'avviso di Model non valido resta sotto l'intestazione.
      - **Jobs** (`job`): titolo `Jobs`, nessun controllo, a tutta larghezza sopra entrambi i pannelli. Le barre interne del pannello destro (il percorso di un sub-agent, `Back to questions` di una conversazione Ask more) restano dentro quel pannello, sotto l'intestazione. I 12 px tra il bordo superiore della lista e la prima riga restano.
      - **Files** (`file-view`): titolo `Files`, nessun controllo, a tutta larghezza sopra l'albero e sopra schede ed editor; la barra delle schede resta sotto l'intestazione.
      - **Settings** (`developer.CHANGE_SETTINGS`): titolo `Settings`, nessun controllo, a tutta larghezza sopra la colonna centrata di ~720 px, che scorre sotto l'intestazione ferma. I titoli dei gruppi (omp, Models, Agents, Editor, Appearance, Project) scendono a 14 px semibold, mai più grandi del titolo della vista.

      I test esistenti che cercano il titolo nascosto delle viste o le loro regioni per ruolo vanno aggiornati al nuovo titolo visibile.
  qth6aykt:
    title: Nascondere selettore overlay
    refs:
    - app
    - requirements/ui#app
    - requirements/ui#ask-conversation
    technical: false
    proposal: b8x2b02z
    label: Overlay
    finished: 2026-11-03T13:22:37.439Z
    body: |-
      Il selettore dell'overlay non esiste più in nessuna vista. L'intestazione della vista Requirements (l'intestazione comune delle viste, titolo `Requirements`) contiene a destra solo il pulsante `Ask more` sui Requirements nel loro insieme, con tooltip, nome accessibile e comportamento di oggi; l'intestazione della vista Specs (titolo `Specs`) non ha controlli, solo il titolo. La barra del focus resta, a tutta larghezza sotto l'intestazione della vista. Il componente del selettore e il codice che serviva solo a lui sono tolti.

      L'overlay di una Proposal resta, unico per finestra e condiviso da Requirements e Specs, ma segue il focus:

      - `View requirements` su una Proposal che si può sovrapporre (aperta e non legata a un Drift; mai una Draft, una Proposal in refinement o allegata) disegna quella Proposal in entrambe le viste, applicata sopra le Proposal che richiede, con le etichette e i colori `added`/`changed`/`removed` di oggi. `View requirements` su una Proposal che non si può sovrapporre apre il focus sul Modello corrente.
      - L'overlay è la Proposal dell'ultimo focus aperto, finché quel focus dura. Aprire un focus su un altro soggetto (un Task, un chip, un'altra Proposal) in una qualsiasi delle due viste riporta entrambe le viste al Modello corrente (o all'altra Proposal, se è quella aperta e si può sovrapporre).
      - Chiudere con `Clear` o con Esc il focus che ha disegnato la Proposal chiude anche l'overlay: entrambe le viste tornano al Modello corrente. Prima `Clear` conservava l'overlay; ora non più.
      - Se la Proposal disegnata smette di essere aperta, le viste tornano al Modello corrente e compare l'avviso di oggi (`Proposal <n> is no longer open, so the views show the Current Model.`); il focus resta, ricalcolato sul Modello corrente.
      - Premere `Ask more` sui Requirements passa all'agente la Proposal in overlay in vista, se c'è, come oggi; premere `Ask more` non cambia overlay, focus, `Only these` né scorrimento.
      - Cambiare vista dalla barra laterale conserva overlay e focus come oggi.

      Il codice di oggi: il selettore è `ui/src/components/views/overlay-picker.tsx`, montato in `ui/src/components/views/view-frame.tsx` nella riga in cima alla vista; l'overlay e il focus vivono in `ui/src/components/views/views-state.tsx` (`focusOn` già imposta l'overlay per una Proposal sovrapponibile, ma non lo toglie per gli altri soggetti; `clearFocus` non tocca l'overlay). I test esistenti della UI che riguardano il selettore vanno tolti o adattati al nuovo comportamento.
---
# Intestazione comune delle viste

Board, Chat, Jobs, Files, Requirements, Specs, History e Settings condividono un unico componente di intestazione: titolo semibold a sinistra, controlli propri a destra, altezza fissa di 56 px con bordo inferiore, sempre visibile in alto. Il titolo visibile è l'intestazione principale della vista per le tecnologie assistive. Il selettore dell'overlay non esiste più: l'overlay è la Proposal dell'ultimo focus aperto e termina con quel focus (`Clear` o Esc), oppure quando si apre un focus su un altro soggetto o la Proposal non è più aperta. Requirements mantiene solo `Ask more` nell'intestazione, Specs nessun controllo.
