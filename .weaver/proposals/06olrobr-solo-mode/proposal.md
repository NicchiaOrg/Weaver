---
status: approved
kind: change
base:
  model/actors/developer.md: 0da4d0b57c787fa9d534cfb17260e8c33b09c60c06f6e3adf6a686d3fd256084
  model/files/feature.md: 862ad452941af7310c119a364ad067c55532436d9519c133c4b16bec6fcc74cc
  model/files/file-view.md: 010dfe487e75014cf23b73ac635830e50a51787646f9221d70a7d02cd077e68f
  model/project-setup/feature.md: 6ab1991d69bcb779aef74963f3f31d541f9a4d5d7636f5fb79b40755d1ca063b
  model/project-setup/project.md: 20fe5b7aef0f762aca8d7fca70b713e70e002c8e2a762a4f916e076d9440aa32
  model/project-setup/solo-mode.md: new
  requirements/cli/commands.md: de56732926ea9ee89cfa27bdfc7159f6a19e5740feb85c57cf7d63a0f891f42d
  requirements/data/project-files.md: c74a48b34ab04d973abe8861c6384ec18224a8e5bb673160d7768d4536005bf2
  requirements/data/settings.md: ef3e1ae5bc6b517be140d2d5c514fb0b15908fe0e9da34af0f3cac88bc8c3b0c
  requirements/docs/main.md: b59d504de88e5a8fbf0669b7b3e941133120a56886333f23a360e23a2c36e08f
  requirements/quality/main.md: e393a4d38e36764938c9e7bef627fb5ee512a8f926d8b896b1d6438b1c8ecbc4
  requirements/rules/skills.md: ccccf2336fe25823ac3671025f5ebc5c96463f14cfbfa5a6da6c56c5cd96ff55
  requirements/rules/solo-mode.md: new
  requirements/tech/files.md: 2b7f9d0de6b7d828446709e646ecec756156f87bded75257af5346557b74c0be
  requirements/tech/solo-mode.md: new
  requirements/ui/setup.md: a263360a44a9e922b46123745d922048c82bdf3837503dd4bf249dc1424784cd
  requirements/ui/solo-mode.md: new
created: 2026-11-04T09:01:23.826Z
request: 2026-11-04T09-38-44
label: Solo mode
job: yrgkoam4
---
# Solo mode

Permette di usare Weaver da solo in un repository dove gli altri non lo usano: con Solo mode acceso i file di Weaver restano solo in questa copia di lavoro e non entrano mai in un commit, grazie a un blocco di Weaver nelle esclusioni locali di git.

## Decisioni

- Stato per copia di lavoro, in `.weaver/solo/` (cartella locale con `.gitignore` `*`) con `solo.json`; un altro clone parte spento.
- Due modi: cartelle intere (`.weaver/`, `.agents/skills/`, `.omp/`), predefinito; oppure solo i file di Weaver, che esclude comunque `.weaver/` per intero più le skill `weaver-*`, `.omp/agents/weaver-*.md` e `.omp/WATCHDOG.yml`.
- Un blocco per copia di lavoro nel file `git rev-parse --git-path info/exclude`: righe di inizio e fine con il percorso assoluto della copia; percorsi dalla radice del repository con il prefisso del progetto; le altre righe restano intatte; riscrivere lo stesso blocco non cambia il file.
- File già tracciati: accendere riesce lo stesso; Settings elenca i file tracciati che il modo esclude e avverte; nessuna azione per smettere di tracciarli. L'elenco si ricalcola a ogni apertura di Settings e dopo ogni cambio.
- Riparazione silenziosa a ogni apertura di un progetto inizializzato, anche con il Model vuoto; se fallisce lo dice solo Settings.
- Acceso ma senza git o fuori da un repository all'apertura: resta acceso, Settings avvisa, Weaver riprova alla prossima apertura.
- Senza git o fuori da un repository: spento e non modificabile, con il motivo.
- Spegnere: subito, senza conferma; toglie il blocco della copia e dimentica lo stato. Cambio di modo subito, senza conferma.
- Se `.weaver/` sparisce a progetto aperto, Weaver toglie il blocco della copia.
- La vista File continua a mostrare ciò che i blocchi di Solo mode escludono.
- Inizializzazione: scelta in Get Started con lo stesso controllo a tre voci di Settings, `Off` di partenza; il blocco si scrive prima di ogni file di Weaver; se non si riesce il progetto non viene creato e un fallimento successivo toglie il blocco. Il testo di Get Started segue la scelta.
- Settings, gruppo Project: controllo segmentato `Off` / `Whole folders` / `Weaver's files only`; disabilitato con nota in una cartella non inizializzata; avviso sui file non versionati in Settings e in Get Started; elenco dei file tracciati in un riquadro di avviso con percorsi monospace che scorrono oltre 8 righe; testi inglesi approvati.
- Riga di comando: `solo` (`off | folders | files`) in `project new` (fuori da git con un valore acceso: rifiuto exit 1, nulla creato), in `project edit` (con `solo` cambia solo Solo mode, senza resta Update skills) e in `project show` (stato completo accanto alla Board).
- Ogni chiamata a git ha un limite di 10 s.
- README: sezione `## Solo mode` dopo `## Install`, più una riga nel CHANGELOG.

## Assunzioni

- Un cambio di Solo mode che fallisce (git assente, esclusioni non scrivibili) non cambia nulla e Settings dice perché, anche quando Solo mode è acceso ma non riesce a escludere.
- La riparazione all'apertura e il controllo dei file dell'agente girano nello stesso lavoro in background del progetto; un esito della riparazione non produce il toast né il banner dei file dell'agente.
- Il Merge dal menu, lo scan dopo il merge e i Drift da merge restano fuori, per una Proposal successiva.
