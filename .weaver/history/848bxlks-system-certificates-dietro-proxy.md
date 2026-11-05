---
created: 2026-11-02T21:27:41.954Z
updated: 2026-11-02T21:27:41.954Z
summary: Weaver esporta i certificati di fiducia del sistema, li passa a omp, terminali e agente, li accende da sé con Zscaler e spiega gli errori da proxy.
tasks:
- zsp3xtv8
- gv7mj4j4
- jskmzfwo
- 5a3a4egn
- 0d8657rp
- s269d588
- nwgennrh
- q0i3h62z
- 3a78va2x
cleared:
  0d8657rp:
    title: Installer, terminali e ricontrolli
    refs:
    - omp-setup
    - omp.INSTALL_FAILED
    - requirements/tech#omp-setup
    - requirements/ui#omp-setup
    technical: false
    proposal: 75p82wll
    label: System Certificates
    finished: 2026-11-02T21:22:44.053Z
    body: |-
      Quando il lavoro è finito, la Machine `omp-setup` aspetta l'esportazione dei certificati all'avvio, i terminali dell'installer e del login ricevono le variabili di System Certificates, l'installer lascia il suo codice d'uscita in un marcatore e un installer fallito riporta omp a Missing con il suo testo; omp si ricontrolla dopo ogni esportazione e quando la modalità si spegne.

      ## Checking all'avvio
      - All'avvio dell'app, mentre System Certificates esporta il bundle, il primo controllo di omp aspetta che l'esportazione finisca (al più 30 s, poi controlla comunque). Il riquadro di omp mostra `Checking omp…` con la riga secondaria `Exporting system certificates first…` solo durante quell'attesa.

      ## Ricontrolli
      - `developer.CHANGE_SETTINGS` → Checking da Missing, Not connected e Ready quando cambia il posto di omp **oppure** System Certificates viene spento (da Installing solo per il posto di omp). Accenderlo non ricontrolla qui: lo fa la fine dell'esportazione.
      - `system-certificates.EXPORT_ENDED` → Checking da Missing, Not connected e Ready (dopo ogni esportazione, riuscita o no). Negli altri stati non fa nulla.

      ## Terminali (`requirements/tech#omp-setup`)
      - Con System Certificates acceso, le variabili (dalla funzione che restituisce le variabili dei certificati, gruppo `system-certificates`, lette all'apertura) arrivano al terminale:
        - Windows: nell'ambiente del processo `powershell.exe`;
        - macOS: il comando di `do script` inizia con `/usr/bin/env <NAME>=<bundle path> … ` e il resto gira sotto `/bin/sh -c`;
        - Linux: lo script `sh -c` inizia con `export <NAME>=<bundle path> …;`;
        - senza terminale, il comando da copiare include le variabili.
      - Una sola funzione testata quota per la shell di destinazione; si inseriscono solo il percorso assoluto di omp, un id di provider della lista fissa, il percorso del bundle e quello del marcatore.

      ## Installer e `omp.INSTALL_FAILED`
      - Non più `curl … | sh` / `irm … | iex`: l'installer si scarica in un file temporaneo e poi si esegue, così un download fallito conta come fallimento; poi scrive il codice del primo passo fallito, altrimenti `0`, in `<app local data>/omp-install/<nonce>.code` (decimale come testo) e cancella il temporaneo.
        - macOS/Linux, in `/bin/sh`: `curl -fsSL https://omp.sh/install -o <temp>`, poi `sh <temp>`.
        - Windows: `Invoke-RestMethod https://omp.sh/install.ps1 -OutFile <temp>.ps1`, poi un `powershell.exe -NoProfile -ExecutionPolicy Bypass -File <temp>.ps1` figlio; il codice è `$LASTEXITCODE`; un errore terminante del download vale `1`.
      - Ogni `INSTALL_OMP` usa un nonce nuovo e cancella i marcatori precedenti (anche `Open installer again`, che dimentica il codice di prima); trovare omp li cancella. Weaver non scrive mai `ompPath`.
      - In Installing, un ricontrollo che non trova omp legge il marcatore del nonce corrente: codice ≠ `0` è `omp.INSTALL_FAILED` → Missing; `0`, nessun marcatore o illeggibile è `omp.MISSING` (resta Installing). Il codice resta fino al prossimo `INSTALL_OMP` o finché omp è trovato.
      - In Missing dopo un installer fallito, il riquadro mostra `The omp installer failed (exit code <N>); see the terminal. A corporate proxy that inspects HTTPS traffic is a common cause.` e, con l'impostazione spenta, ` Turning on System Certificates in Weaver's settings may fix it.`; azione `Install omp`.

      ## Riquadro (`requirements/ui#omp-setup`)
      - Riga Missing: oltre ai motivi esistenti, una spiegazione del proxy (con `.` finale, prodotta dal Task di `requirements/tech#omp`) e il testo dell'installer fallito. Il resto della tabella non cambia.

      ## Da dove partire
      - `omp-setup` — crates/core/src/omp/machine.rs (Event/State, nuovo evento per installer fallito ed export finito)
      - `omp-setup` — src-tauri/src/omp.rs:`start` (reagisce solo a `SettingKey::OmpPath`), `change_omp_path`, `open_installer`, `open_login`
      - `omp-setup` — src-tauri/src/lib.rs `.setup` e src-tauri/src/startup.rs (ordine "settings"/"omp" e attesa dell'export)
      - `requirements/tech#omp-setup` — crates/core/src/omp/terminal.rs:`INSTALLER_UNIX`, `INSTALLER_WINDOWS`, `login_command`, `launch_plan`, `open`, `quote_posix`, `quote_powershell`, `applescript_string`
      - `omp.INSTALL_FAILED` — crates/core/src/paths.rs:`AppDirs::local_data_dir` (cartella `omp-install/`)
      - `requirements/ui#omp-setup` — ui/src/components/omp-status.tsx:`boxFace`, `OmpStatusBox`
  3a78va2x:
    title: Documentazione proxy e CHANGELOG
    refs:
    - project-setup
    - requirements/docs#General
    - requirements/docs#system-certificates
    technical: false
    proposal: 75p82wll
    label: System Certificates
    finished: 2026-11-02T20:50:04.111Z
    body: |-
      Quando il Task è finito, `README.md` e un nuovo `CHANGELOG.md` alla radice del repository descrivono System Certificates come dicono i Requisiti dell'Area `docs` (frammento `docs/main.md`), in inglese come tutta la documentazione del progetto. Il Task scrive solo documentazione: nessun codice.

      La Feature `project-setup` dice ora che dietro un proxy aziendale che ispeziona HTTPS omp non riesce a installarsi o a raggiungere il provider perché non si fida del certificato del proxy; System Certificates, spenta di default, fa sì che ogni processo di omp avviato da Weaver si fidi delle autorità di cui si fida il sistema; Weaver la accende da sé una volta quando Zscaler è installato; dove Weaver mostra perché omp è fallito spiega in parole semplici quando la causa probabile è un proxy. La documentazione descrive questo comportamento, realizzato dagli altri Task della stessa Proposal.

      **`CHANGELOG.md` (regole generali, `requirements/docs#General`):**
      - Formato Keep a Changelog 1.1.0: breve intestazione che nomina il formato, poi `## [Unreleased]` in cima e una sezione `## [<version>] - <YYYY-MM-DD>` per ogni versione rilasciata, la più recente prima, con i soli gruppi che hanno voci tra `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`.
      - Da ora ogni modifica visibile a chi usa l'app aggiunge almeno una riga sotto `## [Unreleased]`, nella stessa modifica del codice: cosa cambia per l'utente, in una frase, senza id di Task o Proposal. Le modifiche visibili solo a chi contribuisce (test, CI, refactoring) non ne aggiungono.
      - Al rilascio di una versione, le righe di `## [Unreleased]` passano nella sezione di quella versione.
      - Il file nasce con questo Task; le prime voci di `## [Unreleased]` descrivono System Certificates, l'accensione per Zscaler, le nuove spiegazioni degli errori e l'installer di omp che ora segnala il proprio fallimento.

      **`README.md`:**
      - Mantiene le sezioni per installare e per chi contribuisce; il comportamento che serve per far funzionare Weaver va in una sezione propria dopo `## Install`.
      - Nuova sezione `## Behind a corporate proxy`, dopo `## Install` (oggi `README.md` righe 5-13), che dice:
        - perché omp non si installa o non raggiunge il provider dietro un proxy che ispeziona HTTPS;
        - cosa fa System Certificates, che è spenta di default e dove si trova (Settings → omp, e sotto il riquadro di omp in Get Started e nella finestra di benvenuto);
        - quali certificati usa: quelli di cui il sistema si fida (su macOS le impostazioni di fiducia di utente, amministratore e sistema; su Windows gli archivi radice dell'utente e della macchina; su Linux il bundle di sistema), più le radici pubbliche di Mozilla; che li riesporta all'avvio se hanno più di un giorno, ogni giorno mentre Weaver è aperto, e con `Refresh`;
        - le variabili che imposta (`NODE_EXTRA_CA_CERTS`, `SSL_CERT_FILE`, `REQUESTS_CA_BUNDLE`, `CURL_CA_BUNDLE`, `GIT_SSL_CAINFO`, `PIP_CERT`, `CARGO_HTTP_CAINFO`, `AWS_CA_BUNDLE`, `DENO_CERT`, più `PI_REQUEST_COMPRESSION=0`), che sovrascrivono i valori ereditati e arrivano a ogni processo di omp avviato da Weaver e ai suoi terminali di installer e login, così che i comandi dell'agente le ereditano;
        - l'accensione una sola volta quando Zscaler è installato, solo su macOS e Windows, mai ripetuta dopo che il developer la spegne;
        - le spiegazioni mostrate per errori di certificato, errori HTTP e installer fallito, e il loro suggerimento di accendere System Certificates quando è spenta;
        - che gli strumenti Java e ogni programma che legge un truststore invece di un bundle PEM non sono coperti.
      - `### Where Weaver keeps files` (oggi `README.md` righe 97-99) nomina il bundle `system-certificates.pem` e i marcatori dell'installer in `omp-install/`, nella cartella dei dati locali.

      Punti di partenza:
      - `requirements/docs#system-certificates` — `README.md` (sezioni `## Install` 5-13, `### Where Weaver keeps files` 97-99)
      - `requirements/docs#General` — `CHANGELOG.md` (da creare)
  5a3a4egn:
    title: Notifica Certificati di Sistema
    refs:
    - notification
    - window
    - requirements/tech#notification
    - requirements/ui#notification
    technical: false
    proposal: 75p82wll
    label: System Certificates
    finished: 2026-11-02T20:54:25.869Z
    body: |-
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
  gv7mj4j4:
    title: Interruttore System Certificates
    refs:
    - developer.REFRESH_SYSTEM_CERTIFICATES
    - requirements/ui#system-certificates
    - requirements/ui#developer.CHANGE_SETTINGS
    technical: false
    proposal: 75p82wll
    label: System Certificates
    finished: 2026-11-02T21:12:56.899Z
    body: |-
      Lo sviluppatore accende e spegne System Certificates, vede lo stato dell'esportazione e la ripete con `Refresh`, da Settings e da sotto il riquadro di omp. Si appoggia al Task "Esportazione dei certificati di sistema" (chiave `systemCertificates.enabled`, campo booleano dello schema, evento e comando di stato, comando di `Refresh`), che va fatto prima. I banner sono del Task "Avvio, Zscaler e banner"; le spiegazioni del proxy nei motivi di fallimento di omp sono funzioni del core collegate alle superfici dai Task delle altre parti della Proposal.

      Delle sezioni di `requirements/ui#system-certificates` questo Task realizza "Switch", "Places", "Status line" e "Refresh"; "Turn-on notice" ed "Export failure banner" sono del Task dei banner; "Proxy explanations", "Where they replace the reason" e "Failed installer" sono testi prodotti dal core e mostrati dai Task delle parti `omp-setup`, `omp` e dei lavori. Nessun testo d'errore nomina un prodotto.

      ## Interruttore

      - Nuovo componente shadcn `Switch` in `ui/src/components/ui/`, su Radix `@radix-ui/react-switch` (nuova dipendenza): è la prima impostazione booleana della vista Settings. Titolo `System Certificates`, descrizione dallo schema delle impostazioni: `omp trusts the certificates your system trusts, for proxies that inspect HTTPS. Replaces NODE_EXTRA_CA_CERTS, SSL_CERT_FILE and similar variables.` La chiave non si mostra mai.
      - Si salva subito, senza conferma, come le altre impostazioni (`requirements/ui#developer.CHANGE_SETTINGS`, "Saving"): una scrittura fallita mostra `Could not save System Certificates: <reason>.` sotto il campo e rimette il valore salvato. Un cambio da un'altra finestra si vede ovunque subito.
      - Posti: un solo componente, ultimo nel gruppo `omp` di Settings (dopo `Permissions`: status box, `Providers`, `omp path`, `Permissions`, `System Certificates`), e sotto il riquadro di omp ovunque esso compaia fuori da Settings, cioè Get Started (progetto con Model vuoto, finché il riquadro si vede) e la finestra di benvenuto. Ogni posto mostra lo stesso valore e lo stesso stato, e un cambio in uno si vede in tutti insieme.

      ## Riga di stato

      - Solo a interruttore acceso, una riga di testo secondario sotto la descrizione, un'icona a sinistra e `Refresh` all'estremità destra:
        - in esportazione: `Loader` e `Exporting system certificates…`;
        - pronto: `CircleCheck` (successo) e `<N> certificates, exported <date and time>.`;
        - fallito con un bundle precedente: `TriangleAlert` (avviso) e `Could not export the system certificates (<reason>). Using the ones exported <date and time>.`;
        - fallito senza bundle: `TriangleAlert` (avviso) e `Could not export the system certificates (<reason>). omp gets no certificates from Weaver.`
      - `<date and time>` è quando è stato scritto il bundle in uso: `HH:MM` se oggi, altrimenti `D MMM HH:MM`. `<reason>` è uno dei motivi del backend (`could not read the system certificates: …`, `the export did not finish within 30 s`, `could not write the bundle: …`, `no certificate authority found`). La formattazione è condivisa con il banner del fallimento.
      - La riga è la descrizione accessibile dell'interruttore (`aria-describedby`) e i suoi cambi si annunciano (`aria-live="polite"`). `Loader` è fermo con il movimento ridotto.
      - Lo stato si legge con il comando di stato al caricamento e si segue con l'evento del backend (iscriversi prima, poi chiedere, come `UpdateBanner`). L'interfaccia non legge mai il bundle né i certificati.

      ## Refresh (`developer.REFRESH_SYSTEM_CERTIFICATES`)

      - Bottone `outline` piccolo, icona `RefreshCw`, testo `Refresh`, nome accessibile `Refresh system certificates`; chiama il comando di `Refresh` del backend, che porta da `Ready` o `Failed` a `Exporting` come esportazione chiesta (nessun banner, solo la riga di stato). Disabilitato durante l'esportazione, con il tooltip `Exporting system certificates…`.
      - Sotto circa 300 px va a capo sotto il testo, allineato a sinistra; nulla si sovrappone.

      ## Test (`requirements/quality#system-certificates`)

      Vitest con Testing Library: l'interruttore (stato, salvataggio, errore di scrittura, valore uguale in ogni posto), la riga di stato nei quattro casi con data di oggi e di un altro giorno, `Refresh` e il suo stato disabilitato con tooltip, percorsi da tastiera e nomi accessibili, `aria-describedby` e `aria-live`.

      ## Dove iniziare

      - `requirements/ui#developer.CHANGE_SETTINGS` — ui/src/components/settings-view.tsx: `SettingsView`, `Group title="omp"`, `RadioSetting`/`TextSetting` come modello
      - `requirements/ui#system-certificates` — ui/src/components/omp-status.tsx: `OmpSetup` (usato da project/empty-model.tsx `OmpNotReady` e da welcome-window.tsx)
      - `requirements/ui#system-certificates` — ui/src/components/ui/checkbox.tsx (modello di primitiva Radix); nessuno `Switch` esiste ancora
      - `developer.REFRESH_SYSTEM_CERTIFICATES` — ui/src/components/update-banner.tsx: iscrizione all'evento e comando di stato
  jskmzfwo:
    title: Avvio, Zscaler e banner
    refs:
    - time
    - time.CERTIFICATES_REFRESH_DUE
    technical: false
    proposal: 75p82wll
    label: System Certificates
    finished: 2026-11-02T21:21:24.653Z
    body: |-
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
  nwgennrh:
    title: Certificati per job e General
    refs:
    - requirements/tech#job
    - requirements/data#job
    - requirements/tech#general-chat
    technical: true
    proposal: 75p82wll
    label: System Certificates
    finished: 2026-11-02T21:09:27.585Z
    body: |-
      Ogni processo omp ACP avviato da Weaver, per un job o per il General di un progetto, riceve le variabili dei System Certificates (`requirements/tech#system-certificates`), come tutti gli altri processi omp della tabella di `requirements/tech#omp` (sezione «Certificate variables»). Le variabili sono lette a ogni avvio: con l'impostazione `System Certificates` spenta, o senza bundle, non se ne aggiunge nessuna; con l'impostazione accesa si usano il percorso del bundle e i nomi di variabile prodotti dalla funzione condivisa costruita dal Task del gruppo `system-certificates` (questo Task non la reimplementa, la usa).

      Quando è fatto:
      - **Job, avvio:** l'ambiente del figlio resta quello di oggi (via le `WEAVER_*` ereditate, `WEAVER_JOB`, `WEAVER_ENDPOINT`, `WEAVER_RUN`/`WEAVER_TASK` per le Run, PATH come in `## omp`) più le variabili dei certificati del momento (`requirements/tech#job`, «Environment»).
      - **Job, ripresa:** al resume omp viene rilanciato con gli stessi argomenti ed env salvati nel registro **più** le variabili dei certificati di quel momento, non di quelle del primo avvio (`requirements/tech#job`, «Resume», passo 2). Se nel frattempo l'impostazione è stata spenta, il processo ripreso non le riceve.
      - **Registro:** `<app local data>/jobs/<job id>.json`, campo `env`, contiene solo le variabili impostate da Weaver e **mai** le variabili dei System Certificates, che sono ricalcolate a ogni avvio e ripresa (`requirements/data#job`). Le variabili vanno quindi aggiunte dopo aver costruito l'env salvato, non dentro di esso.
      - **General:** stesso trattamento: `WEAVER_GENERAL`, `WEAVER_ENDPOINT`, PATH e le variabili dei certificati come in `## omp`, lette a ogni avvio del processo (anche dopo che si è fermato e il messaggio successivo lo riavvia) (`requirements/tech#general-chat`, «Environment»).
      - Un processo già in esecuzione non viene riavviato quando l'impostazione cambia: il nuovo valore vale dal prossimo avvio o ripresa.
      - Test: l'env passato al figlio contiene le variabili con l'impostazione accesa e un bundle presente, non le contiene con l'impostazione spenta; il registro scritto non le contiene mai; il resume usa il valore corrente.

      Sezioni da leggere: `requirements/tech/acp.md` (`## job` «Environment», «Resume registry», «Resume»; `## general-chat` «Environment»), `requirements/data/settings.md` (`## job`), `requirements/tech/omp.md` («Certificate variables»), `requirements/tech/system-certificates.md`.

      Punti di partenza nel codice:
      - `requirements/tech#job` — crates/core/src/jobs/launch.rs:`child_env` (unisce ereditato, env impostato e PATH; punto naturale per un parametro con le variabili dei certificati)
      - `requirements/tech#job` — src-tauri/src/jobs/actor.rs:`prepare` / `launch` (env del registro, ramo di resume)
      - `requirements/data#job` — src-tauri/src/jobs/actor.rs:`prepare` (costruzione dell'`env` salvato nel registro)
      - `requirements/tech#general-chat` — src-tauri/src/general/actor.rs:`launch`
      - `requirements/tech#job` — src-tauri/src/jobs/proc.rs:`OmpProcess::spawn` (usa `env_clear`, quindi le variabili devono arrivare nell'env esplicito)
      - `requirements/tech#job` — src-tauri/src/jobs/host.rs:`Host::settings`, `Host::child_path` (accesso a impostazioni e percorsi; fakes in src-tauri/src/jobs/testing.rs)
  q0i3h62z:
    title: Spiegazioni proxy in job e Chat
    refs:
    - requirements/ui#job
    technical: true
    proposal: 75p82wll
    label: System Certificates
    finished: 2026-11-02T21:23:35.690Z
    body: |-
      Quando un job o il General falliscono per un errore di certificato o un errore HTTP, la ragione mostrata allo sviluppatore è sostituita dalla spiegazione proxy di `requirements/ui#system-certificates`, scritta al momento del fallimento con il valore dell'impostazione `System Certificates` di quel momento (accesa o spenta danno testi diversi). Le funzioni di classificazione (`requirements/rules#system-certificates`) e di spiegazione sono costruite dal Task del gruppo `system-certificates`; questo Task le collega a ogni superficie di job e Chat.

      Quando è fatto:
      - **Testi esaminati:** non solo la riga tenuta come ragione, ma l'intero messaggio d'errore ACP e la coda di stderr del job o del General (ultime 40 righe, al massimo 4 KB). Se il testo è un errore di certificato vince sull'HTTP; un errore HTTP vale solo per i codici `403`, `407`, `500`–`599` nelle forme elencate dalla regola.
      - **Job, riga finale** `Failed: <reason>.` (`requirements/ui#job`, «End line»): la spiegazione sostituisce la ragione per l'uscita (`omp exited with code <N>` + stderr), per i fallimenti di avvio (`START_FAILED`, errore in risposta al primo `session/prompt`), di apertura (spawn, `initialize`, `session/new` / `session/load`, inclusa la ripresa) e di turno (errori di richiesta di `session/prompt`). Le ragioni fisse in parole semplici (`omp did not answer within 30 s`, `the agent refused`, `the agent ran out of tokens`, `the agent reached its turn limit`, `the session could not be resumed`) restano quando non c'è un errore di certificato o HTTP. La spiegazione non ha punto finale: la superficie aggiunge il proprio `.` e nient'altro.
      - **Show details** dopo un'uscita apre sempre le ultime righe d'errore di omp, invariate (monospace); la spiegazione non le sostituisce.
      - **Chat (General):** `omp stopped: <reason>.` e la riga del turno fallito usano la spiegazione allo stesso modo; i dettagli restano le righe di omp.
      - **Messaggi:** `Not delivered: <reason>.` di un messaggio rifiutato (risposta `failed` o errore di `_session/steering` / del `session/prompt` che lo consegna) usa la spiegazione quando il testo dell'errore la richiede.
      - Oggi gli errori ACP non ricevono alcun suggerimento: da cambiare. L'attuale `reason_with_hint`/hint dei certificati in `exit_reason` va sostituito dalle nuove spiegazioni, senza lasciare due convenzioni.
      - La ragione salvata (`reason` nel registro, testo della Chat) è quella già sostituita: cambiare l'impostazione dopo non riscrive fallimenti passati.
      - Test: per ogni superficie, un testo con `UNABLE_TO_GET_ISSUER_CERT_LOCALLY` e uno con `HTTP 407`, con impostazione accesa e spenta, producono la spiegazione attesa; un errore non riconosciuto mantiene la ragione di oggi.

      Sezioni da leggere: `requirements/ui/conversations.md` (`## job` «End line», «Queued message»; `## general-chat` «Failures»), `requirements/ui/system-certificates.md` («Proxy explanations», «Where they replace the reason»), `requirements/rules/system-certificates.md` («Texts tested», «Certificate error», «HTTP error»), `requirements/tech/omp.md` («Failure reasons»), `requirements/tech/acp.md` (`## job` «Opening», «Messages», «Turn end», «Stderr»).

      Punti di partenza nel codice:
      - `requirements/ui#job` — src-tauri/src/jobs/proc.rs:`OmpProcess::exit_reason` (oggi aggiunge l'hint dei certificati)
      - `requirements/ui#job` — src-tauri/src/jobs/session.rs:`OpenError::reason`, `turn_end`, `steering_outcome`
      - `requirements/ui#job` — src-tauri/src/jobs/actor.rs:`fail_opening`, gestione di `TurnEnd` fallito e dell'uscita
      - `requirements/ui#job` — src-tauri/src/general/actor.rs:`fail_opening`, turno fallito, uscita; src-tauri/src/jobs/conversation.rs:`omp_stopped`
      - `requirements/ui#job` — crates/core/src/omp/failure.rs (`CERTIFICATE_MARKERS`, `CERTIFICATE_HINT`/`PROXY_HINT`, `reason_with_hint`: da sostituire con le funzioni del gruppo `system-certificates`)
      - `requirements/ui#job` — ui/src/components/jobs/jobs-view.tsx (`FailedLine`, righe della Chat, `Not delivered`): solo visualizzazione, i testi arrivano già sostituiti
  s269d588:
    title: Certificati nei comandi omp
    refs:
    - requirements/tech#omp
    technical: true
    proposal: 75p82wll
    label: System Certificates
    finished: 2026-11-02T21:22:44.148Z
    body: |-
      Quando il lavoro è finito, ogni comando di omp che Weaver esegue da sé (tabella di `requirements/tech#omp`: `omp --version`, `omp models --json`, `omp update --check`, `omp update`, `omp auth-broker logout <provider>`) riceve le variabili di System Certificates, e ogni motivo di fallimento che è un errore di certificato o HTTP viene sostituito dalla spiegazione del proxy.

      ## Variabili
      - Il runner dei comandi di omp aggiunge all'ambiente del figlio, oltre al PATH, le variabili restituite dalla funzione che dà le variabili dei certificati (costruita dal Task del gruppo `system-certificates`): le nove variabili e `PI_REQUEST_COMPRESSION=0`, solo mentre System Certificates è acceso e un bundle esiste. Sovrascrivono quelle ereditate.
      - Si leggono a ogni avvio del comando (l'impostazione e il bundle del momento), mai una volta sola.
      - Solo per omp: la shell di login (`$SHELL -l -c …`) e `reg.exe` che leggono il PATH non le ricevono. Oggi `SystemExec::run` serve tutti e tre: separare l'ambiente di omp dagli altri usi.
      - I processi di omp dei lavori e di General sono coperti dal Task che tocca `requirements/tech#acp`; qui solo i comandi della tabella, ma usando la stessa funzione.

      ## Motivi
      - La regola del motivo resta (ultima riga non vuota di stderr, altrimenti `exit code N`, `could not start omp`, `timed out`), ma prima Weaver prova l'intero stderr con la funzione che classifica il testo come errore di certificato o HTTP (`requirements/rules#system-certificates`, gruppo `system-certificates`). Se lo è, il motivo diventa la spiegazione restituita dalla funzione dei testi del proxy, scelta con l'impostazione com'è nel momento del fallimento (acceso/spento) e mai riscritta dopo.
      - Le spiegazioni non hanno punto finale: il riquadro di omp in Missing aggiunge `.`; il `(<reason>)` di un aggiornamento fallito in Missing (`too_old_text`) e in Ready, `Could not log out of <name>: <reason>.` e la riga del perché l'elenco modelli non è disponibile in Settings le incorporano così come sono. Testi esatti in `requirements/ui#system-certificates` (sezione "Proxy explanations" e "Where they replace the reason").
      - Gli attuali `CERTIFICATE_MARKERS`, `CERTIFICATE_HINT`, `PROXY_HINT` e `reason_with_hint` vengono sostituiti dalle funzioni nuove (nessun secondo meccanismo); aggiornare i test di `failure.rs`.

      ## Da dove partire
      - `requirements/tech#omp` — crates/core/src/omp/run.rs:`SystemExec::run` (solo PATH oggi), `run_command`
      - `requirements/tech#omp` — src-tauri/src/omp.rs:`SystemOps::environment` (costruisce `SystemExec`), crates/core/src/omp/path_env.rs:`run` (shell di login, da lasciare senza variabili)
      - `requirements/tech#omp` — crates/core/src/omp/failure.rs:`hint`, `reason_with_hint`, `status_text`
      - `requirements/tech#omp` — crates/core/src/omp/probe.rs:`probe_found`, `check_for_update`, `run_update`, `logout`, `too_old_text`
      - `requirements/tech#omp` — src-tauri/src/omp.rs: fallimento update (`reason_with_hint`) e logout
  zsp3xtv8:
    title: Esportazione dei certificati di sistema
    refs:
    - system-certificates
    - desktop
    - desktop.CERTIFICATES_EXPORTED
    - desktop.CERTIFICATES_EXPORT_FAILED
    - requirements/data#system-certificates
    - requirements/data#developer.CHANGE_SETTINGS
    - requirements/rules#system-certificates
    - requirements/tech#system-certificates
    - requirements/quality#system-certificates
    - specs#frameworks
    technical: false
    proposal: 75p82wll
    label: System Certificates
    finished: 2026-11-02T21:03:31.381Z
    body: |-
      Weaver deve poter esportare le autorità di certificazione di cui il sistema si fida in un bundle unico, tenerne lo stato, mandarlo alle finestre e offrire agli altri punti dell'app la funzione delle variabili e la classificazione degli errori di certificato e HTTP. Questo Task costruisce il motore della Machine `system-certificates` (stati `Off`, `On` con `Exporting`, `Ready`, `Failed`); l'avvio (età del bundle, accensione per Zscaler), il timer giornaliero e i banner sono di un altro Task, l'interruttore e la riga di stato nell'interfaccia di un altro ancora, e il collegamento delle variabili e delle spiegazioni ai singoli processi e superfici di omp spetta ai Task delle altre parti della Proposal, che chiameranno le funzioni descritte qui.

      ## Impostazione

      - Nuova chiave `systemCertificates.enabled`, booleana, default `false`: è la prima impostazione booleana. Va aggiunta a `Settings` (campo `system_certificates.enabled` o equivalente con serializzazione `systemCertificates.enabled`), a `SettingKey` e a `SettingKey::ALL`, allo schema con titolo `System Certificates` e la descrizione (≤160 caratteri) `omp trusts the certificates your system trusts, for proxies that inspect HTTPS. Replaces NODE_EXTRA_CA_CERTS, SSL_CERT_FILE and similar variables.`, con un nuovo tipo di campo booleano in `FieldKind`.
      - La lettura del file accetta un valore JSON booleano per questa chiave; un tipo sbagliato vale il default e Settings dice quale chiave è stata ignorata e perché, come per le altre. Solo i valori diversi dal default si scrivono: `false` toglie la chiave. Il tipo TypeScript resta generato (`bindings.rs`, `export_all`), mai scritto a mano. Le altre regole di `requirements/data#developer.CHANGE_SETTINGS` (scrittura atomica e serializzata, errore mostrato, effetto solo sui processi avviati dopo) valgono anche qui.

      ## Esportazione (`requirements/tech#system-certificates`, `requirements/rules#system-certificates`)

      - Un modulo nuovo di `weaver-core` (per esempio `weaver_core::system_certificates`) con funzioni pure più un sottile strato di I/O; il backend dell'app lo esegue, tiene lo stato e lo manda a ogni finestra. L'interfaccia non legge mai il bundle né i certificati.
      - Lettura del sistema con `rustls-native-certs` (`load_native_certs`) su un thread bloccante, mai sul thread IPC o UI; nessun processo esterno, nessuna rete. Errori su singoli certificati si saltano finché almeno uno si carica; se nessuno si carica: `could not read the system certificates: <first error>`. Attesa massima 30 s, poi `the export did not finish within 30 s` e il risultato tardivo del thread si scarta.
      - Radici pubbliche di `webpki-root-certs`, compilate in Weaver, aggiunte dopo quelle del sistema. Nuove dipendenze in Cargo: `rustls-native-certs`, `webpki-root-certs`, `x509-parser` (`specs#frameworks`).
      - Costruzione del bundle: una funzione pura dai certificati DER (prima il sistema, poi le radici pubbliche) al testo PEM e al conteggio. Un certificato entra solo se si legge come X.509 con `x509-parser` e i basic constraints dicono che è una CA; illeggibili e non-CA si saltano senza fallire; certificati con gli stessi byte DER compaiono una volta sola, al primo posto. Se le fonti del sistema non danno nessuna CA l'esportazione fallisce con `no certificate authority found`, anche se le radici pubbliche basterebbero. PEM: `-----BEGIN CERTIFICATE-----`, base64 in righe di 64 caratteri, `-----END CERTIFICATE-----`, LF; nessuna chiave.
      - Scrittura (`requirements/data#system-certificates`): `system-certificates.pem` nella cartella dei dati locali (`AppDirs::local_data_dir`, da aggiungere allo schema in testa a `paths.rs` insieme a `omp-install/`), condiviso da tutte le finestre e i progetti. Atomica: `system-certificates.pem.<pid>.tmp` nella stessa cartella, poi rename; un errore lascia intero il bundle precedente e rimuove il file temporaneo; il messaggio è `could not write the bundle: <message>`. L'ora di modifica è l'ora della scrittura; spegnere l'impostazione lascia il file sul disco, inutilizzato.
      - Un bundle è vecchio quando manca o la sua ora di modifica è più di 24 h prima di adesso: questa regola resta una funzione pura con l'orologio iniettato, usata dal Task dell'avvio.

      ## Stato e comandi

      - Una sola esportazione alla volta in tutta l'app: una richiesta mentre una corre si unisce a quella e non avvia nulla. Un'esportazione che finisce dopo che l'impostazione è stata spenta non cambia lo stato e non emette nulla.
      - Stato in memoria: se sta esportando, conteggio e ora di scrittura del bundle in uso, ultimo errore, e se l'esportazione in corso è automatica (avvio, accensione per Zscaler, giornaliera) o chiesta (`Refresh`, o accensione a mano da Settings). All'avvio conteggio e ora vengono dal bundle sul disco (conteggio riletto dal file). Ogni cambio va a ogni finestra come un evento (per esempio `system-certificates-changed`, accanto a `emit_update_changed` in `events.rs`); una finestra legge lo stato corrente con un comando al caricamento (per esempio `system_certificates_status`). Il tipo dello stato è generato per TypeScript.
      - Transizioni della Machine: in `Off`, `developer.CHANGE_SETTINGS` che accende l'impostazione → `Exporting` (non automatica); in `On`, `developer.CHANGE_SETTINGS` che la spegne → `Off`: i processi avviati dopo non ricevono variabili, un'esportazione in corso viene ignorata alla fine. In `Exporting`, `desktop.CERTIFICATES_EXPORTED` → `Ready` con conteggio e ora; `desktop.CERTIFICATES_EXPORT_FAILED` → `Failed` con il motivo, e il bundle precedente resta in uso se esiste. In `Ready` e `Failed`, `developer.REFRESH_SYSTEM_CERTIFICATES` → `Exporting`: un comando (per esempio `system_certificates_refresh`) che l'interfaccia può chiamare solo a impostazione accesa; il backend lo rifiuta se è spenta. Il Task del timer giornaliero e dell'avvio usa lo stesso punto d'ingresso con il flag "automatica".
      - Alla fine di ogni esportazione, riuscita o no, a impostazione accesa, il backend emette `system-certificates.EXPORT_ENDED` in una forma che il servizio di omp può ascoltare (per esempio un canale `broadcast` come `SettingsService::subscribe`). Il ricontrollo di omp su `EXPORT_ENDED` e sullo spegnimento dell'impostazione (dallo stesso punto d'ingresso di un cambio di `ompPath`, solo negli stati che la Machine `omp-setup` nomina) è del Task della parte `omp-setup` della Proposal.
      - Ogni esportazione si registra a livello `info`: automatica o chiesta, esito, numero di certificati, durata e, se fallisce, il motivo. Mai il contenuto del bundle, i soggetti dei certificati o i valori delle variabili.

      ## Variabili per i processi di omp

      - Una funzione pubblica del modulo core (per esempio `system_certificates::variables(enabled, bundle_path) -> Vec<(name, value)>`), letta a ogni avvio di un processo. Impostazione accesa e file del bundle presente: `NODE_EXTRA_CA_CERTS`, `SSL_CERT_FILE`, `REQUESTS_CA_BUNDLE`, `CURL_CA_BUNDLE`, `GIT_SSL_CAINFO`, `PIP_CERT`, `CARGO_HTTP_CAINFO`, `AWS_CA_BUNDLE`, `DENO_CERT`, ciascuna con il percorso assoluto del bundle, più `PI_REQUEST_COMPRESSION=0`. Impostazione spenta o bundle assente: nulla, l'ambiente resta esattamente com'era senza System Certificates.
      - Si applicano dopo l'ambiente ereditato e ne sovrascrivono i valori omonimi; PATH resta come lo imposta `requirements/tech#omp`. Non si registrano mai con un lavoro. Questo Task scrive solo la funzione (con un helper del backend che legge impostazione e percorso dallo stato dell'app); i comandi di omp, i lavori, General, le riprese e i terminali dell'installer e dei login la chiameranno nei Task delle altre parti della Proposal.
      - Weaver non spegne mai il controllo TLS: nessuna `NODE_TLS_REJECT_UNAUTHORIZED`, `GIT_SSL_NO_VERIFY`, `PYTHONHTTPSVERIFY=0`, `-k`/`--insecure` o `strict-ssl=false`. Nessun file di progetto, variabile d'ambiente o messaggio dell'interfaccia sceglie fonti, percorso del bundle, variabili o processi.

      ## Classificazione degli errori e spiegazioni

      - Funzioni pure in `crates/core/src/omp/failure.rs` che, dato un testo (lo stderr intero di un comando di omp fallito, un messaggio d'errore ACP, la coda dello stderr di un lavoro o di General, non solo la riga tenuta come motivo), dicono se è un errore di certificato o HTTP, e con quale codice.
      - Certificato: uno dei codici, con le maiuscole, `UNABLE_TO_GET_ISSUER_CERT_LOCALLY`, `UNABLE_TO_GET_ISSUER_CERT`, `UNABLE_TO_VERIFY_LEAF_SIGNATURE`, `SELF_SIGNED_CERT_IN_CHAIN`, `DEPTH_ZERO_SELF_SIGNED_CERT`, `CERT_HAS_EXPIRED`, `CERT_UNTRUSTED`, `ERR_TLS_CERT_ALTNAME_INVALID`; o una delle frasi, senza badare alle maiuscole, `unable to get local issuer certificate`, `self signed certificate in certificate chain`, `self-signed certificate in certificate chain`, `certificate verify failed`, `SSL certificate problem`, `curl: (60)`.
      - HTTP, solo se non è un errore di certificato: codici `403`, `407` e da `500` a `599` come numero intero (non parte di un numero più lungo), nelle forme, senza badare alle maiuscole, `HTTP <code>`, `HTTP/<version> <code>`, `status <code>`, `status: <code>`, `status code <code>`, `statusCode: <code>`, `returned error: <code>`, oppure il codice seguito dalla sua frase standard (`403 Forbidden`, `407 Proxy Authentication Required`, `500 Internal Server Error`, `501 Not Implemented`, `502 Bad Gateway`, `503 Service Unavailable`, `504 Gateway Timeout`, `505 HTTP Version Not Supported`, `506 Variant Also Negotiates`, `507 Insufficient Storage`, `508 Loop Detected`, `510 Not Extended`, `511 Network Authentication Required`). Il primo riscontro nel testo dà il codice. Altri codici (`401`, `404`, `429`…), un codice seguito da parole che non sono la sua frase standard e numeri nudi non sono errori HTTP.
      - Una funzione dà la spiegazione in inglese, senza punto finale, secondo il tipo e l'impostazione (testi esatti in `requirements/ui#system-certificates`, "Proxy explanations"): certificato a impostazione spenta `omp could not verify a server's certificate; a corporate proxy that inspects HTTPS traffic is the usual cause, and turning on System Certificates in Weaver's settings may fix it`; certificato a impostazione accesa `omp could not verify a server's certificate, even with System Certificates on; a corporate proxy that inspects HTTPS traffic is the usual cause`; HTTP a impostazione spenta `omp received an HTTP <code> error; a corporate proxy that inspects HTTPS traffic may be blocking the request, and turning on System Certificates in Weaver's settings may fix it`; HTTP a impostazione accesa `omp received an HTTP <code> error; a corporate proxy that inspects HTTPS traffic may be blocking the request`. Nessun testo d'errore nomina un prodotto.
      - Questo Task costruisce solo le funzioni di classificazione e di spiegazione. Sostituire i vecchi suggerimenti di `failure.rs` (`CERTIFICATE_MARKERS`, `CERTIFICATE_HINT`, `PROXY_HINT`, `reason_with_hint`) e collegare le spiegazioni alle superfici (riquadro di omp, aggiornamento, logout, lista dei modelli, lavori, Chat, `Not delivered`), così come il testo dell'installer fallito, sono dei Task delle altre parti della Proposal: la nuova classificazione deve poter assorbire i marcatori attuali, perché non restino due convenzioni.

      ## Test (`requirements/quality#system-certificates`)

      Con `cargo test`: costruzione del bundle (duplicati, non-CA e illeggibili, nessuna CA del sistema, radici pubbliche dopo quelle del sistema, forma PEM); scrittura atomica con un rename che fallisce e lascia il bundle precedente; limite di 30 s con un lettore lento iniettato; una sola esportazione alla volta ed esportazione ignorata dopo lo spegnimento; variabili con impostazione accesa, spenta e senza bundle e loro precedenza sugli ereditati; ogni marcatore di certificato e ogni forma e codice HTTP, altri codici, numeri nudi, precedenza del certificato, spiegazione con impostazione accesa e spenta, e che nessun testo d'errore nomini un prodotto. I test della presenza delle variabili nei singoli punti di avvio, dell'avvio e di Zscaler, e dell'interfaccia stanno nei rispettivi Task.

      ## Dove iniziare

      - `requirements/data#developer.CHANGE_SETTINGS` — crates/core/src/settings/mod.rs: `Settings`, `SettingKey`, `SettingKey::ALL`, `set_json` (manca il caso booleano), `SettingChange` (valore testuale)
      - `requirements/data#developer.CHANGE_SETTINGS` — crates/core/src/settings/schema.rs: `FieldKind`
      - `system-certificates` — src-tauri/src/settings.rs: `SettingsService::subscribe`, `apply`
      - `requirements/tech#system-certificates` — crates/core/src/lib.rs (nuovo modulo), crates/core/src/paths.rs: `AppDirs::local_data_dir` e schema in testa, `weaver_core::atomic`
      - `system-certificates` — src-tauri/src/events.rs: `emit_update_changed` come modello; src-tauri/src/bindings.rs: `export_all`
      - `system-certificates` — src-tauri/src/settings.rs: `SettingsService::subscribe` come modello del canale di `EXPORT_ENDED`
      - `requirements/rules#system-certificates` — crates/core/src/omp/failure.rs: `CERTIFICATE_MARKERS`, `hint`, `has_refusing_status` (regole attuali da superare), test in fondo
---
# System Certificates dietro proxy

System Certificates, spenta di default, esporta il bundle delle autorità di cui il sistema si fida e lo rinnova all'avvio se vecchio, ogni giorno e con Refresh. Un interruttore in Settings e sotto il riquadro di omp mostra lo stato dell'esportazione. Le variabili dei certificati arrivano ai comandi di omp, ai processi di job e General e ai terminali di installer e login. Su macOS e Windows, con Zscaler installato, Weaver la accende una sola volta, lo notifica e lo dice in banner in ogni finestra; un'esportazione automatica fallita mostra un banner di avviso. L'installer di omp ora scarica in un file temporaneo, lascia il proprio codice d'uscita e, se fallisce, riporta omp a Missing con una spiegazione del proxy; i ricontrolli di omp seguono l'esportazione e l'avvio la aspetta al più 30 s. Gli errori di certificato o HTTP di job, Chat e comandi omp sono sostituiti da una spiegazione in parole semplici. README e CHANGELOG descrivono il comportamento.
