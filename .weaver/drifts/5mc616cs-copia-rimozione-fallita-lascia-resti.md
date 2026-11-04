---
status: fixed
refs:
- requirements/quality#file-view
- requirements/rules#file-view
- file-view/Open/developer.DROP_EXTERNAL_ENTRIES
proposal: awpz1lmq
task: 7t5j92vb
option: 1
job: zxy6nbwr
created: 2026-11-03T23:36:58.472Z
decided: 2026-11-04T05:46:15.158Z
---
# Copia: rimozione fallita lascia resti

## What happens

Quando una copia da un drop dall'esterno fallisce a metà e anche la rimozione di ciò che era già stato copiato fallisce (ad esempio per permessi), nel progetto resta una parte del drop. Il toast non dice più `Nothing was copied.` ma elenca le entry rimaste con il motivo. Questo ramo non ha test.

## What should happen

La copia è tutto o niente: non lascia mai una parte del drop nel progetto, e un fallimento a metà rimuove tutto ciò che il drop aveva copiato. Il Model e i Requirement non prevedono il caso in cui la rimozione stessa fallisca, quindi non dicono cosa mostrare né come evitare i resti.

## Fix options

### Copia in cartella di staging

Il drop viene copiato in una cartella temporanea dentro il progetto e poi spostato con rename atomici nella destinazione. Un fallimento a metà tocca solo lo staging; resta il rischio che la pulizia dello staging fallisca, ma l'albero visibile non contiene mai parti del drop. Costa più I/O e una cartella temporanea.

### Rimozione con nuovi tentativi e permessi

La rimozione prova a ripristinare i permessi e a ripetere i passi falliti prima di arrendersi. Riduce i casi con resti ma non li elimina; il Model dovrebbe comunque dire cosa succede nel caso residuo.

### Resti dichiarati con test

Il comportamento attuale resta, con test per il ramo e il messaggio con i resti. Richiede di ammettere il caso nel Model e nei Requirement, quindi passa da una Proposal.

## Where in the code

`crates/core/src/file_view/copy_in.rs`: `CopyInError::Failed` (campo `not_removed`) e il ramo di rimozione dopo il fallimento; messaggio nel toast della File view.

## Prepared question

```json
{
  "question": {
    "id": "5mc616cs",
    "question": "Come deve comportarsi la copia di un drop quando anche la rimozione di ciò che aveva già copiato fallisce?",
    "context": "Un drop dall'esterno sull'albero File è tutto o niente: se un passo fallisce a metà, quanto già copiato viene rimosso. Se però anche la rimozione fallisce (ad esempio per permessi), una parte del drop resta nel progetto, mentre i Requirement dicono che non deve mai restare nulla. Oggi l'errore elenca le entry rimaste invece di dire `Nothing was copied.`; questo caso non ha test.",
    "options": [
      {
        "label": "Copia in cartella di staging",
        "description": "Il drop viene copiato in una cartella temporanea e poi spostato con rename atomici nella destinazione: l'albero visibile non contiene mai parti del drop. Costa più I/O e una cartella temporanea."
      },
      {
        "label": "Rimozione con nuovi tentativi",
        "description": "La rimozione ripristina i permessi e ripete i passi falliti prima di arrendersi. Riduce i resti ma non li elimina del tutto."
      },
      {
        "label": "Resti dichiarati con test",
        "description": "Il comportamento resta com'è e un test copre il ramo con il messaggio che elenca le entry rimaste. Comporta ammettere il caso nei Requirement."
      },
      {
        "label": "Ammettere i resti nei Requirement",
        "description": "Una Proposal modifica i Requirement della File view: il drop è tutto o niente tranne quando la rimozione fallisce, e l'errore elenca le entry rimaste."
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
      "decision": "fix-code",
      "option": 3
    },
    {
      "decision": "proposal"
    }
  ],
  "fingerprint": "sha256:407d981700bc35d40f6340b8cc59b8f0141fcc78fbe941a423d23a13b60d2001",
  "prepared": "2026-11-03T23:37:44.565Z"
}
```
