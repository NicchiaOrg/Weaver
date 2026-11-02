---
technical: false
refs:
  - question-round
  - requirements/tech#question-round
  - requirements/cli#question-round
  - requirements/rules#question-round
  - requirements/ui#question-round
---
# Round di domande

Dipende da "Scheletro del progetto", "Riga di comando trama" (canale socket con l'app) e "Lavori dell'agente"; la domanda preparata di un Drift viene da "Drift".

Un `question-round` è un gruppo di decisioni che l'agente di un job chiede al developer insieme: ogni domanda ha opzioni, una raccomandazione, scelta singola o multipla, un contesto facoltativo e sempre spazio per una risposta libera. È anche il modo in cui Weaver stesso pone la domanda preparata di un Drift, senza agente. Un round appartiene al suo job e al suo progetto, non compare mai nella finestra di un altro progetto; né Weaver né l'agente fissano un limite di tempo. Domande e risposte non sono mai scritte nel progetto.

## Machine `question-round`
- **Not asked**: `agent.ASK_QUESTIONS` → Unavailable se chi chiama non è l'agente di un job di Weaver che lavora o Weaver non gira; → Queued se un altro round del progetto è presentato; → Presented altrimenti, con la finestra del progetto in primo piano. `developer.RESOLVE_DRIFT` porta la domanda preparata in Queued o Presented allo stesso modo.
- **Queued**: aspetta i round precedenti, il più vecchio prima, senza sostituire quello presentato. `CLOSED` → Presented quando è il più vecchio in attesa; `agent.STOP_WAITING` → Abandoned; `job.ENDED` e `window.CLOSED` → Interrupted.
- **Presented** (emette `OPENED`): risposte confermate e non confermate sono tenute da Weaver, così il round resta com'era quando il developer si sposta, lo riduce o ci torna. `developer.SEND_ANSWERS` → Answered solo con ogni domanda risposta o saltata; `developer.SKIP_ALL_QUESTIONS` → Answered saltando quelle senza risposta confermata; `agent.STOP_WAITING` → Abandoned; `job.ENDED` e `window.CLOSED` → Interrupted. Ogni fine emette `CLOSED`; l'invio emette `ANSWERED`.
- **Answered**: l'agente riceve ogni risposta, le saltate marcate come tali; per ognuna usa la raccomandazione e la elenca come decisione da verificare. Una domanda preparata di Drift saltata lascia il Drift aperto.
- **Abandoned**: nessuna risposta consegnata. **Interrupted**: risposte scartate; l'agente, se c'è ancora, ne è informato e ferma il lavoro che aspettava senza usare raccomandazioni. **Unavailable**: l'agente sa che Weaver non può chiedere, non deve chiedere in altro modo e deve fermarsi e dirlo al developer.

## Requisiti
- `requirements/cli#question-round`: `trama question new --input -` stampa solo JSON; valida tutto prima di contattare l'app, poi blocca senza limite di tempo. Esiti mai confusi: `answered` (exit 0), `unavailable` con il testo "Weaver cannot show questions for this command. Do not ask in any other way: stop and tell the developer." (exit 0), `invalid-input` (exit 2, nomina domanda e campo), `interrupted` e `cancelled` (exit 1). `--help` chiede testi autosufficienti, niente prima persona, niente opzione "Other", esecuzione senza limite di tempo.
- `requirements/tech#question-round`: invio di `{secret, job, request}` a `TRAMA_ENDPOINT`, nessun timeout da nessuna parte; instradamento solo con `TRAMA_JOB` (sub-agent e worktree arrivano al progetto del job); l'app accetta round solo da job Running o Waiting for the developer; una coda per progetto creata all'apertura. SIGINT, SIGTERM o stdout chiuso chiudono la connessione: è `agent.STOP_WAITING`. Stato e bozze solo in memoria del backend (bozze di testo debounced a 300 ms), mai su disco né nei log.
- `requirements/rules#question-round`: validazione intera con percorso del campo (`questions[1].options[0].label`): almeno 1 domanda, id unici, almeno 2 opzioni con label e description non vuote, label uniche, `recommended` nell'intervallo, `context`/`details` non vuoti. Un round presentato per progetto, gli altri FIFO per istante di invio, domande preparate incluse; i progetti non si aspettano. Il backend rifiuta l'invio con domande mancanti; una saltata non porta nulla.
- `requirements/ui#question-round`: overlay modale su ogni vista (Esc e clic fuori inerti), intestazione `Questions from the agent · <job name>` o `Question about <Drift id> · <Drift title>`, `+<k> more round(s) waiting`, indice con stati `Answered`/`Skipped`/`Missing`, opzioni con tasti `1`–`9` e `Recommended` preselezionata solo alla prima visita, risposta libera, `Back`/`Skip all`/`Skip`/`Next`/`Review`, conferma Skip all, riepilogo con `Send to agent`, barra minimizzata.

## Fatto quando
- Test core su validazione, coda FIFO per progetto e risultato con skip.
- Test di `trama question new` per ogni esito ed exit code, incluso un round lasciato senza risposta a lungo.
- Chiusura della finestra o fine del job interrompono i round; Ctrl+C li abbandona.
- Test Vitest dell'overlay con tastiera, riduzione e ritorno senza perdere le bozze.
