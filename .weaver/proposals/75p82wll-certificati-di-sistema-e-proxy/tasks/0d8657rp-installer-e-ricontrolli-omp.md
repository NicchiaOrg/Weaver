---
technical: false
refs:
  - omp-setup
  - omp.INSTALL_FAILED
  - requirements/tech#omp-setup
  - requirements/ui#omp-setup
---
# Installer, terminali e ricontrolli

Quando il lavoro è finito, la Machine `omp-setup` aspetta l'esportazione dei certificati all'avvio, i terminali dell'installer e del login ricevono le variabili di System Certificates, l'installer lascia il suo codice d'uscita in un marcatore e un installer fallito riporta omp a Missing con il suo testo; omp si ricontrolla dopo ogni esportazione e quando la modalità si spegne.

## Checking all'avvio
- All'avvio dell'app, mentre System Certificates esporta il bundle, il primo controllo di omp aspetta che l'esportazione finisca (al più 30 s, poi controlla comunque). Il riquadro di omp mostra `Checking omp…` con la riga secondaria `Exporting system certificates first…` solo durante quell'attesa.

## Ricontrolli
- `developer.CHANGE_SETTINGS` → Checking da Missing, Not connected e Ready quando cambia il posto di omp **oppure** System Certificates viene spento (da Installing solo per il posto di omp). Accenderlo non ricontrolla qui: lo fa la fine dell'esportazione.
- `system-certificates.EXPORT_ENDED` → Checking da Missing, Not connected e Ready (dopo ogni esportazione, riuscita o no). Negli altri stati non fa nulla.

## Terminali (`requirements/tech#omp-setup`)
- Con System Certificates acceso, le variabili (dalla funzione che restituisce le variabili dei certificati, gruppo `system-certificates`, lette all'apertura) arrivano al terminale:
  - Windows: nell'ambiente del processo `powershell.exe`;
  - macOS: il comando di `do script` inizia con `/usr/bin/env <NAME>=<bundle path> … ` e il resto gira sotto `/bin/sh -c`;
  - Linux: lo script `sh -c` inizia con `export <NAME>=<bundle path> …;`;
  - senza terminale, il comando da copiare include le variabili.
- Una sola funzione testata quota per la shell di destinazione; si inseriscono solo il percorso assoluto di omp, un id di provider della lista fissa, il percorso del bundle e quello del marcatore.

## Installer e `omp.INSTALL_FAILED`
- Non più `curl … | sh` / `irm … | iex`: l'installer si scarica in un file temporaneo e poi si esegue, così un download fallito conta come fallimento; poi scrive il codice del primo passo fallito, altrimenti `0`, in `<app local data>/omp-install/<nonce>.code` (decimale come testo) e cancella il temporaneo.
  - macOS/Linux, in `/bin/sh`: `curl -fsSL https://omp.sh/install -o <temp>`, poi `sh <temp>`.
  - Windows: `Invoke-RestMethod https://omp.sh/install.ps1 -OutFile <temp>.ps1`, poi un `powershell.exe -NoProfile -ExecutionPolicy Bypass -File <temp>.ps1` figlio; il codice è `$LASTEXITCODE`; un errore terminante del download vale `1`.
- Ogni `INSTALL_OMP` usa un nonce nuovo e cancella i marcatori precedenti (anche `Open installer again`, che dimentica il codice di prima); trovare omp li cancella. Weaver non scrive mai `ompPath`.
- In Installing, un ricontrollo che non trova omp legge il marcatore del nonce corrente: codice ≠ `0` è `omp.INSTALL_FAILED` → Missing; `0`, nessun marcatore o illeggibile è `omp.MISSING` (resta Installing). Il codice resta fino al prossimo `INSTALL_OMP` o finché omp è trovato.
- In Missing dopo un installer fallito, il riquadro mostra `The omp installer failed (exit code <N>); see the terminal. A corporate proxy that inspects HTTPS traffic is a common cause.` e, con l'impostazione spenta, ` Turning on System Certificates in Weaver's settings may fix it.`; azione `Install omp`.

## Riquadro (`requirements/ui#omp-setup`)
- Riga Missing: oltre ai motivi esistenti, una spiegazione del proxy (con `.` finale, prodotta dal Task di `requirements/tech#omp`) e il testo dell'installer fallito. Il resto della tabella non cambia.

## Da dove partire
- `omp-setup` — crates/core/src/omp/machine.rs (Event/State, nuovo evento per installer fallito ed export finito)
- `omp-setup` — src-tauri/src/omp.rs:`start` (reagisce solo a `SettingKey::OmpPath`), `change_omp_path`, `open_installer`, `open_login`
- `omp-setup` — src-tauri/src/lib.rs `.setup` e src-tauri/src/startup.rs (ordine "settings"/"omp" e attesa dell'export)
- `requirements/tech#omp-setup` — crates/core/src/omp/terminal.rs:`INSTALLER_UNIX`, `INSTALLER_WINDOWS`, `login_command`, `launch_plan`, `open`, `quote_posix`, `quote_powershell`, `applescript_string`
- `omp.INSTALL_FAILED` — crates/core/src/paths.rs:`AppDirs::local_data_dir` (cartella `omp-install/`)
- `requirements/ui#omp-setup` — ui/src/components/omp-status.tsx:`boxFace`, `OmpStatusBox`
