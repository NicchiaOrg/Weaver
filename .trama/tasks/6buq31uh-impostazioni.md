---
column: done
technical: false
refs:
  - developer.CHANGE_SETTINGS
  - requirements/data#developer.CHANGE_SETTINGS
  - requirements/ui#developer.CHANGE_SETTINGS
drift: null
proposal: qmqh1o3p
agent: false
created: 2026-11-02T07:29:32.875Z
finished: 2026-11-02T08:48:42.567Z
---
# Impostazioni

Realizza `developer.CHANGE_SETTINGS`: le impostazioni di Weaver, il loro file e la vista Settings. Dipende da "Scheletro del progetto"; i box di omp e Update skills che compaiono nella vista vengono da "Controllo di omp" e "Progetto e skill installate".

## File e schema (`requirements/data#developer.CHANGE_SETTINGS`)
- Un solo file `settings.json` nella cartella di configurazione dell'app (`app_config_dir`). Nulla nel progetto, nel repository o nell'ambiente è mai letto come impostazione.
- Uno schema tipato: struct Rust nel core con default e validazione; il tipo TypeScript è generato, mai scritto a mano. Il backend è l'unico lettore e scrittore.
- Chiavi e default: `ompPath` `""` (vuoto = cerca omp); `models.analysis`, `models.scan`, `models.execution` `""` (nome come omp lo accetta, suffisso `:<thinking level>` facoltativo, vuoto = default di omp; mai controllati contro una lista); `maxParallelAgents` `20` (≥ 1); `language` `""` (lingua dei testi dell'agente, testo libero; l'interfaccia resta sempre in inglese); `theme` `auto|light|dark`, default `auto`; `editor.preset` `vscode|cursor|zed|jetbrains|sublime|custom`, default `vscode`; `editor.command` (solo con `custom`, deve contenere `{file}`, `{line}` facoltativo); `permissions` `yolo` (default) o `ask`.
- Preset dell'editor: `code --goto {file}:{line}`, `cursor --goto {file}:{line}`, `zed {file}:{line}`, `idea --line {line} {file}`, `subl {file}:{line}`, programma cercato con le regole del PATH di omp. `custom`: diviso in argomenti con le regole POSIX, `{file}` (assoluto) e `{line}` (da 1, `1` se ignota) sostituiti dentro ogni argomento, eseguito senza shell; vuoto, non divisibile o senza `{file}` è non valido.
- Si salvano solo i valori diversi dal default, con `"version": 1`. File mancante = tutti i default; chiave sconosciuta ignorata e tolta alla scrittura successiva; chiave di tipo o valore errato = default, e Settings dice quale e perché. File illeggibile = tutti i default con l'errore in Settings; non si riscrive finché il developer non cambia qualcosa, e allora è prima rinominato `settings.invalid-<UTC timestamp>.json`.
- Scritture atomiche e serializzate; una scrittura fallita lascia in vigore il file e i valori precedenti e mostra l'errore. Un cambio vale per i processi omp avviati dopo; i lavori in corso tengono i valori con cui sono partiti.
- Segreti solo nel portachiavi (`keyring`, servizio `weaver`), mai in file, argomenti, log, UI o IPC; la v1 non ne salva nessuno.
- Nessun interruttore per le funzioni sempre attive: esecuzione parallela dei Task, Sub-agents Context Sharing, Advisor Scanner, Async Drift Resolver, modello per Proposal, raffinamento all'Approve.

## Vista Settings (`requirements/ui#developer.CHANGE_SETTINGS`)
- Colonna centrata di ~720 px, gruppi in ordine: `omp` (box di stato, `Providers`, `omp path`, `Permissions` con `Run without asking`/`Ask before acting`), `Models` (`Analysis`, `Scan`, `Execution` con i consigli in grassetto), `Agents` (`Max parallel sub-agents`, `Language of the agent's texts`), `Editor` (Select dei preset più `Custom command`), `Appearance` (`Theme` segmentato `Auto`/`Light`/`Dark`, applicato dal vivo a ogni finestra), `Project` (Update skills).
- Ruoli dei modelli: Combobox con testo libero, i modelli che omp elenca e `omp default`; Select del livello di thinking solo quando omp elenca livelli, valore salvato come `<model>:<level>`; se l'elenco manca, una riga dice perché.
- Salvataggio immediato senza conferma (select alla scelta, testo e numeri al blur o `Enter`). Un valore non valido non si salva e il campo dice perché (`Enter a whole number of at least 1.`, `The command must contain {file}.`); una scrittura fallita mostra `Could not save <title>: <reason>.` e rimette il valore salvato. Un cambio da un'altra finestra compare ovunque subito, tranne nel campo con il focus. Titoli, descrizioni (≤160 caratteri), default e validazione vengono dallo schema; le chiavi non si mostrano mai.

## Fatto quando
- Test su cartelle temporanee (mai le impostazioni reali): file mancante, chiave sconosciuta, valore errato, file illeggibile rinominato al primo cambio, ritorno al default che toglie la chiave.
- Test della divisione del comando personalizzato e dei preset dell'editor.
- Test UI della vista: ordine dei gruppi, validazione, errore di scrittura, cambio da un'altra finestra.
