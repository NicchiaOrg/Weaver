---
technical: false
refs:
  - developer.SAVE_FILE
  - developer.RELOAD_FILE
  - developer.OVERWRITE_FILE
  - workspace.OPEN_FILE_CHANGED
  - workspace.OPEN_FILE_DELETED
  - requirements/tech#file-tab
  - requirements/rules#file-tab
  - requirements/quality#file-tab
---

# Lettura e salvataggio schede

Il backend dei file aperti in una scheda della vista File: lettura, osservazione del singolo file, salvataggio con controllo e rilettura. Nessuna analisi del codice: Weaver legge un file solo quando si apre in una scheda, dopo un evento della sua osservazione o una rilettura chiesta dallo sviluppatore.

## Apertura e lettura

- I byte grezzi si leggono una volta sola. Il file è **binario** se i primi 8.192 byte contengono un byte nullo, o se i byte, dopo un eventuale BOM UTF-8, non sono UTF-8 valido: nessun testo va alla UI. È **troppo grande** sopra 2.097.152 byte: sola lettura, senza colori. Un file dentro `.weaver/` è di **sola lettura**.
- La scheda tiene lo SHA-256 dei byte grezzi, se iniziavano con un BOM UTF-8 e la terminazione di riga scelta per il salvataggio: la più usata nel file letto contando CRLF e LF isolati; LF a pari merito o senza a capo; un file creato nell'albero usa LF. La UI riceve il testo senza BOM e con LF.
- Due percorsi sono lo stesso file quando le forme canoniche coincidono, senza distinguere maiuscole su Windows.
- Finché la scheda è aperta Weaver osserva solo quel file; chiudere la scheda libera subito testo e osservazione. Nessun log contiene il contenuto di un file; i percorsi sì. Scrivere nell'editor non avvia processi né lavoro su disco.

## Eventi del disco

- `workspace.OPEN_FILE_CHANGED`: il file è cambiato su disco, o è ricomparso, per opera di altro che il salvataggio della scheda stessa. Dopo un salvataggio la scheda tiene l'hash scritto; un evento il cui file ha quell'hash non è un cambiamento.
- `workspace.OPEN_FILE_DELETED`: il file è sparito (eliminato o spostato da altro che la vista File). La scheda tiene il testo.
- Le conseguenze per stato (Saved si rilegge; Modified resta con la barra e il prossimo salvataggio passa per il controllo; Deleted che ricompare torna Saved o Modified; Read-only si rilegge; Binary toglie l'indicazione di eliminato) sono quelle della macchina `file-tab`; il backend fornisce gli eventi e la rilettura.
- Quando uno sblocco dice dove l'agente ha spostato il file, la scheda prende quel percorso e lo rilegge lì; l'osservazione lo segue.

## Salvataggio

- `developer.SAVE_FILE`: il comando porta il testo della scheda e l'hash letto. Sotto un lock per file nel backend Weaver rilegge il file e confronta lo SHA-256 dei byte grezzi: se differisce, o il file non c'è più, non scrive nulla e dice alla UI quale dei due casi. Altrimenti scrive il testo con la terminazione di riga della scheda, il BOM se c'era (mai aggiunto) e nessun'altra modifica; l'a capo finale resta o manca come nel testo. Un salvataggio da Deleted (`Save again`) scrive solo quando al percorso non esiste alcun file.
- La scrittura passa per la scrittura atomica di `crates/core/src/atomic.rs:write` (temp `.<name>.<random>.tmp` nella stessa cartella, `sync_all`, rename, tentativi su Windows), dando al temp i bit di permesso del file sostituito (oggi non li conserva). Se il percorso è un link simbolico, si scrive il suo bersaglio dentro il progetto (oggi il rename sostituirebbe il link).
- Una scrittura fallita non cambia nulla su disco e restituisce il motivo; la scheda resta Modified col suo testo.
- `developer.OVERWRITE_FILE` (`Overwrite`, `Keep mine`) scrive saltando il confronto. `developer.RELOAD_FILE` perde le modifiche e rilegge dal disco.
- Non usare `crates/core/src/project/text.rs` (`read_text`, `write_text`): normalizza CRLF, toglie il BOM e aggiunge l'a capo finale; vale solo per `.weaver/`. `sha256_hex` è riusabile.

## Test

Funzioni pure per rilevamento binario, limite di dimensione, scelta della terminazione di riga e limite di schede. Salvataggi con LF, CRLF, terminazioni miste, BOM, senza a capo finale, bit eseguibile e tramite link simbolico, in cartelle temporanee su ogni sistema; il controllo dell'hash, `Overwrite`, `Save again`, una scrittura fallita e la regola delle auto-scritture.
