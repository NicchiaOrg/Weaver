---
column: doing
technical: true
refs:
- requirements/release#General
proposal: erbeawnn
holder: omsxhg1t
created: 2026-11-04T15:06:44.317Z
---
# CI obbligatoria ovunque

Nel workflow `ci` (`.github/workflows/ci.yml`) i job `build-test` di Windows e Linux hanno `continue-on-error`. Per questo la run del commit `7146285` su `dev` era verde anche con 7 test Rust falliti. Inoltre `cargo test --workspace --locked` si ferma al primo crate che fallisce: su Windows i test di `weaver-core` non sono partiti.

Quando il Task è fatto:

- la matrice di `build-test` non ha più il campo `required` né `continue-on-error`: un fallimento su macOS, Windows o Linux fa fallire la run, su `dev` come su `main`;
- il timeout di `build-test` è di 40 minuti su tutti e tre i sistemi, gli altri job restano a 15. Il commento che spiegava i job non bloccanti è tolto;
- il passo dei test Rust esegue `cargo test --workspace --locked --no-fail-fast`;
- `scripts/release.mjs` e `release.yml` non cambiano: la release resta solo per macOS, ma parte solo da una `ci` riuscita su tutti e tre i sistemi;
- il Task è chiuso solo dopo una run di `ci` su `dev` riuscita su tutti e tre i sistemi, test end-to-end di Linux e Windows compresi. Ogni altro fallimento emerso grazie a `--no-fail-fast` o ai test end-to-end ora bloccanti va corretto nel codice o nel test, secondo la causa, e non saltato.
