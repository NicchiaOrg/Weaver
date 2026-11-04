---
status: approved
kind: change
base:
  model/actors/developer.md: 751069e137e7ee8e704d9ebb814d1b12049d843fca303155a1a51e1dfbc22038
  model/actors/workspace.md: 3d616878a2b8e077ba1053816031e4a1dd0845bdb190c3d77a4476b1dadaad78
  model/assets/asset-view.md: new
  model/assets/feature.md: new
  model/model-changes/design-request.md: 63587f063004f47c66c77caeee62045f87235a16959277e03be62e289e9e0a58
  model/system.md: 24bae10975482cb99696191c38d432032e6c7559bc87a14187e7ea7d5d60677b
  requirements/data/project-files.md: 357a1c0e3010b6aa6a5c6b6a9e4ff9276ed05edf3e3db1a6e31fe978662f05ab
  requirements/quality/files.md: ccbf016039615fd608dec31cc8c9b8b66ebefa516c6ddcef61cf6bc72f864863
  requirements/rules/files.md: 0bc8f1c6d7792ffb7256e491033419a03631da4a37e9e508384c6b07fb707b60
  requirements/rules/instructions.md: 0a4b418683894de20fbcfd9f65245eaec0fd702b8a487eec80828a45ffc7bb0a
  requirements/tech/architecture.md: 53b92815ef52fe11e2ceb1d8e7f55352bc7cb72d7207203052b43f5e3871ea91
  requirements/tech/assets.md: new
  requirements/ui/assets.md: new
  requirements/ui/look.md: ea46c74f03a91972b653f399e0f3b3b5f478c9e9c8865d01871b30d7c6239634
  specs.yaml: 52e73d6a2023fd15d187ae769d5643a65dc8896d9a342b8b4a8d72c3cfaba8ec
created: 2026-11-04T06:25:21.930Z
request: 2026-11-04T07-09-44
label: Assets
job: mj3brbfq
---
# Vista Assets

Lo sviluppatore vuole tenere nel progetto, in una cartella tracciata da git, file per l'agente (immagini, documenti), gestirli da una vista semplice simile al Finder o a Esplora risorse e citarli con `@NOME` nei messaggi e nelle richieste, così che l'agente li legga senza che si debba scrivere il percorso.

## Decisioni

- Nuova voce `Assets` nella barra laterale, dopo Files, icona `Paperclip`.
- Cartella `.weaver/assets` piatta, solo file, tracciata da git, creata quando entra il primo asset.
- Trascinamento dal Finder o da Esplora risorse dentro Assets: copia, gli originali restano dove sono.
- Trascinamento da Assets verso altre app: solo copia, l'asset resta in Assets; con più asset selezionati si trascinano tutti i selezionati.
- Un nome già presente rifiuta l'intera caduta, come nella vista Files.
- Un file oltre 10 MB entra comunque: toast di avviso alla copia e segno ambra fisso sugli asset oltre la soglia, anche quelli arrivati da git o dall'agente.
- Cancellazione nel cestino di sistema, con conferma; selezione multipla (Cmd/Ctrl+clic, Shift+clic) per cancellare; la rinomina agisce su un asset.
- Il doppio clic non fa nulla; nessuna anteprima.
- Nomi: le regole della vista Files, e in più rifiutati i nomi che iniziano con il punto.
- Nella vista Files `.weaver/assets` resta in sola lettura.
- La vista Assets si aggiorna da sola mentre è mostrata.
- Griglia di icone; icone lucide scelte dall'estensione del file.
- Tag `@` nella Chat, nelle risposte ai job, in Ask more e nell'editor delle richieste aperto da `+ Work Item`, con un elenco di suggerimenti quando si digita `@`.
- Un tag è il nome di asset esistente più lungo che segue `@`, spazi compresi; un `@NOME` che non corrisponde a nessun asset resta testo normale.
- I tag riconosciuti sono in testo blu nei campi di testo mentre si scrive.
- All'invio, per ogni asset taggato, l'agente riceve la riga "Read the file `.weaver/assets/NOME`."; la conversazione mostra solo il testo scritto dallo sviluppatore, anche dopo una riapertura.
- In una richiesta, le stesse righe vanno nell'istruzione del job di design o di setup; il file della richiesta resta com'è scritto.

## Assunzioni da verificare

- Weaver legge di `.weaver/assets` solo nomi e dimensioni, mai il contenuto, tranne quando copia un file dentro o fuori.
- Una caduta che contiene una cartella o un collegamento simbolico è rifiutata per intero; un file preso da un'altra parte del progetto si può copiare in Assets; asset trascinati di nuovo sulla vista Assets non fanno nulla.
- Le altre regole della caduta sono quelle di Files: tutto o niente, una sola copia per finestra alla volta.
- Cancellando più asset, quelli che il cestino rifiuta restano e un toast d'errore dice quanti e perché.
- Gli asset sono ordinati per nome; lasciare la vista azzera la selezione; il nome completo e la dimensione sono nel tooltip della casella.
- Il tag confronta il nome esattamente, maiuscole comprese; dopo il nome vengono fine testo, uno spazio o una punteggiatura; ogni asset compare una volta sola nelle righe aggiunte, nell'ordine del primo tag.
- Le righe aggiunte sono in inglese, come ogni istruzione che Weaver dà a omp, sotto un'intestazione che le dichiara scritte da Weaver.
- I cambiamenti in `.weaver/assets` non ricostruiscono il quadro del progetto: aggiornano solo l'elenco degli asset.
- Il trascinamento verso altre app usa la libreria `drag` di CrabNebula in modalità copia.
- L'elenco dei suggerimenti mostra fino a 8 asset, prima quelli il cui nome inizia con il testo digitato.
