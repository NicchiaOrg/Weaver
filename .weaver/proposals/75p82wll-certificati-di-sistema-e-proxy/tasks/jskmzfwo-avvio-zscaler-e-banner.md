---
technical: false
refs:
  - time
  - time.CERTIFICATES_REFRESH_DUE
---
# Avvio, Zscaler e banner

Questo Task decide quando System Certificates esporta da sé: all'avvio di Weaver, dopo l'accensione automatica per Zscaler e ogni 24 ore mentre Weaver resta aperto; e mostra in ogni finestra i banner dell'accensione e dell'esportazione automatica fallita. Si appoggia al motore costruito dal Task "Esportazione dei certificati di sistema" (modulo `weaver_core::system_certificates`, stato nel backend, punto d'ingresso dell'esportazione con il flag "automatica", regola dell'età del bundle), che va fatto prima.

## Avvio (transizioni di `Off` su `desktop.APP_STARTED`)

- Impostazione `systemCertificates.enabled` accesa e bundle scritto meno di 24 h fa → `Ready`, senza esportare. Accesa e bundle mancante o più vecchio di 24 h → `Exporting`, esportazione automatica.
- Solo su macOS e Windows, impostazione spenta, Zscaler installato e Weaver che non l'ha mai accesa da sé → Weaver la accende, lo ricorda, lo dice una volta e poi esporta, come esportazione automatica.
- Ordine di avvio: il controllo di Zscaler e l'età del bundle si decidono prima del primo controllo di omp (lo step `omp` di `lib.rs`, `omp::start`). Questo Task offre al servizio di omp un modo per sapere se all'avvio è partita un'esportazione e per aspettarne la fine, al più i suoi 30 s (per esempio un segnale che si risolve alla fine dell'esportazione o allo scadere del limite); nient'altro dell'avvio aspetta, né la finestra né l'interfaccia né i lavori. Far aspettare il primo controllo di omp e mostrare nel riquadro `Checking omp…` con la riga secondaria `Exporting system certificates first…` è del Task della parte `omp-setup` della Proposal.
- Cercare Zscaler non ritarda mai l'avvio e un errore nel cercarlo non mostra nulla.

## Accensione per Zscaler (`requirements/tech#system-certificates`, "Zscaler turn-on"; `requirements/rules#system-certificates`)

- Rilevamento con file system iniettato: su macOS esiste `/Applications/Zscaler/Zscaler.app`; su Windows `<ProgramFiles>\Zscaler` è una cartella, con `ProgramFiles` dall'ambiente, altrimenti `C:\Program Files`. Linux e ogni altro sistema: mai. Solo metadati di file: nessun processo, nessun contenuto, nessuna lista dei processi, nessuna rete.
- Un task avviato all'avvio dell'app, fuori dalla lista degli step di `startup` che annunciano i fallimenti; i suoi errori si registrano soltanto.
- Condizioni: impostazione spenta e `state.json` senza `systemCertificates.autoEnabledAt`. Con l'impostazione già accesa non si scrive né si ricorda nulla. Dopo l'accensione Weaver non la riaccende mai più da sé, anche se lo sviluppatore la spegne e Zscaler resta installato.
- Ordine: Weaver scrive l'impostazione tramite il servizio delle impostazioni (che lo dice a ogni finestra); poi scrive `autoEnabledAt`; poi emette `system-certificates.AUTO_ENABLED` e mostra i banner; poi esporta, come esportazione automatica. Se la scrittura dell'impostazione fallisce, nulla si ricorda né si mostra e il prossimo avvio riprova. Se fallisce la scrittura di `state.json` dopo, si registra a livello `warn` e l'impostazione resta accesa. L'accensione si registra a livello `info`.
- `AUTO_ENABLED` è ciò su cui la Machine `notification` mostra la notifica di sistema (anche con una finestra di Weaver in primo piano) e ne instrada il clic a Settings: la notifica e il suo clic sono del Task di quella parte della Proposal; questo Task emette solo l'evento, una volta.
- `state.json` (`requirements/data#system-certificates`): nuovo campo `systemCertificates: {autoEnabledAt}`, istante UTC ISO 8601; presente vuol dire mai più; Weaver non lo toglie mai; un `state.json` illeggibile conta come senza. Il file ha un solo proprietario nel backend, condiviso con i progetti recenti delle finestre, perché nessuna scrittura ne sovrascriva un'altra: lo `StateStore` oggi nasce in `windows::start`, dopo lo step `omp`, e va creato prima e condiviso.

## Aggiornamento giornaliero (`time.CERTIFICATES_REFRESH_DUE`)

- Mentre l'impostazione è accesa, un timer fa scattare `time.CERTIFICATES_REFRESH_DUE` 24 h dopo la fine dell'ultima esportazione, riuscita o no: da `Ready` e da `Failed` porta a `Exporting`, come esportazione automatica. Spegnere l'impostazione ferma il timer; riaccenderla lo riparte dalla fine dell'esportazione che segue. Il timer è iniettato, mai atteso davvero nei test (come `Scheduler`/`Clock` di `omp.rs`).

## Banner in ogni finestra (`requirements/ui#system-certificates`, "Turn-on notice" e "Export failure banner")

- Banner dell'accensione, in cima a ogni finestra, di progetto e di benvenuto, comprese quelle aperte dopo nella stessa esecuzione: `Weaver turned on System Certificates because Zscaler is installed: omp and the agent's commands now trust the certificates your system trusts.`, con `Open Settings` (solo nelle finestre di progetto: mostra la vista Settings) e una `X` solo icona (nome accessibile e tooltip `Close`). Chiuderlo in una finestra, o cliccare `Open Settings`, lo chiude in tutte. Non torna dopo l'uscita da Weaver.
- Banner del fallimento, dopo un'esportazione automatica fallita (avvio, Zscaler, giornaliera): colore di avviso con `TriangleAlert`, il testo della riga di stato fallita (`Could not export the system certificates (<reason>). Using the ones exported <date and time>.` oppure `Could not export the system certificates (<reason>). omp gets no certificates from Weaver.`, con `<date and time>` `HH:MM` se oggi, altrimenti `D MMM HH:MM`), `Open Settings` (solo finestre di progetto) e `X`. Un fallimento più recente lo sostituisce; un'esportazione riuscita o lo spegnimento dell'impostazione lo tolgono; chiuderlo in una lo chiude in tutte. Un'esportazione chiesta con `Refresh` o con l'accensione a mano non mostra banner: solo la riga di stato in Settings.
- Nella finestra di benvenuto i banner non hanno `Open Settings`, perché lì l'interruttore è già sotto il riquadro di omp.
- Lo stato dei banner (visibile o chiuso) vive nel backend, in memoria, dentro lo stato di System Certificates mandato a ogni finestra, così una finestra che si apre dopo lo legge con il comando di stato. Nessun testo d'errore nomina un prodotto; solo il banner dell'accensione nomina Zscaler.
- Componenti React accanto a `UpdateBanner`, montati dove esso è montato (`project-window.tsx`, `welcome-window.tsx`); `Open Settings` usa il meccanismo già esistente per mostrare una vista (`ShowView::Settings`).

## Test (`requirements/quality#system-certificates`)

`cargo test`: rilevamento di Zscaler per sistema con file system iniettato; accensione con flag assente o presente, impostazione già accesa, Zscaler assente, scrittura dell'impostazione fallita, `state.json` che fallisce, evento e banner mandati una volta sola; regola delle 24 h all'avvio e timer giornaliero con orologio iniettato; il segnale d'attesa che si risolve alla fine dell'esportazione o dopo 30 s. Vitest con Testing Library: i due banner, la chiusura condivisa, `Open Settings` solo nelle finestre di progetto, percorsi da tastiera e nomi accessibili.

## Dove iniziare

- `time.CERTIFICATES_REFRESH_DUE` — src-tauri/src/omp.rs: `Scheduler`, `Clock`, `TauriScheduler` come modello di timer iniettato
- `time` — src-tauri/src/lib.rs: `.setup` (step `settings`, `omp`, `windows::start`); src-tauri/src/startup.rs
- `time` — src-tauri/src/omp.rs: `start` (primo controllo di omp, che userà il segnale d'attesa)
- `time` — src-tauri/src/windows/local_state.rs: `LocalState`, `StateStore`; src-tauri/src/windows/mod.rs: creazione dello store in `start`
- `time` — src-tauri/src/settings.rs: `SettingsService::apply`
- `time` — ui/src/components/update-banner.tsx: `UpdateBanner`; ui/src/components/project-window.tsx (`show-requested`); src-tauri/src/windows/manager.rs: `ShowView::Settings`, `focus_project`
