---
technical: true
refs:
  - requirements/ui#General
  - requirements/quality#General
---
# Dialoghi nitidi

Ogni tanto le finestre di dialogo, in particolare il modale delle domande, appaiono sfocate. La causa probabile è nei componenti condivisi `ui/src/components/ui/dialog.tsx` (pannello alla riga ~37) e `ui/src/components/ui/alert-dialog.tsx` (pannello alla riga ~32): sono posizionati con `fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2`, cioè spostati di metà della propria dimensione. Quella metà cade spesso su frazioni di pixel (altezze dipendenti dal contenuto, come il titolo `text-lg leading-tight` di 22,5 px; il tetto `max-h-[85vh]`; finestre di larghezza o altezza dispari), e WebKit disegna allora il testo fuori dalla griglia dei pixel. Inoltre l'apertura `animate-in fade-in-0 zoom-in-95` (keyframe `enter` di `tw-animate-css`, che anima `transform` e `filter`) mette il pannello su un livello grafico separato che può restare sfocato.

Quando il Task è finito:

- Ogni `Dialog` e `AlertDialog` condiviso, quindi anche il modale delle domande (`ui/src/components/questions/question-overlay.tsx`), i dialoghi di dettaglio e dei moduli (`ui/src/components/dialogs.tsx`) e tutte le conferme, è centrato nella finestra dal layout (per esempio un contenitore flex o grid a tutta finestra), senza alcuno spostamento in percentuale della propria dimensione. L'aspetto resta quello di oggi: centrato in verticale e in orizzontale, stesse larghezze massime (720 px, 760 px per le domande, `max-w-lg` per le conferme), altezza massima 85% della finestra con il contenuto che scorre all'interno.
- L'angolo in alto a sinistra di ogni dialogo sta sempre su pixel CSS interi, qualunque sia l'altezza del contenuto (anche quando cambia passando da una domanda all'altra) e la dimensione della finestra, dispari compresa.
- I dialoghi si aprono con una sola dissolvenza di 150 ms, senza ingrandimento, scala né altre trasformazioni o filtri. Con `prefers-reduced-motion: reduce` compaiono subito, come oggi. Popover, DropdownMenu, Select e Tooltip mantengono la loro animazione di apertura; le bolle delle Drift sul Diagramma non cambiano.
- Il comportamento resta invariato: Esc, il pulsante `X` e il clic fuori chiudono un Dialog; un AlertDialog si chiude solo con i suoi pulsanti o Esc; nel modale delle domande Esc e il clic fuori non fanno nulla; il focus resta intrappolato e torna al controllo che ha aperto il dialogo.
- Il test end-to-end (tauri-driver sull'app impacchettata) porta la finestra del progetto a una larghezza e un'altezza dispari, apre il dialogo di dettaglio di una card e verifica che `left` e `top` del suo riquadro siano numeri interi di pixel CSS.
