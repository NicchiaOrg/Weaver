---
technical: false
refs:
  - proposal/Open
  - proposal/Draft
  - requirements/ui#proposal
  - requirements/ui#General
---
# Pulsante Refine giallo

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
