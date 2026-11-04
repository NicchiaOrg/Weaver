---
status: approved
kind: change
base:
  model/actors/developer.md: 808827eece1d9836cdf4ca70e5633861796e4f20a46d6b4514bdd70252df61e1
  model/files/feature.md: 083c52adc7f654bdabaa69eaf7ed7559b748ad6e54e06abd6dc2cee004f77f9c
  model/files/file-view.md: e6b862346a0d35fe485a004a0c6a44707c759b1e70d03a9208449425a8f02bf3
  requirements/quality/files.md: b660cd60a2c1d575a99fcc264d757e5ff0c36c3cb545713e486c20274e0912a2
  requirements/rules/files.md: 96db18a2aa3d4dc6072ae560860d9eb82a398c3e7608814eb46e01be8d2e3a3d
  requirements/tech/files.md: f55070eae49af2a3233afd2dbb8546cc059b4c07dbe93d525eaabde42947844a
  requirements/ui/files.md: a2c199d017161d5dac1585fca85c5204f53cd286ab88d1c6a554113c91553d27
created: 2026-11-03T19:44:57.973Z
request: 2026-11-03T20-28-29
label: Files drag and drop
job: lpzsuhva
---
# Drag and drop nei File

Nella vista Files lo spostamento di righe trascinandole nel tree, già previsto dal Model, non funziona nell'app: la finestra lascia attivo il gestore drag and drop nativo di Tauri, che impedisce alla pagina di ricevere gli eventi HTML5 `dragover` e `drop` usati dal tree. I test Vitest non se ne accorgono perché simulano quegli eventi in jsdom. La Proposal ripara il trascinamento nel tree e aggiunge il drop di file e cartelle dal Finder/Explorer.

## Decisioni

- Il drag and drop in Files copre lo spostamento di righe nel tree e l'importazione dal Finder/Explorer; trascinare sulle tab per aprire resta escluso.
- Il tree trascina con pointer events propri, non con il drag and drop HTML5; il gestore nativo di Tauri resta attivo, così l'editor delle richieste non cambia e fornisce i percorsi del drop dal Finder.
- Test: Vitest con gli stessi eventi che l'app riceve (pointer events, eventi drag-drop di Tauri simulati) e un test sulla creazione delle finestre che verifica che il gestore nativo resti attivo.
- Il drop dal Finder copia, non sposta: gli originali restano dove sono.
- Si possono trascinare file e cartelle; le cartelle entrano con tutto il contenuto, i link simbolici restano link senza seguire la destinazione.
- Un elemento trascinato dal Finder che si trova già dentro il progetto è rifiutato con il motivo: per quello si usa il tree.
- Se un elemento qualsiasi del drop è rifiutato (nome già presente, nome non valido, destinazione in `.weaver/`, già nel progetto), tutto il drop è rifiutato e nulla cambia.
- Se la copia si interrompe a metà, Weaver toglie quanto quel drop aveva scritto e il progetto torna com'era.
- Durante la copia un toast di avanzamento resta finché finisce, senza annullamento; la vista resta usabile.
- Dopo un drop riuscito la cartella di destinazione si espande, se chiusa, e il primo elemento copiato viene selezionato.
- Aiuti del trascinamento: Esc annulla, una cartella chiusa su cui il puntatore resta circa un secondo si espande, vicino ai bordi il tree scorre.
- Lasciare la vista Files o chiudere la finestra durante una copia non la ferma: prosegue fino alla fine; cartella espansa e selezione solo se la vista è ancora mostrata.
- Un nuovo drop dal Finder mentre un'altra copia della stessa finestra è in corso è rifiutato con il motivo.

## Assunzioni dell'agente

- Il trascinamento di una riga parte dopo 4 px di movimento del puntatore; la cartella sotto il puntatore si espande dopo 1 s; lo scorrimento automatico parte entro 24 px dai bordi del tree.
- Un drop dal Finder fuori dal tree (tab, editor, altre viste) non fa nulla nella vista Files; l'editor delle richieste continua a ricevere i propri drop.
- Due elementi trascinati con lo stesso nome rifiutano il drop, come un nome già presente nella cartella di destinazione.
- La copia di cartelle non salta nulla di ciò che contengono, nemmeno quanto git ignora.
- I testi dell'interfaccia restano in inglese, come il resto di Weaver.
