---
column: done
technical: true
refs:
  - specs#deployment
  - requirements/release#General
drift: null
proposal: qmqh1o3p
agent: false
created: 2026-11-02T07:29:32.875Z
finished: 2026-11-02T08:42:44.629Z
---
# Rilascio e aggiornamenti

Il workflow `release` su GitHub Releases e gli aggiornamenti di Weaver dall'app. Dipende da "Scheletro del progetto" e da "CI e test", perché il rilascio parte solo dopo un `ci` riuscito su `main`. Decisione dell'intervista: per ora build non firmate, la soluzione più veloce; firma e notarizzazione su macOS e firma su Windows arriveranno con una Proposal dedicata.

## Workflow `release` (`requirements/release#General`)

- File `.github/workflows/release.yml`, avviato da `workflow_run` di `ci` completato sul ramo `main` con esito `success`; lavora sullo `head_sha` di quella esecuzione. Mai per `dev` o pull request, mai su un commit i cui test sono falliti.
- Primo job (15 min): legge `version` da `src-tauri/tauri.conf.json` e cerca `refs/tags/v<version>` su `origin`. Se il tag esiste il workflow finisce con successo dopo un `::notice::` che dice che il rilascio di `v<version>` è saltato perché il tag esiste: il push su `main` ha fatto solo i test. Altrimenti crea una GitHub Release in bozza `Weaver v<version>` per il tag `v<version>` su `head_sha` (la bozza non crea il tag).
- Job di build (25 min ciascuno) con `tauri-apps/tauri-action`, che caricano sulla bozza i file con i nomi che Tauri produce per il prodotto `Weaver`: `macos-latest` target `universal-apple-darwin` → `.dmg` e `.app.tar.gz` + `.sig`; `windows-latest` target `x86_64-pc-windows-msvc` → installer NSIS `-setup.exe` + `.sig` (niente MSI); `ubuntu-22.04` target `x86_64-unknown-linux-gnu` → `.AppImage` + `.sig` e `.deb`.
- `trama` compilato per lo stesso target dell'app e incluso come sidecar; su macOS le build arm64 e x64 unite con `lipo` in un binario universale.
- `latest.json` dell'updater (piattaforme `darwin-aarch64`, `darwin-x86_64`, `windows-x86_64`, `linux-x86_64`) allegato alla release.
- Un job finale pubblica la bozza, e così crea il tag `v<version>` su `head_sha`, solo se tutti i job di build sono riusciti; se uno è fallito o annullato la bozza è cancellata e nessun tag nasce. Il workflow non sposta, cancella né ricrea mai un tag e non modifica mai una release pubblicata.
- Note di rilascio: per ogni piattaforma il nome esatto del file da scaricare e i passi per aprire una build non firmata.

## Firma

- Artefatti dell'updater firmati con la chiave dell'updater di Tauri, necessaria anche senza firma del codice: segreti GitHub Actions `TAURI_SIGNING_PRIVATE_KEY` e `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`; chiave pubblica in `tauri.conf.json` (`plugins.updater.pubkey`); `bundle.createUpdaterArtifacts` a `true`. La chiave privata va conservata anche fuori da GitHub: perderla ferma gli aggiornamenti di ogni copia installata.
- Nessuna firma del codice né notarizzazione per ora. Il README spiega come aprire le build non firmate: macOS, clic destro → **Open** o System Settings → Privacy & Security → **Open Anyway**; Windows SmartScreen, **More info** → **Run anyway**; Linux, `chmod +x` dell'AppImage.

## Aggiornamenti dall'app

- Plugin updater di Tauri con endpoint `https://github.com/nicchia-code/weaver/releases/latest/download/latest.json`.
- Controllo all'avvio e poi al massimo una volta ogni 24 ore mentre l'app è aperta; l'istante dell'ultimo controllo sta nelle impostazioni. Un controllo fallito è scritto nel log e ritentato alla scadenza successiva, mai mostrato come errore.
- Un aggiornamento trovato mostra versione e note e si installa solo quando il developer sceglie **Install and restart**; **Later** lo nasconde fino al controllo successivo. Install and restart passa dalla stessa strada dell'uscita da Weaver, compresa la sua conferma quando ci sono lavori in corso.
- Un aggiornamento la cui firma non si verifica è rifiutato e scritto nel log.

## Fatto quando

- Un push su `main` con una versione nuova, dopo un `ci` riuscito, produce una release pubblicata con `.dmg`, installer NSIS, AppImage, deb, firme e `latest.json`, e il tag `v<version>`.
- Un push su `main` con una versione già taggata termina con il `::notice::` e non costruisce nulla.
- Un job di build fallito non lascia né release né tag.
- Test del backend con un updater finto: controllo a 24 ore, errore non mostrato, Later, firma non valida rifiutata.
