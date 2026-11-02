---
column: done
technical: true
refs:
  - requirements/rules#project
  - requirements/rules#job
drift: null
proposal: qmqh1o3p
agent: false
created: 2026-11-02T07:29:32.875Z
finished: 2026-11-02T08:43:33.138Z
---
# Testi delle skill

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
