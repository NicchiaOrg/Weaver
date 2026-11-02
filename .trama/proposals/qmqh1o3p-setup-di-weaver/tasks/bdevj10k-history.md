---
technical: false
refs:
  - history-entry
  - requirements/data#history-entry
  - requirements/cli#history-entry
  - requirements/ui#history-entry
---
# History

Weaver tiene la History del progetto secondo la Machine `history-entry`: ciò che un job ha fatto, raccontato dal suo agente per una macro-feature, con titolo, riassunto di una riga, testo e i Task finiti che raggruppa. Dipende da "Scheletro del progetto" e "Core dei file del progetto"; il ciclo dei Task è in "Task".

## Comportamento
- L'agente scrive le voci alla fine di un Run, una per macro-feature, e può scriverne per qualunque altro job che abbia finito Task. Weaver non scrive mai una voce da sé: un Task finito e mai citato resta fuori dalla History e viene dimenticato dopo altri 20 Task finiti.
- Stato unico `Written` (iniziale), datato al momento della scrittura o dell'ultimo aggiornamento. `agent.UPDATE_HISTORY_ENTRY` → `Written`: l'agente riscrive il testo o aggiunge Task finiti, mai ne toglie; la data diventa quella dell'aggiornamento.
- L'Event `TASKS_CITED` è emesso quando la voce è scritta o aggiornata, per i Task citati la prima volta: ognuno passa a `Recorded in History`. Prima di cancellare il file del Task, Weaver scrive la sua copia (titolo, corpo, riferimenti, Proposal, label, `finished`) in ogni voce che lo cita; se la copia fallisce il Task resta e la Board mostra un avviso.

## File (`requirements/data#history-entry`)
- `.trama/history/<id>-<slug>.md`, versionato; front matter `created`, `updated` (la data della voce), `summary` (una riga di 1–280 caratteri, solo Markdown inline), `tasks` (identità citate, senza duplicati, mai in diminuzione), `cleared` (mappa identità → copia `{title, refs, technical, proposal?, label?, finished, body}` scritta una volta e mai cambiata; scriverla non cambia `updated`). Corpo: un H1 con il titolo, poi il testo.
- Le identità dei Task copiati restano uniche anche rispetto alle nuove.

## Comandi (`requirements/cli#history-entry`)
- `trama history new {title, summary, body?, tasks}`: titolo su una riga, riassunto di 1–280 caratteri, almeno un Task, ciascuno Done e non chiuso senza modifiche; ogni problema elencato, nulla scritto.
- `trama history edit <id> {title?, summary?, body?, tasks?}` sostituisce campi e aggiunge Task, mai ne toglie.
- Entrambi stampano `id`, `title`, `file`, `recorded` (Task ora registrati). `trama history list {}` dal più recente; `trama history show <id>` include i Task citati.

## Vista History (`requirements/ui#history-entry`)
- Voci dalla più recente per data, raggruppate per giorno locale: `Today · <n>`, `Yesterday · <n>`, poi la data lunga (`Monday, 2 November 2026 · <n>`).
- Riga: titolo, riassunto su una riga (tagliato con `…`, testo intero come tooltip), ora `HH:MM` in Today e Yesterday, numero di Task.
- Filtro periodo (Select): `Today`, `Last 7 days` (predefinito a ogni apertura della finestra), `Last 30 days`, `All`; ogni periodo va dalla mezzanotte locale del primo giorno a ora; una voce datata nel futuro compare solo in `All`. Vuoto: `Nothing done in this period`.
- Clic su una voce: Dialog con titolo, data, riassunto, testo renderizzato e i Task dalle copie nell'ordine di citazione (id, titolo, badge della label, ora di fine); una riga si espande con corpo e riferimenti, in sola lettura.

## Fatto quando
- Test del core: `TASKS_CITED` solo per i Task citati la prima volta, copia scritta prima della cancellazione del file, `tasks` mai in diminuzione, copia fallita che lascia il Task.
- Test CLI: `history new` rifiuta Task non Done o chiusi senza modifiche elencando ogni problema.
- Test Vitest della vista: raggruppamento per giorno, filtro e confini dei periodi, dialogo con i Task copiati.
