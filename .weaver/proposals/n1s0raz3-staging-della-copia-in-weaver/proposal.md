---
status: rejected
kind: change
base:
  requirements/data/project-files.md: 357a1c0e3010b6aa6a5c6b6a9e4ff9276ed05edf3e3db1a6e31fe978662f05ab
  requirements/tech/files.md: 3fcfaa92e8659b7059c3075a01b536976bc2531896ecef91b15279947db2ff49
created: 2026-11-04T06:02:04.063Z
label: Staging copia
job: pehb2wx1
---
# Staging della copia in .weaver

Il codice fa lo staging di una copia da fuori in `.weaver/short-numbers/`. La Proposal lo ammette nei Requirement: la cartella locale ospita, oltre alla cache degli short number, lo staging temporaneo di una copia, e la copia lo descrive.

## Already present

- `requirements/tech#file-view` — `crates/core/src/file_view/copy_in.rs` (`make_staging`, `copy_and_place`, ramo di rimozione)
- `requirements/data#General` — `crates/core/src/file_view/copy_in.rs` (`make_staging`, cartella locale `short-numbers/`)
