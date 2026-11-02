---
created: 2026-11-02T12:31:02.933Z
updated: 2026-11-02T12:31:02.933Z
summary: Job affidati a omp via ACP, chat General libera, round di domande al developer, controllo e installazione di omp e notifiche di sistema quando tocca al developer.
tasks:
  - qxv8fksb
  - cdoi70sr
  - uyj9ftam
  - mlyua7vr
  - jodwoiri
---
# Lavori dell'agente, General, round di domande, controllo di omp e notifiche

Il `job` è un lavoro che Weaver affida a omp (setup, design, import, scan, Run, resolve, rework, refinement) con un processo omp, una sessione ACP e una conversazione; i permessi passano dal developer e i job si sospendono e riprendono.

General è la conversazione libera con omp di un progetto e, su richiesta esplicita, avvia lavori con `trama job new`.

Il round di domande raccoglie le decisioni che l'agente chiede al developer, con opzioni, raccomandazione e risposta libera, ed è usato anche per la domanda preparata di un Drift.

La Machine `omp-setup` verifica che omp sia pronto: lo cerca, lo installa, lo aggiorna o chiede il collegamento a un provider, e nessun lavoro parte finché non lo è.

Le notifiche di sistema avvisano che un job è finito o aspetta il developer, solo quando la finestra del progetto non è in primo piano.
