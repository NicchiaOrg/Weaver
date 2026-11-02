---
technical: false
refs:
  - omp-setup
  - time
  - specs#infrastructure
  - requirements/tech#omp
  - requirements/tech#omp-setup
  - requirements/ui#omp-setup
---
# Controllo di omp

Realizza la Machine `omp-setup`, unica per tutta l'app: dice se omp è pronto. Nessun lavoro e nessuna General parte finché non lo è; un lavoro creato intanto aspetta. Dipende da "Scheletro del progetto" e legge `ompPath` da "Impostazioni".

## Machine `omp-setup`
- `Checking` (iniziale, all'avvio e quando qualcosa può aver cambiato omp): `omp.READY` → `Ready`, `omp.MISSING` → `Missing`, `omp.OUTDATED` → `Updating` (esegue l'update di omp), `omp.NO_PROVIDER` → `Not connected`.
- `Missing`: `developer.INSTALL_OMP` → `Installing` aprendo il terminale di sistema con l'installer ufficiale; `developer.CHANGE_SETTINGS` → `Checking` solo se il percorso di omp è cambiato.
- `Installing`: `time.RECHECK_DUE` ricontrolla; `MISSING` resta; `READY`, `OUTDATED`, `NO_PROVIDER` come in `Checking`; `INSTALL_OMP` riapre l'installer; `CHANGE_SETTINGS` come sopra.
- `Updating`: i lavori creati aspettano. `omp.UPDATE_ENDED` → `Checking`; `omp.UPDATE_FAILED` → `Ready` se omp risponde ancora con la versione minima e un modello (fallimento mostrato, non blocca), altrimenti → `Missing` con l'installer offerto e il fallimento.
- `Not connected`: elenca i fornitori; `developer.LOG_IN_PROVIDER` → `Logging in` aprendo `omp login <provider>` nel terminale; `CHANGE_SETTINGS` come sopra.
- `Logging in`: Weaver non legge mai ciò che il developer scrive nel terminale. `RECHECK_DUE` ricontrolla i modelli; `NO_PROVIDER` resta; `READY` → `Ready`; `MISSING` → `Missing`; `LOG_IN_PROVIDER` riapre il login.
- `Ready`: mostra versione, modelli e fornitori connessi. `LOG_IN_PROVIDER` → `Logging in`; `developer.LOG_OUT_PROVIDER` → `Checking` (omp dimentica solo le credenziali che ha salvato; se il fornitore resta connesso Weaver dice che altre credenziali dell'ambiente lo connettono); `CHANGE_SETTINGS` come sopra; `time.UPDATE_CHECK_DUE` chiede in background se esiste una versione nuova, senza mai far aspettare un lavoro; `omp.UPDATE_AVAILABLE` → `Updating` prima del prossimo lavoro, quelli in corso continuano; `MISSING` → `Missing`.
- Un risultato calcolato per un percorso di omp che non è più quello corrente è ignorato.

## Requisiti (`requirements/tech#omp`, `requirements/tech#omp-setup`)
- Ricerca: `ompPath` (se impostato, da solo), poi PATH, poi le cartelle dell'installer per sistema. PATH di login letto una volta all'avvio su macOS e Linux (`$SHELL -l -c`, 5 s); su Windows dal registro a ogni ricerca.
- `MIN_OMP_VERSION = 18.15.0` nel core, semver; versione illeggibile = vecchia. Comandi senza shell con limiti: `--version` 15 s, `models --json` 15 s (nessun modello = `NO_PROVIDER`), `update --check` 30 s, `update` 5 min, `auth-broker logout <provider>` 30 s. Motivi di fallimento dall'ultima riga di stderr, con suggerimenti per certificati e proxy.
- Terminale di sistema (scelta del developer): Terminal via `osascript` su macOS, PowerShell 5.1 con nuova console su Windows, primo emulatore disponibile su Linux; senza terminale si mostra il comando da copiare. Una sola funzione testata fa il quoting; si inseriscono solo il percorso assoluto di omp e un id fornitore della lista fissa. Installer `curl -fsSL https://omp.sh/install | sh` o `irm https://omp.sh/install.ps1 | iex`; Weaver non scrive mai `ompPath`.
- Ricontrollo ogni 3 s, uno alla volta, solo in `Installing` e `Logging in`. Controllo aggiornamenti al massimo una volta all'ora per tutta l'app, un solo `omp update` alla volta.
- Fornitori in ordine: `openai-codex`, `anthropic`, `github-copilot`, `google-gemini-cli`, `google-antigravity`, `cursor`, `xai-oauth`; connesso se un modello lo ha come `provider`. Cursor solo tramite login, nessun campo per chiavi API, nessun segreto salvato in v1.

## Interfaccia (`requirements/ui#omp-setup`)
Un unico box di stato (`aria-live="polite"`) in Settings, Get Started e finestra di benvenuto, con icona, testo e azione per ogni State come da tabella del Requisito; righe `Providers` con `Log in`/`Log in again` e `Log out` con conferma `Log out of <name>?`; errore `Could not open a terminal: <reason>.` con il comando e `Copy`.

## Fatto quando
- Test unitari della Machine con eventi e orologio iniettati (nessun sleep, nessun omp reale): ogni transizione, i ricontrolli che si fermano uscendo dallo State, il risultato di un percorso vecchio scartato.
- Test del quoting per i tre sistemi e della ricerca di omp su cartelle temporanee.
- Test UI del box per ogni State e del dialog di logout, tastiera inclusa.
