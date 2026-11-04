---
status: fixed
refs:
- requirements/tech#file-view
- requirements/quality#file-view
- requirements/rules#file-view
origin: 7t5j92vb
previous: 5mc616cs
proposal: 3y0pm14n
task: v1v89jif
option: 2
job: tdhtmllx
created: 2026-11-04T05:49:47.659Z
decided: 2026-11-04T05:55:35.508Z
---
# Copia: cartella di staging visibile nell'albero

## What happens

La copia di un drop dall'esterno ora scrive prima in una cartella temporanea (`.copy-in.<random>.tmp`) dentro la cartella di destinazione e poi sposta le entry con rename. Il watcher la ignora, ma l'albero la elenca: il listing mostra ogni nome che inizia con un punto, quindi finché dura la copia chi espande la cartella vede la cartella di staging. Se la pulizia dello staging (o delle entry già spostate) fallisce, la cartella con parte del drop resta nel progetto e l'errore la elenca invece di dire `Nothing was copied.`. Il passo di rinomina in blocco non è nemmeno quello descritto per la copia.

## What should happen

I Requirement dicono che una copia non lascia mai parte di un drop nel progetto, che i nomi con il punto sono tutti elencati, e descrivono la copia diretta con registrazione delle entry di primo livello create. Non dicono nulla su una cartella di staging: né se può comparire nel listing, né come si chiama, né cosa mostrare quando la sua rimozione fallisce. La scelta di staging fatta dallo sviluppatore non si concilia con queste regole senza una decisione sui Requirement.

## Fix options

### Nascondere lo staging nel listing

Il listing salta i nomi di staging di Weaver (`.copy-in.<random>.tmp`), che non compaiono mai nell'albero. Richiede di riscrivere la regola dei nomi con il punto e di descrivere lo staging nel Requirement di copia. Resta il caso di pulizia fallita, ma l'albero non mostra mai il residuo.

### Staging in una cartella nascosta di Weaver

Lo staging va in una cartella già ignorata (ad esempio sotto `.weaver/`), non elencata e non osservata. Costa un nuovo uso di `.weaver/` che oggi è scritta solo dalle decisioni, e un rename che può attraversare volumi se `.weaver/` è un mount.

### Ammettere staging e residuo nei Requirement

Una Proposal dichiara che durante la copia e dopo una pulizia fallita lo staging può essere visibile, e l'errore elenca cosa resta. Il codice resta com'è; la garanzia di non lasciare parti del drop vale solo se la rimozione riesce.

## Where in the code

`crates/core/src/file_view/copy_in.rs` (`copy_in_with`, staging e ramo di rimozione); `crates/core/src/file_view/listing.rs` (`list`, nomi con il punto elencati)

## Prepared question

```json
{
  "question": {
    "id": "gtit2hiq",
    "question": "Come deve comportarsi la cartella temporanea di una copia da drop esterno?",
    "context": "Una copia da un drop dall'esterno scrive prima in una cartella temporanea (`.copy-in.<random>.tmp`) dentro la cartella di destinazione e poi sposta le entry con rename.\n\n- I Requirement non menzionano questa cartella: dicono che la copia non lascia mai parti del drop nel progetto e che i nomi che iniziano con un punto sono tutti elencati.\n- Durante la copia chi espande la cartella di destinazione vede la cartella temporanea nell'albero.\n- Se la rimozione della cartella temporanea, o delle entry già spostate, fallisce, nel progetto resta parte del drop e l'errore elenca i resti invece di dire `Nothing was copied.`.",
    "options": [
      {
        "label": "Nascondere lo staging nell'albero",
        "description": "L'albero salta i nomi di staging di Weaver: la cartella temporanea non compare mai.",
        "details": "Richiede di riscrivere la regola dei nomi con il punto e di descrivere lo staging nel Requirement di copia. Se la pulizia fallisce restano comunque dei resti, ma l'albero non li mostra."
      },
      {
        "label": "Staging in una cartella di Weaver",
        "description": "La cartella temporanea va sotto `.weaver/`, mai elencata né osservata.",
        "details": "Costa un nuovo uso di `.weaver/`, oggi scritta solo da Weaver per le sue decisioni, e il rename può attraversare volumi se `.weaver/` è un mount."
      },
      {
        "label": "Lasciare lo staging visibile e dichiararlo",
        "description": "Una Proposal dichiara che lo staging può essere visibile durante la copia e dopo una pulizia fallita.",
        "details": "Il codice non cambia; la garanzia di non lasciare parti del drop vale solo se la rimozione riesce, e l'errore elenca ciò che resta."
      },
      {
        "label": "Mantenere il comportamento attuale",
        "description": "Le regole descrivono la copia con cartella di staging visibile durante la copia e i resti elencati nell'errore se la pulizia fallisce.",
        "details": "Applica la Proposal allegata: aggiunge ai Requirement il passo di staging e il caso di rimozione fallita. Il codice non cambia."
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
  "fingerprint": "sha256:98e3678b369fe45bdd1ed546658f0a56a7de68720e800169f27a977e4a4434b6",
  "prepared": "2026-11-04T05:52:17.604Z"
}
```
