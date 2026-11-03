---
technical: true
refs:
  - requirements/rules#project
  - requirements/rules#job
  - requirements/tech#job
  - requirements/tech#project
  - requirements/data#General
  - requirements/docs#General
---
# Prefisso weaver nel codice

Le skill, i sub-agent e le skill dei Requirement che Weaver installa o genera passano dal prefisso `trama` al prefisso `weaver`. A Task finito valgono tutte queste condizioni.

## Nomi nuovi

- Skill installate in `.agents/skills/`: `weaver-setup`, `weaver-design` (con `FORMAT.md` e `COMMANDS.md`), `weaver-import`, `weaver-scan`, `weaver-compare`, `weaver-implement`, `weaver-resolve-drifts`, al posto di `trama-setup`, `trama-design`, `trama-import`, `trama-scan`, `trama-compare`, `trama-implement`, `trama-resolve-drifts`. Il `name:` di ogni `SKILL.md` è il nome nuovo.
- Sub-agent omp: `.omp/agents/weaver-analysis.md` e `.omp/agents/weaver-implementer.md` al posto di `trama-analysis.md` e `trama-implementer.md`, con `name: weaver-analysis` e `name: weaver-implementer`. Nell'overlay delle Run, `task.agentAdvisor` abilita `weaver-implementer`.
- Skill dei Requirement generate: `weaver-req-<area>-<fragment>` (fragment mode) e `weaver-req-<area>` (index mode). Il budget di 4.000 caratteri conta il nome nuovo, un carattere più lungo per fragment. La rimozione delle skill che il modo corrente non produce e la lettura del modo precedente (per riportare un cambio di modo) guardano solo le cartelle `weaver-req-*`.
- Istruzioni dei lavori, prima frase: `Use the weaver-setup skill …`, `Use the weaver-design skill …` (Design e Refinement), `Use the weaver-import skill …`, `Use the weaver-scan skill …`, `Use the weaver-implement skill …` (Run di un Task e Run all), `Use the weaver-resolve-drifts skill …`; nel Rework la skill segue il tipo: `weaver-design`, `weaver-setup` (setup con Model vuoto), `weaver-import`, `weaver-resolve-drifts`. Il resto delle frasi non cambia.
- Ogni riferimento a questi nomi nei testi installati (le skill, compresi i link relativi `../weaver-design/COMMANDS.md` e `../weaver-design/FORMAT.md` e il testo sul budget in `FORMAT.md`, i sub-agent, `WATCHDOG.yml`), nel codice e nei suoi commenti, nei test (Rust, contratto della riga di comando, interfaccia) e nel `README.md` segue i nomi nuovi. `COMMANDS.md` si rigenera nel percorso nuovo e il test che lo confronta con il registro dei comandi passa.
- `--slow` resta passato solo per i lavori di tipo Scan con il modello di scansione impostato; il commento che spiega il motivo nomina `weaver-analysis`.

## Niente migrazione nel codice

- Weaver non conosce i vecchi nomi: non riconosce, non rinomina, non rimuove e non conta `.agents/skills/trama-*` e `.omp/agents/trama-*` nei progetti già inizializzati, nemmeno all'aggiornamento delle skill; restano finché lo sviluppatore non li toglie. Il test della rimozione delle skill dei Requirement verifica che una skill installata (`weaver-design`), una skill dello sviluppatore e una vecchia cartella `trama-req-*` sopravvivano.
- Restano come sono: la cartella `.trama` che Weaver ignora, l'eseguibile `trama` / `trama.exe` che Weaver cancella da `bin/`, il test che vieta la parola "Trama" nei testi installati e la frase vietata `trama-merge`.
- I file installati di questo repository (`.agents/skills/`, `.omp/`) non si toccano in questo Task: li migra il Task "Migrazione del repository".

## Documentazione

- Una riga sotto `## [Unreleased]` → `### Changed` del `CHANGELOG.md`: le skill e i sub-agent che Weaver installa si chiamano `weaver-…`, e i vecchi `trama-*` di Weaver nei progetti già inizializzati si possono eliminare a mano. Nessuna sezione nuova nel `README.md`.

## Dove nel codice

- Sorgenti installati: `crates/core/assets/skills/trama-*/` e `crates/core/assets/omp/agents/trama-*.md`, `crates/core/assets/omp/WATCHDOG.yml:23`; i percorsi installati seguono i nomi delle cartelle (`crates/core/src/agent_files.rs:86-95`).
- `SKILL_NAMES`, `COMMAND_REFERENCE_ASSET`, `COMMAND_REFERENCE_PATH` — `crates/core/src/agent_files.rs:50-68`; test con nomi scritti a mano — `agent_files.rs:222-225,451,464-469,485-488`.
- Frasi delle istruzioni — `crates/core/src/job_instruction.rs:17-25,77-86,194-274`; test — `job_instruction.rs:307-374,552-566`.
- `agentAdvisor` scritto a mano — `crates/core/src/jobs/launch.rs:84-97`, test `:316-323`; commento su `--slow` — `crates/core/src/jobs/kinds.rs:68-72`; fixture — `crates/core/src/jobs/subagents.rs:211,226`.
- `SKILL_PREFIX` e budget — `crates/core/src/project_files/requirement_skills.rs:1-10,25-26,70-106`, test `:344-515`; rimozione per prefisso — `crates/core/src/project_files/plan.rs:136-171`, test `:385-435`; modo precedente — `crates/core/src/project_files/sync.rs:151-185,199-301`, test `:320-476`; `crates/core/src/project_files/initialise.rs:40,172-187,226-233`; `crates/core/src/project_files/mod.rs:5`; `crates/core/src/project/proposal/tests.rs:969-974`; `crates/core/tests/format_md.rs:1-15`.
- Riga di comando — `crates/cli/src/help.rs:234`, `crates/cli/src/registry.rs:35`, `crates/cli/examples/write_command_reference.rs:1`, test `crates/cli/src/commands/project.rs:352,383,392,398` (il file del conflitto resta un file installato da Weaver: `weaver-design/SKILL.md`).
- App — test `src-tauri/src/project_setup.rs:814,1010`, `src-tauri/src/jobs/conversation.rs:925,1197,1203`, `src-tauri/src/jobs/tests.rs:90,147,761,776,1700`.
- Interfaccia e contratto — `ui/src/components/jobs/jobs-view.test.tsx:82,92`, `ui/src/components/project/get-started.test.tsx:18,81,88`, `ui/src/components/project/update-skills.test.tsx:39,75`, `tests/cli-contract/contract.test.mjs:716,724,731`.
- `README.md:103` (percorso di `COMMANDS.md`); `CHANGELOG.md:7` (`## [Unreleased]`).
