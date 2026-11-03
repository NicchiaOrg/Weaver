---
technical: false
refs:
  - proposal
  - requirements/ui#proposal
  - requirements/ui#General
---
# Controllo modello a destra

Il controllo che cambia il modello di una Proposal (evento `developer.CHOOSE_PROPOSAL_MODEL`) cambia posizione e icona; il suo comportamento resta lo stesso: compare solo quando la Proposal crea Task, apre il Popover con la Select dei modelli di omp (`Use the Settings models` per primo) e la scelta si applica subito.

Al termine deve valere quanto segue:

- **Card della Proposal:** i controlli sono, nell'ordine, `Approve` (o `Refine` al suo posto), `Reject` con il suo split button, `View requirements` (solo quando il suo focus ha almeno un passo) e per ultimo il controllo del modello. Il controllo del modello è spinto al bordo destro della card, anche quando la riga dei controlli va a capo su una card stretta; gli altri controlli restano allineati a sinistra. Vale anche per la card di risoluzione, che usa gli stessi controlli.
- **Dialogo dei dettagli:** mostra gli stessi controlli della card nello stesso ordine, quindi il controllo del modello è l'ultimo dei controlli della card, seguito da `Ask more`, che resta l'ultimo elemento a destra.
- **Icona del controllo:** `Settings` di lucide-react (ingranaggio), la stessa della voce Settings della barra laterale, al posto di `Cpu`. Il pulsante resta con sola icona, con nome accessibile e tooltip invariati.
- **Badge del modello scelto:** resta invariato, con l'icona `Cpu` e il nome del modello.
- Una Draft lasciata a metà da un affinamento continua a non mostrare il controllo del modello.
