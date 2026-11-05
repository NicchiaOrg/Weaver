---
created: 2026-11-04T07:02:37.762Z
updated: 2026-11-04T07:02:37.762Z
summary: 'Le carte fantasma sulla Board diventano uno skeleton: i testi noti restano fermi e solo i segnaposto delle parti mancanti hanno lo shimmer.'
tasks:
- tmyub3zs
cleared:
  tmyub3zs:
    title: Skeleton dei ghost
    refs:
    - work
    - requirements/ui#work
    technical: false
    proposal: z892t9yt
    label: ghost skeleton
    finished: 2026-11-04T06:32:10.402Z
    body: |-
      Il ghost sulla Board (la card che un job ha annunciato e non ha ancora creato) oggi applica lo shimmer al titolo e alla riga di fase, cioè a testi già noti. Quando la Task è finita lo skeleton è usato al contrario: i testi noti sono fermi e brillano solo i segnaposto delle parti che la card reale avrà e che il ghost non conosce ancora. La modifica è solo grafica: posizione del ghost, pulsante unico che apre la conversazione del job, nome accessibile e comportamento all'arrivo della card restano come sono.

      Deve valere:

      - La cornice resta: bordo tratteggiato e opacità 70%.
      - In alto, fermi e senza shimmer: il tipo (`Proposal`, `Task`, `Drift`), il titolo, la riga di fase, poi l'ultima nota del job in muted su una riga troncata con `…` (testo intero come tooltip).
      - La riga di fase (`Preparing`, `Writing`, `Waiting for your answers`, `Being refined`) è testo muted fermo, senza icone né animazione.
      - Sotto i testi noti c'è sempre un blocco di segnaposto (barre e blocchi muted, senza testo), nell'ordine della card reale del tipo e con la sua forma:
        - Proposal: barra corta per l'id, barra per la riga file, barra per la riga Tasks (sempre presente, anche se la Proposal potrebbe non portare Tasks), poi una riga con due blocchi delle dimensioni di un pulsante per i controlli.
        - Task: un piccolo cerchio per il pallino di stato accanto a una barra corta per l'id, poi un blocco delle dimensioni di un pulsante per `Run`.
        - Drift: barra corta per l'id, poi una riga con due blocchi delle dimensioni di un pulsante per `Resolve` e `Dismiss Drift`.
      - Solo i segnaposto hanno lo shimmer: la banda chiara che scorre da destra a sinistra in 2,4 s, quella usata oggi. Con la riduzione del movimento richiesta dal sistema operativo i segnaposto sono barre ferme.
---
# Skeleton dei ghost

Un ghost mostra subito tipo, titolo, riga di fase e ultima nota senza animazione, e sotto di essi segnaposto muted nell'ordine della carta reale per Proposal, Task e Drift. Con il movimento ridotto i segnaposto restano barre ferme.
