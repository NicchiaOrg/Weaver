---
status: approved
kind: change
base:
  model/actors/agent.md: 730e22bbcb12ef490c9ebaa4771cdb9179b4dcb0f7ec17f65a1b4de023762505
  model/actors/developer.md: 6ed234e631e7e390d6735687862cf763e0b08651d19e6c6cae690bcc06f70d1e
  model/actors/omp.md: d8294c6ee5340b1ca7288d2fdb085fd5fa97b21ae5c850a529d8426293a74c5e
  model/actors/time.md: e380304f9c5bd3aef528aae0a31a760c19c32ab2cb85f77cb994e87725c5d57a
  model/actors/workspace.md: 945b77096177d50977074a5c16d0de2865e98bdbec59c12a05c4e52d880e69d6
  model/app/feature.md: e918c58987bb89c05e11109bff57880a826370d92183bd5d38cd63f3dc6d8454
  model/app/window.md: f79243aab9d174fda468e19fc1387b36bbab85b186f72bc247fd3566ff82a138
  model/files/feature.md: new
  model/files/file-lock.md: new
  model/files/file-tab.md: new
  model/files/file-view.md: new
  model/system.md: 21a864391e87209ac92a28b676d63c06a925a6b1f5e56d802235a116510c9597
  requirements/quality/files.md: new
  requirements/rules/files.md: new
  requirements/tech/architecture.md: 9472cf3e82d953d09786b9b359656b27371c1963ea028b52a8259bcb7b2bd28b
  requirements/tech/files.md: new
  requirements/tech/windows.md: new
  requirements/ui/files.md: new
  requirements/ui/look.md: 8f09fe6db3fd71f04d29ad1b3fdccd38a488845c78fe6c915b108adaf2c26482
  specs.yaml: 91ffb93f6b29641ba9afb619056aff49582d3684be6c0c2ee5f6218cf5de2b1c
created: 2026-11-03T07:58:02.872Z
reworks: zm5opqbb
label: Vista File
job: c3ymh1p0
---
# Vista File

Aggiunge la vista File: un albero dei file del progetto e un editor CodeMirror 6 a schede, con i colori della sintassi, per piccole modifiche accanto all'editor dello sviluppatore. Weaver legge e osserva solo ciò che la vista mostra, e rende di sola lettura un file aperto mentre l'agente lo modifica.

Rifà la Proposal zm5opqbb, rifiutata senza riscontro, sul Model attuale: le decisioni sono le stesse. Nel frattempo il Model ha cambiato `developer.ASK_MORE` (Ask more sui Requirements nel loro insieme, dalla vista Requirements): resta com'è, la vista File non lo tocca.

## Decisioni

- Il principio del System diventa: Weaver non analizza mai il codice; legge e scrive un file solo quando lo sviluppatore lo apre o lo cambia nella vista File.
- Il blocco è prudente: uno strumento di modifica dell'agente senza `locations` all'avvio (modalità `hashline` di omp) blocca ogni file aperto finché finisce; con `locations` blocca solo quei file.
- Contano gli strumenti di ogni conversazione del progetto: job, General e Ask more; mai sub-agenti, comandi o altri programmi.
- L'albero segna ogni file aperto in una scheda e bloccato, anche se la sua scheda non è in vista.
- L'editor apre i file in schede, al massimo 10: un clic apre sempre una nuova scheda (o mostra quella già aperta); all'undicesima si chiude la scheda senza modifiche usata meno di recente; se tutte hanno modifiche non salvate, il file non si apre e un avviso lo dice.
- Passare a un'altra vista chiude la vista File e tutte le schede: con modifiche non salvate un solo dialogo chiede Save all / Discard all / Cancel; dopo 10 s si liberano osservazioni ed elenchi; tornando prima, l'albero è com'era.
- Chiudere la finestra o uscire con modifiche non salvate mostra un dialogo nativo del backend Save all / Discard all / Cancel, unito a quello dei job.
- Se con Save all un file non si salva, gli altri si salvano e la vista (o la finestra) resta aperta sulla scheda di quel file, che spiega perché.
- `.weaver/` si vede ma è di sola lettura; si nasconde solo `.git`; le regole di `.gitignore` si applicano con `git check-ignore`, come l'albero di Import from Code.
- La vista File esiste solo nei progetti con `.weaver/`, come le altre viste; la sua voce `Files` sta nella barra laterale dopo Jobs.
- Resta l'osservazione della radice del progetto che serve a vedere comparire o sparire `.weaver/`.
- Crea, rinomina, sposta ed elimina da un menu contestuale (clic destro, tasto menu, Shift+F10), con F2, Canc, trascinamento e `Move to…`; l'eliminazione va nel cestino del sistema; un nome già esistente è rifiutato; un file bloccato dall'agente non si rinomina, sposta o elimina.
- Rinominare o spostare un file aperto (o la sua cartella) fa seguire la scheda; eliminarlo chiude la scheda, dopo una conferma che avvisa delle modifiche non salvate.
- Un file con un byte nullo nei primi 8 KB o non UTF-8 valido è binario e non si apre.
- Il salvataggio usa la terminazione di riga più usata nel file (a pari merito LF; un file nuovo LF), conserva BOM, a capo finale e permessi, e scrive nel bersaglio di un link simbolico dentro il progetto.
- Un file sparito dal disco lascia la scheda aperta con una barra; i file modificabili offrono di salvarlo di nuovo; se ricompare, la scheda lo rilegge senza modifiche non salvate, altrimenti avvisa.
- Lo sblocco con modifiche non salvate, il file cambiato su disco e il file eliminato si mostrano in una barra nella scheda; il conflitto al salvataggio in un dialogo.
- Editor con numeri di riga, annulla e ripeti, cerca e sostituisci, parentesi corrispondenti, piegatura e più cursori; linguaggi da `@codemirror/language-data`, caricati al primo file del tipo.
- Nei dialoghi a tre scelte Cancel ha il focus iniziale e le scelte che perdono testo sono rosse.
- Con la vista File in primo piano Cmd/Ctrl+W chiude la scheda attiva e Ctrl+Tab / Ctrl+Shift+Tab cambiano scheda.

## Assunzioni da verificare

- Mentre la vista si sta liberando (i 10 s), una cartella osservata che cambia si rilegge, così l'albero è giusto se la vista si riapre.
- Un salvataggio che fallisce in scrittura lascia la scheda con le modifiche e mostra un errore con il motivo.
- Se il cestino del sistema non è disponibile, l'eliminazione è rifiutata con il motivo: non c'è eliminazione definitiva.
- Creare e rinominare avvengono con un campo di testo nella riga dell'albero (Invio conferma, Esc annulla); `Move to…` sceglie la cartella in un dialogo che legge le cartelle una alla volta.
- `Open in editor` nella barra della scheda apre il file alla riga del cursore.
- Gli strumenti dell'agente in corso si tengono in memoria anche a vista chiusa (senza letture dal disco), così un file aperto durante uno strumento si apre già bloccato.
- I frammenti dei Requirements: la nuova vista va nei frammenti `files` di `ui`, `tech`, `rules` e `quality`; `## window` e `## notification` passano senza cambiare testo (salvo la conferma di chiusura) dal frammento `architecture` al nuovo frammento `windows`, perché `architecture` supererebbe 12 KB.
- I testi dell'interfaccia (messaggi, etichette dei pulsanti, barre, errori), il campo di testo nella riga per creare e rinominare, `Cmd+Backspace` su macOS per eliminare e le regole sui nomi non validi (caratteri vietati, nomi riservati di Windows) sono scelti dall'agente.
- Rinominare e spostare non sostituiscono mai un elemento esistente; uno spostamento tra volumi diversi è rifiutato.
- Uno strumento dell'agente smette di contare anche quando General viene svuotato o una conversazione Ask more si chiude, perché in quei casi omp viene fermato senza un'uscita segnalata.

## Already present

- `workspace.FILES_CHANGED` — `src-tauri/src/windows/watch.rs:route` (i percorsi del codice sono già `Route::Ignored`)
- `project/Empty Model/workspace.FILES_CHANGED` — `src-tauri/src/windows/watch.rs:route`, `src-tauri/src/pictures.rs:Pictures`
- `project/Has a Model/workspace.FILES_CHANGED` — `src-tauri/src/windows/watch.rs:route`, `src-tauri/src/pictures.rs:Pictures`
- `requirements/tech#notification` — `src-tauri/src/notifications.rs` (sezione solo spostata nel frammento `windows`, testo invariato)
