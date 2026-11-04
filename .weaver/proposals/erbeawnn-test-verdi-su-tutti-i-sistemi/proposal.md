---
status: approved
kind: change
base:
  requirements/release/pipeline.md: 1d055b5b1ff8edfe6fed4d5e81f813f6efe2e9200183f03f27284318f69538ab
created: 2026-11-04T10:58:08.751Z
label: Test multipiattaforma
---
# Test verdi su tutti i sistemi

La run `ci` del commit `7146285` su `dev` era verde solo perché i job di Windows e Linux hanno `continue-on-error`: i test Rust falliscono su Linux (1) e su Windows (6), e su Windows i test di `weaver-core` non sono nemmeno partiti, perché `cargo test --workspace` si ferma al primo crate che fallisce.

Decisioni:

- Su `dev` e `main` tutti e tre i sistemi tornano obbligatori, come dice già `requirements/release#General`: niente più `continue-on-error`. Un fallimento su Windows o Linux blocca quindi anche la release di macOS, che parte solo da una `ci` riuscita su `main`.
- Il rename di una cartella osservata su Windows si corregge nel codice di Files, non saltando il test.
- `cargo test` gira con `--no-fail-fast`, così un crate che fallisce non nasconde i fallimenti degli altri.

Assunzioni da verificare:

- Il timeout dei job `build-test` passa da 25 a 40 minuti su tutti e tre i sistemi: oggi macOS ha impiegato 23 minuti e Linux, in una run precedente, 24. Con 25 minuti un job lento farebbe fallire tutta la run. Gli altri job restano a 15 minuti.
- Restano invariati: la release solo di macOS (`scripts/release.mjs` `RELEASED`) e i test end-to-end solo su Linux e Windows.
