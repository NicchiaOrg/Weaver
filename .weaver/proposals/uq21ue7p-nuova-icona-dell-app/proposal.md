---
status: approved
kind: change
base:
  requirements/release/pipeline.md: 9d7f31f3964ea251784ec60e2cbd4a742300b9f927de5463c25a9b1c7404d593
created: 2026-11-04T07:33:38.260Z
request: 2026-11-04T08-29-36
label: Icona app
job: cg2kknji
---
# Nuova icona dell'app

L'icona dell'app passa dalla W bianca su quadrato blu al nuovo logo: una W intrecciata di nastri pastello (rosa salmone, lavanda, pesca, verde salvia) su un quadrato arrotondato color crema.

Decisioni:
- Il logo diventa solo l'icona dell'app: pacchetti di installazione, Dock su macOS, barra delle applicazioni, icona nella tray su Windows e Linux, notifiche di sistema. La finestra di benvenuto resta con l'icona `Blocks` di lucide.
- Il margine bianco opaco attorno al quadrato arrotondato diventa trasparente e la forma occupa il riquadro come l'icona attuale (896 px su 1024, margine trasparente di 64 px per lato).
- Le misure piccole (16–32 px) si ottengono per riduzione diretta, senza una variante semplificata.
- L'originale diventa un PNG preparato nel codice dell'app; l'originale vettoriale precedente viene eliminato.
- L'anteprima del trascinamento di un asset fuori da Weaver segue la nuova icona, come oggi.
- Logo.png viene allegato al Task che rigenera l'icona.
