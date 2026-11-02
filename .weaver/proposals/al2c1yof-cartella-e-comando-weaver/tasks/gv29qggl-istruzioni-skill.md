---
refs:
  - job
  - requirements/rules
---

# Istruzioni e skill installate

La cartella del progetto diventa `.weaver` al posto di `.trama` e il comando diventa `weaver` al posto di `trama`. Questo Task porta il cambio nei testi che Weaver manda agli agenti o installa nei progetti: le istruzioni dei job, il preambolo di General, i testi delle skill e dei sub-agent installati e i testi delle regole dei Requirement.

## Cosa deve essere vero alla fine

**Istruzioni dei job.** In `job/Queued`, Weaver avvia omp nel progetto con la command line `weaver` sul path e il job già noto, poi manda l'istruzione. Le transizioni da Not started verso Queued (SEND_REQUEST, IMPORT_CODE, SCAN_SYSTEM, RUN_TASK, RUN_ALL, proposal.APPROVED, RESOLVE_DRIFT, question-round.ANSWERED, REWORK_PROPOSAL, APPROVE_PROPOSAL, agent.START_JOB) non cambiano, ma i job ricevono i nuovi testi. La tabella delle istruzioni in rules#job diventa:

- Setup: ``Use the trama-setup skill on the setup request in `.weaver/requests/<request>.md`.``
- Design: ``Use the trama-design skill on the design request in `.weaver/requests/<request>.md`.``
- Run all: ``Use the trama-implement skill on the runnable Tasks: take them from `weaver task list --input '{"runnable":true,"max":<N>,"groups":true}'`, one batch at a time, until it says there is none.``
- frase aggiunta 1: `Run at most <N> sub-agents at once.`, dove N è anche il `max` di `weaver task list`;
- frase aggiunta 3: ``Ask the developer only with `weaver question new`, as the skill says, never with your own question tool; run it with no time limit and wait until it ends.``
- Le altre righe della tabella non cambiano.

**Preambolo di General.** Nomina `.weaver` e il comando `weaver` invece di `.trama` e `trama`.

**Skill e sub-agent installati.** Nessun testo installato menziona `.trama` o il comando `trama`. Restano solo i nomi delle skill e dei sub-agent `trama-*`. In dettaglio:

- Ogni comando `trama <verb>` diventa `weaver <verb>`.
- Ogni percorso `.trama/...` diventa `.weaver/...`.
- La parola «Trama» diventa «Weaver» dove indica il prodotto.
- Le description delle skill che dicono "a project with a `.trama/` folder" passano a `.weaver/`. Gli agenti scelgono la skill da questa frase, quindi va cambiata ovunque.
- La regola greenfield di `trama-scan` ("only `.trama/`") diventa "only `.weaver/`".
- I nomi restano invariati: `trama-setup`, `trama-design` (con `FORMAT.md` e `COMMANDS.md`), `trama-import`, `trama-scan`, `trama-compare`, `trama-implement`, `trama-resolve-drifts`, i sub-agent `.omp/agents/trama-analysis.md` e `.omp/agents/trama-implementer.md`, `.omp/WATCHDOG.yml` e le skill generate `trama-req-*`.

**Testi delle regole (requirements/rules).** Rispecchiano la versione del Model:

- La prosa dell'area dice: "Every rule here is computed in the Rust core, the same for the app and the `weaver` command line, never in the interface."
- rules#General: le identità sono uniche tra `.weaver/tasks/` e i `tasks/` di ogni Proposal; la cache dei numeri brevi sta "under `.weaver/`".
- rules#task: "Slots: `weaver task list` with `runnable` and `max` N…".
- rules#drift: il fingerprint usa il testo che stampa `weaver model show <ref>`; "`weaver drift list` with `refs` marks Dismissed Drifts…".
- rules#project: i file installati sono incorporati "in the binary that the app and the `weaver` command line share". La rigenerazione avviene "1 s after the last change under `.weaver/requirements/`". "`weaver model check` warns … each fragment over 12,288 bytes…".

Ovunque nel codice ci siano testi derivati da queste regole (messaggi, avvisi, commenti), vanno allineati.

## Indizi nel codice (da ricontrollare)

- `crates/core/src/job_instruction.rs`: intorno a :194 e :199 le istruzioni Setup e Design scrivono `.trama/requests/{request}.md`. I test intorno a :309 e :313 vanno aggiornati. Nello stesso file ci sono anche la stringa di Run all (`trama task list ...`) e "Ask the developer only with `trama question new`", da verificare.
- `src-tauri/src/jobs/general.rs` intorno a :480: il preambolo di General nomina `.trama`/`trama`.
- Asset installati, incorporati con `include_dir` da `crates/core/assets` (`crates/core/src/agent_files.rs:40`). Menzioni di `.trama` contate durante l'esplorazione:
  - `skills/trama-design/SKILL.md`: 9
  - `skills/trama-design/FORMAT.md`: 3
  - `skills/trama-setup/SKILL.md`: 5
  - `skills/trama-scan/SKILL.md`: 4, inclusa la regola greenfield
  - `skills/trama-import/SKILL.md`: 3
  - `skills/trama-implement/SKILL.md`: 3
  - `skills/trama-resolve-drifts/SKILL.md`: 3
  - `skills/trama-compare/SKILL.md`: 1
  - `omp/agents/trama-implementer.md`: 2
  
  Vanno cercati anche i comandi `trama <verb>` e la parola «Trama».
- `crates/core/src/project_files/requirement_skills.rs`: le skill generate non contengono `.trama`. `SKILL_PREFIX` `trama-req-` (circa :26) resta.

## Fuori dallo scope di questo Task

- `crates/core/assets/skills/trama-design/COMMANDS.md`: è generato dal registry della CLI e lo rigenera un altro Task della Proposal. Non va modificato a mano.
- La costante della cartella, i percorsi `.trama` in codice, test e UI, il fatto che `.trama` venga ignorata, il testo di Get Started e la macchina del progetto: li gestisce un altro Task della Proposal.
- Altre parti che gestisce un altro Task della Proposal:
  - rinomina dei crate (`weaver-core`, `weaver-cli`);
  - binario `weaver-cli`, sidecar e copia `weaver` in `<app local data>/bin/` con rimozione della vecchia copia `trama`;
  - variabili d'ambiente `WEAVER_ENDPOINT`, `WEAVER_JOB`, `WEAVER_GENERAL`, `WEAVER_RUN`, `WEAVER_TASK`;
  - help e usage della CLI;
  - testo fisso di rifiuto dei form di omp;
  - messaggi di log del watcher;
  - README, descrizioni Cargo e doc comment;
  - workflow CI.
- Le copie installate nella repository di Weaver (`.agents/skills/*`, `.omp/agents/*`) e la cartella `.trama` della repository non vanno toccate né spostate. Lo sviluppatore le sposta a mano in seguito.

## Fatto quando

- Ogni istruzione che Weaver manda nomina `.weaver/requests/<request>.md` e comandi `weaver`.
- Nessun testo degli asset installati menziona `.trama` o il comando `trama`, tranne i nomi `trama-*` delle skill e dei sub-agent.
- I test delle istruzioni dei job sono aggiornati.
- Quando anche il Task che mette `weaver` sul PATH e imposta le variabili `WEAVER_*` è completato, il controllo finale è che l'agente di un job riesca a eseguire `weaver job show`.
