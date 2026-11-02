---
created: 2026-11-02T12:31:02.830Z
updated: 2026-11-02T12:31:02.830Z
summary: Machine project (inizializzazione, Modello vuoto, Aree iniziali, file per l'agente) e testi delle skill, dei sotto-agenti e delle istruzioni dei lavori che Weaver installa nel progetto.
tasks:
  - 2wlt7hk8
  - sxe0jtid
---
# Progetto e skill installate

La Machine `project` gestisce il ciclo da `Not initialised` a `Empty Model`: crea `.trama` con un Modello vuoto, le Aree iniziali `ui`, `tech`, `data`, `rules`, `quality` e tutti i file per l'agente, senza mai toccare cartelle genitore e senza sovrascrivere file esistenti con contenuto diverso.

I testi installati comprendono le skill `trama-setup`, `trama-design`, `trama-import`, `trama-scan`, `trama-compare`, `trama-implement`, `trama-resolve-drifts`, le definizioni dei sotto-agenti omp e le istruzioni di una riga di ogni lavoro. Le skill `trama-req-*` sono generate dai Requisiti.
