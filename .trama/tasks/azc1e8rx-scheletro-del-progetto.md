---
column: done
technical: true
refs:
  - specs#languages
  - specs#frameworks
  - specs#repository
  - specs#data
  - specs#notes
  - requirements/tech
  - requirements/tech#General
drift: null
proposal: qmqh1o3p
agent: false
created: 2026-11-02T07:29:32.875Z
finished: 2026-11-02T07:46:47.203Z
---
# Scheletro del progetto

La base su cui poggiano tutti gli altri Task: il repository, il cargo workspace, l'app Tauri 2 e l'interfaccia vuota ma funzionante. Questo Task copre l'Area `requirements/tech` solo come base dell'architettura (`requirements/tech#General`); le sezioni legate alle singole Machine (watcher, round di domande, finestre, notifiche, omp e ACP) sono realizzate dai Task che le riguardano.

## Repository

- Repository GitHub `nicchia-code/weaver`, con i rami `dev` (dove arriva il lavoro) e `main` (ciò che è rilasciato).
- `README.md` come pagina d'ingresso: cos'è Weaver, come si installa, e la nota delle Specs (`specs#notes`): le build non sono firmate né notarizzate per ora; firma e notarizzazione su macOS e firma su Windows arriveranno con una Proposal dedicata. Le istruzioni per aprire le build non firmate le completa il Task "Rilascio e aggiornamenti".

## Cargo workspace e linguaggi (`specs#languages`, `specs#frameworks`)

- Rust stable, edition 2021; TypeScript 5 in modalità `strict`, moduli ES, `noUncheckedIndexedAccess`, verificato con `tsc --noEmit`.
- `crates/core`: libreria `trama-core`, senza dipendenza da Tauri né runtime async, solo I/O `std` sincrono. Contiene i tipi del protocollo CLI↔app e deriva dall'identificatore Tauri `dev.nicchia.weaver` le cartelle di config, dati locali e runtime, così la CLI le trova anche ad app chiusa.
- `crates/cli`: binario `trama` (clap, serde), qui solo l'ossatura con il client del socket.
- `src-tauri`: crate dell'app `weaver` su Tokio; il lavoro su disco gira in `spawn_blocking`, mai sul main thread né sul thread IPC.
- `ui/`: Vite + React 19 + TypeScript, pnpm, shadcn/ui (base zinc, primario blu), Radix UI, Tailwind CSS 4, lucide-react, class-variance-authority, clsx, tailwind-merge.
- `trama` incluso come sidecar Tauri `externalBin`; a ogni avvio Weaver lo copia in `<app local data>/bin/` quando il contenuto differisce (percorso stabile anche sotto AppImage); quella cartella contiene solo `trama`.

## Tauri, IPC e dati (`specs#data`)

- Plugin `single-instance` (registrato per primo), `window-state`, `notification`, `dialog`, `opener`, `updater`, chiamati da Rust. Nessun plugin `deep-link`, `shell`, `fs` o `http`. La capability della webview concede solo i comandi di Weaver e i permessi core di eventi e finestre; la CSP non ammette origini remote.
- Ossatura backend ↔ UI: le azioni sono comandi Tauri che restituiscono un risultato o un errore tipizzato, gli aggiornamenti sono eventi del backend (per esempio `project-changed` con id del progetto, revisione e porzioni cambiate). I tipi TypeScript di comandi, eventi e payload sono generati dai tipi Rust con `ts-rs`; un controllo fallisce se il file generato non è aggiornato. La UI non tiene stato di dominio.
- Dati solo in file: `.trama/` nel progetto, cartella di configurazione del sistema per impostazioni e stato locale, portachiavi del sistema come unica sede ammessa per segreti (nessuno nella prima versione). Nessun database.

## Canale CLI ↔ app

- Crate `interprocess`, nessuna porta TCP o HTTP. Unix: socket `weaver-<8 casuali>.sock` in una cartella `0700` (`$XDG_RUNTIME_DIR/weaver/`, altrimenti `<app local data>/run/`, oppure `/tmp/weaver-<uid>/` se il percorso supererebbe 100 byte). Windows: named pipe `\\.\pipe\weaver-<16 casuali>`.
- A ogni avvio l'app scrive in modo atomico `<app local data>/runtime.json` (modo `0600`) con `{endpoint, secret, pid, version}`. Il segreto: 32 byte dal CSPRNG, in hex, nuovo a ogni avvio, mai in variabili d'ambiente, argomenti, log o `.trama/`; confrontato a tempo costante, uno sbagliato o assente chiude la connessione senza risposta.
- Messaggi JSON separati da newline, al massimo 1 MiB, una richiesta per connessione, ciascuna con una versione del protocollo; con versione diversa la CLI si comporta come se nessuna app rispondesse.

## Scritture e processi

- Una coda seriale di mutazioni per progetto. Ogni mutazione tiene un lock consultivo del sistema (`fd-lock`: `flock` / `LockFileEx`) su `<app local data>/locks/<project hash>.lock`, lo stesso che prende la CLI quando scrive da sé; `<project hash>` sono i primi 16 caratteri hex dello SHA-256 della radice canonica. Sotto il lock la mutazione rilegge da disco, valida il risultato e poi scrive; un rifiuto non scrive nulla. Lock non ottenuto entro 10 s: errore "project busy".
- Ogni processo figlio parte senza shell, con percorso del programma e array di argomenti; nessun testo dell'utente interpolato in una riga di comando; su Windows `CREATE_NO_WINDOW`. Una sola funzione di avvio condivisa.

## Fatto quando

- `cargo build --workspace` e `pnpm build` in `ui/` riescono; l'app si avvia e mostra una finestra con l'interfaccia di base.
- Test: la CLI ad app chiusa trova le stesse cartelle dell'app; un client con segreto sbagliato non riceve risposta; un messaggio oltre 1 MiB o di altra versione è rifiutato; due mutazioni concorrenti (app e CLI) sullo stesso progetto si serializzano e un lock non ottenuto in 10 s dà "project busy".
- Il controllo dei tipi generati con `ts-rs` fallisce su un file non aggiornato.
- Il README esiste con la nota sulle build non firmate.
