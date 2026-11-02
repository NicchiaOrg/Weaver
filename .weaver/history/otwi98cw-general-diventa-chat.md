---
created: 2026-11-02T16:23:14.792Z
updated: 2026-11-02T16:23:14.792Z
summary: La conversazione generale si chiama Chat in vista, barra laterale, conferma di Clear, errori e rimando dai job terminati; il testo di un job concluso punta a Chat.
tasks:
- f2ws7caj
- lyktflgx
- h6spvmt3
cleared:
  f2ws7caj:
    title: Chat nella vista
    refs:
    - general-chat
    - requirements/ui#general-chat
    technical: false
    proposal: bcf31ae8
    label: Chat
    finished: 2026-11-02T16:10:07.631Z
    body: |-
      La conversazione generale del progetto si chiama `Chat` in ogni testo dell'interfaccia, mai `General`. Quando il Task è concluso valgono queste condizioni:

      - Il titolo della vista nell'intestazione è `Chat` (oggi `General`, in `ui/src/components/general/general-view.tsx`).
      - La finestra di conferma del pulsante `Clear` (icona `Eraser`) ha il titolo `Clear Chat?` (oggi `Clear General?`); restano invariati il testo `The conversation is deleted and a new one starts.` (con la sua variante durante un turno), i pulsanti `Clear` (distruttivo) e `Cancel` e il focus su `Cancel`.
      - Una richiesta rifiutata perché la Chat non è aperta riporta `Chat is not open for this project` (oggi `General is not open for this project` in `src-tauri/src/general/service.rs`, in tutti i punti in cui compare); il testo arriva nella vista tramite il `problem` di `use-general.tsx`.
      - Nessun altro testo visibile nella vista nomina `General`: anche i motivi di errore che possono comparire come `omp stopped: <reason>.` (per esempio `cannot write the instruction of General: …` in `src-tauri/src/general/actor.rs`) nominano la conversazione come `Chat`.
      - Restano `General` i testi destinati all'agente (istruzione della conversazione, riga di comando, skill), il Model, i Requirement `tech`, `data` e `cli` e gli identificatori interni (id di vista `general`, `WEAVER_GENERAL`, comandi `general_*`, nomi di file e cartelle). L'icona `MessageSquare` non cambia.
      - La voce della barra laterale, il suo tooltip e l'ordine delle voci non fanno parte di questo Task.
      - I test che fissano i vecchi testi (per esempio `ui/src/components/general/general-view.test.tsx` e i test Rust del servizio) sono aggiornati ai nuovi testi e passano.
  h6spvmt3:
    title: 'Job terminato: rimando a Chat'
    refs:
    - job
    - requirements/ui#job
    technical: false
    proposal: bcf31ae8
    label: Chat
    finished: 2026-11-02T16:09:50.816Z
    body: |-
      Quando una conversazione di un job è terminata resta in sola lettura e, al posto del campo messaggio, mostra il testo `This job has ended. Continue in Chat.` con il pulsante `Open Chat` (prima `This job has ended. Continue in General.` e `Open General`).

      `Open Chat` apre la vista Chat (la conversazione libera del progetto), esattamente come faceva `Open General`. L'identificatore interno della vista resta `general`: cambia solo il testo mostrato all'utente, non gli identificatori, i comandi o i testi destinati all'agente.

      I test che verificano i vecchi testi vanno aggiornati ai nuovi, `This job has ended. Continue in Chat.` e `Open Chat`.
  lyktflgx:
    title: Barra laterale con Chat
    refs:
    - requirements/ui#General
    technical: true
    proposal: bcf31ae8
    label: Chat
    finished: 2026-11-02T16:10:03.695Z
    body: |-
      La barra laterale della finestra di progetto mostra le voci in questo ordine: Board, Chat, Jobs, Requirements, Specs, History, Settings. La voce della conversazione libera con omp si chiama `Chat` invece di `General`, in tutti i punti in cui compare il suo nome: il testo della voce, il tooltip quando la barra mostra solo le icone (sotto i 1024 px di larghezza) e il titolo nascosto per le tecnologie assistive della vista selezionata. L'icona resta `MessageSquare`; anche Jobs sale al terzo posto, mentre Requirements, Specs, History e Settings mantengono il loro ordine relativo.

      Restano invariati: gli identificatori interni (l'id di vista `general`, `ShowView::General`, i comandi e gli eventi `general_*`), i conteggi delle voci Board e Jobs, il comportamento della selezione (cambiare voce conserva lo stato di ogni vista).

      I test che fissano l'ordine e il nome delle voci, come la lista `ORDER` e i titoli nel test della barra laterale, usano il nuovo ordine e il nome `Chat`.

      Punti noti nel codice: l'array `VIEWS` in `ui/src/views.ts` (righe 13-21, etichetta a riga 19), letto in ordine da `ui/src/components/sidebar.tsx` e `ui/src/components/project-window.tsx` (titolo nascosto alle righe 82-84); test in `ui/src/components/sidebar.test.tsx` (righe 17, 30, 84).
---
# General diventa Chat

La vista, il titolo, la finestra di conferma `Clear Chat?`, i messaggi di errore (`Chat is not open for this project`, `omp stopped: …`) e la voce della barra laterale nominano la conversazione `Chat`. Una conversazione di job terminata rimanda a Chat con `Open Chat`. Restano `General` i testi per l'agente e gli identificatori interni.
