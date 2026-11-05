---
technical: true
refs:
  - requirements/release#General
---
# CI solo dal push per PR verso main

Oggi `.github/workflows/ci.yml` gira su `push: [dev, main]` e su `pull_request: [dev, main]`. Una PR da `dev` a `main` fa quindi girare `ci` due volte sullo stesso commit, una per il push su `dev` e una per la PR, e una terza volta dopo il merge, sul push a `main`.

Quando il Task è fatto:

- il trigger `pull_request` di `ci.yml` ha solo `branches: [dev]`; `push: [dev, main]` resta com'è, e così `concurrency`, i job e `release.yml`;
- il README, nella sezione "Tests and CI", dice che `ci` gira su ogni push a `dev` o `main` e su ogni pull request verso `dev`, e che una pull request da `dev` a `main` prende i check dalla run del push su `dev` sullo stesso commit;
- dopo il push della modifica su `dev`, su una PR da `dev` a `main` i check richiesti dalla protezione di `main` (`versions`, `build-test (macos-arm64)`) compaiono dalla run del push e nessuna run `pull_request` parte per quella PR. La verifica si fa con `gh run list --workflow ci` e `gh pr checks`.
