---
technical: true
refs:
  - requirements/tech#job
  - requirements/data#job
  - requirements/tech#general-chat
---
# Certificati per job e General

Ogni processo omp ACP avviato da Weaver, per un job o per il General di un progetto, riceve le variabili dei System Certificates (`requirements/tech#system-certificates`), come tutti gli altri processi omp della tabella di `requirements/tech#omp` (sezione «Certificate variables»). Le variabili sono lette a ogni avvio: con l'impostazione `System Certificates` spenta, o senza bundle, non se ne aggiunge nessuna; con l'impostazione accesa si usano il percorso del bundle e i nomi di variabile prodotti dalla funzione condivisa costruita dal Task del gruppo `system-certificates` (questo Task non la reimplementa, la usa).

Quando è fatto:
- **Job, avvio:** l'ambiente del figlio resta quello di oggi (via le `WEAVER_*` ereditate, `WEAVER_JOB`, `WEAVER_ENDPOINT`, `WEAVER_RUN`/`WEAVER_TASK` per le Run, PATH come in `## omp`) più le variabili dei certificati del momento (`requirements/tech#job`, «Environment»).
- **Job, ripresa:** al resume omp viene rilanciato con gli stessi argomenti ed env salvati nel registro **più** le variabili dei certificati di quel momento, non di quelle del primo avvio (`requirements/tech#job`, «Resume», passo 2). Se nel frattempo l'impostazione è stata spenta, il processo ripreso non le riceve.
- **Registro:** `<app local data>/jobs/<job id>.json`, campo `env`, contiene solo le variabili impostate da Weaver e **mai** le variabili dei System Certificates, che sono ricalcolate a ogni avvio e ripresa (`requirements/data#job`). Le variabili vanno quindi aggiunte dopo aver costruito l'env salvato, non dentro di esso.
- **General:** stesso trattamento: `WEAVER_GENERAL`, `WEAVER_ENDPOINT`, PATH e le variabili dei certificati come in `## omp`, lette a ogni avvio del processo (anche dopo che si è fermato e il messaggio successivo lo riavvia) (`requirements/tech#general-chat`, «Environment»).
- Un processo già in esecuzione non viene riavviato quando l'impostazione cambia: il nuovo valore vale dal prossimo avvio o ripresa.
- Test: l'env passato al figlio contiene le variabili con l'impostazione accesa e un bundle presente, non le contiene con l'impostazione spenta; il registro scritto non le contiene mai; il resume usa il valore corrente.

Sezioni da leggere: `requirements/tech/acp.md` (`## job` «Environment», «Resume registry», «Resume»; `## general-chat` «Environment»), `requirements/data/settings.md` (`## job`), `requirements/tech/omp.md` («Certificate variables»), `requirements/tech/system-certificates.md`.

Punti di partenza nel codice:
- `requirements/tech#job` — crates/core/src/jobs/launch.rs:`child_env` (unisce ereditato, env impostato e PATH; punto naturale per un parametro con le variabili dei certificati)
- `requirements/tech#job` — src-tauri/src/jobs/actor.rs:`prepare` / `launch` (env del registro, ramo di resume)
- `requirements/data#job` — src-tauri/src/jobs/actor.rs:`prepare` (costruzione dell'`env` salvato nel registro)
- `requirements/tech#general-chat` — src-tauri/src/general/actor.rs:`launch`
- `requirements/tech#job` — src-tauri/src/jobs/proc.rs:`OmpProcess::spawn` (usa `env_clear`, quindi le variabili devono arrivare nell'env esplicito)
- `requirements/tech#job` — src-tauri/src/jobs/host.rs:`Host::settings`, `Host::child_path` (accesso a impostazioni e percorsi; fakes in src-tauri/src/jobs/testing.rs)
