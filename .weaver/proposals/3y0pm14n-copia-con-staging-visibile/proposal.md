---
status: discarded
kind: change
base:
  requirements/rules/files.md: 2b67a619990a23d288a773774a4b52884c01d8d27022b8c56281eaf59b08b907
  requirements/tech/files.md: 3fcfaa92e8659b7059c3075a01b536976bc2531896ecef91b15279947db2ff49
created: 2026-11-04T05:51:40.715Z
label: copy-in staging
job: tdhtmllx
---
# Copia con staging visibile

Descrive la copia di un drop dall'esterno come oggi la fa il codice: scrive in una cartella di staging temporanea nella cartella di destinazione, sposta le entry con rename e, se la rimozione fallisce, elenca nell'errore ciò che resta.
