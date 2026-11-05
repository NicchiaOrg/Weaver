---
technical: true
refs:
  - requirements/rules#ask-conversation
---
# Istruzione di Ask more

Quando il Task è finito, `weaver-core` ha una funzione pura che costruisce l'istruzione di Weaver per una conversazione di Ask more, come dice `requirements/rules#ask-conversation`, dai dati della cosa scelta letti dai file (o, per una domanda, dal round in memoria) e dalla lingua scelta dal developer.

- Il testo fisso è in inglese, incluso nell'app (come la costante dell'istruzione di General in `crates/core/src/jobs/general.rs`), e comincia con la riga `Weaver: Ask more`. Chiede all'agente di: leggere con i comandi elencati ciò che gli serve; rispondere al messaggio del developer e spiegare; cambiare un Task, un Drift o una Proposal solo quando il developer lo chiede e solo con i comandi di `weaver` che lo permettono; non cambiare mai il Model né i Requisiti, che cambiano solo con una Proposal approvata; non scrivere mai a mano nei file sotto `.weaver/`; avviare un job di Weaver solo su richiesta esplicita del developer, come in Chat; chiedere al developer solo nelle sue risposte.
- Segue la parte che descrive la cosa scelta, su più righe, per ciascuno dei sei tipi:
  - **Task:** identità permanente, numero breve, titolo, corpo completo, riferimenti, percorsi delle immagini e, quando ci sono, la Proposal del suo design (identità e titolo) e il Drift che corregge (identità).
  - **Proposal:** identità permanente, numero breve, titolo, stato, corpo completo, i suoi Task (identità e titolo) e, per una risoluzione, ogni Drift che risolve con la sua decisione.
  - **Drift:** identità permanente, numero breve, titolo, `What happens` e `What should happen` per intero, riferimenti e, quando ci sono, la Proposal allegata, il Task da cui nasce e il Drift che ripete.
  - **Task di una voce della History:** dalla copia nella voce: identità, titolo, corpo, riferimenti, Proposal, etichetta e ora di fine; poi identità, titolo e riassunto della voce.
  - **Domanda:** la domanda in vista per intero (testo, contesto, ogni opzione con etichetta, descrizione e dettagli, l'opzione raccomandata, se è a scelta multipla), poi testo e opzioni (etichetta e descrizione) delle altre domande del round nel loro ordine, segnando quella in vista; poi nome e identità del job che ha aperto il round o, per la domanda pronta di un Drift, l'identità del Drift. Dice che il round è ancora aperto e che l'agente non deve rispondere al posto del developer. Mai scelte, testo scritto, risposte o salti del developer: la funzione non li riceve nemmeno.
  - **Sezione dei Requisiti:** il riferimento `requirements/<area>#<sezione>`, l'Area, il frammento, il titolo e il testo completo come lo mostra la vista; con l'overlay di una Proposal, identità, numero breve e titolo della Proposal e il cambiamento che porta alla sezione: `added`, `changed` con il testo di prima e quello nuovo, `removed`, `moved from <fragment>`.
- Dopo ogni parte, i comandi di lettura: per le schede `weaver task show <id>`, `weaver proposal show <id>` o `weaver drift show <id>` per la scheda scelta e per ogni scheda collegata, e `weaver model show <ref>` per ogni riferimento; per un Task della History `weaver history show <voce>`, `weaver proposal show <id>` solo se la Proposal esiste ancora, e `weaver model show <ref>` per ogni riferimento; per la domanda pronta di un Drift `weaver drift show <id>`; per una sezione `weaver model show requirements/<area>#<sezione>`, tranne per una sezione `added`, e con l'overlay `weaver proposal show <id>`.
- Solo identità permanenti, tranne il numero breve della scheda scelta e quello della Proposal dell'overlay, ciascuno detto come il numero breve che vede il developer.
- Con una lingua scelta si aggiunge solo la frase `Write the texts for the developer in <language>.` (la stessa di General: niente rimando a una skill), con la lingua ripulita come fa già `instruction()`. Nessuna delle frasi delle istruzioni dei job (limite dei sotto-agenti, Sub-agents context sharing, `weaver question new`).
- La regola di `requirements/rules#job` sull'istruzione di una riga non vale qui: questa istruzione è su più righe e contiene testi di schede e dei Requisiti.

Test unitari della funzione: la parte di ogni tipo con tutti i campi e con quelli facoltativi assenti; una domanda senza risposte né scelte; la sezione `added` senza `weaver model show`; la frase della lingua presente solo con una lingua scelta e nessun'altra frase aggiunta.
