---
created: 2026-11-02T12:31:02.779Z
updated: 2026-11-02T12:31:02.779Z
summary: "Lettura, validazione e scrittura di .trama/ in trama-core come funzioni pure, e binario trama con cornice comune e regole dell'Actor agent: il solo canale con cui l'agente raggiunge Weaver."
tasks:
  - vxdhoylk
  - kmvi79rq
cleared:
  kmvi79rq:
    title: Riga di comando trama
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T09:06:48.876Z
    refs:
      - agent
      - requirements/cli
      - requirements/cli#General
      - requirements/cli#agent
    body: |-
      Il binario `trama` (`crates/cli`, sopra `trama-core`) è il modo in cui l'agente raggiunge Weaver. Questo Task costruisce la cornice comune e le regole dell'Actor `agent`; i comandi dei singoli oggetti nascono nei Task "Proposal", "Task", "Drift", "History", "Round di domande", "Lavori dell'agente" e "Progetto e skill installate". Dipende da "Scheletro del progetto" e "Core dei file del progetto".

      ## Cornice comune (`requirements/cli#General`)
      - Forma unica `trama <object> <verb> [<id>] [<state>] [--input <json>|-] [--root <dir>] [--json]` più `-h`/`--help`; nessun'altra opzione, nessun alias. `--root` usa `<dir>/.trama`, mai una cartella genitore; senza `.trama/` tutto tranne `project new` e help esce con 1 dicendo di inizializzare il progetto.
      - Un solo registro dei comandi produce parsing, dispatch, help, JSON Schema e il riferimento dei comandi nelle skill installate. Un comando esistente non risponde mai "unknown command".
      - Help: `trama --help` elenca oggetti e verbi; `<object> <verb> --help` dà uso, campi con tipi e limiti, un esempio e i campi di output; `--help --json` stampa `{command, input, output}` (JSON Schema 2020-12). L'help non legge stdin né file e non contatta l'app.
      - Input: un oggetto JSON camelCase, inline o da stdin con `--input -` (BOM accettato), validato per intero prima di tutto; ogni problema (campo sconosciuto, tipo, lista, intervallo, obbligatorio mancante) è elencato col suo percorso JSON (`questions[0].options[1].label`), exit 2. Un comando senza campi accetta solo `{}` o niente.
      - Carte per identità, prefisso univoco o numero breve (`P3`, `t3`: lettera più cifre è sempre un numero breve, la lettera deve combaciare col tipo), risolto in identità sotto il lock del progetto; un numero non vivo è rifiutato (exit 1).
      - Output Markdown su stdout; con `--json` un oggetto in cui ogni carta ha `id` e `number` (`null` senza numero vivo). Errori su stderr e, con `--json`, anche `{error: {code, message}}`. Exit `0` fatto, `1` rifiutato o fallito, `2` uso o input errato; un rifiuto non scrive nulla. Nessun file di log.
      - Le letture girano nella CLI. Un comando che scrive va all'app (via `TRAMA_ENDPOINT` o `runtime.json`, `requirements/tech#General`), che lo esegue nella coda del progetto; se nessuna app risponde la CLI lo esegue da sé sotto il lock, quindi funziona ad app chiusa. I comandi legati a un lavoro escono allora con 1 dicendo che Weaver non è in esecuzione.

      ## Regole dell'Actor `agent` (`requirements/cli#agent`)
      - Event come comandi: `ANNOUNCE_CARD` `announcement new`; `DECLARE_NOTE` `job edit`; `EDIT_PROPOSAL` `proposal edit`; `SUBMIT_PROPOSAL` `proposal move <id> open`; `TAKE_TASK`, `RELEASE_TASK`, `FINISH_TASK`, `CLOSE_TASK_UNCHANGED` `task move doing|todo|done {unchanged?}`; `PREPARE_DRIFT_QUESTION` `drift edit {prepared}`; `ASK_QUESTIONS`/`STOP_WAITING` `question new`; `UPDATE_HISTORY_ENTRY` `history edit`; `START_JOB` `job new`.
      - `TRAMA_JOB`, messo da Weaver nell'ambiente di ogni omp, viaggia col comando; l'app lo accetta solo per un `job` del progetto in `Running` o `Waiting for the developer`.
      - `announcement new`, `job edit` e `task move` sono rifiutati (exit 1, nulla scritto) per ogni altro chiamante, dicendo che solo l'agente di un lavoro di Weaver può farlo (per esempio prendere un Task fuori da un lavoro); `question new` risponde `unavailable`; `job new` richiede l'identità di General (`TRAMA_GENERAL`).
      - `proposal new`, `task new` e `drift new` funzionano senza `TRAMA_JOB` e lo registrano come `job` quando c'è.
      - I comandi di decisione (`proposal move approved|rejected|discarded`, `drift move`, `task delete`) accettano qualsiasi chiamante e contano come Event del developer, con le stesse condizioni ricontrollate all'applicazione; il loro `--help` permette all'agente di usarli solo se il developer ha chiesto esplicitamente quella decisione. Scelta del modello di una Proposal e risposta a una domanda preparata esistono solo nell'app.
      - Help ed errori in inglese, senza prima persona, nominano `Weaver` e `the agent`.

      ## Fatto quando
      - Test di contratto sul binario, per ogni comando del registro: `--help`, `--help --json`, exit 2 con percorsi dei campi, exit 0 e 1, output Markdown e `--json`, `--input -` con e senza BOM.
      - Un test fallisce se il riferimento nelle skill differisce dal registro.
      - `task move <id> doing` senza un `TRAMA_JOB` valido esce con 1 e non scrive nulla; ad app chiusa un comando che scrive non legato a un lavoro riesce sotto il lock.
      - `t3` usato per una Proposal, o un numero non vivo, esce con 1.
      - La suite passa uguale con o senza variabili `TRAMA_*` ereditate.
  vxdhoylk:
    title: Core dei file del progetto
    proposal: qmqh1o3p
    tag: Weaver
    finished: 2026-11-02T08:04:59.077Z
    refs:
      - requirements/data
      - requirements/data#General
      - requirements/rules
      - requirements/rules#General
    body: |-
      Il cuore di `trama-core`: lettura, validazione e scrittura di `.trama/` come funzioni pure e testate, e l'immagine in memoria di un progetto. È l'unico codice che tocca i file del progetto; app e CLI `trama` passano entrambe da qui, l'interfaccia mai. Dipende da "Scheletro del progetto" (crate, lock del progetto, coda delle mutazioni). Le regole delle singole carte (Proposal, Task, Drift, History, richieste) le aggiungono i Task che le riguardano, sopra queste fondamenta.

      ## Formato (`requirements/data#General`)

      - Layout di `.trama/` come da `requirements/data#General`: `model/`, `requirements/<area>/`, `specs.yaml` facoltativo, `proposals/`, `tasks/`, `drifts/`, `history/` versionati; `requests/`, `short-numbers/`, `task-images/` solo locali, ciascuno con un proprio `.gitignore` contenente `*`, riscritto quando manca. File e cartelle che iniziano con `.` non sono mai letti come Model, Requisiti, Specs o carte.
      - Parser di Model, Requisiti e Specs esattamente secondo il riferimento `FORMAT.md` della skill `trama-design` (Actor, Machine con `## States` e `## Events`, un solo `(initial)` per livello, `(final)` senza sotto-State né Transition, Transition come voce di lista che inizia con uno span di codice `owner.EVENT` → target nella stessa Machine, nomi di State unici senza `/ : ( ) →`, Event `UPPER_SNAKE_CASE`, namespace unico di Feature/Machine/Actor e nomi riservati; Area con `area.md` senza sezioni e frammenti con `## General` o una sola referenza per sezione, sezione unica nell'Area; campi e voci delle Specs). Le referenze agli elementi (`feature`, `machine`, `actor`, `machine/State`, `machine/State/owner.EVENT`, `owner.EVENT`, `requirements/<area>#<section>`, `specs#<field>`) si risolvono qui. Queste regole sono quelle di `trama model check`, che segnala anche gli avvisi (frammento oltre 12.288 byte) senza fallire.
      - Il parser e `FORMAT.md` sono testati insieme: ogni esempio del file viene interpretato e ogni regola che enuncia è applicata. Il testo di `FORMAT.md` lo scrive il Task "Testi delle skill".
      - Identità di carte e lavori `^[a-z0-9]{8}$` dal CSPRNG, riestratta se già usata dallo stesso tipo (per i Task: `.trama/tasks/`, copie `cleared` in `history/` e `tasks/` di ogni Proposal); letta sempre dal nome del file o della cartella. Slug: minuscolo, senza diacritici, ogni sequenza fuori da `[a-z0-9]` diventa un `-`, niente `-` ai bordi, al massimo 40 caratteri senza `-` finale, `untitled` se vuoto; fissato alla creazione.
      - Una sola funzione di lettura: toglie il BOM e porta CRLF a LF prima di interpretare, calcolare hash o confrontare. Scrittura in UTF-8 senza BOM, LF, newline finale. Front matter YAML con le chiavi nell'ordine dei Requisiti, chiavi facoltative assenti non scritte, chiavi sconosciute conservate. Istanti UTC ISO 8601 con millisecondi dall'orologio iniettato, mai un valore segnaposto.
      - Scrittura atomica: `.<name>.<random>.tmp` nella stessa cartella, flush, rename; su Windows un rename rifiutato per condivisione o accesso negato è ritentato ogni 50 ms per al massimo 1 s.
      - Un file illeggibile o non interpretabile diventa una diagnostica con percorso e motivo, mostrata nella finestra e nell'output della CLI; mai scartato in silenzio né riscritto, e gli altri file si caricano comunque.

      ## Regole comuni (`requirements/rules#General`)

      - Numeri brevi `P`/`T`/`D` + il più basso intero libero del tipo, assegnati in ordine di creazione alle carte vive, liberati alla chiusura (un Task Done di un design tiene il numero finché tutti i Task del design sono Done). Input in qualsiasi maiuscola; lettera + cifre è sempre un numero breve, con lettera sbagliata o numero non vivo rifiutato. Cache solo in `short-numbers/assigned` (`<P|T|D> <identity> <number>`), scritta in modo atomico; mancante, illeggibile, con duplicati o carte sconosciute è ricostruita in silenzio e nessun comando fallisce per essa. Mai numeri brevi in file versionati.
      - Ordine: `created` più vecchio prima, parità per identità in ordine di byte.
      - Ogni cambiamento di carte, sotto il lock del progetto, legge, valida tutto contro lo stato dopo il cambiamento, poi scrive; tutto o niente: se una scrittura successiva fallisce i file già scritti tornano al contenuto precedente; un rifiuto non lascia file, cache né lock.

      ## Immagine del progetto

      Un valore in memoria con Model, Requisiti, Specs, carte e diagnostiche; analisi incrementale (ogni file tenuto con il suo hash, si rianalizzano solo i percorsi cambiati o rimossi), dati derivati ricalcolati per intero, costo lineare nel numero di file.

      ## Fatto quando

      - Test unitari per ogni regola di `FORMAT.md`, con un test che interpreta tutti gli esempi del file installato.
      - Test su cartelle temporanee: BOM e CRLF in lettura, LF in scrittura, percorsi Windows con spazi e caratteri non ASCII, file illeggibile riportato senza bloccare gli altri, scrittura multi-file annullata al fallimento.
      - Test di slug, identità (riestrazione su collisione), numeri brevi (assegnazione, riuso, ricostruzione della cache) e ordine, con orologio e casualità iniettati.
---
# Core dei file del progetto e riga di comando trama

Il core dei file del progetto è l'unico codice che tocca `.trama/`: layout, formato delle carte, validazione e scrittura come funzioni pure e testate, più l'immagine in memoria del progetto. App e CLI passano entrambe da qui, l'interfaccia mai.

La riga di comando `trama` (crates/cli, sopra trama-core) fornisce la cornice comune `trama <object> <verb> ...` con `--input`, `--root` e `--json`, e le regole dell'Actor agent. Su questa cornice si innestano i comandi dei singoli oggetti.
