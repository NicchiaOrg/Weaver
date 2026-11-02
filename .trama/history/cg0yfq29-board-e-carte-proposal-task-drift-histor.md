---
created: 2026-11-02T12:31:02.881Z
updated: 2026-11-02T12:31:02.881Z
summary: "Board con Backlog e Active e le Machine delle carte: Proposal, Task, Drift, voci di History e richiesta di design, con regole, comandi CLI e interfaccia di ciascuna."
tasks:
  - 4jlsx639
  - wtf2puh1
  - zov4m67w
  - afvb4s7c
  - bdevj10k
  - qfxwz5qt
---
# Board e carte: Proposal, Task, Drift, History, richiesta di design

La Board della Feature `work` mostra ciò che aspetta una decisione (Proposal e Drift aperti) e i Task ancora da finire, in due colonne: `Backlog` e `Active`. Il lavoro finito la lascia e si racconta in History.

Le carte seguono ciascuna la propria Machine. La Proposal porta le copie dei file cambiati e i Task, con approvazione, rifiuto, rework, staleness e raffinamento. Il Task passa da To do a In progress a Done, con blocco dai Drift. Il Drift è una differenza fra codice e Modello decisa solo dal developer, con opzioni di correzione e domanda preparata. La History raccoglie le voci scritte dall'agente per macro-feature con i Task finiti. La richiesta di design si scrive in un editor Markdown nell'app e viene consumata alla consegna della Proposal.
