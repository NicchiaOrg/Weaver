---
technical: false
refs: ["design-request", "requirements/ui#design-request"]
---

# Avvisi nell'editor richieste

Nell'editor di una richiesta scritta (`design-request`) i due avvisi sui cambiamenti esterni al file diventano toast di avviso (warning) invece di toast semplici (info):

- Quando il file della richiesta cambia su disco mentre ci sono modifiche in sospeso, Weaver scrive la propria versione e mostra il toast di avviso `The request changed on disk; Weaver kept your version.` Senza modifiche in sospeso l'editor si ricarica, come oggi, senza toast.
- Quando una richiesta in scrittura viene cancellata fuori da Weaver, l'editor si chiude e mostra il toast di avviso `The request was deleted outside Weaver.`

Il rifiuto di un'immagine (`Only PNG, JPEG, GIF or WebP images up to 10 MB.`) e gli errori di salvataggio delle immagini restano toast di errore, come già sono.

L'aspetto dei toast (colore ambra con icona `TriangleAlert` per gli avvisi, rosso con `CircleX` per gli errori, nessuna icona per gli info, pulsante di chiusura `X` focalizzabile con `Alt+T` e `Tab`, Esc che non chiude, scomparsa automatica dopo 4 s) è trattato da un altro Task su `requirements/ui#General`: qui conta solo il tipo di toast usato dall'editor. I test che simulano `sonner` per questi toast vanno aggiornati al nuovo tipo.
