---
status: approved
base:
  requirements/cli/area.md: 33a8520779d8a0933ffd9edfe4284e0bc611f84ff0ec773fd83c05db2bcf51f9
  requirements/cli/commands.md: 946680eb03a0deba08ded31c90946a1a50e32eb0a74d06c32763adbe73d7c2e2
  requirements/data/area.md: 30b4a655e077b3a32f57f7588977b860b82b8baaa360a1810fc7d8018926be37
  requirements/data/project-files.md: e2fbaba392d76fdb0e465d4526a2d1cea3929615f08489f8265bb1452e37b940
  requirements/quality/main.md: 03fa92f1e09b7a62ea94ed7d1f46d75218e715346f5f0a7d425999de54eb7571
  requirements/release/pipeline.md: cd86442ac08376ba51efcd55655b77d1393b466fb108325e0500e5027eeabdeb
  requirements/rules/area.md: ed150fb857efe7dbb43fcc56bcc62574d26b1da25ecba40c1c44e4f1ce730c67
  requirements/rules/cards.md: 978bf75a2ec2f57b1e90327f8c4e46858b8af048b264bccde84cc48d3d3225fe
  requirements/rules/skills.md: b2306ca5894ae4c5c2091f8d054964986b41a56bea1611820b7abc5d5a9442a7
  requirements/tech/architecture.md: a43889530b90b3a66c8081b0c6aa01fdd1b7a58f082e3d107b06006be24dc5bd
  requirements/tech/area.md: b1aa28a64644ad1c24a3eecd99b7c319bbd38b62bcd6dfdf105a0514fa432dba
  requirements/tech/omp.md: 9312a76f597544f7ca17a99a1319fcadec61cdd28e9012219ba103f85170efe0
  requirements/ui/board.md: d5e3527efd8c70644ceb12bad540985902917009fab6875e97630796efa182b4
  requirements/ui/setup.md: 5953b024afe845e6b972857360c5726cc23660d5e0e1d41ec46d21d801b8caf4
  specs.yaml: 634d1c5af89ad430ec1e47e3c1dfae8079ad0c189d4ab653e995f11b44d6bd0e
  system.md: cb9c70283ffe4a6895076a548419ea938662bc6734fef270cfd1c10cfe80b16b
  actors/agent.md: df9d5e74e035eadd89cf0acbc85ef2d99b20fc73dd168b7439ced7cef324f2ac
  actors/workspace.md: 33f076046b512466ae4c5c87eb0e36e40f9c8dbab0ae2ab77c6ee2401decc1ef
  agent-work/job.md: 3f50b66cd79ec1a237b899bfec0e0b8adb618a7c00afe22c9668682b2c67df06
  project-setup/feature.md: 763f53f681404c03f2ded01dd6aa00d29f00a8f97a3c7a8549b047000b38b8ca
  project-setup/project.md: 907c661073a047f7be5b53cf9e5e64cb62feb09788039e9bb54d7a2e5548e032
delete: []
request: 2026-11-02T14-45-32
created: 2026-11-02T13:54:37.086Z
label: .weaver e weaver
---
# Cartella e comando weaver

Weaver usa la cartella `.weaver` al posto di `.trama` e il comando `weaver` al posto di `trama`, così che il prodotto abbia un solo nome in ciò che lo sviluppatore e l'agente vedono.

## Decisioni

- La cartella del progetto è `.weaver`; ogni percorso `.trama/…` nel Model, nei Requirements, nelle Specs, nel codice e nei testi installati diventa `.weaver/…`.
- Il comando diventa `weaver`; le skill installate (`trama-setup`, `trama-design`, …, `trama-req-*`) e i sub-agent di omp (`trama-analysis`, `trama-implementer`) mantengono il nome.
- Una cartella con la sola `.trama` non viene riconosciuta: risulta Not initialised e Get Started crea una `.weaver` nuova; `.trama` non viene mai letta, spostata né rimossa.
- Una cartella con sia `.weaver` sia `.trama` usa `.weaver`, senza avvisi; `.trama` è una cartella qualsiasi.
- Nessun alias: esiste solo `weaver`; all'avvio Weaver toglie dalla cartella `bin` dei suoi dati locali la copia `trama` lasciata dalle versioni precedenti.
- Le variabili d'ambiente diventano `WEAVER_ENDPOINT`, `WEAVER_JOB`, `WEAVER_GENERAL`, `WEAVER_RUN`, `WEAVER_TASK`.
- Le crate Rust diventano `weaver-core` (`crates/core`) e `weaver-cli` (`crates/cli`).
- La crate CLI produce il binario `weaver-cli`, incluso così nel pacchetto accanto all'eseguibile `weaver` dell'app; Weaver lo copia come `weaver` nella cartella `bin`, e help e messaggi chiamano sempre il comando `weaver`.
- Nei testi (skill installate, documentazione, commenti) «Trama» diventa «Weaver» dove indica il prodotto; i nomi `trama-*` delle skill restano.
- La cartella `.trama` del repository di Weaver stesso la sposta lo sviluppatore a mano (`git mv .trama .weaver`) quando passa alla nuova versione; nessun Task lo fa.

## Assunzioni

- I record dei job scritti dalle versioni precedenti, che salvano le variabili `TRAMA_*`, non vengono migrati: coerente con il taglio netto scelto per `.trama`.
