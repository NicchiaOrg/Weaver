---
created: 2026-11-02T21:58:50.468Z
updated: 2026-11-02T21:58:50.468Z
summary: Nuova finestra Usage con i limiti d'uso dei provider connessi a omp; Settings passa in fondo alla barra laterale.
tasks:
- hyna2s1o
- cvoe3spr
cleared:
  cvoe3spr:
    title: Settings in fondo alla barra
    refs:
    - requirements/ui#General
    technical: true
    proposal: 26mz2o3f
    label: Usage
    finished: 2026-11-02T21:31:03.251Z
    body: |-
      Nella finestra di un progetto la barra laterale occupa tutta l'altezza della finestra e ha due gruppi di voci:

      - in alto, in quest'ordine: Board, Chat, Jobs, Requirements, Specs, History;
      - in basso a sinistra, staccato dal gruppo in alto dallo spazio libero tra i due: Settings, che resta l'ultima voce della barra.

      Settings continua ad aprire la sua vista a destra come oggi, con la stessa icona `Settings`, e il cambio di voce conserva lo stato di ogni vista (focus, overlay, scroll). Larghezza 208 px; sotto i 1024 px di larghezza della finestra la barra mostra solo le icone, 56 px, e ogni voce, Settings compreso, ha il suo nome come tooltip. I numeri sulle voci Board e Jobs restano come sono.

      Il gruppo in basso è pensato per contenere anche la voce Usage, sopra Settings: quella voce, con l'icona `Gauge` e il Dialog che apre, è il Task della finestra Usage. Qui basta che il gruppo in basso possa ospitare voci che non sono viste, cioè che aprono qualcosa senza diventare la voce selezionata e senza `aria-current`.

      Dove nel codice: l'elenco `VIEWS` in `ui/src/views.ts` (ordine e icone, Settings in fondo), il componente `Sidebar` in `ui/src/components/sidebar.tsx` (oggi un'unica `<ul>` senza parte in basso), il layout in `ui/src/components/project-window.tsx`. Aggiornare i test di `ui/src/components/sidebar.test.tsx` che controllano l'ordine delle voci, e aggiungere una riga sotto `## [Unreleased]` di `CHANGELOG.md`.
  hyna2s1o:
    title: Finestra Usage
    refs:
    - project-setup
    - omp
    - usage
    - developer.OPEN_USAGE
    - developer.RETRY_USAGE
    - developer.CLOSE_USAGE
    - omp.USAGE_READ
    - omp.USAGE_FAILED
    - requirements/ui#usage
    - requirements/tech#usage
    - requirements/tech#omp
    technical: false
    proposal: 26mz2o3f
    label: Usage
    finished: 2026-11-02T21:44:12.790Z
    body: |-
      ## Obiettivo

      Da ogni finestra di progetto lo sviluppatore apre Usage e vede quanto resta dei limiti d'uso dei provider a cui omp è connesso e quando ciascun limite si azzera. Weaver legge i limiti con `omp usage --json` solo all'apertura, mai in background, mai con un timer e mai chiamando un modello. Nessun consumo di token, nessun badge sull'entry, nessun pulsante Refresh. La finestra di benvenuto non ha Usage.

      ## Voce Usage nella sidebar

      - In `ui/src/views.ts` e `ui/src/components/sidebar.tsx`: nuova entry `Usage`, icona `Gauge` (lucide), nel gruppo in basso a sinistra, sopra Settings. Lo spostamento di Settings nel gruppo in basso è un altro Task: questo Task aggiunge solo Usage in quel gruppo (se il gruppo non c'è ancora, Usage va comunque subito sopra Settings, in fondo alla lista).
      - Usage non è una view: non entra in `ViewId`/`VIEWS` come view selezionabile, non diventa mai l'entry selezionata, non ha mai `aria-current`. Il click apre il dialog sopra la view mostrata (stato del dialog in `ui/src/components/project-window.tsx`).
      - Sotto 1024 px (sidebar compatta, solo icone) l'entry ha il tooltip `Usage` come le altre (`Entry`, Tooltip side=right).
      - Test in `ui/src/components/sidebar.test.tsx`: ordine (Usage subito prima di Settings), niente `aria-current`, tooltip in modalità compatta, il click apre il dialog e non cambia view.

      ## Machine `usage`

      Stati e transizioni come nel Model:

      - **Closed** (iniziale): `developer.OPEN_USAGE` → **Reading** solo se omp è Ready (Weaver chiede i limiti dei provider connessi); → **Waiting for omp** se omp non è pronto (mancante, in installazione, in aggiornamento, in login o connesso a nessun provider).
      - **Waiting for omp**: il dialog dice perché omp non è pronto con il testo del box di stato di omp per lo stato corrente (`ui/src/components/omp-status.tsx`) e offre `Open Settings`. `omp.READY` → **Reading** subito, senza chiudere e riaprire il dialog (stato omp da `useOmp` in `ui/src/hooks/use-omp.tsx`). `developer.CLOSE_USAGE` → **Closed**; se lo sviluppatore ha scelto `Open Settings`, il dialog si chiude e la finestra mostra Settings (`showView` di `ui/src/components/project-context.tsx`).
      - **Reading**: Weaver aspetta la risposta di omp. I limiti letti per ultimi, se ci sono, restano visibili marcati come in aggiornamento. `omp.USAGE_READ` → **Shown**: i nuovi limiti sostituiscono i precedenti in tutte le finestre. `omp.USAGE_FAILED` → **Failed**, anche quando omp non risponde entro 30 s; i limiti precedenti, se ci sono, restano visibili con l'ora della lettura. `developer.CLOSE_USAGE` → **Closed**: la lettura prosegue e il suo risultato conta come ultimo letto per la prossima apertura.
      - **Shown**: ogni provider connesso, nell'ordine del setup di omp, con i suoi limiti; un provider connesso senza limiti dice perché; se nessun provider ha limiti, Usage lo dice sopra i blocchi. `developer.CLOSE_USAGE` → **Closed**.
      - **Failed**: errore e `Retry`. `developer.RETRY_USAGE` → **Reading** se omp è Ready; → **Waiting for omp** se omp non è più pronto. `developer.CLOSE_USAGE` → **Closed**.
      - Un risultato letto per un percorso di omp che non è più quello corrente viene ignorato.

      ## Backend: lettura (`requirements/tech#omp`, `requirements/tech#usage`)

      - Nuova riga nella tabella dei comandi in `crates/core/src/omp/probe.rs` (o nuovo modulo `crates/core/src/omp/usage.rs` registrato in `crates/core/src/omp/mod.rs`): `omp usage --json`, limite 30 s, nuova costante `USAGE` = 30 s in `crates/core/src/omp/run.rs` accanto a `MODELS`/`LOGOUT`. Valgono le regole esistenti: niente shell, argomenti fissi, stdout parsato, stderr tenuto per le ragioni, processo ucciso al timeout. Ragioni di errore: ultima riga non vuota di stderr, altrimenti `exit code N`, `could not start omp` o `timed out`; errori di certificato/HTTP sostituiti dalla spiegazione dei System Certificates (come per gli altri comandi).
      - Mai `invalidate`, `reset`, `--history` né altre azioni che scrivono lo stato di omp. Ci si affida alla cache di omp.
      - In `src-tauri/src/omp.rs`: nuova operazione in `Ops` (accanto a `update_check`/`logout`) e comando Tauri `omp_usage` (accanto a `omp_status`/`omp_install`/`omp_log_in`), eseguito solo quando omp è Ready (`ready()`); una sola lettura alla volta per tutta l'app: un'apertura o un `Retry` durante una lettura attende quella in corso. Un dialog in Waiting for omp avvia la lettura quando lo stato omp globale diventa Ready.
      - Ultimo risultato: un unico valore app-wide in memoria nel backend (stato in `src-tauri/src/omp.rs` o `src-tauri/src/state.rs`), etichettato con il percorso di omp per cui è stato letto, mai scritto su disco; sostituito solo da una lettura riuscita (anche se il dialog è già chiuso) e scartato quando cambia il percorso di omp (`change_omp_path`). Ogni finestra lo riceve tramite un nuovo evento in `src-tauri/src/events.rs` (come `emit_omp_changed`), insieme allo stato di lettura (in corso / fallita con ragione); l'interfaccia non tiene una copia propria.

      ## weaver-core: parser, frazione usata, stato

      In `crates/core/src/omp/` (nuovo `usage.rs`), puro e testato:

      - Parsing di `reports[]`: `provider`, `fetchedAt` e per ogni `limits[]` `label`, `window.resetsAt`, `amount` (`used`, `limit`, `usedFraction`, `remainingFraction`, `unit`) e `status`; da `accountsWithoutUsage[]` e `disabledCredentials[]` solo `provider`. Campi sconosciuti ignorati. Email, account, organizzazione, plan e altri campi d'identità (`metadata`) non vengono mai parsati, tenuti o inviati all'interfaccia. Output non JSON valido di questa forma → errore con ragione `omp did not return valid usage`.
      - Provider: solo la lista fissa di `crates/core/src/omp/providers.rs` (openai-codex, anthropic, github-copilot, google-gemini-cli, google-antigravity, cursor, xai-oauth), nel suo ordine, e solo quelli connessi (come in `crates/core/src/omp/machine.rs`, connessi dai modelli); i report di altri provider vengono scartati. Più account dello stesso provider: i limiti uno dopo l'altro nello stesso blocco. Provider connesso senza report: `No usage data` se è in `accountsWithoutUsage`, `Login expired` se è in `disabledCredentials`, altrimenti `Not reported by omp`.
      - Frazione usata: `usedFraction`, altrimenti `used / limit` se `limit` > 0, altrimenti `used / 100` se l'unità è `percent`, altrimenti `1 − remainingFraction`, altrimenti sconosciuta; limitata a 0–1 per la barra.
      - Stato: `status` di omp (`ok`, `warning` = quasi esaurito, `exhausted` = esaurito); senza, quasi esaurito da 0.8 e esaurito da 1 della frazione usata.
      - Ora dei dati: la più vecchia `fetchedAt` dei provider mostrati.
      - Unit test su un campione come l'output reale di omp 18.16.0 (anthropic `Claude 5 Hour` 17% percent e `Claude 7 Day`; github-copilot `Premium Requests` 3437/100000 requests; cursor `Personal Usage` 0.5/1000 usd; con `metadata` con email/accountId, `accountsWithoutUsage`, `disabledCredentials`, `capacity`, un provider non in lista), più i casi della frazione usata, dello stato, del JSON non valido e della mancata esposizione di identificatori.
      - Tipi esportati con ts-rs in `src-tauri/src/bindings.rs` (come `ProviderRow`/`OmpModel`).

      ## Interfaccia: dialog (`requirements/ui#usage`)

      Nuovo componente (es. `ui/src/components/usage-dialog.tsx`), montato in `ui/src/components/project-window.tsx`:

      - shadcn `Dialog` (`ui/src/components/ui/dialog.tsx`, larghezza max 480 px via className), titolo `Usage`; Esc, `X` o click fuori chiudono e il focus torna all'entry `Usage`.
      - Un blocco per provider connesso, con il nome breve in grassetto. Riga di limite: nome come lo dà omp, shadcn `Progress` della frazione usata con nome accessibile `<name>: <value>`, valore e reset.
        - Valore per unità: percent `17% used`; requests `3,437 / 100,000 requests`; tokens `<used> / <limit> tokens`; credits `<used> / <limit> credits`; minutes `<used> / <limit> min`; USD `$0.50 / $1,000.00`. Senza limite, solo l'usato; senza frazione nota, niente barra e `Usage unknown`.
        - Reset: `Resets in <duration>` se mancano meno di 24 h, altrimenti `Resets on <date>` (formati di `requirements/ui#General`); niente se omp non dà un'ora di reset.
        - Stato: quasi esaurito in `amber` con `Almost used up`, esaurito in `red` con `Used up` (600 in chiaro, 400 in scuro); gli altri neutri.
      - Provider senza limiti: riga secondaria `No usage data`, `Login expired` o `Not reported by omp`. Nessun limite: `No usage limits to show.` sopra i blocchi.
      - Riga attenuata sotto il titolo `Updated at HH:MM`.
      - Reading: con limiti precedenti restano visibili e la riga dell'ora mostra `Loader` e `Updating…`; senza, `Loader` centrato con `Reading usage…`. `aria-busy="true"` durante la lettura; `Loader` statico con reduced motion.
      - Failed: `Alert` destructive `Could not read usage: <reason>.` con pulsante `Retry` `outline` small, icona `RotateCw`; sotto, i limiti precedenti con la loro ora.
      - Waiting for omp: testo del box di stato omp e pulsante `Open Settings`.
      - Test del componente per ogni stato, unità, reset, stati di colore, testi dei provider senza limiti.

      ## Changelog

      Una riga sotto `## [Unreleased]` in `CHANGELOG.md`: Usage nella sidebar apre un dialog con i limiti d'uso dei provider connessi a omp e il loro reset.
---
# Usage e Settings nella barra

La finestra Usage legge i limiti con omp, mostra barra, valore e reset per ogni limite e gestisce attesa, errore e ripetizione. Settings è ora un gruppo in fondo alla barra laterale, separato dalle viste.
