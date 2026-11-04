---
status: approved
kind: change
base:
  model/actors/developer.md: 6c019044236e9fb62b006d6fb13d08210ae08f5d3feaf35a4f67fbf7dca3c59b
  model/agent-work/job.md: cac6cd046252560ba5a667f1fba6aa9ed0f6a03343dca8a95dd691b22b6e857e
  model/model-changes/proposal.md: 330efab72c28c3cdb89a8a58676716370666361fedfa48e1da0b897a6eec3d5e
  model/work/drift.md: f246144e54eaec753ed510648c1d7ea92f2a42f80cea6b0bbd984c6741ac8f94
  requirements/cli/commands.md: c25c318c9fb81896d8c89356095a8e11ec753c3e250ddb203b1f985b1ada09e2
  requirements/rules/cards.md: 399277dcc3fdb8ec236202521d223886f7340f5904fdfb84a3ccfce4c8854df6
  requirements/rules/instructions.md: 3944db73460049cc7f3247a67227527014610d80b9fa0dde9b3e12d664321b77
  requirements/ui/board-cards.md: new
  requirements/ui/board.md: 14d9e0f7a396b2018e431914ddafb9a59364f6366cf6909f2a14dfc437b86005
created: 2026-11-04T07:55:24.852Z
request: 2026-11-04T08-44-09
label: Refine ovunque
job: 3iyxr8ey
---
# Refine su ogni Proposal stale

# Refine su ogni Proposal stale

Una Proposal stale si può sempre raffinare, di qualunque kind e anche allegata a un Drift, anche quando la blocca anche altro: il refinement la adatta al Model e ai Requirements attuali invece di lasciare solo `Reject` o `Rework with feedback…`.

## Decisioni

- `Refine` sostituisce `Approve` su ogni Proposal Open stale, di qualunque kind (`change`, `setup`, `import`, `resolution`), allegata o no, qualunque altro motivo la blocchi; resta abilitato durante una Run.
- Mentre l'agente sta ancora finendo, compare un `Refine` disabilitato con il solo motivo `The agent is still finishing`.
- La skill del refinement segue il kind come nel rework: `change` → `weaver-design`; `setup` → `weaver-setup` con Model vuoto, altrimenti `weaver-design`; `import` → `weaver-import`; `resolution` → `weaver-resolve-drifts` sui soli Drift ancora aperti, nominati nell'istruzione.
- Il refinement di `setup`, `import` e `resolution` segue le regole del refinement di una `change`: niente codice, niente annunci, decisioni del corpo già prese, al massimo un giro di domande solo su decisioni contraddette, `Remove`/`Keep` se la modifica è già tutta nel Model. In una `resolution` le decisioni già prese per i Drift ancora aperti restano.
- Alla consegna gli errori del Model, lo scope non coperto, i riferimenti di Task inesistenti, la label mancante e i fix non applicabili continuano a rifiutare la consegna: l'agente li corregge. La Proposal torna Open solo se la blocca ancora una Proposal richiesta non approvata.
- Nei dettagli del Drift `Refine` sostituisce `Accept Proposal` quando la Proposal allegata è stale, con le stesse regole.
- Alla consegna del refinement una Proposal allegata torna sempre Open, mai Approved: il developer la accetta di nuovo dal Drift.
- Una Proposal richiesta rifiutata, scartata o mancante, o un ciclo di `requires`: il refinement toglie quel `requires` e adatta la Proposal senza; se serviva qualcosa della Proposal tolta, l'agente lo chiede al developer.
- Weaver toglie quei `requires` quando il refinement riparte dai file attuali e li elenca all'agente.
- In un ciclo la Proposal raffinata toglie il proprio `requires` verso il ciclo.
- Una Proposal richiesta rifiutata con rework non ancora consegnato resta: conta come una ancora in scrittura.
- Una `resolution` senza file non è mai stale e non mostra `Refine`, anche con un fix non più applicabile.
- Una `setup` raffinata su un Model non vuoto resta `setup`: approvarla avvia ancora la Run di tutte le Tasks.
- `Resolve` continua a riscrivere la Proposal allegata stale, accanto a `Refine`.
- Tooltip: `Refine on the current Model, then approve` / `Continue refining, then approve` quando può finire approvata; `Refine on the current Model` / `Continue refining` per una Proposal allegata o bloccata da una Proposal richiesta non ancora approvata.
- Una `resolution` stale senza Drift ancora aperti mostra `Refine` disabilitato con `None of its Drifts is still open`, anche da Draft interrotta.
- Weaver toglie da una `resolution` le righe dei Drift non più aperti quando il refinement riparte dai file attuali.
- Il refinement interrotto di una Proposal allegata si continua o si scarta nei dettagli del Drift: `Refine` (`Continue refining`) e `Discard`.
- Mentre la Proposal allegata è in refinement, `Accept Proposal`, `Fix the code`, `Dismiss Drift` e `Resolve` sono disabilitati con `Being refined`.
- Una `resolution` in Refining conta per i suoi Drift come in scrittura; con refinement interrotto come in attesa di decisione.

## Scelte dell'agente

- Il developer chiede il refinement di una Proposal allegata con un nuovo Event `developer.REFINE_DRIFT_PROPOSAL`, che lascia il Drift Open.
- La frase dell'istruzione per una `resolution` termina con ``, on the Drifts `<d1>`, `<d2>` ``.
