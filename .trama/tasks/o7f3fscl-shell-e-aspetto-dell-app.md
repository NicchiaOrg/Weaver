---
column: done
technical: true
refs:
  - requirements/ui
  - requirements/ui#General
  - requirements/quality#General
drift: null
proposal: qmqh1o3p
agent: false
created: 2026-11-02T07:29:32.875Z
finished: 2026-11-02T08:05:15.327Z
---
# Shell e aspetto dell'app

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
