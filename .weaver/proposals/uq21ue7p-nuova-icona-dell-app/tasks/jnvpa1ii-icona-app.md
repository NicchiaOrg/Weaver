---
technical: true
refs:
  - requirements/release#General
  - requirements/docs#General
---
# Rigenerare l'icona dell'app

L'icona dell'app passa dalla W bianca su quadrato blu al nuovo logo allegato a questo Task: una W intrecciata di nastri pastello (rosa salmone, lavanda, pesca, verde salvia) su un quadrato arrotondato color crema. L'immagine allegata è 1254×1254 px, senza canale alfa: attorno al quadrato arrotondato c'è un bordo bianco opaco di circa il 5% per lato.

A lavoro finito vale quanto segue (sezione `### App icon` di `requirements/release#General`):

- `src-tauri/icons/source.png` è il nuovo originale: 1024×1024 px con canale alfa, il quadrato arrotondato del logo ridimensionato a 896×896 px e centrato, tutto il resto completamente trasparente (64 px per lato). Il bianco attorno alla forma è stato rimosso, angoli arrotondati compresi, senza aloni chiari sul bordo.
- L'originale vettoriale precedente `src-tauri/icons/source.svg` è eliminato; non resta nessun altro originale.
- `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`, `icon.ico` (l'elenco `bundle.icon` di `src-tauri/tauri.conf.json`, che non cambia) e `src-tauri/icons/icon.png` sono rigenerati dall'originale per semplice riduzione (`tauri icon`), senza un disegno a parte per le misure piccole. I file che `tauri icon` produce e l'app non usa (Windows Store, Android, iOS) non restano nel repository.
- Pacchetti di installazione, Dock su macOS, barra delle applicazioni, icona nella tray su Windows e Linux, notifiche di sistema e anteprima del trascinamento di un asset fuori da Weaver mostrano tutti il nuovo logo, perché leggono questi file; nessuno di essi ha un'immagine propria.
- La finestra di benvenuto resta con l'icona `Blocks` di lucide accanto al nome: il logo non compare dentro l'interfaccia.
- `CHANGELOG.md` ha sotto `## [Unreleased]`, gruppo `Changed`, una riga che dice in una frase che Weaver ha una nuova icona (`requirements/docs#General`).
