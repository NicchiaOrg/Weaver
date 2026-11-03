---
created: 2026-11-03T10:30:32.379Z
updated: 2026-11-03T11:49:37.277Z
summary: Una finestra di progetto può andare in background senza sospendere nulla, con la voce Hide della barra laterale o la scelta nella conferma di chiusura, e torna dal Dock, da un nuovo lancio o dalla tray.
tasks:
- zte65stk
- oqd0rs7q
cleared:
  zte65stk:
    title: Finestre in background
    refs:
    - window
    - developer.SEND_TO_BACKGROUND
    - developer.REOPEN_APP
    - developer.CLOSE_WINDOW
    - developer.QUIT_APP
    - requirements/tech#window
    - requirements/tech#notification
    - requirements/ui#General
    technical: false
    proposal: s1xwol5o
    label: Run in background
    finished: 2026-11-03T10:27:55.564Z
    body: |-
      Quando il Task è finito, la finestra di un progetto può andare in background senza che venga sospeso nulla, e tornare dal Dock, da un nuovo lancio o dall'icona nella tray.

      **Mandare in background (`window/Shown` → `In background`)**
      - Nella barra laterale di ogni finestra di progetto c'è la voce `Run in background`, con l'icona `EyeOff`, nel gruppo in basso a sinistra prima di Usage e Settings. È sempre attiva, non è una vista, non è mai selezionata e non ha mai `aria-current`; con la barra stretta (sotto 1024 px) mostra il nome come tooltip.
      - Un clic manda subito in background solo quella finestra (`developer.SEND_TO_BACKGROUND`), senza conferma: il backend chiama `hide()` sulla finestra, che sparisce dallo schermo e, su Windows e Linux, dalla taskbar; su macOS l'icona nel Dock resta. Si possono mandare in background anche tutte le finestre di progetto.
      - Non viene sospeso nulla e `window.CLOSED` non viene emesso: job, General, conversazioni Ask more e question round continuano; la webview resta viva e, al ritorno, la finestra mostra la stessa vista con lo stato di ogni vista com'era.
      - Il registro delle finestre tiene in memoria quali finestre sono in background. Una finestra in background conta come aperta ovunque: per le notifiche (che la trattano come in background, come già fa il test di primo piano su una finestra non visibile), per la lista di ripristino scritta all'uscita e per la regola dell'ultima finestra.

      **Chiusura con conferma (`developer.CLOSE_WINDOW`)**
      - Quando si chiude una finestra con job in corso e senza modifiche non salvate, la conferma nativa ha i pulsanti `Suspend and Close`, `Keep working in background` e `Cancel` (`MessageDialogButtons::YesNoCancelCustom`, `Cancel` per ultimo). `Suspend and Close` fa quello che fa oggi; `Keep working in background` equivale a `developer.SEND_TO_BACKGROUND`: niente sospeso, finestra non chiusa.
      - Con modifiche non salvate la conferma resta `Save all` / `Discard all` / `Cancel`, senza background. La conferma di uscita (`developer.QUIT_APP`) non offre mai il background.

      **Ritorno (`window/In background` → `Shown`)**
      - `developer.REOPEN_APP` mostra tutte le finestre di progetto in background (`show()`, `unminimize()`), dalla meno usata di recente, poi dà il focus a quella usata più di recente, che finisce sopra. Senza finestre in background porta davanti la finestra usata più di recente, oppure la finestra di benvenuto quando non è aperta nessuna finestra di progetto.
      - È `developer.REOPEN_APP`:
        - su macOS, il clic sul Dock (`RunEvent::Reopen`) ogni volta che una finestra di progetto è in background, qualunque cosa dica `has_visible_windows`; altrimenti resta il comportamento di oggi;
        - un secondo lancio senza cartella (single-instance), anche quello causato da un clic sull'Action Center di Windows;
        - `Show Weaver` dalla tray e, su Windows, il clic sinistro sull'icona.
      - `developer.OPEN_PROJECT` (anche un secondo lancio con cartella), il clic su una notifica e un `Save all` fallito durante una chiusura o un'uscita fanno tornare solo la propria finestra.

      **Icona nella tray (solo Windows e Linux)**
      - Creata una volta all'avvio con la tray di Tauri 2 (feature `tray-icon`, `TrayIconBuilder`) e tenuta fino all'uscita, con l'icona dell'app e il tooltip `Weaver`. Su macOS non c'è.
      - Menu: `Show Weaver`, un separatore, `Quit`, che è `developer.QUIT_APP` con le stesse conferme di `Ctrl+Q`.
      - Su Windows il clic sinistro (pulsante rilasciato) è `Show Weaver` e il clic destro apre il menu (`show_menu_on_left_click(false)`); su Linux c'è solo il menu.
      - Se l'icona non si può creare, viene registrato un warning e non cambia nient'altro.

      **Ultima finestra e riavvio**
      - Chiudere l'ultima finestra di progetto sullo schermo mentre altre sono in background non apre la finestra di benvenuto. Su Windows e Linux, chiudere la finestra di benvenuto non fa uscire da Weaver quando una finestra di progetto è in background.
      - All'uscita, la lista di ripristino comprende anche le finestre in background; alla partenza successiva si riaprono tutte sullo schermo e i loro job ripartono come oggi.

      **Documentazione**
      - `CHANGELOG.md` riceve sotto `## [Unreleased]`, gruppo `Added`, una riga per il tasto `Run in background` con il ritorno dal Dock, da un nuovo lancio o dalla tray, e una per la scelta `Keep working in background` nella conferma di chiusura.
  oqd0rs7q:
    title: Voce Hide
    refs:
    - requirements/ui#General
    technical: true
    proposal: ldsyhkjk
    label: Hide
    finished: 2026-11-03T11:48:44.681Z
    body: |-
      Quando il Task è finito, la voce della barra laterale di ogni finestra di progetto che oggi si chiama `Run in background` si chiama `Hide`. Cambia solo il testo: comportamento, posizione e icona restano quelli di oggi.

      **La voce `Hide`**
      - Sta nel gruppo in basso a sinistra, prima di Usage e Settings (ordine: Hide, Usage, Settings), con l'icona `EyeOff`.
      - Il testo `Hide` è anche il suo nome accessibile e, sotto una larghezza di finestra di 1024 px (barra di sole icone, 56 px), il suo tooltip.
      - È sempre attiva, non è una vista, non è mai selezionata e non ha mai `aria-current`. Un clic manda subito in background la finestra (`developer.SEND_TO_BACKGROUND`, comando `send_to_background`), senza conferma; al ritorno la finestra mostra la stessa vista con lo stato di ogni vista com'era.
      - Quando la finestra non si riesce a nascondere, il toast di errore dice `Could not hide the window.` (al posto di `Could not run the window in the background.`) e dà il motivo come descrizione, come oggi.

      **Cosa non cambia**
      - Il pulsante `Keep working in background` della conferma nativa di chiusura di una finestra con job in corso resta com'è.
      - I nomi interni (comando `send_to_background`, Event `developer.SEND_TO_BACKGROUND`, State `window/In background`) restano invariati.

      **Dove nel codice**
      - `ui/src/views.ts` — `BACKGROUND_ENTRY`: `label` diventa `Hide`; aggiornare il commento che nomina `Run in background`.
      - `ui/src/components/project-window.tsx` — il testo del `toast.error` in `onActivate` e il commento che nomina `Run in background`.
      - `ui/src/components/sidebar.test.tsx` — i test che cercano la voce per nome e il testo del toast usano i nuovi testi.

      **Documentazione**
      - `CHANGELOG.md`, sotto `## [Unreleased]`, gruppo `Added`: la riga che introduce la voce della barra laterale la chiama `Hide` invece di `Run in background`; la riga su `Keep working in background` resta com'è.
---
# Finestre in background

La barra laterale ha la voce Hide (EyeOff), prima di Usage e Settings, che nasconde solo quella finestra: job, Chat, conversazioni Ask more e question round continuano e la finestra conta come aperta per notifiche, lista di ripristino e regola dell'ultima finestra. Se non si riesce a nasconderla, un toast di errore dice Could not hide the window. La conferma di chiusura con job in corso offre Suspend and Close, Keep working in background e Cancel. Il ritorno avviene dal Dock su macOS, da un secondo lancio senza cartella e da Show Weaver nella tray su Windows e Linux, che ha anche Quit.
