---
status: approved
kind: change
base:
  requirements/quality/main.md: 84c05f38267af8ba47b6625e92c127a3a7bd777a6e740645390c7fa78aef3951
  requirements/ui/look.md: fd0985f9ad96ea06a15627bd81c04786e176ddb425bc2fb7b046698c8e76316b
created: 2026-11-02T14:40:11.877Z
request: 2026-11-02T15-36-29
label: Dialoghi nitidi
job: agqe9qie
---
# Dialoghi nitidi

Ogni tanto alcune parti dell'interfaccia, come il modale delle domande, appaiono sfocate. La causa probabile è nel `Dialog` e nell'`AlertDialog` condivisi: sono centrati spostandoli di metà della propria dimensione, uno spostamento che cade spesso su frazioni di pixel (altezze dipendenti dal contenuto, tetto all'85% della finestra, finestre di dimensioni dispari), e WebKit disegna allora il testo fuori dalla griglia dei pixel. L'ingrandimento dal 95% all'apertura può inoltre lasciare la finestra su un livello grafico sfocato.

## Decisioni

- Ambito: tutte le finestre di dialogo (`Dialog` e `AlertDialog` condivisi: domande, dettagli delle card, moduli, conferme). Popover, menu, Select, tooltip e bolle del Diagramma restano come sono.
- Posizione: centrata come oggi, ma calcolata dal layout e su pixel interi, senza spostamenti di metà dimensione.
- Animazione di apertura: solo dissolvenza di 150 ms, senza ingrandimento; nessuna animazione con il movimento ridotto.
- Verifica: il test end-to-end apre un dialogo in una finestra di dimensioni dispari e controlla che la sua posizione sia su pixel interi.
