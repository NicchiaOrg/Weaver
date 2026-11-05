---
status: approved
kind: change
base:
  requirements/ui/board.md: de7032af6a97748be274f60f33d037c25d46c8c78255869e6da8342eb0b397c7
  requirements/ui/look.md: 663fd819cced35a0f093b70c6dbefefc010e2a0849221a8b6159536cebfab138
  requirements/ui/setup.md: 7fc0c2e54f585af2c09304ba3b2f917c107df33fc44658f6a5e80d09e8745af4
  requirements/ui/views.md: 1387027f9e5e7eae9fd8aba1433f9cf7d08dda0460afa0db6df7d81a981059b3
created: 2026-11-02T14:38:47.282Z
request: 2026-11-02T15-32-49
label: Toast chiudibili
job: 9s278nrb
---
# Toast e avvisi chiudibili

Il developer deve poter chiudere i toast di avviso e di errore e l'avviso fisso sulla History in cima alla Board; oggi i toast spariscono dopo 4 s senza pulsante di chiusura e l'avviso `Could not record … in History` non si chiude mai.

Decisioni:
- Tutti i toast, anche quelli informativi, hanno un pulsante di chiusura `X`.
- Tutti i toast restano 4 s come oggi.
- `The request changed on disk; Weaver kept your version.` e `The request was deleted outside Weaver.` diventano toast di avviso.
- Avvisi in ambra con `TriangleAlert`, errori in rosso con `CircleX`; i toast informativi restano neutri.
- Da tastiera si chiude con il pulsante `X` focalizzabile (`Alt+T`, poi `Tab`); `Esc` non chiude i toast.
- Il fallimento di `Open in editor` e dell'apertura di un link resta un toast di errore: i Requirements si allineano al codice.
- L'avviso `Could not record T3 in History: …` sulla Board si chiude e resta nascosto finché la finestra resta aperta; ricompare alla riapertura se è ancora vero, e un avviso nuovo o con testo diverso compare subito.
- Nessun altro riquadro di errore fisso diventa chiudibile.

Il Modello non cambia: cambiano solo i Requirements dell'Area `ui`.

Raffinamento sui Requirements attuali:
- `requirements/ui/look.md` riscritto sulla versione attuale: restano le due regole nuove di `Dialogs and confirmations` (Dialog centrati per layout su pixel interi, apertura con sola dissolvenza di 150 ms), approvate dopo l'inizio di questa Proposta; i toast restano una sezione `### Toasts` a sé subito dopo. Assunzione dell'agente: le due regole non toccano i toast, quindi nessuna decisione cambia e i Task restano invariati.
