---
status: rejected
kind: change
base:
  requirements/quality/files.md: 601c3fe115ce46e3cec8f20f6e276e2812a947d294259b878595e3123cacd882
  requirements/rules/files.md: 2b67a619990a23d288a773774a4b52884c01d8d27022b8c56281eaf59b08b907
created: 2026-11-03T23:37:15.861Z
label: File drop cleanup
job: zxy6nbwr
---
# Resti dopo copia fallita

Ammette nei Requirement il caso in cui, dopo un fallimento a metà di un drop dall'esterno, anche la rimozione di quanto già copiato fallisca: l'app non può garantirlo. Il drop resta tutto o niente in ogni altro caso; in questo caso il messaggio di errore elenca le entry rimaste nel progetto con il motivo, invece di dire che nulla è stato copiato.

## Already present

- `requirements/quality#file-view` — `crates/core/src/file_view/copy_in.rs`, `CopyInError::Failed` con `not_removed`
- `requirements/rules#file-view` — `crates/core/src/file_view/copy_in.rs`, ramo di rimozione dopo il fallimento
