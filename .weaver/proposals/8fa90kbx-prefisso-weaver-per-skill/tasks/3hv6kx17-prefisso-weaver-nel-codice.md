---
technical: true
refs:
  - requirements/rules#project
  - requirements/rules#job
  - requirements/rules#ask-conversation
  - requirements/tech#job
  - requirements/tech#project
  - requirements/tech#ask-conversation
  - requirements/data#General
  - requirements/docs#General
---
# Prefisso weaver nel codice

Le skill, i sub-agent e le skill dei Requirement che Weaver installa o genera passano dal prefisso `trama` al prefisso `weaver`. A Task finito valgono tutte queste condizioni.

## Nomi nuovi

- Skill installate in `.agents/skills/`: `weaver-setup`, `weaver-design` (con `FORMAT.md` e `COMMANDS.md`), `weaver-import`, `weaver-scan`, `weaver-compare`, `weaver-implement`, `weaver-resolve-drifts`, al posto di `trama-setup`, `trama-design`, `trama-import`, `trama-scan`, `trama-compare`, `trama-implement`, `trama-resolve-drifts`. Il `name:` di ogni `SKILL.md` è il nome nuovo.
- Sub-agent omp: `.omp/agents/weaver-analysis.md` e `.omp/agents/weaver-implementer.md` al posto di `trama-analysis.md` e `trama-implementer.md`, con `name: weaver-analysis` e `name: weaver-implementer` (il `name:` è il nome del file senza `.md`). Nell'overlay delle Run, `task.agentAdvisor` abilita `weaver-implementer`.
- Skill dei Requirement generate: `weaver-req-<area>-<fragment>` (fragment mode) e `weaver-req-<area>` (index mode). Il budget di 4.000 caratteri conta il nome nuovo, un carattere più lungo per fragment. La rimozione delle skill che il modo corrente non produce e la lettura del modo precedente (per riportare un cambio di modo) guardano solo le cartelle `weaver-req-*`.
- Istruzioni dei lavori, prima frase: `Use the weaver-setup skill …`, `Use the weaver-design skill …` (Design e Refinement), `Use the weaver-import skill …`, `Use the weaver-scan skill …`, `Use the weaver-implement skill …` (Run di un Task e Run all), `Use the weaver-resolve-drifts skill …`; nel Rework la skill segue il tipo: `weaver-design`, `weaver-setup` (setup con Model vuoto), `weaver-import`, `weaver-resolve-drifts`. Il resto delle frasi non cambia.
- Istruzione dell'Ask more sui Requirements: dice che i Requirements stanno nelle skill dei Requirement `weaver-req-*` installate nel progetto; il resto del testo non cambia.
- Ogni riferimento a questi nomi nei testi installati (le skill, compresi i link relativi `../weaver-design/COMMANDS.md` e `../weaver-design/FORMAT.md` e il testo sul budget in `FORMAT.md`, i sub-agent, `WATCHDOG.yml`), nel codice e nei suoi commenti, nei test (Rust, contratto della riga di comando, interfaccia) e nel `README.md` segue i nomi nuovi. `COMMANDS.md` si rigenera nel percorso nuovo e il test che lo confronta con il registro dei comandi passa.
- `--slow` resta passato solo per i lavori di tipo Scan con il modello di scansione impostato: decide il tipo di lavoro, non il testo dell'istruzione; il commento che spiega il motivo nomina `weaver-analysis`.

## Niente migrazione nel codice

- Weaver non conosce i vecchi nomi: non riconosce, non rinomina, non rimuove e non conta `.agents/skills/trama-*` e `.omp/agents/trama-*` nei progetti già inizializzati, nemmeno all'aggiornamento delle skill; restano finché lo sviluppatore non li toglie. Il test della rimozione delle skill dei Requirement verifica che una skill installata (`weaver-design`), una skill dello sviluppatore e una vecchia cartella `trama-req-*` sopravvivano.
- Restano come sono: la cartella `.trama` che Weaver ignora, l'eseguibile `trama` / `trama.exe` che Weaver cancella da `bin/`, il test che vieta la parola "Trama" nei testi installati e la frase vietata `trama-merge`.
- I file installati di questo repository (`.agents/skills/`, `.omp/`) non si toccano in questo Task: li migra il Task "Migrazione del repository".

## Documentazione

- Una riga sotto `## [Unreleased]` → `### Changed` del `CHANGELOG.md`: le skill e i sub-agent che Weaver installa si chiamano `weaver-…`, e i vecchi `trama-*` di Weaver nei progetti già inizializzati si possono eliminare a mano. Nessuna sezione nuova nel `README.md`.

## Dove nel codice

- Sorgenti installati: `crates/core/assets/skills/trama-*/` e `crates/core/assets/omp/agents/trama-*.md` (anche i `name:` e i riferimenti interni), `crates/core/assets/omp/WATCHDOG.yml:23`, `crates/core/assets/skills/trama-design/FORMAT.md:129`; i percorsi installati seguono i nomi delle cartelle (`crates/core/src/agent_files.rs:86-95`).
- `agent_files.rs`: commento del modulo `:13,30`; `SKILL_NAMES` `:51-59`, `COMMAND_REFERENCE_ASSET` `:65`, `COMMAND_REFERENCE_PATH` `:68`; test `:222-225,451,464-469,485-488` (restano `:175` `trama-merge` e `:305` il test sulla parola "Trama").
- Frasi delle istruzioni — `crates/core/src/job_instruction.rs:17-25` (costanti prese da `SKILL_NAMES`), commenti `ReworkKind` `:78-84`; test `:311-373,553-565`.
- Ask more — testo `crates/core/src/jobs/ask.rs:494`, commento `:13`, test `:1031,1093,1099`; commento `src-tauri/src/ask/build.rs:9`; test `src-tauri/src/ask/tests.rs:320`.
- `agentAdvisor` — `crates/core/src/jobs/launch.rs:94`, commento `:85`, test `:318,322`; commento su `--slow` — `crates/core/src/jobs/kinds.rs:68-69`; fixture — `crates/core/src/jobs/subagents.rs:211,226`.
- `SKILL_PREFIX` e budget — `crates/core/src/project_files/requirement_skills.rs:1-9,26,70,75`, test `:346-514`; rimozione per prefisso — `crates/core/src/project_files/plan.rs:17,136,147`, test `:392-433`; modo precedente — `crates/core/src/project_files/sync.rs:151,160,166,202`, test `:324-474`; `crates/core/src/project_files/initialise.rs:40`, test `:172-187,227-231`; `crates/core/src/project_files/mod.rs:5`; `crates/core/src/project/proposal/tests.rs:970,974`; `crates/core/src/project/model.rs:1`; `crates/core/tests/format_md.rs:1,12`.
- Riga di comando — `crates/cli/src/help.rs:234`, `crates/cli/src/registry.rs:35`, `crates/cli/examples/write_command_reference.rs:1`, test `crates/cli/src/commands/project.rs:352,383,392,398` (il file del conflitto resta un file installato da Weaver: `weaver-design/SKILL.md`).
- App — test `src-tauri/src/project_setup.rs:814,1010`, `src-tauri/src/jobs/conversation.rs:925,1197,1203`, `src-tauri/src/jobs/tests.rs:90,147,761,776,1700`.
- Interfaccia e contratto — `ui/src/components/jobs/jobs-view.test.tsx:82,92`, `ui/src/components/project/get-started.test.tsx:18,81,88`, `ui/src/components/project/update-skills.test.tsx:39,75`, `tests/cli-contract/contract.test.mjs:716,724,731`.
- `README.md:103` (percorso di `COMMANDS.md`); `CHANGELOG.md:19` (`### Changed` sotto `## [Unreleased]`).
- Restano come sono: `src-tauri/src/sidecar.rs` (`trama` / `trama.exe`), i test e il watcher sulla cartella `.trama` (`src-tauri/src/project_setup.rs:821-856`, `src-tauri/src/windows/watch.rs:167,395-398`, `tests/cli-contract/contract.test.mjs:248-256,694-701`, `crates/cli/src/exec.rs:349-373`, `crates/cli/src/commands/project.rs:370-372`, `crates/core/src/project_files/initialise.rs:236-268`).
