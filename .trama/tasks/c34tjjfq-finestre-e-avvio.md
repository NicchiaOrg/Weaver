---
column: done
technical: false
refs:
  - app
  - window
  - desktop
  - requirements/tech#window
  - requirements/data#window
  - requirements/ui#window
drift: null
proposal: qmqh1o3p
agent: false
created: 2026-11-02T07:29:32.875Z
finished: 2026-11-02T09:33:55.718Z
---
# Finestre e avvio

Dipende da "Scheletro del progetto", "Shell e aspetto dell'app" e "Impostazioni" (editor scelto); la sospensione dei job è in "Lavori dell'agente".

Weaver è un solo processo per tutti i progetti aperti, con una finestra per progetto. Senza finestre di progetto mostra la finestra di benvenuto. Per scelta del developer, all'avvio Weaver riapre i progetti dell'ultima sessione; se non ce ne sono, mostra il benvenuto con i progetti recenti e `Open Folder`.

## Machine `window`
- **Closed** (iniziale): `developer.OPEN_PROJECT` → Open; `desktop.APP_STARTED` → Open solo per i progetti la cui finestra era aperta all'ultima uscita, e i loro job che lavoravano riprendono.
- **Open**: `developer.OPEN_PROJECT` porta la finestra in primo piano (una cartella già aperta non apre una seconda finestra); `notification.CLICKED` la porta in primo piano mostrando ciò di cui parlava la notifica; `developer.CLOSE_WINDOW` → Closed, job attivi sospesi, il progetto non si riapre al prossimo avvio; `developer.QUIT_APP` → Closed, job sospesi, il progetto si riapre al prossimo avvio. Emette `OPENED` all'apertura e `CLOSED` alla chiusura, anche per uscita da Weaver.
- Chiusura della finestra con job attivi e uscita con job attivi in qualunque progetto chiedono conferma (eventi `developer.CLOSE_WINDOW`, `developer.QUIT_APP`).

## Requisiti
- `requirements/tech#window`: `tauri-plugin-single-instance` registrato per primo; un secondo avvio passa argomenti e cwd all'istanza in corso ed esce; una cartella come argomento è `developer.OPEN_PROJECT`, altrimenti viene avanti la finestra usata più di recente. Identità del progetto: percorso canonico (`dunce`), senza distinzione di maiuscole su Windows. Etichette `p-<project hash>` e `welcome`. `tauri-plugin-window-state` salva dimensione, posizione, massimizzato e fullscreen per etichetta, mai la visibilità; posizione su monitor assente → centrata sul primario; default 1280×800, minimo 960×600. `CloseRequested` impedito con job attivi del progetto, `ExitRequested` con job attivi in qualunque progetto; conferma nativa con `tauri-plugin-dialog` che dice quanti job lavorano (funziona anche con webview bloccata); dopo la conferma i job sono sospesi, poi la finestra si chiude o l'app esce. Chiudere l'ultima finestra di progetto apre il benvenuto; chiudere il benvenuto esce su Windows e Linux, su macOS l'app resta nel Dock e `RunEvent::Reopen` mostra il benvenuto. `Cmd+Q` su macOS, `Ctrl+Q` e File › Quit altrove. Lista di ripristino scritta solo all'uscita. Spegnimento senza conferma possibile: job sospesi senza chiedere; dopo un crash il registro di ripresa li riporta Suspended.
- `requirements/data#window`: `state.json` nella cartella dati locale con `recent` (fino a 10, più recente prima, uno per percorso canonico, esce solo se scalzato) e `lastSession` (percorsi delle finestre aperte all'uscita, in ordine di apertura; chiudere una finestra la toglie, uscire no).
- `requirements/ui#window`: benvenuto 640 × 480 non ridimensionabile con nome e versione, recenti (nome cartella, percorso in muted, `Not found` rimovibile), `Open Folder` col selettore di sistema e riga di stato di omp. Il benvenuto si chiude quando una finestra di progetto è aperta. Titolo della finestra: nome della cartella del progetto.
- Editor (`app`): ovunque siano noti file e riga, `Open in editor` li apre nell'editor delle impostazioni (preset VS Code, Cursor, Zed, JetBrains, Sublime Text o comando personalizzato con `{file}` e `{line}`, eseguito senza shell); un errore mostra un avviso col motivo.

## Fatto quando
- Test: secondo avvio con cartella apre o porta avanti la finestra giusta, mai due finestre per un progetto.
- Avvio dopo un'uscita con due progetti aperti li riapre e riprende i loro job; senza ultima sessione compare il benvenuto con i recenti.
- Chiusura e uscita con job attivi chiedono la conferma nativa col numero di job; annullare non cambia nulla.
- Stato della finestra ripristinato per progetto; il test end-to-end apre benvenuto, progetto, chiude ed esce senza errori.
- `Open in editor` lancia il preset scelto con file e riga corretti.
