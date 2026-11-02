---
created: 2026-11-02T12:31:02.727Z
updated: 2026-11-02T12:31:02.727Z
summary: Repository, cargo workspace e app Tauri 2 con interfaccia vuota; workflow CI con impianto di test; workflow di rilascio su GitHub Releases con aggiornamenti dall'app.
tasks:
  - azc1e8rx
  - 7e0saact
  - xeiop1ne
---
# Fondamenta: repository, CI e rilascio

Base su cui poggia tutto il resto di Weaver.

Lo scheletro del progetto crea il cargo workspace, l'app Tauri 2 e un'interfaccia vuota ma funzionante, con le scelte di linguaggi, framework e dati delle Specs.

Il workflow `ci` su GitHub Actions esegue l'impianto di test: cargo test per core, CLI e backend, Vitest con Testing Library per l'interfaccia, tauri-driver con WebdriverIO per gli end-to-end su Linux e Windows.

Il workflow `release` parte solo dopo un `ci` riuscito su `main` e pubblica su GitHub Releases le build, per ora non firmate, con gli aggiornamenti dall'app.
