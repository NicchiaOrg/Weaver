---
created: 2026-11-02T14:59:02.830Z
updated: 2026-11-02T14:59:48.511Z
summary: I dialoghi sono centrati dal layout su pixel interi e si aprono con una sola dissolvenza. I toast hanno tipi info, warning ed error con pulsante di chiusura, e gli avvisi della Board sono chiudibili.
tasks:
- wzk7t3fl
- r64fs27c
- xv2vawao
cleared:
  wzk7t3fl:
    title: Dialoghi nitidi
    refs:
    - requirements/ui#General
    - requirements/quality#General
    technical: true
    proposal: 0k1yai6z
    label: Dialoghi nitidi
    finished: 2026-11-02T14:46:56.993Z
    body: |-
      Ogni tanto le finestre di dialogo, in particolare il modale delle domande, appaiono sfocate. La causa probabile è nei componenti condivisi `ui/src/components/ui/dialog.tsx` (pannello alla riga ~37) e `ui/src/components/ui/alert-dialog.tsx` (pannello alla riga ~32): sono posizionati con `fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2`, cioè spostati di metà della propria dimensione. Quella metà cade spesso su frazioni di pixel (altezze dipendenti dal contenuto, come il titolo `text-lg leading-tight` di 22,5 px; il tetto `max-h-[85vh]`; finestre di larghezza o altezza dispari), e WebKit disegna allora il testo fuori dalla griglia dei pixel. Inoltre l'apertura `animate-in fade-in-0 zoom-in-95` (keyframe `enter` di `tw-animate-css`, che anima `transform` e `filter`) mette il pannello su un livello grafico separato che può restare sfocato.

      Quando il Task è finito:

      - Ogni `Dialog` e `AlertDialog` condiviso, quindi anche il modale delle domande (`ui/src/components/questions/question-overlay.tsx`), i dialoghi di dettaglio e dei moduli (`ui/src/components/dialogs.tsx`) e tutte le conferme, è centrato nella finestra dal layout (per esempio un contenitore flex o grid a tutta finestra), senza alcuno spostamento in percentuale della propria dimensione. L'aspetto resta quello di oggi: centrato in verticale e in orizzontale, stesse larghezze massime (720 px, 760 px per le domande, `max-w-lg` per le conferme), altezza massima 85% della finestra con il contenuto che scorre all'interno.
      - L'angolo in alto a sinistra di ogni dialogo sta sempre su pixel CSS interi, qualunque sia l'altezza del contenuto (anche quando cambia passando da una domanda all'altra) e la dimensione della finestra, dispari compresa.
      - I dialoghi si aprono con una sola dissolvenza di 150 ms, senza ingrandimento, scala né altre trasformazioni o filtri. Con `prefers-reduced-motion: reduce` compaiono subito, come oggi. Popover, DropdownMenu, Select e Tooltip mantengono la loro animazione di apertura; le bolle delle Drift sul Diagramma non cambiano.
      - Il comportamento resta invariato: Esc, il pulsante `X` e il clic fuori chiudono un Dialog; un AlertDialog si chiude solo con i suoi pulsanti o Esc; nel modale delle domande Esc e il clic fuori non fanno nulla; il focus resta intrappolato e torna al controllo che ha aperto il dialogo.
      - Il test end-to-end (tauri-driver sull'app impacchettata) porta la finestra del progetto a una larghezza e un'altezza dispari, apre il dialogo di dettaglio di una card e verifica che `left` e `top` del suo riquadro siano numeri interi di pixel CSS.
  r64fs27c:
    title: Toast e avvisi chiudibili
    refs:
    - requirements/ui#General
    - requirements/ui#work
    - requirements/ui#app
    technical: true
    proposal: sjnrrawz
    label: Toast chiudibili
    finished: 2026-11-02T14:49:07.977Z
    body: |-
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
  xv2vawao:
    title: Avvisi nell'editor richieste
    refs:
    - design-request
    - requirements/ui#design-request
    technical: false
    proposal: sjnrrawz
    label: Toast chiudibili
    finished: 2026-11-02T14:45:17.077Z
    body: |-
      Nell'editor di una richiesta scritta (`design-request`) i due avvisi sui cambiamenti esterni al file diventano toast di avviso (warning) invece di toast semplici (info):

      - Quando il file della richiesta cambia su disco mentre ci sono modifiche in sospeso, Weaver scrive la propria versione e mostra il toast di avviso `The request changed on disk; Weaver kept your version.` Senza modifiche in sospeso l'editor si ricarica, come oggi, senza toast.
      - Quando una richiesta in scrittura viene cancellata fuori da Weaver, l'editor si chiude e mostra il toast di avviso `The request was deleted outside Weaver.`

      Il rifiuto di un'immagine (`Only PNG, JPEG, GIF or WebP images up to 10 MB.`) e gli errori di salvataggio delle immagini restano toast di errore, come già sono.

      L'aspetto dei toast (colore ambra con icona `TriangleAlert` per gli avvisi, rosso con `CircleX` per gli errori, nessuna icona per gli info, pulsante di chiusura `X` focalizzabile con `Alt+T` e `Tab`, Esc che non chiude, scomparsa automatica dopo 4 s) è trattato da un altro Task su `requirements/ui#General`: qui conta solo il tipo di toast usato dall'editor. I test che simulano `sonner` per questi toast vanno aggiornati al nuovo tipo.
---
# Dialoghi nitidi e toast chiudibili

I dialoghi condivisi, anche il modale delle domande, sono centrati dal layout senza spostamenti in percentuale e si aprono con una sola dissolvenza di 150 ms. Per portare l'angolo in alto a sinistra su pixel CSS interi con spazio libero dispari, un hook corregge la frazione residua con offset in pixel (nessun transform). Verifica: la correzione è coperta solo da un test unitario con misure simulate; il passo end-to-end con finestra a dimensioni dispari è scritto ma non è mai stato eseguito, quindi su questo punto il Task non è verificato. I toast usano il toaster Sonner con tre tipi (info neutro, warning ambra con icona di avviso, error rosso con icona di errore), pulsante di chiusura raggiungibile con Alt+T e Tab, scomparsa dopo 4 s. Gli avvisi sulla History in cima alla Board si chiudono e restano nascosti finché la finestra è aperta. Nell'editor delle richieste i due avvisi sui cambiamenti esterni al file sono ora toast di avviso.
