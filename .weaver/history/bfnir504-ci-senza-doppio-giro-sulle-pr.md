---
created: 2026-11-05T08:46:18.820Z
updated: 2026-11-05T08:46:18.820Z
summary: 'Una pull request da dev a main non avvia più una run propria di ci: prende i check dalla run del push su dev sullo stesso commit; le pull request verso dev girano come prima.'
tasks:
- xfqio580
cleared:
  xfqio580:
    title: CI solo dal push per PR verso main
    refs:
    - requirements/release#General
    technical: true
    proposal: 6r7d6ttq
    label: CI una volta per PR
    finished: 2026-11-05T08:45:41.503Z
    body: |-
      Oggi `.github/workflows/ci.yml` gira su `push: [dev, main]` e su `pull_request: [dev, main]`. Una PR da `dev` a `main` fa quindi girare `ci` due volte sullo stesso commit, una per il push su `dev` e una per la PR, e una terza volta dopo il merge, sul push a `main`.

      Quando il Task è fatto:

      - il trigger `pull_request` di `ci.yml` ha solo `branches: [dev]`; `push: [dev, main]` resta com'è, e così `concurrency`, i job e `release.yml`;
      - il README, nella sezione "Tests and CI", dice che `ci` gira su ogni push a `dev` o `main` e su ogni pull request verso `dev`, e che una pull request da `dev` a `main` prende i check dalla run del push su `dev` sullo stesso commit;
      - dopo il push della modifica su `dev`, su una PR da `dev` a `main` i check richiesti dalla protezione di `main` (`versions`, `build-test (macos-arm64)`) compaiono dalla run del push e nessuna run `pull_request` parte per quella PR. La verifica si fa con `gh run list --workflow ci` e `gh pr checks`.
---
# CI senza doppio giro sulle PR

Il trigger `pull_request` del workflow `ci` ora riguarda solo `dev`. Prima una PR da `dev` a `main` faceva girare `ci` due volte sullo stesso commit (push su `dev` e PR). Ora i check richiesti dalla protezione di `main` arrivano dalla run del push su `dev`, e la run del push su `main` testa il commit di merge prima di `release`. Il README, nella sezione sui test e la CI, descrive il nuovo comportamento. La verifica sul remoto con `gh run list --workflow ci` e `gh pr checks` resta da fare dopo il push su `dev`.
