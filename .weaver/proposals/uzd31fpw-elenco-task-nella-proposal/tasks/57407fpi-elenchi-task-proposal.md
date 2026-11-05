---
technical: false
refs: [proposal, requirements/ui#proposal]
---

# Elenchi Task nella Proposal

Nel details dialog della Proposal, le sezioni `Tasks it creates` e `Code-removal Tasks` mostrano una riga compatta per Task con il solo titolo: niente chip delle refs e niente badge `technical`. Il dialog della voce di History non cambia.

Ogni riga di `Tasks it creates` è un pulsante. Un click (o la tastiera) apre un Dialog annidato sopra il details dialog, con il titolo del Task e il suo body reso come Markdown, senza refs e senza badge; se il body è vuoto mostra `No description.`, come già fa il body della Proposal. `Esc` o `Close` chiudono solo il Dialog annidato e il focus torna alla riga che l'ha aperto; il details dialog resta aperto. Quando il details dialog si chiude, il Dialog annidato si chiude con lui.

Perché ciò sia possibile, il body di ogni Task della Proposal deve arrivare alla UI: la vista della Proposal inviata dal backend include, per ogni Task, anche il suo body (e un'identità stabile per distinguere Task con lo stesso titolo).

Le righe di `Code-removal Tasks` usano lo stesso stile di elenco ma sono testo semplice, non controlli e non cliccabili; la ref nel loro titolo (``Remove code of `<ref>` ``) è resa come codice inline, non con i backtick letterali.

I test esistenti della scheda Proposal che verificano chip delle refs e badge `technical` nel details dialog vanno aggiornati al nuovo comportamento, con copertura di apertura e chiusura del Dialog annidato, ritorno del focus e testo `No description.`.
