---
created: 2026-11-03T09:18:59.268Z
updated: 2026-11-03T09:18:59.268Z
summary: 'La finestra di un progetto ha la vista File: albero dei file, editor CodeMirror a schede con colori della sintassi, salvataggio con controllo del file su disco, blocco dell''agente e conferma alla chiusura.'
tasks:
- 3pk2yh8s
- 7v9sqgpy
- fxbxhm60
- o8lkmkhy
- mtcfhymz
- r8ujtjvg
- u6vqa38i
cleared:
  3pk2yh8s:
    title: README della vista File
    refs:
    - app
    - files
    - agent
    - time
    - workspace
    technical: false
    proposal: 4kt8tmpv
    label: Vista File
    finished: 2026-11-03T09:08:13.857Z
    body: |-
      Il principio di Weaver cambia: Weaver non analizza mai il codice dello sviluppatore (lo fa l'agente) e legge e scrive un file del codice solo quando lo sviluppatore lo apre o lo cambia nella vista File; a vista chiusa non legge nulla del codice. La finestra di un progetto, oltre alle viste in sola lettura, ha la vista File (Feature `files`): un albero dei file del progetto e un editor CodeMirror 6 a schede, con i colori della sintassi, senza completamento, diagnostica o language server, per piccole modifiche accanto all'editor dello sviluppatore.

      Questo Task porta quel cambiamento nei testi che lo descrivono, quando il resto della vista File è costruito dagli altri Task della stessa Proposal (albero e vista, schede ed editor, lettura e salvataggio, blocco dell'agente, conferma alla chiusura della finestra).

      Quando il Task è finito:

      - `README.md`: l'introduzione (il primo paragrafo sotto `# Weaver`) dice che la finestra ha anche la vista File per piccole modifiche ai file del progetto, accanto all'editor dello sviluppatore. Le tabelle `### \`weaver-core\` modules` e `### \`weaver\` (app) modules` elencano i nuovi moduli della vista File (elenco delle cartelle, lettura e salvataggio dei file del codice, osservazione, blocco dell'agente) con la loro responsabilità, con i nomi che quei moduli hanno davvero nel codice. `### Where Weaver keeps files` resta vero: la vista File non scrive nulla fuori dai file che lo sviluppatore cambia o crea, sposta ed elimina dall'albero (l'eliminazione va nel cestino del sistema), e non tiene nulla su disco.
      - Ogni testo di Weaver che dice che Weaver non legge mai il codice (istruzioni e skill installate in `crates/core/assets/`, testi dell'interfaccia in `ui/src/`) si allinea al nuovo principio. Una ricerca fatta durante il design non ne ha trovati; si ricontrolla, e se non ce ne sono non si cambia nulla.
      - L'agente (`agent`) legge, analizza e scrive il codice dello sviluppatore; Weaver legge e scrive solo i file che lo sviluppatore apre e cambia nella vista File.
      - Il tempo (`time`) serve anche a liberare ciò che la vista File tiene in memoria 10 s dopo che smette di essere mostrata; mai a decidere che un agente ha smesso di lavorare.
      - Il workspace (`workspace`) osserva, oltre a `.weaver/` e alla radice del progetto per vedere comparire o sparire `.weaver/`, solo le cartelle espanse e i file aperti della vista File; i file del codice non diventano mai `workspace.FILES_CHANGED` e non ricostruiscono la fotografia del progetto.

      La riga del CHANGELOG sotto `## [Unreleased]` per la vista File la scrive il Task dell'albero; questo Task non ne aggiunge un'altra (`requirements/docs#General`).
  7v9sqgpy:
    title: Blocco file dell'agente
    refs:
    - file-lock
    - omp.FILE_TOOL_STARTED
    - omp.FILE_TOOL_ENDED
    - requirements/ui#file-lock
    - requirements/tech#file-lock
    - requirements/rules#file-lock
    - requirements/quality#file-lock
    technical: false
    proposal: 4kt8tmpv
    label: Vista File
    finished: 2026-11-03T09:08:14.145Z
    body: |-
      Quando l'agente di un job, di General o di una conversazione Ask more del progetto sta cambiando un file aperto in una scheda della File view, la scheda diventa di sola lettura e l'albero segna il file. È solo un'indicazione: la protezione resta il controllo al salvataggio della scheda. Contano solo gli strumenti che cambiano file (modifiche, cancellazioni e spostamenti) mentre sono in corso; gli strumenti dei sub-agenti, i comandi che l'agente esegue, gli altri programmi e i percorsi fuori dal progetto non contano mai.

      ## Sorgente e strumenti che contano

      - Sorgente: le notifiche ACP `session/update` dal vivo di ogni job, di General e di ogni conversazione Ask more del progetto. Le update rigiocate mentre una sessione si carica non contano mai. Le trascrizioni dei sub-agenti e lo strumento `task` non contano mai.
      - Contano solo `tool_call` e `tool_call_update` con `kind` `edit`, `delete` o `move`. omp riporta come `edit` le sue modifiche, le scritture e le cancellazioni e gli spostamenti dentro un edit; `execute`, `other` e gli altri non contano mai.
      - Inizio (`omp.FILE_TOOL_STARTED`): uno strumento conta dal suo `tool_call`. Con `locations` conta per quei percorsi; senza `locations` conta per ogni file aperto in una scheda, finché non finisce. È il blocco prudente deciso: nella modalità di edit predefinita di omp (`hashline`) il `tool_call` iniziale non ha `locations`. `locations` arrivate in update successive di uno strumento in corso aggiungono i loro percorsi.
      - Fine (`omp.FILE_TOOL_ENDED`): un `tool_call_update` con status `completed` o `failed` termina lo strumento. Ogni strumento di una conversazione termina inoltre quando il suo turno finisce o fallisce, quando il suo omp esce o la sua connessione si chiude, quando il suo job finisce (Done, Failed, Stopped), quando General viene pulito e quando una conversazione Ask more si chiude. Uno strumento smette di contare una volta sola, al primo di questi eventi. La pulizia di General e la chiusura di Ask more contano a sé perché lì omp viene fermato senza un'uscita segnalata.
      - Spostamenti: su un edit completato le `locations` finali contengono la destinazione; quando `rawOutput.details` del risultato nomina come sorgente (`sourcePath`) un file aperto in una scheda, il blocco rilasciato porta con sé la destinazione. Questi campi sono interni di omp: vanno ricontrollati a ogni aumento di `MIN_OMP_VERSION`, come i campi dei sub-agenti di `requirements/tech#job`; aggiungere il promemoria nel codice accanto a quello esistente.
      - Percorsi: `locations[].path` sono assoluti. Si confrontano con i file aperti in forma canonica; un percorso che non esiste più si confronta come forma canonica del genitore più il suo nome. I percorsi fuori dalla radice canonica del progetto, worktree git compresi, si ignorano. Il confronto deve funzionare anche su Windows.
      - Memoria: gli strumenti in corso di ogni conversazione restano nella memoria del backend anche a File view chiusa, senza lavoro su disco, così un file aperto durante uno strumento si apre già bloccato.

      ## Stati del blocco (per ogni file)

      - Not tracked (iniziale): il file non è aperto in una scheda. All'apertura (`developer.OPEN_FILE`) va in Held se uno strumento che conta per quel file è già in corso, altrimenti in Free.
      - Tracked: il file è aperto in una scheda; la chiusura della scheda (`file-tab.CLOSED`) lo riporta a Not tracked. Contiene Free e Held.
      - Free (iniziale dentro Tracked): nessuno strumento conta per il file. Un `omp.FILE_TOOL_STARTED` che nomina il file, o che non nomina nessun file, lo porta a Held.
      - Held: almeno uno strumento in corso conta per il file. Un nuovo `omp.FILE_TOOL_STARTED` che nomina il file o non nomina file conta anch'esso e resta Held. `omp.FILE_TOOL_ENDED` porta a Free se era l'ultimo strumento che contava per il file, altrimenti resta Held. Con `omp.TURN_ENDED`, `omp.TURN_FAILED` e `omp.EXITED` smettono di contare gli strumenti di quella conversazione; con `job.ENDED` quelli di quel job; con `developer.CLEAR_GENERAL` quelli di General; con `developer.CLOSE_ASK` quelli di quella conversazione Ask more. In tutti questi casi va in Free se nessuno strumento di un'altra conversazione conta ancora per il file, altrimenti resta Held.
      - Eventi: `LOCKED` quando il blocco di un file aperto in una scheda passa da Free a Held; `RELEASED` quando passa da Held a Free, con il posto in cui l'agente ha spostato il file quando omp lo dice. La scheda (`file-tab`, altro Task) li usa per diventare Locked o Read-only, rileggere il file e seguirlo alla nuova posizione.

      ## Interfaccia

      Lo stato del blocco arriva dal backend; l'interfaccia non lo ricava mai dalle conversazioni. Finché il blocco del file di una scheda è tenuto, il suo editor è di sola lettura e una riga sopra il testo, con l'icona `Lock` (`lucide-react`), dice `The agent is editing this file.`; la scheda e la riga dell'albero portano `Lock`. L'albero segna ogni file aperto in una scheda e bloccato, anche quando la sua scheda non è quella in vista. La riga è annunciata alle tecnologie assistive quando appare e quando sparisce (come le regioni `aria-live="polite"` già in uso).

      ## Affidabilità e test

      Un blocco non sopravvive mai alla fine del suo strumento, al turno della sua conversazione, al suo processo omp, al suo job o alla sua conversazione. Test con l'agente ACP finto scriptato: strumenti con e senza `locations`, più strumenti su un file, strumenti di due conversazioni, fine per `completed`, `failed`, fine turno, uscita, fine job, General pulito e Ask more chiuso, update rigiocate durante il caricamento di una sessione, uno spostamento con la sua destinazione, una cancellazione, percorsi fuori dalla radice e su Windows.

      ## Da dove partire

      - Update dal vivo e rigiocate: `src-tauri/src/jobs/actor.rs` e `src-tauri/src/general/actor.rs`, `Actor::incoming`: con `self.loading` le update vanno in `self.replay` (poi `Jobs::adopt_replay` / `General::session_opened`), altrimenti in `apply_update` della conversazione viva. Il blocco va alimentato solo dal ramo dal vivo, non da dentro `Conversation::apply_update`, che serve anche alla conversazione rigiocata e ai test dei sub-agenti. Le conversazioni Ask more girano sullo stesso attore di General (`Conversation::ask()` quando c'è `rebuild`) e stanno negli slot di `General` con `slot.ask`.
      - Fine turno e uscita: `Actor::turn_done` (`TurnEnd::Ended` / `TurnEnd::Failed`) e `Actor::exited` / `Msg::Exited` nei due attori.
      - Lettura degli strumenti: `src-tauri/src/jobs/conversation.rs`, `tool_of`, `Conversation::tool_call`, `Conversation::tool_update`, `tool_kind` (oggi fonde `edit|delete|move` in `ToolKind::Edit`, quindi il `kind` originale va letto dalla update), `status_of`, `looks_like_task_call` (strumento `task`). `locations` oggi non viene mai letto e `ui/src/bindings/Tool.ts` non le espone.
      - Sub-agenti: le loro trascrizioni entrano in conversazioni proprie tramite `src-tauri/src/subagents/records.rs` (`apply`, `Conversation::tool_started` / `tool_finished`, che mappa anch'esso `edit|write|delete|move` in `ToolKind::Edit`): questa via non deve mai alimentare il blocco.
      - Fine del job: `src-tauri/src/jobs/registry.rs`, `Jobs::dispatch_in`, `Effect::Ended` (Done, Stopped, Failed) e `Effect::ShutDown`; Machine in `crates/core/src/jobs/machine.rs`.
      - Pulizia di General e chiusura Ask more: `src-tauri/src/general/service.rs`, `General::carry_out` (`Effect::Clear`, `Effect::ShutDown`, `Effect::Stopped`), `General::clear`, `General::close_ask` / `General::end_ask` (chiamato da `ask_close` in `src-tauri/src/ask/commands.rs`); Machine di General in `crates/core/src/jobs/general.rs`.
      - Percorsi: `crates/core/src/paths.rs`, `canonical_root` (usa `dunce::canonicalize`, adatto a Windows).
      - Promemoria su omp: `crates/core/src/jobs/subagents.rs` (commento del modulo sugli interni di omp da riverificare a ogni aumento di `MIN_OMP_VERSION`); costante in `crates/core/src/omp/version.rs`.
      - Test: `src-tauri/src/jobs/testing.rs`, `fake_agent`, con lo script di `crates/fake-acp/src/main.rs` (passi `{"update": {...}}`, `replay`, `exit`, `hold`, `end`); test di General in `src-tauri/src/general/tests.rs`.
  fxbxhm60:
    title: Lettura e salvataggio schede
    refs:
    - developer.SAVE_FILE
    - developer.RELOAD_FILE
    - developer.OVERWRITE_FILE
    - workspace.OPEN_FILE_CHANGED
    - workspace.OPEN_FILE_DELETED
    - requirements/tech#file-tab
    - requirements/rules#file-tab
    - requirements/quality#file-tab
    technical: false
    proposal: 4kt8tmpv
    label: Vista File
    finished: 2026-11-03T09:08:14.455Z
    body: |-
      Il backend dei file aperti in una scheda della vista File: lettura, osservazione del singolo file, salvataggio con controllo e rilettura. Nessuna analisi del codice: Weaver legge un file solo quando si apre in una scheda, dopo un evento della sua osservazione o una rilettura chiesta dallo sviluppatore.

      ## Apertura e lettura

      - I byte grezzi si leggono una volta sola. Il file è **binario** se i primi 8.192 byte contengono un byte nullo, o se i byte, dopo un eventuale BOM UTF-8, non sono UTF-8 valido: nessun testo va alla UI. È **troppo grande** sopra 2.097.152 byte: sola lettura, senza colori. Un file dentro `.weaver/` è di **sola lettura**.
      - La scheda tiene lo SHA-256 dei byte grezzi, se iniziavano con un BOM UTF-8 e la terminazione di riga scelta per il salvataggio: la più usata nel file letto contando CRLF e LF isolati; LF a pari merito o senza a capo; un file creato nell'albero usa LF. La UI riceve il testo senza BOM e con LF.
      - Due percorsi sono lo stesso file quando le forme canoniche coincidono, senza distinguere maiuscole su Windows.
      - Finché la scheda è aperta Weaver osserva solo quel file; chiudere la scheda libera subito testo e osservazione. Nessun log contiene il contenuto di un file; i percorsi sì. Scrivere nell'editor non avvia processi né lavoro su disco. L'osservatore esistente `src-tauri/src/windows/watch.rs` (`NotifyWatch` sul crate `notify`, `Batcher`) guarda solo `.weaver/` e la radice senza ricorsione, e ignora i temp di `atomic::is_temp_name`: è il precedente per osservare il singolo file.

      ## Eventi del disco

      - `workspace.OPEN_FILE_CHANGED`: il file è cambiato su disco, o è ricomparso, per opera di altro che il salvataggio della scheda stessa. Dopo un salvataggio la scheda tiene l'hash scritto; un evento il cui file ha quell'hash non è un cambiamento.
      - `workspace.OPEN_FILE_DELETED`: il file è sparito (eliminato o spostato da altro che la vista File). La scheda tiene il testo.
      - Le conseguenze per stato (Saved si rilegge; Modified resta con la barra e il prossimo salvataggio passa per il controllo; Deleted che ricompare torna Saved o Modified; Read-only si rilegge; Binary toglie l'indicazione di eliminato) sono quelle della macchina `file-tab`; il backend fornisce gli eventi e la rilettura.
      - Quando uno sblocco dice dove l'agente ha spostato il file, la scheda prende quel percorso e lo rilegge lì; l'osservazione lo segue.

      ## Salvataggio

      - `developer.SAVE_FILE`: il comando porta il testo della scheda e l'hash letto. Sotto un lock per file nel backend Weaver rilegge il file e confronta lo SHA-256 dei byte grezzi: se differisce, o il file non c'è più, non scrive nulla e dice alla UI quale dei due casi. Altrimenti scrive il testo con la terminazione di riga della scheda, il BOM se c'era (mai aggiunto) e nessun'altra modifica; l'a capo finale resta o manca come nel testo. Un salvataggio da Deleted (`Save again`) scrive solo quando al percorso non esiste alcun file.
      - La scrittura passa per la scrittura atomica di `crates/core/src/atomic.rs:write` (temp `.<name>.<random>.tmp` nella stessa cartella, `sync_all`, rename con `rename_with_retry` su Windows), dando al temp i bit di permesso del file sostituito: oggi `write` accetta solo un `unix_mode` fisso applicato alla creazione (soggetto alla umask) e nessun chiamante conserva i permessi esistenti. Se il percorso è un link simbolico, si scrive il suo bersaglio dentro il progetto (oggi il rename sostituirebbe il link).
      - Una scrittura fallita non cambia nulla su disco e restituisce il motivo; la scheda resta Modified col suo testo.
      - `developer.OVERWRITE_FILE` (`Overwrite`, `Keep mine`) scrive saltando il confronto. `developer.RELOAD_FILE` perde le modifiche e rilegge dal disco.
      - Non usare `crates/core/src/project/text.rs` (`read_text`, `decode`, `write_text`, `file_content`): normalizzano CRLF, tolgono il BOM e aggiungono l'a capo finale; valgono solo per `.weaver/`. `sha256_hex` prende un `&str` e ne hasha i byte: va bene sui byte grezzi di un file di testo (UTF-8 valido, BOM compreso), non sul testo normalizzato.

      ## Test

      Funzioni pure per rilevamento binario, limite di dimensione, scelta della terminazione di riga e limite di schede. Salvataggi con LF, CRLF, terminazioni miste, BOM, senza a capo finale, bit eseguibile e tramite link simbolico, in cartelle temporanee su ogni sistema; il controllo dell'hash, `Overwrite`, `Save again`, una scrittura fallita e la regola delle auto-scritture.
  mtcfhymz:
    title: Conferma chiusura con modifiche
    refs:
    - window/Open/developer.CLOSE_WINDOW
    - window/Open/developer.QUIT_APP
    - developer.CLOSE_WINDOW
    - developer.QUIT_APP
    - requirements/tech#window
    technical: false
    proposal: 4kt8tmpv
    label: Vista File
    finished: 2026-11-03T09:09:57.524Z
    body: |-
      Chiudere la finestra di un progetto o uscire da Weaver deve tenere conto, oltre che dei job al lavoro, delle modifiche non salvate della vista File. Le sezioni `## window` e `## notification` passano senza cambiare testo da `requirements/tech/architecture.md` al nuovo frammento `requirements/tech/windows.md`; cambiano solo i punti della conferma di chiusura.

      Quando tutto è fatto:

      - La UI di ogni finestra comunica al backend, a ogni cambiamento, se la sua vista File ha modifiche non salvate e in quali file. Lo stato non salvato e il salvataggio (`developer.SAVE_FILE`) sono costruiti dai Task file-view e file-tab di questa Proposal: questo Task li consuma.
      - `WindowEvent::CloseRequested` è impedito mentre lavorano job di quel progetto o la sua vista File ha modifiche non salvate; `RunEvent::ExitRequested` mentre lavorano job di qualunque progetto o una qualunque vista File ha modifiche non salvate.
      - La conferma resta un dialogo nativo di `tauri-plugin-dialog` (funziona anche con la webview bloccata), unito a quello dei job: dice quanti job lavorano ed elenca i file con modifiche non salvate. Con modifiche non salvate i pulsanti sono `Save all`, `Discard all` e `Cancel` (`MessageDialogButtons::YesNoCancelCustom`, presente in tauri-plugin-dialog 2.8.4); con soli job al lavoro chiede di confermare o annullare, come oggi.
      - All'uscita, ogni finestra con modifiche non salvate chiede a turno, e il suo dialogo conta anche i job di tutti i progetti; `Cancel` in uno qualunque ferma l'uscita e non si chiude nient'altro.
      - `Save all`: il backend chiede alla UI di quella finestra i testi delle sue schede, come fa `requests-flush` per l'editor delle richieste, e aspetta al massimo lo stesso limite (`FLUSH_LIMIT`); poi li salva come `developer.SAVE_FILE`. Se la UI non risponde in tempo o un file non si può salvare, gli altri file sono salvati, la finestra resta aperta, la UI mostra la scheda di quel file che dice perché, nulla viene sospeso e (all'uscita) Weaver non esce.
      - `Discard all` scarta le modifiche senza salvarle.
      - Dopo la conferma e il salvataggio i job sono sospesi (omp terminato come in `requirements/tech#job`), poi la finestra si chiude o l'app esce. Chiudendo la finestra il progetto non si riapre al prossimo avvio; uscendo, si riapre.

      Punti di partenza: `src-tauri/src/windows/mod.rs` (prevenzione, nel gestore di `RunEvent`/`WindowEvent`, di `ExitRequested`/`CloseRequested`); `src-tauri/src/windows/manager.rs` (`close_requested`, `close_window`, `quit`, `close_prompt`, `quit_prompt`, `ConfirmPrompt`, oggi a due soli pulsanti); `src-tauri/src/windows/tauri_shell.rs` (`confirm` con `OkCancelCustom`, `flush_before_close`); `src-tauri/src/requests.rs:flush_window` (con `FLUSH_LIMIT`, oggi 2 secondi) e `ui/src/components/requests/use-requests.tsx` (risposta `requests_flushed`) come precedente del flush.
  r8ujtjvg:
    title: Schede ed editor CodeMirror
    refs:
    - file-tab
    - developer.OPEN_FILE
    - developer.CLOSE_TAB
    - developer.EDIT_FILE
    - requirements/ui#file-tab
    - specs#frameworks
    technical: false
    proposal: 4kt8tmpv
    label: Vista File
    finished: 2026-11-03T09:09:58.102Z
    body: |-
      Le schede della vista File con l'editor CodeMirror 6, secondo la macchina `file-tab` (stati Closed, Open con Saved, Modified, Locked, Deleted, Read-only, Binary). Lettura, salvataggio ed eventi del disco vengono dal backend delle schede.

      ## Aprire e chiudere

      - `developer.OPEN_FILE`: un file già aperto mostra solo la sua scheda (al massimo una scheda per file). Al massimo 10 schede: all'undicesima si chiude la scheda senza modifiche non salvate mostrata meno di recente e il nuovo file si apre al suo posto; se tutte hanno modifiche, nulla si apre e appare il toast di avviso `Every tab has unsaved changes: save or close one to open another file.`
      - All'apertura: binario → pannello `Binary file` con `Open in editor`, senza testo; oltre 2 MB → riga `This file is larger than 2 MB: read-only, without colours.`, sola lettura, senza colori; in `.weaver/` → riga `Files in .weaver are written by Weaver: read-only here.`; un file non binario né di sola lettura che l'agente sta già modificando si apre Locked; altrimenti Saved, con i colori del suo tipo o testo semplice se il tipo è sconosciuto.
      - `developer.CLOSE_TAB`: con modifiche non salvate, dialogo `Save changes to <name>?` con `Save`, `Discard` (rosso) e `Cancel`, `Cancel` col focus iniziale; un salvataggio fallito o rifiutato lascia la scheda aperta. La scheda si chiude anche lasciando la vista File (salvo un Save all fallito), eliminando il file o una sua cartella (le modifiche si perdono) e alla chiusura della finestra; ogni chiusura emette `file-tab.CLOSED` e libera subito editor, testo e osservazione.
      - `developer.EDIT_FILE`: Saved → Modified; in Deleted resta Deleted. Nessun lavoro su disco mentre si scrive.

      ## Barra delle schede e scorciatoie

      - Barra sopra l'editor, una `tab` ARIA in stile shadcn per file in ordine di apertura: nome del file (percorso nel tooltip, monospace), un punto con nome accessibile `Unsaved changes` se ci sono modifiche, `Lock` mentre il blocco è tenuto, `TriangleAlert` mentre una barra attende, e `X` solo icona (`Close`). Sinistra/Destra si spostano tra le schede.
      - Solo con la vista File in primo piano: `Cmd+S`/`Ctrl+S` salva la scheda attiva (nulla se non modificabile); `Cmd+W`/`Ctrl+W` chiude la scheda attiva invece della finestra; `Ctrl+Tab` e `Ctrl+Shift+Tab` mostrano la scheda successiva e precedente. Non c'è un livello di scorciatoie globale: oggi il menu di `src-tauri/src/windows/menu.rs:build` è `Menu::default` su macOS (con Close Window su `Cmd+W`) e, su Windows e Linux, ha **File › Close Window** predefinito accanto a `Quit`; nessuno dei due ha una voce Save.
      - Barra strumenti: percorso relativo al progetto (monospace) e `Open in editor`, che apre il file alla riga del cursore (`ui/src/components/open-in-editor.tsx`, comando `open_in_editor` con `line`).

      ## Editor

      - CodeMirror 6 (nessun `@codemirror/*` ancora in `ui/package.json`), font monospace, tema chiaro o scuro risolto da Weaver che cambia subito con esso (`ui/src/components/theme-provider.tsx:useResolvedTheme`); colori della sintassi conformi al contrasto di `requirements/quality#General` in entrambi i temi. Numeri di riga, annulla/ripeti, cerca e sostituisci (`Cmd+F`/`Ctrl+F`), parentesi corrispondenti, piegatura, più cursori; niente completamento, diagnostica o language server.
      - `@codemirror/language-data` sceglie il linguaggio per nome del file, poi per estensione; ogni linguaggio si carica con un import dinamico al primo file del suo tipo (precedente: `ui/src/components/mermaid-block.tsx`). Nessun WASM; la CSP di `src-tauri/tauri.conf.json` resta com'è.

      ## Barre e dialoghi

      - Barre `Alert` in cima all'editor, finché risolte: Modified con file cambiato su disco → `This file changed on disk. Saving will ask what to keep.`; sblocco con modifiche → `The agent changed this file while it had unsaved changes.` con `Take the agent's version` (rosso, rilegge dal disco), `Keep mine` (sovrascrive) e `Cancel` (toglie la barra, tiene le modifiche; il prossimo salvataggio passa per il controllo); eliminato → `The agent deleted this file.` dopo un blocco, `This file was deleted on disk.` altrimenti, con `Save again` se la scheda è modificabile. Un Deleted che ricompare torna Saved senza modifiche, altrimenti Modified con la barra del cambiamento.
      - Salvataggio che trova il file cambiato: `AlertDialog` `This file changed on disk since it was opened.` con `Reload from disk` (rosso), `Overwrite` (rosso) e `Cancel` col focus iniziale.
      - Scrittura fallita: toast di errore `Could not save <name>.` col motivo come descrizione; la scheda tiene il testo.
      - Locked: testo non modificabile, le modifiche non salvate restano; allo sblocco si rilegge (anche al nuovo percorso se omp ha detto dove l'agente ha spostato il file) o appare la barra dello sblocco. Read-only si rilegge ai cambiamenti e mostra la barra di eliminazione senza poter salvare. Binary dice se il file è stato eliminato e smette quando ricompare.
      - Precedente di rilettura nell'interfaccia: `ui/src/components/requests/request-pane.tsx`.

      ## Test

      In Vitest: schede, barre, dialoghi e scorciatoie (`Cmd/Ctrl+S`, `Cmd/Ctrl+W`, `Ctrl+Tab`) con i percorsi da tastiera e i nomi accessibili.
  u6vqa38i:
    title: Vista File e albero
    refs:
    - file-view
    - developer.OPEN_FILE_VIEW
    - developer.LEAVE_FILE_VIEW
    - developer.EXPAND_FOLDER
    - developer.COLLAPSE_FOLDER
    - developer.CREATE_ENTRY
    - developer.RENAME_ENTRY
    - developer.MOVE_ENTRY
    - developer.DELETE_ENTRY
    - workspace.CODE_FOLDER_CHANGED
    - time.FILE_VIEW_RELEASE_DUE
    - requirements/ui#file-view
    - requirements/ui#General
    - requirements/tech#files
    - requirements/tech#file-view
    - requirements/tech#General
    - requirements/tech#project
    - requirements/rules#file-view
    - requirements/quality#file-view
    technical: false
    proposal: 4kt8tmpv
    label: Vista File
    finished: 2026-11-03T09:13:33.620Z
    body: |-
      Una nuova vista `Files` nella finestra di progetto mostra l'albero dei file del progetto e, a destra, le schede dell'editor (le schede, l'editor e il lock dell'agente sono altri Task; qui servono solo l'area vuota e i punti d'aggancio: aprire un file dall'albero, chiudere o spostare le schede su un percorso, sapere se una scheda ha modifiche non salvate o un lock dell'agente).

      ## Voce e layout
      - Voce `Files` nella sidebar dopo Jobs, icona `FolderTree`; ordine del gruppo in alto: Board, Chat, Jobs, Files, Requirements, Specs, History; il gruppo in basso (Usage, Settings) non cambia. Come le altre viste tranne Settings, mostra Get Started finché il progetto non ha `.weaver/`.
      - A sinistra l'albero, a destra schede ed editor; senza schede la destra dice `Open a file from the tree.` in testo muted.
      - A differenza delle altre viste, scegliere un'altra voce chiude la vista File (non ne conserva lo stato).

      ## Stati (backend, per finestra di progetto, solo in memoria)
      - **Closed** (iniziale): Weaver non legge nessuna cartella del codice e non osserva nulla del codice.
      - `OPEN_FILE_VIEW` da Closed → Open: legge solo nomi e tipi delle voci della radice (mai il contenuto di un file), escludendo `.git` e ciò che git ignora, e osserva solo la radice. Da Releasing → Open: l'albero torna com'era senza rileggere nulla.
      - In Open, `EXPAND_FOLDER` legge solo quella cartella (come la radice) e la osserva da sola, senza le sottocartelle; `COLLAPSE_FOLDER` smette di osservare quella cartella e ogni cartella espansa al suo interno, e ne dimentica le voci. Ogni cartella parte chiusa.
      - `CODE_FOLDER_CHANGED` (una voce apparsa, sparita o rinominata in una cartella osservata, da chiunque, Weaver incluso): in Open e in Releasing Weaver rilegge quella cartella.
      - `LEAVE_FILE_VIEW` (un'altra vista scelta dalla sidebar, da un link o da una notifica): se ci sono modifiche non salvate un `AlertDialog` chiede `Save changes to <n> files?` elencando i percorsi, con `Save all`, `Discard all` (destructive) e `Cancel` (ultimo, focus iniziale); Cancel lascia la vista File e non cambia nulla. Poi ogni scheda si chiude → Releasing. Se con `Save all` un file non si salva, gli altri vengono salvati, l'altra vista non viene mostrata e la vista File resta sulla scheda di quel file, che dice perché (resta Open).
      - **Releasing**: vista non mostrata, nessuna scheda; Weaver tiene albero e osservazioni per 10 s, misurati con un clock iniettato. `FILE_VIEW_RELEASE_DUE` → Closed: osservazioni e listing scartati. Anche la chiusura della finestra scarta tutto subito, da Open o Releasing.

      ## Albero (UI)
      - Righe con icona (`Folder`, `FolderOpen`, `File`) e nome, circa 16 px di rientro per livello come l'albero di Import from Code; prima le cartelle poi i file, ciascuno per nome.
      - ARIA `tree`: Su/Giù tra righe, Destra espande o va al primo figlio, Sinistra chiude o va al genitore, Enter apre un file in scheda o alterna una cartella; il click fa lo stesso.
      - Le voci di `.weaver/` appaiono come le altre ma le azioni che le cambierebbero sono disabilitate (`aria-disabled`, motivo in tooltip e descrizione accessibile) con il motivo `.weaver is written by Weaver: read-only here.`
      - Un file aperto in una scheda con lock dell'agente mostra `Lock` con nome accessibile e tooltip `The agent is editing this file` (lo stato del lock arriva dal Task del lock).

      ## Azioni
      - shadcn `ContextMenu` (da aggiungere in `ui/src/components/ui/`) su ogni riga e sullo spazio vuoto dell'albero per la radice, aperto con click destro, tasto Menu o `Shift+F10`: `New file`, `New folder` (solo cartelle e radice), `Rename`, `Move to…`, `Delete`; icone `FilePlus`, `FolderPlus`, `Pencil`, `FolderInput`, `Trash2`.
      - `F2` rinomina la riga selezionata; `Delete` (`Cmd+Backspace` su macOS) la elimina.
      - `New file`, `New folder`, `Rename` mostrano un campo di testo nella riga col nome selezionato (senza estensione quando si rinomina un file); Enter conferma, Esc o uscire dal campo annulla.
      - **CREATE_ENTRY**: file vuoto (`create_new`) o cartella (`create_dir`), mai sostituendo una voce esistente; appare nell'albero.
      - **RENAME_ENTRY** e **MOVE_ENTRY**: un solo `rename` sullo stesso volume che non sostituisce mai una voce (controllo prima, più `renameat2(RENAME_NOREPLACE)` su Linux, `renamex_np(RENAME_EXCL)` su macOS, `MoveFileExW` senza `MOVEFILE_REPLACE_EXISTING` su Windows); lo spostamento tra volumi è rifiutato col motivo. Le schede aperte sulla voce o su file al suo interno seguono il nuovo percorso e tengono le modifiche non salvate. Spostare: trascinando una riga su una cartella o sullo spazio vuoto della radice (cartella sotto il puntatore evidenziata; rilascio dove è rifiutato non fa nulla), oppure con `Move to…`, un `Dialog` con le cartelle del progetto ad albero, lette una cartella alla volta all'espansione, e `Move` / `Cancel`.
      - **DELETE_ENTRY**: `AlertDialog` `Move "<name>" to the trash?` più `<n> open files have unsaved changes that will be lost.` quando schede su di essa hanno modifiche non salvate; `Move to Trash` (destructive) e `Cancel`. Va nel cestino di sistema con il crate `trash` (da aggiungere, e in `specs#trash`); se il cestino rifiuta o non c'è, niente viene eliminato. Le schede sulla voce o su file al suo interno si chiudono.
      - **Rifiuti** (controllati di nuovo nel backend all'esecuzione; un comando rifiutato non cambia nulla; la UI mostra un toast di errore col motivo, es. `A file or folder named "<name>" already exists here.`, `The agent is editing this file.`, `Could not move "<name>" to the trash: <reason>`):
        - nome non valido: vuoto, `.` o `..`, contiene `/`, `\` o NUL, oppure su Windows contiene `<>:"|?*`, finisce con spazio o punto, o è un nome riservato (`CON`, `NUL`, `COM1`…);
        - nome già presente nella cartella di destinazione, confrontato come fa il file system (senza distinzione di maiuscole su macOS e Windows); un rename che cambia solo le maiuscole è permesso;
        - creare dentro `.weaver/`; rinominare, spostare o eliminare `.weaver/` o qualcosa al suo interno; spostare qualcosa dentro `.weaver/`;
        - spostare una cartella in se stessa o in una sua sottocartella;
        - rinominare, spostare o eliminare un file con lock dell'agente, o una cartella che ne contiene uno.

      ## Backend
      - Listing, lettura, scrittura, creazione, rename, spostamento ed eliminazione vivono in `weaver-core` (I/O `std` sincrono); `src-tauri` li espone come comandi Tauri in `spawn_blocking`. La UI non tocca mai un file e non riceve il plugin `fs`.
      - **Percorsi**: relativi alla radice con `/`. Rifiutati: assoluti, parti `..`, qualsiasi percorso dentro `.git`, e percorsi la cui forma canonica (`dunce`, senza maiuscole su Windows) esce dalla radice canonica. Un link simbolico con destinazione fuori dalla radice è elencato ma non si può aprire, scrivere, rinominarci dentro o spostarci dentro.
      - **Listing**: una cartella alla volta con `read_dir`, solo nomi e tipi (file, cartella, link simbolico), mai ricorsione, indice o ricerca, mai aprire un file. `.git` escluso per nome; gli ignorati da git esclusi con un `git check-ignore -z --stdin` per cartella con `GIT_OPTIONAL_LOCKS=0`; senza git o fuori da un repository nulla è ignorato. Gli altri nomi col punto sono elencati (diversamente da `folder_tree::list`, che li salta tutti). Cartelle prima, poi file, per nome senza distinzione di maiuscole.
      - **Osservazione**: un watcher `notify` per finestra per la vista File, separato da quello del progetto: una watch non ricorsiva per cartella espansa, radice inclusa, e una per file aperto in scheda. Eventi raggruppati per cartella, 150 ms dopo l'ultimo; un gruppo rilegge la cartella. File temporanei delle scritture atomiche ignorati. Nessuno di questi eventi raggiunge la picture, il suo batcher o `workspace.FILES_CHANGED`. Su Linux un `ENOSPC` di inotify passa al poll watcher come per il progetto.
      - **Stato**: per finestra, solo in memoria: stato della vista, cartelle espanse e loro voci, schede (percorso, hash letto, fine riga, BOM, motivo di sola lettura) e lock. Niente su disco, nella picture o in `project-changed`. La UI rende ciò che il backend invia e tiene solo i testi degli editor e la loro cronologia di annullamento.
      - **Eventi** al UI quando una cartella elencata cambia, quando il file di una scheda cambia, sparisce o ricompare, e quando un lock è preso o rilasciato; ognuno porta l'id di progetto della finestra e un numero di generazione. Tipi generati con `ts-rs`.
      - Il watcher del progetto resta com'è: radice non ricorsiva solo per `.weaver`, `.weaver/` ricorsivo, nessun codice; i file del codice non entrano mai nella picture e non causano rebuild.

      ## Qualità
      - A vista chiusa nessuna cartella del codice letta e niente osservato fuori da `.weaver/` oltre alla watch della radice per `.weaver`. Mostrare la vista legge solo la radice; espandere solo quella cartella; espandere avvia al massimo un `git check-ignore`, nient'altro della vista avvia processi.
      - 10 s dopo che la vista smette di essere mostrata non resta in memoria nessuna watch o listing del codice; testato con clock iniettato.
      - Test Rust su cartelle temporanee con temporizzazione del watcher iniettata: listing (`.git` escluso, ignorati, nomi col punto, senza git), watch aggiunte e tolte su espansione, chiusura cartella, rilascio e chiusura finestra, tutti i rifiuti, rename/move che non sostituiscono mai, eliminazione nel cestino e suo rifiuto. Test Vitest: albero, context menu, `F2`, `Delete`, trascinamento, `Move to…` e dialogo di uscita, con percorsi da tastiera e nomi accessibili; aggiornare `ORDER` in `sidebar.test.tsx` a Board, Chat, Jobs, Files, Requirements, Specs, History, Settings.

      ## CHANGELOG
      Aggiungere sotto `## [Unreleased]` una riga per l'intera vista File (albero, editor in schede con colori della sintassi, lock dell'agente), come chiede `requirements/docs`.

      ## Da dove partire
      - Sidebar e viste: `ui/src/views.ts` (`VIEWS` con `group: "top"`, `ViewId`; la Chat ha id `general`; `USAGE_ENTRY` non è una vista), `ui/src/components/sidebar.tsx`, `ui/src/components/sidebar.test.tsx` (`ORDER`).
      - Instradamento viste: `src-tauri/src/windows/manager.rs` (`ShowView`) → `ui/src/bindings/ShowView.ts`; `ui/src/components/project-window.tsx` (`ProjectWindow`: le viste restano montate con `hidden`, `show-requested`; qui va intercettata l'uscita dalla vista File col dialogo, e la vista File va smontata invece di restare nascosta); `ui/src/components/project-content.tsx` (`ProjectShell`) e `ui/src/components/project/project-gate.tsx` (`ProjectGate`); `ui/src/components/view-shown.ts` (`useViewShown`).
      - Albero precedente: `crates/core/src/project_files/folder_tree.rs` (`list`, che salta i nomi col punto) → `src-tauri/src/project_setup.rs` (`project_folder`) → `ui/src/lib/backend.ts` (`project_folder`); UI `ui/src/components/project/import-panel.tsx`.
      - Watcher: `src-tauri/src/windows/watch.rs` (`NotifyWatch::start`, `run`, `route`, `Batcher`, `WatchGuard`); durata in `src-tauri/src/windows/manager.rs` (`watchers`, `open_reserved`, chiusura, uscita).
      - Picture (da non toccare con il codice): `src-tauri/src/pictures.rs`.
      - Lavoro su disco: `src-tauri/src/blocking.rs` (`disk`, `disk_named`); file temporanei: `crates/core/src/atomic.rs` (`is_temp_name`).
      - Nuovi comandi: `src-tauri/src/commands.rs`, `CommandName`, `generate_handler!` in `lib.rs`, `COMMANDS` in `build.rs`, `capabilities/default.json`, tipi in `bindings.rs`.
      - Dialoghi e toast: `ui/src/components/ui/alert-dialog.tsx`, `ui/src/components/dialogs.tsx`, `ui/src/components/ui/sonner.tsx`; `ui/src/components/ui/context-menu.tsx` non esiste ancora.
  o8lkmkhy:
    title: Barra disabilitata nei sub-agent
    refs:
    - job
    - requirements/ui#job
    technical: false
    proposal: 1ghbqgki
    label: Sub-agent message box
    finished: 2026-11-03T08:44:51.703Z
    body: |-
      La conversazione di un sub-agent resta di sola lettura (il developer la legge mentre il job lavora e dopo, finché il job è in elenco), ma in basso mostra sempre la barra per scrivere, disabilitata, per coerenza visiva con la conversazione di un job.

      A lavoro finito vale quanto segue, come dice `requirements/ui#job` (**Sub-agent conversation**):

      - In fondo alla vista del sub-agent, fissata in basso, c'è una barra con la stessa disposizione della barra di un job: casella di testo e `Send` (`ArrowUp`), entrambi disabilitati, segnaposto `Sub-agents take no messages.`, nessuno `Stop`.
      - Vale per tutte e tre le viste dei sub-agent, che oggi condividono la stessa vista: sub-agent di un job (Jobs view), della Chat (General, con `Back to Chat`) e di una conversazione Ask more. Il testo è lo stesso ovunque.
      - La barra è sempre la stessa barra disabilitata, qualunque sia lo stato del sub-agent o del job, della Chat o dell'Ask more a cui appartiene, anche quando sono finiti: mai la riga `This job has ended. Continue in Chat.` con `Open Chat`.
      - Compare anche quando la conversazione non si può leggere: sotto l'intestazione resta solo `Conversation not available: <reason>.`, e la barra disabilitata resta in basso.
      - La casella è disabilitata davvero: fuori dall'ordine di tabulazione e annunciata come non disponibile, come la casella di un job in coda. Nessun messaggio può essere inviato da questa barra.
      - Restano assenti `Stop`, la scheda dei permessi, le note e la riga di fine.

      Da aggiornare anche i test esistenti che oggi verificano l'assenza della barra nella vista di un sub-agent (Jobs view e Chat) e il commento della vista che dice "No message box".

      Riferimenti nel codice trovati in esplorazione:

      - `requirements/ui#job` — `ui/src/components/jobs/jobs-view.tsx` `SubAgentConversationView` (vista condivisa da job, Chat e Ask more) e `MessageBox` (barra del job, privata al file; richiede un job, quindi serve una variante solo visiva o una barra presentazionale condivisa)
      - `general-chat` — `ui/src/components/general/general-view.tsx` mostra `SubAgentConversationView` al posto della Chat
      - `ask-conversation` — `ui/src/components/jobs/jobs-view.tsx` apre i sub-agent dell'Ask more con la stessa vista
      - test — `ui/src/components/jobs/jobs-view.test.tsx` e `ui/src/components/general/general-view.test.tsx` (asserzioni di assenza della barra nella vista sub-agent)
      - `ui/src/components/ui/textarea.tsx` — il `Textarea` condiviso ha già lo stile `disabled:`
---
# Vista File con albero ed editor

La finestra di un progetto, oltre alle viste in sola lettura, ha la vista File (Feature `files`) per piccole modifiche accanto all'editor dello sviluppatore.

- **Albero:** mostra file e cartelle del progetto senza ciò che git ignora e senza `.git`; `.weaver/` è visibile ma in sola lettura. Crea, rinomina, sposta ed elimina (l'eliminazione va nel cestino del sistema).
- **Schede ed editor:** CodeMirror 6 a schede, con i colori della sintassi, senza completamento, diagnostica o language server.
- **Lettura e salvataggio:** Weaver legge solo le cartelle espanse e i file aperti; un salvataggio scrive solo se il file su disco è ancora quello letto dalla scheda, altrimenti lo sviluppatore sceglie cosa tenere.
- **Blocco dell'agente:** mentre un agente del progetto cambia un file aperto, la scheda è in sola lettura fino alla fine dei suoi strumenti su quel file; nei sub-agent la barra è disabilitata.
- **Chiusura:** la finestra con modifiche non salvate chiede conferma.
- **Principio:** Weaver non analizza il codice e a vista chiusa non legge nulla; il README descrive la nuova vista e i suoi moduli.
