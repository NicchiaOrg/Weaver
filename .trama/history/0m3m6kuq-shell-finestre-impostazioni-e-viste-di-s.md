---
created: 2026-11-02T12:31:02.985Z
updated: 2026-11-02T12:31:02.985Z
summary: Fondamenta dell'interfaccia e regole di qualità, finestre per progetto con riapertura all'avvio, file e vista delle impostazioni, viste Diagram, Requirements e Specs con Proposal sovrapposta e focus.
tasks:
  - o7f3fscl
  - c34tjjfq
  - 6buq31uh
  - y73la6jb
---
# Shell, finestre, impostazioni e viste di sola lettura

La shell fissa le fondamenta dell'interfaccia: testi in inglese, nessuno stato proprio di progetto, eventi dal backend, tema, icone e regole di qualità valide in tutto Weaver.

La Machine `window` gestisce un'istanza con una finestra per progetto, la finestra di benvenuto con i recenti e la riapertura dei progetti dell'ultima sessione, con ripresa dei job che lavoravano.

Le impostazioni vivono in un solo file con schema tipato e vista Settings, che ospita anche i box di omp e Update skills.

Le viste Diagram (cytoscape con elkjs), Requirements e Specs mostrano in sola lettura Modello, Requisiti e Specs, con Proposal aperta sovrapposta e focus, calcolati dal backend.
