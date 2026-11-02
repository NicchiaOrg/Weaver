---
column: done
technical: false
refs:
  - task
  - requirements/data#task
  - requirements/cli#task
  - requirements/rules#task
  - requirements/ui#task
drift: null
proposal: qmqh1o3p
agent: false
created: 2026-11-02T07:29:32.875Z
finished: 2026-11-02T09:26:14.455Z
---
# Task

Weaver gestisce la Machine `task`: un'unità di lavoro per l'agente con titolo, corpo in parole di dominio e i riferimenti che riguarda (un Task tecnico non nomina elementi del Modello, gli altri almeno uno). Nasce con l'approvazione della sua Proposal, dall'agente durante uno scan o un Run, o come fix Task di un Drift. Dipende da "Scheletro del progetto" e "Core dei file del progetto"; i Run sono in "Lavori dell'agente", la regola «concerns the same thing» in "Board".

## Stati
- `To do`: `agent.TAKE_TASK` → `In progress` solo per l'agente di un job attivo e solo se il Task non è bloccato, altrimenti rifiutato nominando il Drift (numero e titolo) o il fix Task; il job lo tiene (`holder`). `developer.DELETE_TASK` → `Deleted` senza traccia, rifiutato se un Drift aperto lo nomina come origine, se è il fix Task di un Drift in `Code to be fixed`, o se il job che l'ha creato lavora ancora.
- `In progress`: lo tiene un solo job. `agent.FINISH_TASK` → `Done` e `agent.RELEASE_TASK` → `To do` solo dal job che lo tiene; `agent.CLOSE_TASK_UNCHANGED` → `Done` solo per un fix Task, con nota non vuota (`unchanged`). `job.ENDED` → `To do`: il job che lo teneva è finito senza finirlo, qualunque sia il motivo. `drift.CODE_TO_BE_FIXED` → `To do` per il Task origine di quel Drift, che aspetta il fix Task. All'apertura del progetto un `holder` sconosciuto al registro dei job, o `doing` senza `holder`, conta come job finito.
- `Done`: lascia subito la Board ma resta disponibile (scelta del developer) finché una voce di History lo cita (`history-entry.TASKS_CITED` → `Recorded in History`, vedi "History") oppure finché nel progetto non finiscono altri 20 Task (`FINISHED` → `Forgotten`), salvo che un Drift aperto lo nomini come origine. Il file e `task-images/<id>/` sono cancellati in entrambi i casi.
- `Deleted`, `Recorded in History`, `Forgotten` sono finali. Eventi emessi: `FINISHED` (finito dall'agente) e `CLOSED_UNCHANGED`.

## Regole (`requirements/rules#task`)
- Bloccato, calcolato a ogni lettura e mai salvato: un Drift aperto lo nomina come origine o riguarda la stessa cosa, oppure è origine di un Drift in `Code to be fixed` il cui fix Task non è Done. Un Task Done non è mai bloccato; il blocco non sposta mai un Task.
- Eseguibile: To do e non bloccato; il prossimo è il più vecchio.
- Slot (esecuzione parallela, sempre attiva): con `runnable` e `max` N si scorrono i Task eseguibili dal più vecchio, prendendo ciascuno che non riguarda la stessa cosa di uno già preso o In progress.
- Gruppi: Task in To do legati transitivamente dalla stessa cosa o dallo stesso percorso di file tra backtick; con `groups` ogni Task preso porta fino ad altri 2 del gruppo, al massimo 3 per sotto-agente.
- Oblio: a ogni `FINISHED` ogni Task Done non citato conta i Task finiti dopo di lui (per `finished`, pari per identità), su file e copie in History, esclusi i chiusi senza modifiche; a 20 viene cancellato.
- Run all raggruppa per modello effettivo (modello scelto della Proposal di design, altrimenti modello di esecuzione, confronto esatto dopo trim): un Run per gruppo, avviati insieme, che contano come un solo Run; il limite di sotto-agenti diviso in proporzione, almeno 1 ciascuno, il resto alle parti frazionarie maggiori.

## File e comandi
- `.trama/tasks/<id>-<slug>.md` con `column`, `technical`, `refs`, `proposal`, `drift`, `job`, `holder`, `created`, `finished`, `unchanged` (`requirements/data#task`).
- `trama task new|show|list|move|delete` come in `requirements/cli#task`: `move` è legato al job e solo chi tiene il Task lo sposta fuori da `doing`; `list` con `runnable`, `max`, `waiting`, `heldBack`, `proposals`, `notProposals`.

## Card (`requirements/ui#task`)
- Pallino di stato (`To do`, `In progress`, `Agent working`), id, titolo, badge `technical`, `blocked` e `waiting <id>` con Popover; badge della label; bordo sinistro nel colore della label; bordo conico rotante mentre un job lo tiene (ciano statico con movimento ridotto).
- `Run` con i motivi disabilitati in ordine; nei dettagli `Delete` solo in To do, con conferma `Delete <id> for good?` e motivi disabilitati in ordine. Nessun altro controllo: i Task non si spostano a mano.

## Fatto quando
- Test del core su blocco, presa rifiutata con il bloccante, rilascio alla fine del job, slot, gruppi, divisione di Run all e oblio dopo 20 Task.
- Un Task citato da una voce di History sparisce da `.trama/tasks/` e resta leggibile nella voce.
- Test Vitest della card: badge, popover, motivi disabilitati di `Run` e `Delete`.
