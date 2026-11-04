---
status: approved
kind: change
base:
  model/files/feature.md: f095feeaf80001f8b958bdb0c839c74736800c35f18b7c2ac2b06282b9a0fc4d
  model/files/file-tab.md: e2077c13f1b17e8bde51302fb358e42ab9b2cbf17ae8ec88ab1205628c0c737a
  requirements/quality/files.md: 601c3fe115ce46e3cec8f20f6e276e2812a947d294259b878595e3123cacd882
  requirements/rules/files.md: 2b67a619990a23d288a773774a4b52884c01d8d27022b8c56281eaf59b08b907
  requirements/tech/files.md: 3fcfaa92e8659b7059c3075a01b536976bc2531896ecef91b15279947db2ff49
  requirements/ui/files.md: 5245098d6aafc1c009dddcfce8fc154740c4c67eeed125592aed89ac8205980b
created: 2026-11-04T06:02:28.955Z
request: 2026-11-04T06-56-45
label: Image preview
job: 9j2nelvv
---
# Anteprima immagini in Files

Nella vista Files un file binario oggi mostra solo il pannello `Binary file`. Con questa Proposal le immagini binarie si vedono nella scheda e i file SVG, che restano testo modificabile, hanno un'anteprima.

## Decisioni

- Formati mostrati: PNG, JPEG, GIF, WebP, BMP e ICO, riconosciuti dai primi byte del file, mai dall'estensione.
- Limite: un'immagine oltre 10 MB non viene mostrata; la scheda mostra il pannello binario con una riga che lo dice.
- Un'immagine aperta che cambia sul disco viene riletta e mostrata di nuovo da sola.
- Un'immagine cancellata dal disco resta visibile con una barra che lo dice; se il file ricompare viene riletto.
- Agente al lavoro su un'immagine: nessuna indicazione, come per i file binari; contano solo i cambiamenti sul disco.
- Immagine che la webview non riesce a disegnare: pannello `Binary file` con `Open in editor` e una riga che dice che l'immagine non si può mostrare.
- Visualizzazione: centrata, rimpicciolita per stare nel pannello e mai ingrandita; un clic alterna con la dimensione reale, con scorrimento; sfondo a scacchi dietro le trasparenze.
- Barra in alto per le immagini: percorso, larghezza × altezza in pixel, peso del file e `Open in editor`.
- I byte arrivano all'interfaccia come `data:` URL nel risultato di apertura e di rilettura, dalla stessa lettura del file; la CSP non cambia.
- SVG: resta testo modificabile, con l'interruttore `Code` / `Preview` nella barra in alto; si apre sull'anteprima e la scelta vale per la scheda finché resta aperta.
- L'anteprima SVG mostra il testo della scheda, anche con modifiche non salvate.
- Un file è SVG quando il nome finisce con `.svg`, maiuscole o minuscole.
- L'anteprima SVG è disponibile in ogni stato in cui la scheda mostra il testo (sola lettura, agente al lavoro, file cancellato).
- L'anteprima SVG usa lo stesso visualizzatore delle immagini, con dimensioni e peso.

## Assunzioni da verificare

- Una scheda binaria il cui file cambia sul disco e diventa un'immagine mostrabile passa a mostrarla, e un'immagine che diventa non mostrabile torna al pannello binario: conseguenza dell'aggiornamento automatico.
- Il peso nell'anteprima SVG è quello del testo della scheda in UTF-8; i pesi sono in unità binarie (1 KB = 1.024 byte).
- Il clic che alterna la dimensione reale funziona anche da tastiera (Invio e Spazio); il testo alternativo dell'immagine è il nome del file.
- L'anteprima SVG è disegnata come immagine da un `data:` URL, così script e risorse esterne dell'SVG non vengono mai eseguiti né caricati.
