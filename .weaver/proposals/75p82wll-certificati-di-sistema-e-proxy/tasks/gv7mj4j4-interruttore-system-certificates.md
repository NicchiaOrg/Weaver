---
technical: false
refs:
  - developer.REFRESH_SYSTEM_CERTIFICATES
  - requirements/ui#system-certificates
  - requirements/ui#developer.CHANGE_SETTINGS
---
# Interruttore System Certificates

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
