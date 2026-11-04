---
status: fixed
refs:
- requirements/rules#file-view
- requirements/quality#file-view
- requirements/tech#file-view
previous: gtit2hiq
proposal: n1s0raz3
task: sxx57v25
option: 1
job: pehb2wx1
created: 2026-11-04T06:01:42.531Z
decided: 2026-11-04T06:09:41.065Z
---
# Staging copia in .weaver

## What happens

La copia di un drop dall'esterno scrive lo staging in `.weaver/short-numbers/`, la cartella locale che i Requirement descrivono come **cache degli short number**, ricostruibile in qualsiasi momento. Dentro il progetto finiscono quindi i file del drop, che non sono cache. Se la pulizia fallisce, parte del drop resta in quella cartella, nascosta ma ancora su disco. Senza git nulla è ignorato, e chi espande quella cartella durante la copia vede lo staging. I test della copia non guardano più dentro `.weaver`, e che l'ignore tenga lo staging fuori dall'albero non è verificato.

## What should happen

I Requirement dicono che le cartelle locali di `.weaver/` hanno ciascuna il proprio scopo (`short-numbers/` è solo la cache degli short number) e che nient'altro vi è ignorato, che una copia non lascia mai parte di un drop nel progetto, che copiare qualcosa dentro `.weaver/` è rifiutato, e descrivono la copia diretta con registrazione delle entry di primo livello create. Nessuno prevede uno staging dentro `.weaver/` né dice come riportarne la pulizia fallita.

## Fix options

### Copia diretta senza staging

Il drop si copia direttamente nella destinazione, come descrivono i Requirement, registrando le entry di primo livello create. Un fallimento a metà rimuove solo ciò che è stato registrato; se la rimozione fallisce l'errore elenca le entry rimaste. Nessuna cartella nuova; la destinazione mostra per un attimo le entry parziali.

### Staging nella cartella temporanea del sistema

Il drop si copia prima nella cartella temporanea del sistema, poi si copia (non si rinomina, i volumi possono differire) nella destinazione, registrando le entry create. Nulla di Weaver resta nel progetto prima del passo finale; costa una seconda copia dei byte e un temp da ripulire.

## Where in the code

`crates/core/src/file_view/copy_in.rs` (`make_staging`, `copy_and_place`, ramo di rimozione e helper `tree()` dei test); `.weaver/requirements/data/project-files.md` (cartelle locali)

## Prepared question

```json
{
  "question": {
    "id": "ubfcee9g",
    "question": "Dove deve stare lo staging di una copia da fuori nell'albero File?",
    "context": "Un drop dall'esterno è tutto o niente, e i Requirement descrivono la copia diretta nella destinazione con registrazione delle entry di primo livello create. Oggi il drop viene prima copiato in una cartella temporanea dentro `.weaver/short-numbers/`, la cartella locale che i Requirement descrivono come cache degli short number, e poi spostato con rename. Questo lascia i file del drop in una cartella che i Requirement definiscono cache; se la pulizia fallisce i resti restano lì, nascosti ma su disco; senza git la cartella non è ignorata; i test della copia non guardano più dentro `.weaver`.\n\nDifferenza già incontrata:\n- `gtit2hiq` — Copia: cartella di staging visibile nell'albero — `fix-code`, opzione 2 «Staging in una cartella nascosta di Weaver».\n- `5mc616cs` — Copia: rimozione fallita lascia resti — `fix-code`, opzione 1 «Copia in cartella di staging».",
    "options": [
      {
        "label": "Copia diretta senza staging",
        "description": "Il drop si copia direttamente nella destinazione e un fallimento rimuove solo le entry registrate.",
        "details": "Segue i Requirement come scritti: nessuna cartella nuova, nessun rename. Costo: la destinazione mostra per un attimo entry parziali e un fallimento a metà può lasciarle finché la rimozione non finisce."
      },
      {
        "label": "Staging nella cartella temporanea del sistema",
        "description": "Il drop si copia nella cartella temporanea del sistema, poi si copia nella destinazione.",
        "details": "Nulla di Weaver resta nel progetto fino al passo finale. Costo: una seconda copia dei byte, e il temp da ripulire; nessun rename atomico perché i volumi possono differire."
      },
      {
        "label": "Staging in .weaver/short-numbers",
        "description": "Lo staging resta nella cartella locale di `.weaver/`, e i Requirement lo dicono.",
        "details": "I Requirement ammettono in `short-numbers/` anche lo staging della copia e i suoi eventuali resti. Costo: la cartella non è più solo cache; senza git lo staging è elencabile durante la copia."
      }
    ],
    "recommended": 0,
    "multi": false
  },
  "decisions": [
    {
      "decision": "fix-code",
      "option": 1
    },
    {
      "decision": "fix-code",
      "option": 2
    },
    {
      "decision": "proposal"
    }
  ],
  "fingerprint": "sha256:d60629621de7b947c5801cb9ac9da63bef0d3041c8b622ce3b0d7f4d2e1dea42",
  "prepared": "2026-11-04T06:02:28.033Z"
}
```
