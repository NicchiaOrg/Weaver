---
created: 2026-11-04T23:43:58.186Z
updated: 2026-11-04T23:43:58.186Z
summary: 'Un errore su macOS, Windows o Linux fa fallire la ci: tutti i crate Rust vengono eseguiti, i test end-to-end di Linux e Windows bloccano, e i problemi emersi sono corretti.'
tasks:
- 2zhiuv0e
cleared:
  2zhiuv0e:
    title: CI obbligatoria ovunque
    refs:
    - requirements/release#General
    technical: true
    proposal: erbeawnn
    label: Test multipiattaforma
    finished: 2026-11-04T23:43:47.011Z
    body: |-
      Nel workflow `ci` (`.github/workflows/ci.yml`) i job `build-test` di Windows e Linux hanno `continue-on-error`. Per questo la run del commit `7146285` su `dev` era verde anche con 7 test Rust falliti. Inoltre `cargo test --workspace --locked` si ferma al primo crate che fallisce: su Windows i test di `weaver-core` non sono partiti.

      Quando il Task è fatto:

      - la matrice di `build-test` non ha più il campo `required` né `continue-on-error`: un fallimento su macOS, Windows o Linux fa fallire la run, su `dev` come su `main`;
      - il timeout di `build-test` è di 40 minuti su tutti e tre i sistemi, gli altri job restano a 15. Il commento che spiegava i job non bloccanti è tolto;
      - il passo dei test Rust esegue `cargo test --workspace --locked --no-fail-fast`;
      - `scripts/release.mjs` e `release.yml` non cambiano: la release resta solo per macOS, ma parte solo da una `ci` riuscita su tutti e tre i sistemi;
      - il Task è chiuso solo dopo una run di `ci` su `dev` riuscita su tutti e tre i sistemi, test end-to-end di Linux e Windows compresi. Ogni altro fallimento emerso grazie a `--no-fail-fast` o ai test end-to-end ora bloccanti va corretto nel codice o nel test, secondo la causa, e non saltato.
---
# CI obbligatoria su ogni sistema

Il workflow `ci` non ha più job non bloccanti: la matrice di `build-test` non ha il campo `required` né `continue-on-error`, il timeout è di 40 minuti su tutti e tre i sistemi e i test Rust usano `--no-fail-fast`, così un crate che fallisce non nasconde gli altri. La release resta solo per macOS ma parte solo da una `ci` riuscita ovunque.

Con i controlli bloccanti sono emersi e sono stati corretti: quattro test di `weaver-core` che su Windows assumevano percorsi e separatori Unix; un arresto del server locale che su Windows poteva non terminare mai (la connessione di risveglio veniva chiusa prima dell'accettazione); la chiusura dell'ultima finestra nel test end-to-end di Linux, ora fatta dal percorso di chiusura dell'app con il permesso `core:window:allow-close`, perché WebKitWebDriver chiude la sessione quando non resta alcuna finestra; le cartelle `AppData` nel profilo di prova dei test del contratto CLI su Windows; il rilevamento della versione di Edge; un test Vitest sull'albero dei file che non attendeva le righe; l'attesa del titolo nel test end-to-end. Su Windows i test end-to-end girano a integrità media con `gsudo`, perché i runner sono elevati e il runtime WebView2 dalla versione 150 ignora in quel caso la variabile con cui `msedgedriver` apre la porta di debug.
