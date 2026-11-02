---
column: done
technical: false
refs:
  - agent
  - requirements/cli
  - requirements/cli#General
  - requirements/cli#agent
drift: null
proposal: qmqh1o3p
agent: false
created: 2026-11-02T07:29:32.875Z
finished: 2026-11-02T09:06:48.876Z
---
# Riga di comando trama

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
