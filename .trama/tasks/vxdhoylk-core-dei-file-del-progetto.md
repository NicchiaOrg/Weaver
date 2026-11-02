---
column: done
technical: true
refs:
  - requirements/data
  - requirements/data#General
  - requirements/rules
  - requirements/rules#General
drift: null
proposal: qmqh1o3p
agent: false
created: 2026-11-02T07:29:32.875Z
finished: 2026-11-02T08:04:59.077Z
---
# Core dei file del progetto

Il cuore di `trama-core`: lettura, validazione e scrittura di `.trama/` come funzioni pure e testate, e l'immagine in memoria di un progetto. È l'unico codice che tocca i file del progetto; app e CLI `trama` passano entrambe da qui, l'interfaccia mai. Dipende da "Scheletro del progetto" (crate, lock del progetto, coda delle mutazioni). Le regole delle singole carte (Proposal, Task, Drift, History, richieste) le aggiungono i Task che le riguardano, sopra queste fondamenta.

## Formato (`requirements/data#General`)

- Layout di `.trama/` come da `requirements/data#General`: `model/`, `requirements/<area>/`, `specs.yaml` facoltativo, `proposals/`, `tasks/`, `drifts/`, `history/` versionati; `requests/`, `short-numbers/`, `task-images/` solo locali, ciascuno con un proprio `.gitignore` contenente `*`, riscritto quando manca. File e cartelle che iniziano con `.` non sono mai letti come Model, Requisiti, Specs o carte.
- Parser di Model, Requisiti e Specs esattamente secondo il riferimento `FORMAT.md` della skill `trama-design` (Actor, Machine con `## States` e `## Events`, un solo `(initial)` per livello, `(final)` senza sotto-State né Transition, Transition come voce di lista che inizia con uno span di codice `owner.EVENT` → target nella stessa Machine, nomi di State unici senza `/ : ( ) →`, Event `UPPER_SNAKE_CASE`, namespace unico di Feature/Machine/Actor e nomi riservati; Area con `area.md` senza sezioni e frammenti con `## General` o una sola referenza per sezione, sezione unica nell'Area; campi e voci delle Specs). Le referenze agli elementi (`feature`, `machine`, `actor`, `machine/State`, `machine/State/owner.EVENT`, `owner.EVENT`, `requirements/<area>#<section>`, `specs#<field>`) si risolvono qui. Queste regole sono quelle di `trama model check`, che segnala anche gli avvisi (frammento oltre 12.288 byte) senza fallire.
- Il parser e `FORMAT.md` sono testati insieme: ogni esempio del file viene interpretato e ogni regola che enuncia è applicata. Il testo di `FORMAT.md` lo scrive il Task "Testi delle skill".
- Identità di carte e lavori `^[a-z0-9]{8}$` dal CSPRNG, riestratta se già usata dallo stesso tipo (per i Task: `.trama/tasks/`, copie `cleared` in `history/` e `tasks/` di ogni Proposal); letta sempre dal nome del file o della cartella. Slug: minuscolo, senza diacritici, ogni sequenza fuori da `[a-z0-9]` diventa un `-`, niente `-` ai bordi, al massimo 40 caratteri senza `-` finale, `untitled` se vuoto; fissato alla creazione.
- Una sola funzione di lettura: toglie il BOM e porta CRLF a LF prima di interpretare, calcolare hash o confrontare. Scrittura in UTF-8 senza BOM, LF, newline finale. Front matter YAML con le chiavi nell'ordine dei Requisiti, chiavi facoltative assenti non scritte, chiavi sconosciute conservate. Istanti UTC ISO 8601 con millisecondi dall'orologio iniettato, mai un valore segnaposto.
- Scrittura atomica: `.<name>.<random>.tmp` nella stessa cartella, flush, rename; su Windows un rename rifiutato per condivisione o accesso negato è ritentato ogni 50 ms per al massimo 1 s.
- Un file illeggibile o non interpretabile diventa una diagnostica con percorso e motivo, mostrata nella finestra e nell'output della CLI; mai scartato in silenzio né riscritto, e gli altri file si caricano comunque.

## Regole comuni (`requirements/rules#General`)

- Numeri brevi `P`/`T`/`D` + il più basso intero libero del tipo, assegnati in ordine di creazione alle carte vive, liberati alla chiusura (un Task Done di un design tiene il numero finché tutti i Task del design sono Done). Input in qualsiasi maiuscola; lettera + cifre è sempre un numero breve, con lettera sbagliata o numero non vivo rifiutato. Cache solo in `short-numbers/assigned` (`<P|T|D> <identity> <number>`), scritta in modo atomico; mancante, illeggibile, con duplicati o carte sconosciute è ricostruita in silenzio e nessun comando fallisce per essa. Mai numeri brevi in file versionati.
- Ordine: `created` più vecchio prima, parità per identità in ordine di byte.
- Ogni cambiamento di carte, sotto il lock del progetto, legge, valida tutto contro lo stato dopo il cambiamento, poi scrive; tutto o niente: se una scrittura successiva fallisce i file già scritti tornano al contenuto precedente; un rifiuto non lascia file, cache né lock.

## Immagine del progetto

Un valore in memoria con Model, Requisiti, Specs, carte e diagnostiche; analisi incrementale (ogni file tenuto con il suo hash, si rianalizzano solo i percorsi cambiati o rimossi), dati derivati ricalcolati per intero, costo lineare nel numero di file.

## Fatto quando

- Test unitari per ogni regola di `FORMAT.md`, con un test che interpreta tutti gli esempi del file installato.
- Test su cartelle temporanee: BOM e CRLF in lettura, LF in scrittura, percorsi Windows con spazi e caratteri non ASCII, file illeggibile riportato senza bloccare gli altri, scrittura multi-file annullata al fallimento.
- Test di slug, identità (riestrazione su collisione), numeri brevi (assegnazione, riuso, ricostruzione della cache) e ordine, con orologio e casualità iniettati.
