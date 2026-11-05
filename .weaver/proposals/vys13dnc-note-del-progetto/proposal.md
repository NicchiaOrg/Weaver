---
status: approved
kind: change
base:
  model/actors/developer.md: 5332edc34b7f167f7c03c9f9f30f93c756376f5538a3224f28b10b05b61dc383
  model/actors/workspace.md: 4a12c68dc36d1402c2fcd535eedcccd7dc8440b477cb7eb2711578a3bfd8eea1
  model/app/feature.md: 15f15b9e4e8691a51276531477d1966874325c625d24140426f86e037b59a0b3
  model/notes/feature.md: new
  model/notes/notes-view.md: new
  model/system.md: 88c591cf47f817afb39b390620fe3a1ad1f9f866bb58b9c373527514724982ca
  requirements/data/project-files.md: 94798da33aea42c11b1125162669821f2f55bc6f5c37ffa5c2744f603c9dc257
  requirements/quality/files.md: 05cf2cf640c296d24d01692f488259db3af37cc3cf1c17a147a359826c3dd8e6
  requirements/rules/files.md: 7a5a795e2ed294c4dda1b1066099e2a73d8f3495e44096701684b215dd2caac9
  requirements/tech/architecture.md: 3a87fb91005dff53597b046f6b06fcd893438a97403488df6390e9c02112582c
  requirements/tech/notes.md: new
  requirements/ui/look.md: 8303302138f93adffd3b2290c736036fa1c1f39da515316cb54a2ed6427bcdcb
  requirements/ui/notes.md: new
  specs.yaml: 72898799f92f82c8700a075d966f371122e4a1d26e6753f2580d3afd9ff023f3
created: 2026-11-04T07:45:26.976Z
request: 2026-11-04T08-30-37
label: Notes
job: pjaoixlp
---
# Note del progetto

Introduce la vista **Notes**: un unico foglio Markdown per progetto, scritto in un editor WYSIWYG alla Notion, con le checklist al primo posto, per le note che lo sviluppatore vuole ricordare sul progetto.

## Decisioni
- Vista a tutta area nella barra laterale, voce `Notes` dopo Assets, icona `NotebookPen`; mostra Get Started finché il progetto non ha `.weaver/`.
- Editor WYSIWYG con TipTap 3 (circa 135 KB compressi, caricato solo all'apertura della vista); il file resta Markdown GFM.
- Solo scorciatoie Markdown durante la scrittura (`# ` diventa un titolo, `[] ` una voce di checklist…): niente menu `/`, barra di formattazione o maniglie.
- Elementi: titoli, paragrafi, elenchi puntati e numerati, checklist, grassetto, corsivo, barrato, codice in linea e a blocchi, citazioni, link (aperti nel browser con `Cmd/Ctrl`+clic), linea divisoria, immagini.
- Una voce spuntata resta al suo posto, barrata e attenuata.
- Salvataggio automatico come l'editor delle richieste (500 ms dopo l'ultima modifica, all'uscita dal campo, al cambio di vista, alla chiusura); una spunta salva subito; stato del salvataggio nell'intestazione.
- Modifica esterna del file: si ricarica se nulla è in attesa, altrimenti vince la versione del foglio e un avviso lo dice.
- Contenuto non supportato (HTML, tabelle, marcatori di conflitto…) resta come testo e viene riscritto invariato.
- Il file non esiste finché non si scrive: foglio vuoto con testo segnaposto; se sparisce il foglio torna vuoto.
- Note e immagini in `.weaver/notes/`, cartella solo locale con `.gitignore` `*`: non versionate, non negli Assets.
- L'agente non vede mai le note.
- Immagini incollate o trascinate dal Finder/Explorer: copiate in `.weaver/notes/`, con il nome originale se c'è, altrimenti data e ora, con un numero se il nome è preso; mostrate a grandezza naturale, al massimo larghe quanto la colonna.
- Immagini oltre 10 MB rifiutate; file che non sono immagini ignorati.
- Un'immagine tolta dal foglio va nel cestino all'uscita dalla vista o alla chiusura della finestra, se il foglio non la usa più; le immagini mai tolte nell'editor restano, anche se inutilizzate.
- Immagine non trovata: segnaposto con nome o indirizzo; nessuna immagine letta fuori da `.weaver/notes/` né scaricata dal web.
- Foglio in una colonna centrata larga al massimo 760 px.
- Il testo delle note e le immagini non compaiono mai nei log.

## Assunzioni dell'agente da verificare
- Formati di immagine accettati: PNG, JPEG, GIF, WebP, come l'editor delle richieste; un'immagine di altro tipo (per esempio HEIC) viene rifiutata con il motivo.
- `Cmd/Ctrl+Enter` spunta la voce dove sta il cursore, per l'uso da tastiera; Tab e Shift+Tab annidano le voci di elenco.
- Un indirizzo `http`/`https` scritto o incollato diventa un link.
- Il nome di un'immagine senza nome è `pasted-AAAA-MM-GG-HHMMSS.<ext>`; con nome già preso si aggiunge `-2`, `-3`… prima dell'estensione.
- I titoli del foglio sono più grandi del titolo della vista: la regola sui titoli del Markdown reso non vale per l'editor delle Note.
