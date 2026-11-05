---
refs:
  - job
  - requirements/ui#job
---

# Spazio sopra lista job

Nella vista dei job, la lista dei job a sinistra ha 12 px di spazio fra il suo bordo superiore e la prima riga, così la prima riga (e il suo sfondo quando è selezionata) non tocca più il bordo superiore della vista.

- Lo spazio fa parte del contenuto che scorre nella lista: quando la lista scorre, sparisce insieme alle righe, che salgono fino al bordo.
- Nessuno spazio aggiunto sotto l'ultima riga.
- Le righe (spazio interno, bordi, sfondo della selezione), la conversazione a destra, il riquadro dell'istruzione `Weaver` e il testo `No jobs yet` della lista vuota non cambiano.
- 12 px è lo stesso spazio superiore che la conversazione ha già, quindi le due colonne partono alla stessa altezza.
- La larghezza della lista non fa parte di questo Task: resta com'è nel codice (300 px fissi), anche se il requisito della vista dei job la descrive ridimensionabile.

Dove si trova oggi: la lista è il `<nav aria-label="Jobs">` (contenitore che scorre) con il suo `<ul>` senza padding in `ui/src/components/jobs/jobs-view.tsx` (circa righe 403-412); il testo `No jobs yet` è un `<p>` nello stesso `<nav>`, fuori dal `<ul>` (circa riga 405); le righe sono `Row` (circa righe 74-108); lo spazio della conversazione è il `p-3` del suo contenitore (circa riga 361).
