---
technical: true
refs:
  - requirements/ui#General
  - requirements/ui#work
  - requirements/ui#app
---
# Toast e avvisi chiudibili

Il developer deve poter chiudere ogni toast e gli avvisi sulla History in cima alla Board.

## Toast (`requirements/ui#General`, sezione `Toasts`)
- I toast vengono dal toaster shadcn `Sonner`, montato una volta per finestra, in basso a destra, al massimo 3 visibili insieme.
- Tre tipi: info, warning, error. Info: aspetto neutro del popover, nessuna icona. Warning: `amber` con icona `TriangleAlert`. Error: `red` con icona `CircleX`. Colori Tailwind 600 in tema chiaro e 400 in tema scuro, applicati a bordo, icona e titolo. Lo stato non è mai indicato dal solo colore: l'icona accompagna sempre il colore.
- Ogni toast, di qualunque tipo, sparisce da solo 4 s dopo essere apparso (come oggi).
- Ogni toast, di qualunque tipo, ha un pulsante di chiusura solo icona `X` (nome accessibile e tooltip `Close`) che lo rimuove subito. Il pulsante è raggiungibile da tastiera: `Alt+T` porta il focus sui toast, poi `Tab` raggiunge ogni pulsante di chiusura. `Esc` non chiude mai un toast; trascinare via un toast continua a chiuderlo.
- Nella tabella delle icone: `Warning toast / Error toast` → `TriangleAlert` / `CircleX`.
- Nel rendering Markdown, quando il browser di sistema non si apre per un link, compare un toast di errore `Could not open the link.`

## Open in editor (`requirements/ui#app`)
- Quando `Open in editor` fallisce, compare un toast di errore `Could not open the file in the editor.` con il motivo come descrizione (il codice lo fa già; resta un toast di errore, ora chiudibile).

## Avvisi sulla History nella Board (`requirements/ui#work`)
- Sotto le notice, sopra le due colonne, un `Alert` ambra con `TriangleAlert` per ogni Task citata la cui copia nella History non è riuscita (`Could not record T3 in History: <reason>. The Task stays until its copy can be made.`), ciascuno ora con un pulsante di chiusura (`X`, nome accessibile e tooltip `Close`).
- Un avviso chiuso resta nascosto finché la finestra resta aperta, anche quando la Board cambia; alla successiva apertura della finestra ricompare se è ancora vero.
- Un avviso nuovo, o il cui testo è cambiato, compare subito anche se altri sono stati chiusi.
- Lo stato degli avvisi chiusi vive solo nella finestra: nessun file del progetto e nessuna impostazione cambia.
- Nessun altro riquadro di errore fisso (errori di avvio, `Could not update the agent files`, `The Model has <n> errors`, errori di caricamento) diventa chiudibile.

## Dove nel codice
- Toaster: `ui/src/components/ui/sonner.tsx` (oggi imposta solo `theme`, `className` e `toastOptions.classNames`; nessun `closeButton`, nessuna icona per tipo), montato in `ui/src/components/app-providers.tsx:16`.
- Toast di errore esistenti: `ui/src/components/markdown.tsx:60`, `ui/src/components/open-in-editor.tsx:27-29`, `ui/src/components/questions/use-question-round.tsx:82`, `ui/src/components/requests/use-requests.tsx:94`, `ui/src/components/requests/request-pane.tsx:262,269,285,291,329,342`; toast informativo `ui/src/components/project/update-skills.tsx:107`.
- Avvisi della Board: `ui/src/components/board/board-view.tsx:187-196` (dal campo `warnings` dello snapshot, `src-tauri/src/board.rs:396`); il componente chiudibile esistente `NoticeStack` è in `ui/src/components/notices.tsx:17-32`.
- I test esistenti simulano `sonner` (`open-in-editor.test.tsx`, `markdown.test.tsx`, `update-skills.test.tsx`); il test degli avvisi è in `ui/src/components/board/board-view.test.tsx:383`.

## Fatto quando
- Ogni tipo di toast mostra il pulsante `X` e si chiude con un clic o da tastiera; warning ed error hanno colore e icona del loro tipo; tutti spariscono comunque dopo 4 s.
- Un avviso sulla History chiuso non ricompare con gli aggiornamenti della Board finché la finestra resta aperta; un avviso con testo diverso compare.
