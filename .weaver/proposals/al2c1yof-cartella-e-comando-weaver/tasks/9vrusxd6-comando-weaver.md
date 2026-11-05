---
refs:
  - agent
  - requirements/cli
  - requirements/tech
  - requirements/release
  - specs#languages
  - specs#frameworks
---
# Comando weaver

Il comando che l'agente usa per raggiungere Weaver si chiama `weaver` e non più `trama`. Le skill installate (`trama-setup`, `trama-design`, `trama-import`, `trama-scan`, `trama-compare`, `trama-implement`, `trama-resolve-drifts`, le `trama-req-*` generate) e i sub-agent di omp (`trama-analysis`, `trama-implementer`) mantengono il nome.

L'Actor `agent` raggiunge Weaver solo attraverso il comando `weaver`, che conosce il job a cui appartiene; ciò che riporta lì sono i suoi Event. Solo un agente di un job avviato da Weaver può tenere un Task, annunciare una card o porre un giro di domande; un comando eseguito fuori da un job del genere viene rifiutato per queste azioni.

## Cosa deve essere vero alla fine

- **Crate e binari.** `crates/core` è la libreria `weaver-core` (prima `trama-core`); `crates/cli` è la crate `weaver-cli` (prima `trama-cli`) e produce il binario `weaver-cli`, mai `weaver`, che è il nome dell'eseguibile dell'app (crate `src-tauri`, binario `weaver`): due binari `weaver` nello stesso workspace Cargo o nella stessa cartella del pacchetto non possono convivere. Tutti gli `use trama_core::…`, i `Cargo.toml` del workspace, `Cargo.lock` e i riferimenti alle crate passano ai nuovi nomi.
- **Nome mostrato.** Help, usage, esempi e messaggi del comando lo chiamano sempre `weaver`, qualunque sia il nome del file da cui gira (`weaver-cli` nella build e nel pacchetto). Questo vale anche per `crates/core/assets/skills/trama-design/COMMANDS.md`, generato dal registro dei comandi: va rigenerato, e il test che lo confronta con il registro (`registry.rs:123` circa) deve passare.
- **Sidecar e cartella `bin`.** `weaver-cli` è il sidecar Tauri `externalBin`, incluso nel pacchetto con quel nome accanto all'eseguibile `weaver` dell'app (`src-tauri/tauri.conf.json:31` oggi `binaries/trama`; `src-tauri/build.rs:78-150` costruisce il sidecar). A ogni avvio Weaver lo copia in `<app local data>/bin/` come `weaver` (`weaver.exe` su Windows) quando il contenuto è diverso, e toglie da quella cartella il file `trama` (`trama.exe`) lasciato dalle versioni precedenti. Quella cartella contiene solo `weaver` e va per prima nel PATH di ogni omp che Weaver avvia (`src-tauri/src/sidecar.rs`, il passo di avvio in `src-tauri/src/lib.rs:218-227`, `crates/core/src/paths.rs:96-98`). Nessun alias `trama`.
- **Variabili d'ambiente.** Weaver passa al comando, nei job e in General, `WEAVER_ENDPOINT`, `WEAVER_JOB`, `WEAVER_GENERAL`, `WEAVER_RUN`, `WEAVER_TASK` al posto di `TRAMA_*`, con lo stesso significato; le variabili `WEAVER_*` ereditate dall'ambiente di Weaver vengono tolte prima di avviare omp. Il lato app e il lato comando cambiano insieme: `crates/cli/src/cli.rs:37-43`, `crates/core/src/jobs/launch.rs:105-115`, `crates/core/src/jobs/general.rs:451-454`, `crates/core/src/jobs/record.rs:94-101`, `src-tauri/src/jobs/actor.rs:245`, il commento di `crates/fake-acp/src/main.rs:16` e i test che le usano (`crates/cli/tests/question_round.rs`, `src-tauri/src/general/tests.rs`, `src-tauri/src/jobs/tests.rs`, `src-tauri/src/jobs/testing.rs`). I record dei job scritti da versioni precedenti non vengono migrati. La suite passa uguale con o senza qualunque variabile `WEAVER_*` nell'ambiente.
- **Testi del comando.** Gli help e gli schemi che nominano la cartella o il comando dicono `.weaver` e `weaver`: `crates/cli/src/help.rs:11` (`--root <dir>` usa `<dir>/.weaver`), `crates/cli/src/commands/project.rs:19-26`, `commands/proposal.rs:91,112` (immagini copiate in `.weaver/task-images/<task>/`), `commands/history.rs:20`, `commands/model.rs:68`, il commento di `crates/cli/src/spec.rs:202`.
- **Testi fissi dell'integrazione omp.** La risposta fissa a ogni richiesta di form o di ask di omp in un job è: "In Weaver jobs, ask the developer only with `weaver question new` (bash timeout 0). Do not ask in any other way."
- **Watcher.** I messaggi di errore e di log del watcher dicono `.weaver/` (`src-tauri/src/windows/watch.rs:150,196`, oggi `"cannot watch .trama/: …"`). Il watcher guarda la radice del progetto solo per vedere `.weaver` comparire o sparire e `.weaver/` ricorsivamente finché esiste; una cartella `.trama` che compare, cambia o sparisce viene ignorata.
- **Release.** I workflow CI e release costruiscono e includono `weaver-cli`; lo smoke test usa il binario appena costruito: `weaver-cli --help`, `weaver-cli project new --input '{}'`, `weaver-cli model check` su una cartella temporanea come `--root`, ognuno con exit 0; su macOS i build arm64 e x64 di `weaver-cli` vengono uniti con `lipo`. I test di contratto del CLI (`tests/cli-contract/contract.test.mjs`) eseguono il binario `weaver-cli`.
- **Nome «Trama» nei testi.** Dove «Trama» indica il prodotto diventa «Weaver»: `README.md` (righe 3, 28, 46, 99 circa, compresi «Trama projects», i comandi `trama` e la cartella `.trama`), la `description` di `src-tauri/Cargo.toml` («the desktop app of Trama») e di `crates/cli/Cargo.toml`, i commenti di `crates/core/src/project/layout.rs` e `crates/core/src/agent_files.rs`. Nessun file `.trama` del repository di Weaver stesso, né le sue copie installate in `.agents/skills` e `.omp`, vengono spostati o modificati: lo sviluppatore sposta `.trama` a mano.

## Fuori da questo Task

Altri Task della stessa Proposal fanno: la costante della cartella del progetto e tutti i percorsi `.trama` nel codice, nei test e nell'interfaccia (compresi il riconoscimento di `.weaver`, il fatto che `.trama` non conti e il testo di Get Started); le stringhe delle istruzioni dei job, il preambolo di General e i testi delle skill e degli agent installati, tranne `COMMANDS.md`, che rigenera questo Task.
