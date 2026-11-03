---
technical: false
refs:
  - notification
  - window
  - requirements/tech#notification
  - requirements/ui#notification
---
# Notifica Certificati di Sistema

Quando Weaver attiva da solo System Certificates (evento `system-certificates.AUTO_ENABLED`, emesso dal gruppo `system-certificates`, che possiede anche il rilevamento di Zscaler, l'attivazione e i banner nelle finestre), deve comparire una notifica di sistema di un nuovo tipo, a livello di app.

Al termine deve valere:

- **Invio sempre:** la notifica passa da Not shown a Shown sempre, anche quando una finestra di Weaver è in primo piano. Non appartiene a nessun progetto e salta il test di primo piano usato per le notifiche dei job (`in_background`). Se il sistema non la mostra mentre Weaver è davanti, non cambia altro: i banner lo dicono comunque.
- **Testo:** titolo `System Certificates turned on`, messaggio `Weaver turned on System Certificates because Zscaler is installed: omp and the agent's commands now trust the certificates your system trusts.` (vedi `requirements/ui#system-certificates`, voce Turn-on notice). È l'unica notifica che nomina un prodotto; nome e icona dell'app, suono di sistema predefinito, nessuna impostazione.
- **Mappatura:** riceve un id `i32` dal contatore globale come le altre, ma mappato a nessun progetto (un bersaglio "app"), quindi non viene scartato come "id non mappato" né eliminato alla chiusura di una finestra di progetto. Non viene mai raggruppata né ritirata dagli eventi dei job.
- **Click (`desktop.NOTIFICATION_CLICKED` → Clicked, emette `notification.CLICKED`):** porta in primo piano (mostrata, non minimizzata, con focus) solo la finestra di progetto usata più di recente, secondo l'ordine di focus del registro finestre, e dice alla sua UI di mostrare la vista Settings (`ShowView::Settings`, oggi definito ma inutilizzato). Se nessuna finestra di progetto è aperta, porta in primo piano la finestra di benvenuto (creandola se serve).
- La UI di progetto, ricevendo `show-requested` con view Settings, apre la vista Settings.
- Test nel modulo notifications: mostrata anche con la finestra in primo piano; testo esatto; click con più finestre → la più recente su Settings; click senza finestre → welcome.

Sezioni da leggere: `requirements/tech#notification` (punto App notification), `requirements/ui#notification` (riga System Certificates turned on e Click target).

Punti di partenza:
- `notification` — src-tauri/src/notifications.rs: `Notifications` (`in_background`, `clicked`, `window_destroyed`), `Desktop::bring_forward`, `TauriDesktop::show`/`bring_forward`, `install` (on_action "tap"); crates/core/src/notification.rs: `Notice`, `Click`, `Destination`, `Book::clicked`
- `window` — src-tauri/src/windows/manager.rs: `ShowRequest`, `ShowView::Settings`, `focus_project`, `emit_show`, `show_welcome`, ordine di focus del `Registry`
- `requirements/ui#notification` — ui: gestione di `show-requested` nella finestra di progetto (apertura di Settings)
