---
created: 2026-11-02T16:23:14.861Z
updated: 2026-11-02T16:23:14.861Z
summary: Il pulsante Refine è giallo ovunque compaiano i controlli della carta; i dettagli di una Proposal elencano i Task in righe compatte che aprono il testo del Task.
tasks:
- nd8wrsne
- 57407fpi
cleared:
  57407fpi:
    title: Elenchi Task nella Proposal
    refs:
    - proposal
    - requirements/ui#proposal
    technical: false
    proposal: uzd31fpw
    label: Proposal Tasks list
    finished: 2026-11-02T16:11:59.128Z
    body: |-
      Nel details dialog della Proposal, le sezioni `Tasks it creates` e `Code-removal Tasks` mostrano una riga compatta per Task con il solo titolo: niente chip delle refs e niente badge `technical`. Il dialog della voce di History non cambia.

      Ogni riga di `Tasks it creates` è un pulsante. Un click (o la tastiera) apre un Dialog annidato sopra il details dialog, con il titolo del Task e il suo body reso come Markdown, senza refs e senza badge; se il body è vuoto mostra `No description.`, come già fa il body della Proposal. `Esc` o `Close` chiudono solo il Dialog annidato e il focus torna alla riga che l'ha aperto; il details dialog resta aperto. Quando il details dialog si chiude, il Dialog annidato si chiude con lui.

      Perché ciò sia possibile, il body di ogni Task della Proposal deve arrivare alla UI: la vista della Proposal inviata dal backend include, per ogni Task, anche il suo body (e un'identità stabile per distinguere Task con lo stesso titolo).

      Le righe di `Code-removal Tasks` usano lo stesso stile di elenco ma sono testo semplice, non controlli e non cliccabili; la ref nel loro titolo (``Remove code of `<ref>` ``) è resa come codice inline, non con i backtick letterali.

      I test esistenti della scheda Proposal che verificano chip delle refs e badge `technical` nel details dialog vanno aggiornati al nuovo comportamento, con copertura di apertura e chiusura del Dialog annidato, ritorno del focus e testo `No description.`.
  nd8wrsne:
    title: Pulsante Refine giallo
    refs:
    - proposal/Open
    - proposal/Draft
    - requirements/ui#proposal
    - requirements/ui#General
    technical: false
    proposal: hx0c7330
    label: Refine button
    finished: 2026-11-02T16:11:20.701Z
    body: |-
      Quando un clic sul controllo di approvazione di una Proposal avvia un job di affinamento, il controllo si chiama `Refine` invece di `Approve` ed è giallo, così si capisce che parte un'operazione dell'agente. Il comportamento non cambia. Una Proposal Open che è un cambiamento ordinario, non è legata a un Drift, è stale e non ha altri blocchi passa a Refining con `developer.APPROVE_PROPOSAL`: un job di affinamento la adatta al Model e ai Requirements attuali senza una nuova intervista e, se alla fine non resta nessun blocco, la approva. Una Draft lasciata a metà da un affinamento passa anch'essa a Refining con lo stesso evento, e un nuovo job di affinamento la continua.

      Quando il lavoro è finito, deve valere quanto segue:

      - **Proposal stale con la sola staleness come blocco.** Il controllo si chiama `Refine`: testo e nome accessibile sono `Refine`. È abilitato anche durante una Run e ha il tooltip `Refine on the current Model, then approve`.
      - **Draft con affinamento interrotto** (badge `refinement interrupted`). La card ha solo `Refine`, con il tooltip `Continue refining, then approve`, e `Discard`.
      - **Aspetto di `Refine`.** L'icona è `Sparkles` al posto di `Check`. Il pulsante è giallo pieno: sfondo `yellow-500` nel tema chiaro e `yellow-400` nel tema scuro, un tono più scuro al passaggio del mouse, testo `zinc-950`.
      - **Nessuna conferma.** Il clic avvia subito il job, come oggi.
      - **Proposal stale con altri blocchi.** Il controllo resta `Approve`, disabilitato, e il tooltip elenca tutti i motivi, compreso stale. Ogni altro caso resta `Approve` primario (blu) con icona `Check`.
      - **Tutte le card.** `Refine` compare ovunque compaiono i controlli della card: sulla card di un cambiamento, sulla card di una risoluzione e nel piede del dialogo dei dettagli.
      - **Tabella delle icone.** In `requirements/ui#General` la tabella ha la riga `Refine` → `Sparkles`.

      Il backend decide quale caso vale. Oggi però invia al frontend solo il tooltip, e il frontend non deve riconoscere il caso confrontando il testo del tooltip. Il `ProposalControl` / `ProposalView` deve quindi dire esplicitamente se il clic affina, per esempio con un flag. Il binding TypeScript generato con `ts-rs` deve restare allineato.

      Punti già noti nel codice:

      - `src-tauri/src/proposal_view.rs`, `controls()` e le costanti `TOOLTIP_REFINE` / `TOOLTIP_CONTINUE`: qui si sceglie il caso. I test Rust in fondo al file controllano i tooltip.
      - `crates/core/src/project/proposal/analysis.rs`, `Analysis::refinable`: il caso con la staleness come unico blocco.
      - `ui/src/components/board/proposal-card.tsx`, `ProposalControls`: il `GuardedButton` di approvazione (testo fisso `Approve`, icona `Check`, variante default). È condiviso da card, card di risoluzione e dialogo dei dettagli.
      - `ui/src/components/ui/button.tsx`: non c'è una variante gialla. `ui/src/lib/tones.ts` ha il tono `yellow`.
      - `ui/src/components/board/proposal-card.test.tsx`: i test del caso stale e della Draft interrotta cercano il pulsante per nome `Approve` e vanno aggiornati a `Refine`.
---
# Carte Proposal: Refine e Task

`Refine` ha icona `Sparkles` e sfondo giallo, anche nel dialogo dei dettagli. I Task creati dalla Proposal e quelli di rimozione del codice sono righe compatte con solo il titolo; ogni riga dei Task creati apre un dialogo annidato con titolo e corpo.
