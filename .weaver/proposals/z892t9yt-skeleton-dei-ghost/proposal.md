---
status: approved
kind: change
base:
  requirements/ui/board.md: d6bc1330fd9adfd0a5eeddbb6d0de43ce0b351c156e861fc0115de5ed2bd332d
created: 2026-11-04T06:17:57.100Z
request: 2026-11-04T07-13-41
label: ghost skeleton
job: 48pou82k
---
# Skeleton dei ghost

Oggi il ghost sulla Board applica lo shimmer al titolo e alla riga di fase, testi già noti: lo skeleton viene usato al contrario. La modifica è solo grafica: i testi noti restano fermi e brillano solo i segnaposto delle parti che la card reale avrà e che il ghost non conosce ancora.

Decisioni:
- Lo shimmer sta solo sui segnaposto delle parti ignote; tipo, titolo, fase e nota sono testo fermo.
- C'è sempre un blocco che brilla sotto i testi noti.
- I segnaposto riproducono la forma della card reale del tipo (Proposal, Task, Drift).
- Tutti i segnaposto stanno sotto i testi noti, in un blocco, nell'ordine della card reale.
- Il ghost di una Proposal mostra sempre sia la riga file sia la riga Tasks.
- Cornice invariata: bordo tratteggiato e opacità 70%.
- Animazione: la banda che scorre di oggi, sulle barre grigie; ferma con movimento ridotto.
- Riga di fase: testo fermo in muted, senza icone né animazione.
