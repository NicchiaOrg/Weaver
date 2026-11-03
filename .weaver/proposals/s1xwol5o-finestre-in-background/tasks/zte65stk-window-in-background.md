---
refs:
  - window
  - developer.SEND_TO_BACKGROUND
  - developer.REOPEN_APP
  - developer.CLOSE_WINDOW
  - developer.QUIT_APP
  - requirements/tech#window
  - requirements/tech#notification
  - requirements/ui#General
---
# Finestre in background

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
