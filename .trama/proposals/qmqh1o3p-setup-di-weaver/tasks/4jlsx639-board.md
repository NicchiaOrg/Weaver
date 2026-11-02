---
technical: false
refs:
  - work
  - developer
  - requirements/ui#work
  - requirements/rules#work
---
# Board

Weaver mostra la Board della Feature `work` e offre nell'app le decisioni dell'Actor `developer`. La Board mostra ciò che aspetta una decisione (Proposal e Drift aperti) e i Task ancora da finire; il lavoro finito la lascia e si racconta in History. Dipende da "Scheletro del progetto" e "Shell e aspetto dell'app"; le card di ogni tipo sono descritte in "Proposal", "Task" e "Drift", i ghost vengono da "Lavori dell'agente".

## Colonne e intestazione (`requirements/ui#work`)
- Due colonne di uguale larghezza: `Backlog` (Proposal aperte e Drift aperti in un unico ordine di creazione, più vecchi prima) e `Active` (Task in To do e In progress, bloccati compresi). Nessuna colonna Done, né altra colonna.
- Ogni titolo di colonna mostra il numero delle card reali (i ghost non contano, una card di resolution conta una volta); testi vuoti `Nothing to decide` e `Nothing active`. Il raggruppamento del Backlog arriva calcolato dal backend; l'interfaccia non raggruppa.
- Nessuna card si trascina, si sposta o si toglie. Clic sulla card fuori dai controlli, o Invio sul titolo, apre il dialogo dei dettagli. Ogni colonna scorre da sola, con il footer sempre visibile.
- Footer: `+ Work Item` (sempre abilitato, apre una nuova richiesta, vedi "Richiesta di design") e `Run all` con i motivi disabilitati `Drifts are being resolved`, `A run is in progress`, `No Task is in To do`, `All the Tasks in To do are blocked`.
- Intestazione: `Scan System` con `· <n> done since last scan` (Task finiti, non chiusi senza modifiche, dall'avvio dell'ultimo Scan System; nascosto a 0). Chiede conferma `Compare the whole System with the code?` (`Scan System` / `Cancel`) e confronta sempre tutto il Sistema.

## Ghost, dettagli, avvisi
- Ghost in fondo alla colonna di destinazione, in ordine di annuncio: bordo tratteggiato, 70% di opacità, shimmer, tipo, titolo, fase (`Preparing`, `Writing`, `Waiting for your answers`, `Being refined`) e ultima nota del job su una riga. È un unico pulsante che apre la conversazione del job; all'arrivo della card il ghost sparisce nello stesso quadro.
- Il dialogo dei dettagli segue la sua card senza spostare il focus e si chiude da solo quando la card lascia la Board, riportando il focus alla colonna.
- Avvisi in cima alla Board, più recenti prima, fino alla chiusura: decisioni rifiutate all'applicazione (`Could not delete T4: the agent that filed it is still working.`) e Proposal rimandate in Draft mentre il developer le decideva.

## «Concerns the same thing» (`requirements/rules#work`)
- Chiave di un riferimento: un elemento è se stesso; `requirements/<area>#<element>` è l'elemento; `requirements/<area>#General` solo se stesso; `requirements/<area>` l'Area; `specs#<field>` il campo; `specs` le Specs.
- Contenimento: Feature ⊃ Machine; Machine ⊃ State, Event e contenuti; State ⊃ sotto-State e Transition uscenti; Actor ⊃ Event; Area ⊃ solo la sua General; `specs` ⊃ ogni campo.
- Due riferimenti riguardano la stessa cosa se le chiavi coincidono o una contiene l'altra; due card se un riferimento dell'una lo fa con uno dell'altra; una card senza riferimenti non riguarda nulla. Una sola funzione del core serve blocco, slot, gruppi e ricerca dei Drift.

## Decisioni del developer
- Ogni decisione (Proposal, Drift, Run, Delete) avviene intera o non cambia nulla, e le sue condizioni sono ricontrollate dal backend nel momento in cui si applica; un rifiuto produce un avviso e nessuna modifica.
- Un controllo disabilitato elenca tutti i motivi che valgono, uno per riga, nell'ordine fissato; `The agent is still finishing` viene sempre primo; si riabilita da solo senza spostare il focus.
- Scelta del developer: i Task non si spostano a mano; dall'app solo `Run` e, in To do, `Delete`. Per altro si chiede all'agente in General, che decide da riga di comando solo su richiesta esplicita del developer, contando come Event del developer.

## Fatto quando
- Test del core sulla funzione «concerns the same thing» per ogni tipo di riferimento e contenimento.
- Test Vitest: colonne, conteggi senza ghost, testi vuoti, motivi disabilitati di `Run all` in ordine, conferma di `Scan System`.
- Una decisione rifiutata all'applicazione mostra l'avviso e non scrive nulla.
- Un dialogo aperto si aggiorna e si chiude quando la sua card lascia la Board.
