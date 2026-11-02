---
column: done
technical: false
refs:
  - notification
  - requirements/tech#notification
  - requirements/ui#notification
drift: null
proposal: qmqh1o3p
agent: false
created: 2026-11-02T07:29:32.875Z
finished: 2026-11-02T12:16:35.974Z
---
# Notifiche di sistema

Dipende da "Scheletro del progetto", "Lavori dell'agente" (eventi `job.ENDED`, `job.WAITING`), "Round di domande" e "Finestre e avvio" (porta la finestra in primo piano).

Una `notification` dice al developer che tocca a lui: un job di un progetto è finito, si è fermato in modo anomalo, o aspetta le sue risposte o il suo permesso. Per scelta del developer è mostrata subito e solo quando la finestra di quel progetto non è in primo piano; altrimenti nulla. Nomina progetto, job e cosa è successo, mai ciò che l'agente ha scritto, chiesto o fatto. I job fermati dal developer e General non notificano mai. Una notifica non cambia nulla nel progetto, nei job o nei round. Non esiste un'impostazione: decidono i permessi del sistema.

## Machine `notification`
- **Not shown** (iniziale): `job.ENDED` → Shown quando il job è finito (Done) o fallito (Failed) e la finestra non è in primo piano; `job.WAITING` → Shown quando la finestra non è in primo piano.
- **Shown**: `desktop.NOTIFICATION_CLICKED` → Clicked; `question-round.CLOSED` → Withdrawn (il round annunciato si è chiuso); `developer.ANSWER_PERMISSION` → Withdrawn (il permesso annunciato ha avuto risposta); `job.ENDED` → Withdrawn quando annunciava un'attesa di un job ora finito.
- **Clicked** (finale): il developer è portato al progetto; emette `CLICKED`, che la Machine `window` usa per portare avanti la finestra. **Withdrawn** (finale): non è più vera e lascia l'elenco del sistema.

## Requisiti
- `requirements/tech#notification`: `tauri-plugin-notification` ≥ 2.8, chiamato solo da Rust; i clic arrivano da `Notification::on_action`. Test del primo piano al momento dell'invio: finestra del progetto visibile, non minimizzata e `is_focused()`, seguita da `WindowEvent::Focused`; un'altra finestra di Weaver a fuoco non conta. Ogni notifica ha un id `i32` da un contatore dell'app, mappato a project hash e destinazione (job, round o permesso); un `tap` su un id mappato mostra, de-minimizza e mette a fuoco la finestra e dice all'interfaccia cosa mostrare; clic su id non mappati o progetti chiusi ignorati. Ritiro: su Linux `remove_active`; su macOS e Windows si toglie solo la mappatura, così un clic successivo porta solo avanti la finestra. Tutte le mappature cadono alla chiusura della finestra. Su Windows un clic dall'Action Center avvia Weaver e single-instance mette a fuoco l'ultima finestra.
- `requirements/ui#notification`: testi esatti — Finished `<project> – <job> finished` / `Weaver is waiting for you.`; Several finished `<project> – <n> jobs finished` / `Weaver is waiting for you.`; Failed `<project> – <job> stopped` / `The agent stopped before finishing its work.`; Question round `<project> – <job> asks a question` / `The agent is waiting for your answers.`; Permission `<project> – <job> asks permission` / `The agent is waiting for your approval.` `<project>` è il nome della cartella, `<job>` il nome fisso del job. Raggruppamento: un job che finisce mentre è mostrata una notifica di fine dello stesso progetto la sostituisce con `<n> jobs finished`, al massimo una per progetto; gli altri tipi uno per evento. Id di progetto + job + tipo per sostituire o ritirare. Destinazione del clic: finito/fallito → conversazione del job (più job → vista Jobs); domanda → overlay del round; permesso → conversazione scorsa alla card. Nome e icona dell'app, suono predefinito del sistema.

## Fatto quando
- Test core puri sulla decisione di notificare (primo piano, job fermato dal developer, General esclusa) e sul raggruppamento.
- Con finestra in primo piano nessuna notifica; in secondo piano compaiono i testi esatti senza contenuti dell'agente.
- Il clic apre la vista giusta; round chiuso, permesso risposto o job finito ritirano la notifica d'attesa.
