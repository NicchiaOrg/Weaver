---
technical: true
refs:
  - requirements/ui#app
---
# Diagramma, Requisiti e Specs

Dipende da "Scheletro del progetto", "Core dei file del progetto" (immagine del progetto, validazione, scope e applicazione delle Proposal), "Shell e aspetto dell'app" e "Finestre e avvio" (`Open in editor`).

Le viste Diagram, Requirements e Specs della finestra di progetto mostrano in sola lettura il Modello, i Requisiti e le Specs, con una Proposal aperta sovrapposta e un focus. Tutto ciò che mostrano arriva calcolato dal backend; l'interfaccia non legge file. Il layout del Diagramma non gira sul thread dell'interfaccia né su quello dei comandi IPC.

## `requirements/ui#app`
- **Diagram**: cytoscape con layout elkjs (`layered`, direzione `RIGHT`); la radice usa una griglia senza archi. Livelli System → Whole System o un Actor → Feature → Machine → State. La radice mostra `Whole System`, gli Actor in ordine di Modello, poi le Feature non raggiunte da alcun Actor. Whole System: ogni Actor e Feature con archi etichettati col numero di Event; livello Actor: l'Actor, le Feature che ricevono i suoi Event e gli archi; livello Feature: le Machine e gli archi di Event, più Actor e Machine esterni come nodi compatti tratteggiati; livello Machine: State (iniziale col punto pieno, finale col bordo doppio) e Transition `<sender>.<EVENT>`; livello State: lo State con le Transition entranti e uscenti. Sottotitoli `<n> features` / `<n> machines`; un Actor senza Event ricevuti mostra `No events received`, tratteggiato, non apribile.
- Clic seleziona e apre il pannello destro (360 px): titolo, descrizione renderizzata, Transition o Event, Drift aperti e Task che lo riguardano, `Open in editor`. Doppio clic o Enter apre il livello; sul livello Feature una Machine si espande e richiude sul posto. Breadcrumb `System › <Actor | Whole System> › <Feature> › <Machine> › <State>` cliccabile (da un link parte da `System › <Feature>`). Hover: titolo e prima frase della descrizione. Badge rosso col numero di Drift aperti. Ai cambi di Modello o overlay restano livello e selezione, con ripiego sul contenitore più vicino. `Fit` e viewport tenuto per livello.
- **Overlay**: un selettore per finestra condiviso dalle tre viste: `Current Model`, poi ogni Proposal aperta non legata a un Drift (mai Draft, in refinement o allegate). Se la Proposal smette di essere aperta si torna a `Current Model` con un avviso. Il Modello è disegnato con la Proposal applicata sopra quelle richieste. Colori (600 chiaro / 400 scuro): aggiunto `green`, cambiato `amber`, rimosso `red` con bordo tratteggiato; sempre anche l'etichetta testuale `added`/`changed`/`removed` in dettagli, hover e nome accessibile.
- **Focus**: proprio per ciascuna vista, aperto da `View on diagram` / `View requirements` (scope di una Proposal o refs di un Task) o da un chip (un singolo ref). Passi: livelli in ordine di Modello sul Diagram, sezioni/Area/campi in ordine di pagina altrove. Barra con soggetto (`Proposal <id> – <title>`, `Task <id> – <title>` o il ref), nome del passo, `<n> of <N>`, `Previous`/`Next`, `<M> not found` con tooltip, `Only these` sui Requirements, `Clear`. Sul Diagram elementi a fuoco in `cyan` (o colore di cambio), il resto al 30% ma cliccabile; il viewport contiene il passo senza superare `Fit`. Ricalcolo ai cambi di file restando sullo stesso passo; Esc toglie prima la selezione poi il focus.
- **Requirements view**: Aree in ordine di cartella, frammenti in ordine di file (`<fragment>.md` in muted), sezioni con `General` prima; chip di riferimento che apre l'elemento sul Diagram e `Open in editor` alla riga del titolo. Etichette overlay `new fragment`, `fragment removed`, `added`, `removed`, `changed`, `moved from <fragment>`; rimossa barrata, cambiata con `Show before`.
- **Specs view**: campi in ordine fisso (languages, frameworks, data, testing, CI, repository, deployment, infrastructure, notes), vuoti nascosti; larghezza massima 1100 px, griglia di colonne ≥ 300 px; voce con nome, versione, nota e `Sparkles` `agent's choice` col motivo in tooltip; overlay come sui Requirements.
- **Errori**: Modello o Requisiti invalidi mostrano in cima alle tre viste l'`Alert` rosso `The Model has <n> errors` con elenco espandibile (messaggio, file, riga, `Open in editor`), file illeggibili inclusi. `Open in editor` ovunque file e riga sono noti; un errore dà un avviso col motivo.

## Fatto quando
- Test Vitest: navigazione dei livelli da tastiera, breadcrumb, pannello dettagli, focus con passi e `not found`, overlay con etichette testuali, banner errori.
- Ai cambi di file livello, selezione e focus restano o ripiegano come descritto.
- Il test end-to-end vede il Diagram renderizzato nel progetto di prova.
