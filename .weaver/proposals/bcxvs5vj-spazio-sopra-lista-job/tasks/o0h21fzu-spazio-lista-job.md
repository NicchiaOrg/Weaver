---
refs:
  - job
  - requirements/ui#job
---

# Spazio sopra lista job

Nella vista dei job, la lista dei job a sinistra ha 12 px di spazio fra il suo bordo superiore e la prima riga, così la prima riga (e il suo sfondo quando è selezionata) non tocca più il bordo superiore della vista.

- Lo spazio fa parte del contenuto che scorre nella lista: quando la lista scorre, sparisce insieme alle righe, che salgono fino al bordo.
- Nessuno spazio aggiunto sotto l'ultima riga.
- Le righe (spazio interno, bordi, sfondo della selezione), la conversazione a destra e il riquadro dell'istruzione `Weaver` non cambiano.
- 12 px è lo stesso margine superiore che la conversazione ha già, quindi le due colonne partono alla stessa altezza.

Dove si trova oggi: la lista è il `<nav aria-label="Jobs">` con il suo `<ul>` in `ui/src/components/jobs/jobs-view.tsx` (circa righe 402-413), senza padding; le righe sono `Row` (circa righe 79-108).
