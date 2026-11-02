---
created: 2026-11-02T12:31:02.830Z
updated: 2026-11-02T12:31:02.830Z
summary: Machine project (inizializzazione, Modello vuoto, Aree iniziali, file per l'agente) e testi delle skill, dei sotto-agenti e delle istruzioni dei lavori che Weaver installa nel progetto.
tasks:
  - 2wlt7hk8
  - sxe0jtid
cleared:
  2wlt7hk8:
    title: Progetto e skill installate
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T12:06:53.593Z
    refs:
      - project-setup
      - project
      - workspace
      - requirements/tech#project
      - requirements/rules#project
      - requirements/cli#project
      - requirements/ui#project
    body: |-
      Realizza la Machine `project` e i file che Weaver installa per l'agente. Dipende da "Scheletro del progetto", "Core dei file del progetto" e "Riga di comando trama"; i testi installati vengono da "Testi delle skill".

      ## Machine `project`
      - `Not initialised`: nessuna `.trama`; ogni azione che richiede un progetto dice di inizializzarlo, mai cartelle genitore. `developer.INITIALISE_PROJECT` → `Empty Model`: crea `.trama` con un Modello vuoto (System col nome della cartella), le Aree iniziali `ui`, `tech`, `data`, `rules`, `quality` (solo `area.md`, titoli e descrizioni di `requirements/rules#project`) e ogni file per l'agente. Se file da installare esistono con contenuto diverso, non crea nulla finché il developer non conferma la sovrascrittura; un fallimento non lascia nulla. `workspace.PROJECT_FOLDER_APPEARED` → `Empty Model` o `Has a Model` secondo il contenuto.
      - `Empty Model` (nessuna Feature né Actor; Drift e Requisiti non contano): `proposal.APPROVED` e `workspace.FILES_CHANGED` portano a `Has a Model` se ora c'è una Feature o un Actor, altrimenti restano e rigenerano le skill dei Requisiti (per `FILES_CHANGED` solo se i Requisiti sono cambiati). `developer.UPDATE_SKILLS` riscrive i file diversi, crea le Aree iniziali se mancano i Requisiti e riporta sempre cosa ha cambiato o l'errore. `workspace.PROJECT_FOLDER_REMOVED` → `Not initialised`.
      - `Has a Model`: all'apertura della finestra confronta in background ogni file installato e riscrive quelli diversi, anche modificati a mano; parla solo se ha cambiato qualcosa e mostra un fallimento a ogni apertura finché non sparisce. `proposal.APPROVED` rigenera le skill dei Requisiti; `FILES_CHANGED` resta o torna a `Empty Model` se non c'è più Feature né Actor; `UPDATE_SKILLS` come sopra; `PROJECT_FOLDER_REMOVED` → `Not initialised`, i lavori in corso continuano.

      ## File installati (`requirements/rules#project`, `requirements/tech#project`)
      - Un solo elenco nel core, incorporato con `include_dir` (app e CLI scrivono byte identici): le sette skill `trama-*` in `.agents/skills/`, `.omp/agents/trama-analysis.md`, `.omp/agents/trama-implementer.md`, `.omp/WATCHDOG.yml` e le skill generate `trama-req-*`.
      - Confronto puro (CRLF→LF, byte per byte), senza versioni né rete; si scrive atomicamente solo ciò che manca o differisce, nient'altro si tocca tranne le `trama-req-*` che il modo corrente non produce. Un controllo per progetto alla volta, le richieste intanto si fondono; mai ritarda la finestra.
      - Skill dei Requisiti: budget = caratteri di `trama-req-<area>-<fragment>` più descrizione, sommati sui fragment con contenuto; fino a 4.000 incluso modo fragment, oltre modo index (una skill per Area), tutte le Aree insieme. Rigenerate da una sola funzione del core all'approvazione (app o CLI), all'inizializzazione, a Update skills, al controllo d'apertura e 1 s dopo l'ultima modifica sotto `.trama/requirements/`. Un unico risultato strutturato (file scritti e rimossi, modo, budget, cambio di modo) per app e CLI.
      - Watcher `notify` per progetto: radice non ricorsiva per `.trama` che compare o sparisce, `.trama/` ricorsiva; le scritture proprie (hash registrato) non sono `FILES_CHANGED`; la cancellazione in `requests/` è `workspace.REQUEST_DELETED`.

      ## Interfaccia (`requirements/ui#project`)
      - Get Started in ogni vista tranne Settings, con l'elenco dei file installati e `Initialise`; dialog `Some agent files already exist` con `Overwrite and initialise`/`Cancel`; errore `Could not initialise: <reason>.`.
      - In `Empty Model` due card sopra la Board: `Start from Scratch` apre una richiesta di setup (vedi "Richiesta di design"); `Import from Code` apre l'albero delle cartelle con caselle, `<N> excluded`, `Start import` (disabilitato se nulla è incluso) che avvia il lavoro di import (vedi "Lavori dell'agente"). Le esclusioni valgono solo per quell'import, mai salvate. Avviso su Start from Scratch quando la cartella ha altri nomi senza punto; le card si nascondono mentre lavora un setup o un import o c'è una Proposal aperta.
      - Update skills in Settings (gruppo `Project`) col report; toast all'apertura se ha riscritto file, banner con `Retry` se fallisce.

      ## Comandi (`requirements/cli#project`)
      `project new {force?}` rifiuta se `.trama/` esiste o se file installati differiscono (tutti elencati) senza `force: true`; `project edit {}` è Update skills e stampa `changed`, `areas`, `generated`; `project show {}` stampa la Board (`toDecide`, `inWriting`, `todo`, `doing`).

      ## Fatto quando
      - Test su cartelle temporanee: inizializzazione completa, conflitto senza conferma che non crea nulla, fallimento senza residui.
      - Test del budget a 4.000 e 4.001 caratteri, del cambio di modo e della rimozione delle `trama-req-*` non più prodotte.
      - Cancellare e ricreare `.trama` porta la finestra a Get Started e indietro; un file installato modificato a mano viene riscritto all'apertura con il toast.
      - Test UI di Get Started, dialog di conflitto e albero di Import from Code con tastiera.
  sxe0jtid:
    title: Testi delle skill
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T08:43:33.138Z
    refs:
      - requirements/rules#project
      - requirements/rules#job
    body: |-
      I testi che Weaver porta con sé e installa per l'agente, e le istruzioni di una riga di ogni lavoro. Dipende da "Scheletro del progetto" e da "Core dei file del progetto"; l'installazione e il confronto dei file, e la generazione delle skill `trama-req-*`, li realizza "Progetto e skill installate"; il registro dei comandi da "Riga di comando trama".

      ## File da scrivere (`requirements/rules#project`)

      - Skill in `.agents/skills/<name>/SKILL.md`: `trama-setup`, `trama-design` con il riferimento `FORMAT.md`, `trama-import`, `trama-scan`, `trama-compare`, `trama-implement`, `trama-resolve-drifts`. Definizioni dei sotto-agenti omp: `.omp/agents/trama-analysis.md` (confronta una Machine seguendo `trama-compare`, `model: "@slow"`, cioè il modello di scan) e `.omp/agents/trama-implementer.md` (realizza da uno a tre Task in ordine, nessun modello: eredita quello del Run). Istruzioni dell'advisor dell'Advisor Scanner: `.omp/WATCHDOG.yml`.
      - Tutti in inglese, LF, senza prima persona (dicono `Weaver` e `the agent`); ogni definizione di sotto-agente sotto 6 KB. Un'unica lista nel core, incorporata con `include_dir` in `trama-core`, così app e CLI scrivono gli stessi byte.

      ## Come riscriverli

      Fonte: le skill attuali di Trama in `.agents/skills/` di questo repository e i file in `.omp/`, da leggere per intero. Resta la stessa disciplina: ogni cambiamento di Model, Requisiti e Specs passa da una Proposal (Draft → hand-over con `trama proposal move <id> open`, riaperta con `trama proposal edit <id>`); intervista a round di domande, ogni domanda con 2–5 opzioni e una raccomandazione, solo con `trama question new` eseguito con bash `timeout: 0` e attesa senza limite, mai con lo strumento di domande dell'agente né nella conversazione; decisioni (approvare, rifiutare, scartare, decidere un Drift, cancellare un Task) solo su richiesta esplicita del developer nella conversazione; numeri brevi solo verso il developer, identità permanenti in comandi e file; titoli brevi; sezioni "Language" (testi per il developer nella lingua dell'istruzione, Model, Requisiti, Specs e codice sempre in inglese, nessuna prima persona) e "Sub-agents context sharing"; Task autosufficienti; regole di scrittura dei Drift e dei Fix options; "Already present".

      Adattamenti a Weaver (`requirements/cli/commands.md`):
      - Solo i comandi di Weaver: annunci con `trama announcement new {kind: proposal|task|drift, title}` e campo `announcement`; note di avanzamento con `trama job edit {note}` (3–6 per lavoro, una riga); `trama task list {runnable, max, groups, proposals, notProposals}`; `trama drift list {all, refs, currentJob, unprepared}`; `trama drift edit <id> {prepared}`; `trama history new|edit`. Esiti di `trama question new`: `answered`, `skipped` per domanda (si usa la raccomandazione e la si dichiara come assunzione), `unavailable` (fermarsi e dirlo al developer), `interrupted` e `cancelled` (fermare il lavoro, nessuna risposta parziale), `invalid-input` (correggere il campo).
      - Sempre attivi, senza frase condizionale: esecuzione parallela dei Task (gruppi, al massimo 3 Task per `trama-implementer`), Sub-agents Context Sharing, Advisor Scanner (nessun pre-flight né scan di fine Run; note dell'advisor corrette prima di Done; Drift dell'advisor senza origine), Async Drift Resolver (alla fine del lavoro, domande preparate per i Drift del lavoro), rework con feedback, raffinamento di una Proposal stale (`rebase: true`).
      - Setup e import scritti su un Modello vuoto sono approvati da soli alla consegna se nulla li blocca; la Proposal di setup porta Model, Requisiti, Specs e Task; l'import descrive il codice com'è, senza Task, anche più Proposal a catena con `requires`; `trama-compare` legge da sé Machine, Task e Drift esistenti e registra i suoi Task e Drift con `trama task new`/`trama drift new`; `trama-scan` confronta sempre tutto il Sistema, una Machine per sotto-agente `trama-analysis`.
      - Fuori ambito, da togliere: Team e handoff, `trama-merge` e Solo, PR Check, Unify Drifts, Ask more, terminali, hook, comparison, check e fasi annunciate, timing, progress, pre-flight e scan di fine Run.
      - Il riferimento dei comandi nelle skill resta coerente con il registro dei comandi (un test lo verifica). Dove si nomina una skill dei Requisiti, si nomina quella del modo corrente.
      - `WATCHDOG.yml`: due advisor con strumenti `read`, `grep`, `glob`, `bash` e nessun modello (turn reviewer `reviewMode: turn`, `syncBacklog: off`; final reviewer `reviewMode: agent-end`, `syncBacklog: strict`); le istruzioni eseguono `trama job show` per primo in ogni sessione e senza il marcatore di Run restano in silenzio; mai spostare Task, modificare file, annunciare o decidere.

      ## Istruzioni dei lavori (`requirements/rules#job`)

      Una funzione pura del core compone l'istruzione: una riga in inglese, solo testo fisso, identità permanenti, percorsi relativi tra backtick, numeri e il nome della lingua; mai un numero breve né il testo di una richiesta. Prima frase per tipo esattamente come la tabella del Requisito (Setup, Design, Import con o senza percorsi esclusi, Scan, Run di un Task, Run all con `proposals`/`notProposals` per gruppo di modello, Resolve con eventuale risposta già data in JSON di una riga, Rework con la skill del tipo, Refinement), poi in ordine: limite di sotto-agenti, Sub-agents context sharing, domande solo con `trama question new` senza limite di tempo, e la lingua solo se impostata. Le funzioni sempre attive non aggiungono frasi.

      ## Fatto quando

      - Test: ogni testo installato è in inglese, LF, senza `I`/`we`/`my`/`our`, ogni sotto-agente sotto 6 KB, nessun comando citato assente dal registro e nessun termine fuori ambito elencato sopra.
      - Test della funzione delle istruzioni: ogni tipo produce la frase attesa, con e senza lingua, senza numeri brevi.
      - `FORMAT.md` passa il test congiunto con il parser di "Core dei file del progetto".
---
# Progetto e skill installate

La Machine `project` gestisce il ciclo da `Not initialised` a `Empty Model`: crea `.trama` con un Modello vuoto, le Aree iniziali `ui`, `tech`, `data`, `rules`, `quality` e tutti i file per l'agente, senza mai toccare cartelle genitore e senza sovrascrivere file esistenti con contenuto diverso.

I testi installati comprendono le skill `trama-setup`, `trama-design`, `trama-import`, `trama-scan`, `trama-compare`, `trama-implement`, `trama-resolve-drifts`, le definizioni dei sotto-agenti omp e le istruzioni di una riga di ogni lavoro. Le skill `trama-req-*` sono generate dai Requisiti.
