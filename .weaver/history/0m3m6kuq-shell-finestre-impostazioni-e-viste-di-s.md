---
created: 2026-11-02T12:31:02.985Z
updated: 2026-11-02T12:31:02.985Z
summary: Fondamenta dell'interfaccia e regole di qualità, finestre per progetto con riapertura all'avvio, file e vista delle impostazioni, viste Diagram, Requirements e Specs con Proposal sovrapposta e focus.
tasks:
  - o7f3fscl
  - c34tjjfq
  - 6buq31uh
  - y73la6jb
cleared:
  6buq31uh:
    title: Impostazioni
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T08:48:42.567Z
    refs:
      - developer.CHANGE_SETTINGS
      - requirements/data#developer.CHANGE_SETTINGS
      - requirements/ui#developer.CHANGE_SETTINGS
    body: |-
      Realizza `developer.CHANGE_SETTINGS`: le impostazioni di Weaver, il loro file e la vista Settings. Dipende da "Scheletro del progetto"; i box di omp e Update skills che compaiono nella vista vengono da "Controllo di omp" e "Progetto e skill installate".

      ## File e schema (`requirements/data#developer.CHANGE_SETTINGS`)
      - Un solo file `settings.json` nella cartella di configurazione dell'app (`app_config_dir`). Nulla nel progetto, nel repository o nell'ambiente è mai letto come impostazione.
      - Uno schema tipato: struct Rust nel core con default e validazione; il tipo TypeScript è generato, mai scritto a mano. Il backend è l'unico lettore e scrittore.
      - Chiavi e default: `ompPath` `""` (vuoto = cerca omp); `models.analysis`, `models.scan`, `models.execution` `""` (nome come omp lo accetta, suffisso `:<thinking level>` facoltativo, vuoto = default di omp; mai controllati contro una lista); `maxParallelAgents` `20` (≥ 1); `language` `""` (lingua dei testi dell'agente, testo libero; l'interfaccia resta sempre in inglese); `theme` `auto|light|dark`, default `auto`; `editor.preset` `vscode|cursor|zed|jetbrains|sublime|custom`, default `vscode`; `editor.command` (solo con `custom`, deve contenere `{file}`, `{line}` facoltativo); `permissions` `yolo` (default) o `ask`.
      - Preset dell'editor: `code --goto {file}:{line}`, `cursor --goto {file}:{line}`, `zed {file}:{line}`, `idea --line {line} {file}`, `subl {file}:{line}`, programma cercato con le regole del PATH di omp. `custom`: diviso in argomenti con le regole POSIX, `{file}` (assoluto) e `{line}` (da 1, `1` se ignota) sostituiti dentro ogni argomento, eseguito senza shell; vuoto, non divisibile o senza `{file}` è non valido.
      - Si salvano solo i valori diversi dal default, con `"version": 1`. File mancante = tutti i default; chiave sconosciuta ignorata e tolta alla scrittura successiva; chiave di tipo o valore errato = default, e Settings dice quale e perché. File illeggibile = tutti i default con l'errore in Settings; non si riscrive finché il developer non cambia qualcosa, e allora è prima rinominato `settings.invalid-<UTC timestamp>.json`.
      - Scritture atomiche e serializzate; una scrittura fallita lascia in vigore il file e i valori precedenti e mostra l'errore. Un cambio vale per i processi omp avviati dopo; i lavori in corso tengono i valori con cui sono partiti.
      - Segreti solo nel portachiavi (`keyring`, servizio `weaver`), mai in file, argomenti, log, UI o IPC; la v1 non ne salva nessuno.
      - Nessun interruttore per le funzioni sempre attive: esecuzione parallela dei Task, Sub-agents Context Sharing, Advisor Scanner, Async Drift Resolver, modello per Proposal, raffinamento all'Approve.

      ## Vista Settings (`requirements/ui#developer.CHANGE_SETTINGS`)
      - Colonna centrata di ~720 px, gruppi in ordine: `omp` (box di stato, `Providers`, `omp path`, `Permissions` con `Run without asking`/`Ask before acting`), `Models` (`Analysis`, `Scan`, `Execution` con i consigli in grassetto), `Agents` (`Max parallel sub-agents`, `Language of the agent's texts`), `Editor` (Select dei preset più `Custom command`), `Appearance` (`Theme` segmentato `Auto`/`Light`/`Dark`, applicato dal vivo a ogni finestra), `Project` (Update skills).
      - Ruoli dei modelli: Combobox con testo libero, i modelli che omp elenca e `omp default`; Select del livello di thinking solo quando omp elenca livelli, valore salvato come `<model>:<level>`; se l'elenco manca, una riga dice perché.
      - Salvataggio immediato senza conferma (select alla scelta, testo e numeri al blur o `Enter`). Un valore non valido non si salva e il campo dice perché (`Enter a whole number of at least 1.`, `The command must contain {file}.`); una scrittura fallita mostra `Could not save <title>: <reason>.` e rimette il valore salvato. Un cambio da un'altra finestra compare ovunque subito, tranne nel campo con il focus. Titoli, descrizioni (≤160 caratteri), default e validazione vengono dallo schema; le chiavi non si mostrano mai.

      ## Fatto quando
      - Test su cartelle temporanee (mai le impostazioni reali): file mancante, chiave sconosciuta, valore errato, file illeggibile rinominato al primo cambio, ritorno al default che toglie la chiave.
      - Test della divisione del comando personalizzato e dei preset dell'editor.
      - Test UI della vista: ordine dei gruppi, validazione, errore di scrittura, cambio da un'altra finestra.
  c34tjjfq:
    title: Finestre e avvio
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T09:33:55.718Z
    refs:
      - app
      - window
      - desktop
      - requirements/tech#window
      - requirements/data#window
      - requirements/ui#window
    body: |-
      Dipende da "Scheletro del progetto", "Shell e aspetto dell'app" e "Impostazioni" (editor scelto); la sospensione dei job è in "Lavori dell'agente".

      Weaver è un solo processo per tutti i progetti aperti, con una finestra per progetto. Senza finestre di progetto mostra la finestra di benvenuto. Per scelta del developer, all'avvio Weaver riapre i progetti dell'ultima sessione; se non ce ne sono, mostra il benvenuto con i progetti recenti e `Open Folder`.

      ## Machine `window`
      - **Closed** (iniziale): `developer.OPEN_PROJECT` → Open; `desktop.APP_STARTED` → Open solo per i progetti la cui finestra era aperta all'ultima uscita, e i loro job che lavoravano riprendono.
      - **Open**: `developer.OPEN_PROJECT` porta la finestra in primo piano (una cartella già aperta non apre una seconda finestra); `notification.CLICKED` la porta in primo piano mostrando ciò di cui parlava la notifica; `developer.CLOSE_WINDOW` → Closed, job attivi sospesi, il progetto non si riapre al prossimo avvio; `developer.QUIT_APP` → Closed, job sospesi, il progetto si riapre al prossimo avvio. Emette `OPENED` all'apertura e `CLOSED` alla chiusura, anche per uscita da Weaver.
      - Chiusura della finestra con job attivi e uscita con job attivi in qualunque progetto chiedono conferma (eventi `developer.CLOSE_WINDOW`, `developer.QUIT_APP`).

      ## Requisiti
      - `requirements/tech#window`: `tauri-plugin-single-instance` registrato per primo; un secondo avvio passa argomenti e cwd all'istanza in corso ed esce; una cartella come argomento è `developer.OPEN_PROJECT`, altrimenti viene avanti la finestra usata più di recente. Identità del progetto: percorso canonico (`dunce`), senza distinzione di maiuscole su Windows. Etichette `p-<project hash>` e `welcome`. `tauri-plugin-window-state` salva dimensione, posizione, massimizzato e fullscreen per etichetta, mai la visibilità; posizione su monitor assente → centrata sul primario; default 1280×800, minimo 960×600. `CloseRequested` impedito con job attivi del progetto, `ExitRequested` con job attivi in qualunque progetto; conferma nativa con `tauri-plugin-dialog` che dice quanti job lavorano (funziona anche con webview bloccata); dopo la conferma i job sono sospesi, poi la finestra si chiude o l'app esce. Chiudere l'ultima finestra di progetto apre il benvenuto; chiudere il benvenuto esce su Windows e Linux, su macOS l'app resta nel Dock e `RunEvent::Reopen` mostra il benvenuto. `Cmd+Q` su macOS, `Ctrl+Q` e File › Quit altrove. Lista di ripristino scritta solo all'uscita. Spegnimento senza conferma possibile: job sospesi senza chiedere; dopo un crash il registro di ripresa li riporta Suspended.
      - `requirements/data#window`: `state.json` nella cartella dati locale con `recent` (fino a 10, più recente prima, uno per percorso canonico, esce solo se scalzato) e `lastSession` (percorsi delle finestre aperte all'uscita, in ordine di apertura; chiudere una finestra la toglie, uscire no).
      - `requirements/ui#window`: benvenuto 640 × 480 non ridimensionabile con nome e versione, recenti (nome cartella, percorso in muted, `Not found` rimovibile), `Open Folder` col selettore di sistema e riga di stato di omp. Il benvenuto si chiude quando una finestra di progetto è aperta. Titolo della finestra: nome della cartella del progetto.
      - Editor (`app`): ovunque siano noti file e riga, `Open in editor` li apre nell'editor delle impostazioni (preset VS Code, Cursor, Zed, JetBrains, Sublime Text o comando personalizzato con `{file}` e `{line}`, eseguito senza shell); un errore mostra un avviso col motivo.

      ## Fatto quando
      - Test: secondo avvio con cartella apre o porta avanti la finestra giusta, mai due finestre per un progetto.
      - Avvio dopo un'uscita con due progetti aperti li riapre e riprende i loro job; senza ultima sessione compare il benvenuto con i recenti.
      - Chiusura e uscita con job attivi chiedono la conferma nativa col numero di job; annullare non cambia nulla.
      - Stato della finestra ripristinato per progetto; il test end-to-end apre benvenuto, progetto, chiude ed esce senza errori.
      - `Open in editor` lancia il preset scelto con file e riga corretti.
  o7f3fscl:
    title: Shell e aspetto dell'app
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T08:05:15.327Z
    refs:
      - requirements/ui
      - requirements/ui#General
      - requirements/quality#General
    body: |-
      Le fondamenta dell'interfaccia su cui si costruiscono tutte le viste, e le regole di qualità valide in tutto Weaver. Dipende da "Scheletro del progetto"; il contenuto delle singole viste lo realizzano i Task che le riguardano ("Board", "Diagramma, Requisiti e Specs", "History", "Lavori dell'agente", "Chat General", "Impostazioni").

      ## Principi (`requirements/ui`)

      - Ogni testo dell'interfaccia è in inglese, qualunque sia l'impostazione della lingua, che cambia solo i testi dell'agente.
      - L'interfaccia mostra soltanto ciò che il backend Rust tiene e gli manda le richieste del developer: nessuno stato proprio di progetto, carte o lavori, nessun accesso ai file. Riceve gli eventi del backend e chiede uno snapshot completo al caricamento o quando vede un salto di revisione.

      ## Aspetto (`requirements/ui#General`)

      - shadcn/ui con base `zinc` e primario blu (`blue-600` in chiaro, `blue-500` in scuro). Tema `auto` (segue il sistema), `light`, `dark`: un cambio dell'impostazione o del sistema in `auto` si applica subito a ogni finestra aperta, senza ricaricare.
      - Ogni controllo è un componente shadcn/ui; colori con significato dalla palette Tailwind (600 in chiaro, 400 in scuro), il resto dai token del tema; carte `shadow-sm`, dialoghi e menu `shadow-lg`. Font di sistema a 14 px; id, percorsi e codice in monospace. Transizioni al massimo 200 ms, nessuna con movimento ridotto richiesto dal sistema.
      - Finestra di un progetto: barra laterale sinistra di 208 px con, in quest'ordine, Board, Diagram, Requirements, Specs, History, Jobs, General, Settings (icone lucide `SquareKanban`, `Network`, `ListChecks`, `Package`, `History`, `Bot`, `MessageSquare`, `Settings`). Cambiare voce conserva lo stato di ogni vista. Sotto 1024 px di larghezza solo icone, 56 px, con il nome come tooltip; la dimensione minima della finestra è 960 × 600 px (`requirements/tech#window`). Board mostra il numero di carte reali in Backlog; Jobs il numero di lavori in attesa del developer in ambra; nessun numero quando è 0.
      - Icone lucide-react a 16 px, decorative, a sinistra del testo; bottoni di sola icona con nome accessibile e tooltip.
      - Tooltip: al massimo due frasi brevi (circa 160 caratteri), aperti dopo 400 ms al passaggio e subito al focus, chiusi con Esc, blur o uscita del puntatore.
      - Controlli disabilitati: `aria-disabled` invece di `disabled`, restano focalizzabili, ignorano l'attivazione, e mostrano tutti i motivi, uno per riga, nell'ordine fisso del controllo, in tooltip e descrizione accessibile; `The agent is still finishing` sempre primo sui controlli di una carta. Si riabilitano da soli senza spostare il focus. Una decisione rifiutata dal backend mostra un avviso in cima alla vista e non cambia nulla. Un unico componente condiviso realizza questo schema.
      - Id: numero breve (`P3`) se c'è, altrimenti id permanente di 8 caratteri, mai entrambi, monospace; nome accessibile `Short number (temporary) P3` o `Permanent ID qmqh1o3p`.
      - Dettagli e moduli in `Dialog` (larghezza massima 720 px, corpo che scorre), conferme in `AlertDialog` con bottone distruttivo `destructive` e Cancel con il focus iniziale; alla chiusura il focus torna a chi l'ha aperto.
      - Un renderer Markdown condiviso: `react-markdown` + `remark-gfm`, HTML grezzo mostrato come testo, mai `dangerouslySetInnerHTML`; tabelle, barrato, liste di attività in sola lettura, autolink, codice; link aperti solo con `http`/`https` nel browser di sistema; immagini solo dentro il progetto, quelle remote come link; titoli mai più grandi di quello dell'elemento che li contiene; niente scorrimento orizzontale fuori da tabelle e codice. Blocchi `mermaid` caricati al primo uso con `securityLevel: 'strict'` e il tema corrente; un errore mostra il codice con l'errore sotto, in testo attenuato.
      - Orari locali `HH:MM` 24 h, date in inglese (`Monday, 2 November 2026`), durate `less than a minute`, `<n> min`, `<h> h <m> min`.

      ## Qualità (`requirements/quality#General`)

      - Prestazioni: nessun processo, scansione di file, analisi del Model o layout sul thread della UI o dell'IPC; un comando lungo risponde subito e il risultato arriva come evento. Scrivere, cambiare vista, aprire carte o scorrere non avvia processi. Le operazioni oltre 200 ms vanno nel log a `info` con nome, progetto e durata.
      - Affidabilità: risultati asincroni etichettati con il loro input e scartati se superati; nessun timer d'inattività chiude un lavoro o un round; avvio come passi isolati che mostrano `Weaver could not start: <reason>` o `<project> could not open: <reason>` con **Open log**; un panic in un'attività di fondo è catturato e riportato.
      - Log in `app_log_dir`, un file al giorno ruotato a 10 MB, 7 conservati, livello `info` (`debug` con `WEAVER_LOG=debug`), riga con istante UTC, livello, modulo, progetto e lavoro; mai prompt, messaggi, risposte, contenuti di file, valori d'ambiente o segreti. La CLI non scrive log.
      - Accessibilità WCAG 2.2 AA: tutto usabile da tastiera, focus visibile (anello ≥ 2 px, contrasto ≥ 3:1), contrasto testo ≥ 4.5:1, mai stato indicato solo dal colore, zoom al 200%.
      - Testi senza prima persona; dicono `Weaver` e `the agent`.

      ## Fatto quando

      - Test Vitest e Testing Library: cambio di tema senza ricarica, ordine della barra laterale e riduzione a icone, componente dei controlli disabilitati (motivi in ordine, focus, nessuna attivazione), renderer Markdown (HTML grezzo come testo, link non http ignorati, immagine remota come link, errore mermaid).
      - Test del backend: log senza contenuti vietati, rotazione, passo d'avvio fallito che non ferma gli altri.
  y73la6jb:
    title: Diagramma, Requisiti e Specs
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T10:33:08.256Z
    refs:
      - requirements/ui#app
    body: |-
      Dipende da "Scheletro del progetto", "Core dei file del progetto" (immagine del progetto, validazione, scope e applicazione delle Proposal), "Shell e aspetto dell'app" e "Finestre e avvio" (`Open in editor`).

      Le viste Diagram, Requirements e Specs della finestra di progetto mostrano in sola lettura il Modello, i Requisiti e le Specs, con una Proposal aperta sovrapposta e un focus. Tutto ciò che mostrano arriva calcolato dal backend; l'interfaccia non legge file. Il layout del Diagramma non gira sul thread dell'interfaccia né su quello dei comandi IPC.

      ## `requirements/ui#app`
      - **Diagram**: cytoscape con layout elkjs (`layered`, direzione `RIGHT`); la radice usa una griglia senza archi. Livelli System → Whole System o un Actor → Feature → Machine → State. La radice mostra `Whole System`, gli Actor in ordine di Modello, poi le Feature non raggiunte da alcun Actor. Whole System: ogni Actor e Feature con archi etichettati col numero di Event; livello Actor: l'Actor, le Feature che ricevono i suoi Event e gli archi; livello Feature: le Machine e gli archi di Event, più Actor e Machine esterni come nodi compatti tratteggiati; livello Machine: State (iniziale col punto pieno, finale col bordo doppio) e Transition `<sender>.<EVENT>`; livello State: lo State con le Transition entranti e uscenti. Sottotitoli `<n> features` / `<n> machines`; un Actor senza Event ricevuti mostra `No events received`, tratteggiato, non apribile.
      - Clic seleziona e apre il pannello destro (360 px): titolo, descrizione renderizzata, Transition o Event, Drift aperti e Task che lo riguardano, `Open in editor`. Doppio clic o Enter apre il livello; sul livello Feature una Machine si espande e richiude sul posto. Breadcrumb `System › <Actor | Whole System> › <Feature> › <Machine> › <State>` cliccabile (da un link parte da `System › <Feature>`). Hover: titolo e prima frase della descrizione. Badge rosso col numero di Drift aperti. Ai cambi di Modello o overlay restano livello e selezione, con ripiego sul contenitore più vicino. `Fit` e viewport tenuto per livello.
      - **Overlay**: un selettore per finestra condiviso dalle tre viste: `Current Model`, poi ogni Proposal aperta non legata a un Drift (mai Draft, in refinement o allegate). Se la Proposal smette di essere aperta si torna a `Current Model` con un avviso. Il Modello è disegnato con la Proposal applicata sopra quelle richieste. Colori (600 chiaro / 400 scuro): aggiunto `green`, cambiato `amber`, rimosso `red` con bordo tratteggiato; sempre anche l'etichetta testuale `added`/`changed`/`removed` in dettagli, hover e nome accessibile.
      - **Focus**: proprio per ciascuna vista, aperto da `View on diagram` / `View requirements` (scope di una Proposal o refs di un Task) o da un chip (un singolo ref). Passi: livelli in ordine di Modello sul Diagram, sezioni/Area/campi in ordine di pagina altrove. Barra con soggetto (`Proposal <id> – <title>`, `Task <id> – <title>` o il ref), nome del passo, `<n> of <N>`, `Previous`/`Next`, `<M> not found` con tooltip, `Only these` sui Requirements, `Clear`. Sul Diagram elementi a fuoco in `cyan` (o colore di cambio), il resto al 30% ma cliccabile; il viewport contiene il passo senza superare `Fit`. Ricalcolo ai cambi di file restando sullo stesso passo; Esc toglie prima la selezione poi il focus.
      - **Requirements view**: Aree in ordine di cartella, frammenti in ordine di file (`<fragment>.md` in muted), sezioni con `General` prima; chip di riferimento che apre l'elemento sul Diagram e `Open in editor` alla riga del titolo. Etichette overlay `new fragment`, `fragment removed`, `added`, `removed`, `changed`, `moved from <fragment>`; rimossa barrata, cambiata con `Show before`.
      - **Specs view**: campi in ordine fisso (languages, frameworks, data, testing, CI, repository, deployment, infrastructure, notes), vuoti nascosti; larghezza massima 1100 px, griglia di colonne ≥ 300 px; voce con nome, versione, nota e `Sparkles` `agent's choice` col motivo in tooltip; overlay come sui Requirements.
      - **Errori**: Modello o Requisiti invalidi mostrano in cima alle tre viste l'`Alert` rosso `The Model has <n> errors` con elenco espandibile (messaggio, file, riga, `Open in editor`), file illeggibili inclusi. `Open in editor` ovunque file e riga sono noti; un errore dà un avviso col motivo.

      ## Fatto quando
      - Test Vitest: navigazione dei livelli da tastiera, breadcrumb, pannello dettagli, focus con passi e `not found`, overlay con etichette testuali, banner errori.
      - Ai cambi di file livello, selezione e focus restano o ripiegano come descritto.
      - Il test end-to-end vede il Diagram renderizzato nel progetto di prova.
---
# Shell, finestre, impostazioni e viste di sola lettura

La shell fissa le fondamenta dell'interfaccia: testi in inglese, nessuno stato proprio di progetto, eventi dal backend, tema, icone e regole di qualità valide in tutto Weaver.

La Machine `window` gestisce un'istanza con una finestra per progetto, la finestra di benvenuto con i recenti e la riapertura dei progetti dell'ultima sessione, con ripresa dei job che lavoravano.

Le impostazioni vivono in un solo file con schema tipato e vista Settings, che ospita anche i box di omp e Update skills.

Le viste Diagram (cytoscape con elkjs), Requirements e Specs mostrano in sola lettura Modello, Requisiti e Specs, con Proposal aperta sovrapposta e focus, calcolati dal backend.
