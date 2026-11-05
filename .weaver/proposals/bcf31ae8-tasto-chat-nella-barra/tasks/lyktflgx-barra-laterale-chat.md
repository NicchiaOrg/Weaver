---
technical: true
refs:
  - requirements/ui#General
---
# Barra laterale con Chat

La barra laterale della finestra di progetto mostra le voci in questo ordine: Board, Chat, Jobs, Requirements, Specs, History, Settings. La voce della conversazione libera con omp si chiama `Chat` invece di `General`, in tutti i punti in cui compare il suo nome: il testo della voce, il tooltip quando la barra mostra solo le icone (sotto i 1024 px di larghezza) e il titolo nascosto per le tecnologie assistive della vista selezionata. L'icona resta `MessageSquare`; anche Jobs sale al terzo posto, mentre Requirements, Specs, History e Settings mantengono il loro ordine relativo.

Restano invariati: gli identificatori interni (l'id di vista `general`, `ShowView::General`, i comandi e gli eventi `general_*`), i conteggi delle voci Board e Jobs, il comportamento della selezione (cambiare voce conserva lo stato di ogni vista).

I test che fissano l'ordine e il nome delle voci, come la lista `ORDER` e i titoli nel test della barra laterale, usano il nuovo ordine e il nome `Chat`.

Punti noti nel codice: l'array `VIEWS` in `ui/src/views.ts` (righe 13-21, etichetta a riga 19), letto in ordine da `ui/src/components/sidebar.tsx` e `ui/src/components/project-window.tsx` (titolo nascosto alle righe 82-84); test in `ui/src/components/sidebar.test.tsx` (righe 17, 30, 84).
