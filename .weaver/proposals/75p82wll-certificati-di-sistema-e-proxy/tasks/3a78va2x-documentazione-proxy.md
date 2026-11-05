---
technical: false
refs:
  - project-setup
  - requirements/docs#General
  - requirements/docs#system-certificates
---
# Documentazione proxy e CHANGELOG

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
