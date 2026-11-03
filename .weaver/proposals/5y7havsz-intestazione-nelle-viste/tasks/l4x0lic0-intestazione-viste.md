---
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
---
# Intestazione in ogni vista

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
