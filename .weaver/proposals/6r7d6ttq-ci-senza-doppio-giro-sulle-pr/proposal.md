---
status: approved
kind: change
base:
  requirements/release/pipeline.md: 091ca026b97ff8ebb636afd0d4893a9b1e45ba5fa49de18774c8138688cd7f1d
created: 2026-11-05T08:40:58.929Z
label: CI una volta per PR
---
# CI senza doppio giro sulle PR

Una PR da `dev` a `main` fa girare `ci` due volte sullo stesso commit: una per il push su `dev` e una per l'evento `pull_request`. Dopo il merge `ci` gira una terza volta su `main`. GitHub associa i check richiesti al commit in testa alla PR, qualunque evento li abbia prodotti, e la testa di una PR da `dev` è sempre un commit già testato dal push su `dev`.

Decisioni:

- `ci` non gira più per le PR verso `main`: per una PR da `dev` valgono i check della run del push su `dev` sullo stesso commit.
- Restano invariati: `ci` sul push a `dev`, sulle PR verso `dev` e sul push a `main`. La run sul push a `main` resta l'unica che avvia `release`, perché è l'unica che prova il merge commit che viene rilasciato.

Conseguenze accettate:

- Prima del merge non si testa più il risultato del merge di `dev` in `main` (`refs/pull/N/merge`). Finché `main` cambia solo da `dev` coincide con `dev`. Se così non fosse, il problema emerge nella `ci` sul push a `main`, prima della release.
- Una PR verso `main` da un branch diverso da `dev` non riceve check e non si può unire, come chiede la regola che `main` cambia solo da `dev`.
