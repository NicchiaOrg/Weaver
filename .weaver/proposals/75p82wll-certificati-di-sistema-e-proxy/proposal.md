---
status: approved
kind: change
base:
  model/actors/desktop.md: 42d6720ac8f8e1fdded139b20ac01e5711e223762ed3c33aca0abef1f80b593d
  model/actors/developer.md: 8b241abd724b649b814b2fc676acded59fd581d3512bd09b37b5b1fd4fcb2fb6
  model/actors/omp.md: ff6c377239d3f3ff06ed6208609b841eb594575b2c29e4b49dbb4e77d190b3d7
  model/actors/time.md: 84423c916dc17b4eafc81c5f2f88f1183f6bed0cae6c5d13bbc7e8f51643a4dc
  model/app/notification.md: a7b9ab77708180b64342fed7e81a3e7c35a73e0cddec78e6430782f441a442d3
  model/app/window.md: 4b1b83f5d164cead0f40019b0552bde4bdf6b9cfc8fc5542ec56990f04e32370
  model/project-setup/feature.md: 47593dfbbd8de2e47e9080bc332dacb7cb472a1cdadc022fe6bea3195e565936
  model/project-setup/omp-setup.md: 189b3bb071d404f71cc8b5f61573c1de87c51f0674bc86fe9776cedfd20eed49
  model/project-setup/system-certificates.md: new
  requirements/data/settings.md: 37f8a6fd0b6de771f63babda56bdb86d52392aba1275673cbd74e3da34d7ddcd
  requirements/docs/area.md: new
  requirements/docs/main.md: new
  requirements/quality/main.md: edbec8a20bef4c7d6fa3028ce5f5441acdb91f1e62b07c631f7c6c31068efb8c
  requirements/rules/system-certificates.md: new
  requirements/tech/acp.md: new
  requirements/tech/architecture.md: 5c67a2926b273fd7c44314c93e414c9fbc7e60805d3632abada6eb0fb77609a7
  requirements/tech/omp.md: 153a668a3297bde1341a2fb9254f22e8734fc6fd930a8e36c3c91559ca4496e0
  requirements/tech/system-certificates.md: new
  requirements/ui/conversations.md: 4b7fe46e55e7c0a46b1800ca9951093cce44fd87359095677420300314758d82
  requirements/ui/setup.md: 6098a4e99a8ff93775b6838c84c11c77073cfced6a8546b6e813a3861b257fc9
  requirements/ui/system-certificates.md: new
  specs.yaml: 8bfa9b869f04bbc961bf14aae8e8816c1f32d3175e7e4ef7ce4b8ce053811e5e
created: 2026-11-02T19:37:18.150Z
request: 2026-11-02T19-47-39
label: System Certificates
job: hokao03f
---
# Certificati di sistema e proxy

Dietro un proxy aziendale che ispeziona HTTPS (Zscaler in particolare) omp non si installa, i lavori falliscono con errori di certificato o HTTP e Weaver mostra l'errore grezzo di omp. La Proposal porta in Weaver la modalità System Certificates della vecchia Trama, adattata all'app desktop: una nuova Machine `system-certificates`, l'installer di omp che dice quando fallisce, le spiegazioni degli errori, la notifica dell'accensione per Zscaler e la documentazione.

## Decisioni

- Nuova Machine `system-certificates` nella Feature `project-setup` (stati Off, On con Exporting, Ready, Failed).
- Impostazione `systemCertificates.enabled` in `settings.json`, spenta di default; si accende da sola solo se Zscaler è installato.
- Autorità lette con `rustls-native-certs` (nessun processo esterno, solo CA che il sistema considera fidate), più le radici pubbliche di Mozilla compilate in Weaver; nel bundle solo CA, senza duplicati.
- Bundle `system-certificates.pem` nei dati locali di Weaver, scrittura atomica, al più 30 s; un fallimento lascia il bundle precedente.
- Esportazione all'accensione, all'avvio se il bundle manca o ha più di 24 ore, ogni 24 ore mentre Weaver è aperto, e con `Refresh`.
- All'avvio il primo controllo di omp aspetta l'esportazione (al più 30 s); il riquadro di omp mostra `Exporting system certificates first…`.
- Dopo ogni esportazione e quando la modalità si spegne, Weaver ricontrolla omp.
- Le nove variabili e `PI_REQUEST_COMPRESSION=0` sovrascrivono quelle ereditate; si calcolano a ogni avvio, anche alla ripresa di un lavoro, e non si registrano con il lavoro.
- Anche il terminale dell'installer e quelli del login ricevono le variabili: ambiente su Windows, prefisso `/usr/bin/env` nel comando su macOS, `export` nello script su Linux.
- L'installer ora scarica lo script e lo esegue, poi scrive il codice d'uscita in un file marcatore nei dati locali; un codice diverso da 0 riporta omp a Missing con il testo dell'installer fallito.
- Accensione per Zscaler una volta per utente, ricordata in `state.json` come `systemCertificates.autoEnabledAt`; avviso con notifica di sistema (mostrata anche con la finestra in primo piano) e banner in ogni finestra; il clic porta a Settings dell'ultima finestra di progetto, altrimenti alla finestra di benvenuto.
- Un'esportazione automatica fallita (avvio, Zscaler, ogni 24 ore) mostra un banner con `Open Settings`; con `Refresh` o con l'accensione a mano solo la riga di stato.
- Marcatori di certificato come Trama; errori HTTP 403, 407 e tutti i 5xx, nelle forme di Trama e con la frase standard del codice.
- La spiegazione del proxy sostituisce il motivo grezzo ovunque Weaver mostra perché omp è fallito, anche negli errori ACP e in `Not delivered`; il testo si scrive al momento del fallimento.
- Interruttore `Switch` (nuovo componente), ultimo nel gruppo `omp` di Settings e sotto il riquadro di omp in Get Started e nella finestra di benvenuto; riga di stato e `Refresh` solo a modalità accesa.
- Regole di qualità di Trama più log di ogni esportazione e dell'accensione automatica.
- Documentazione: sezione `Behind a corporate proxy` nel README e nuovo `CHANGELOG.md` nel formato Keep a Changelog, aggiornato da ora per ogni modifica visibile; regole nella nuova Area `docs`.

## Scelte dell'agente da verificare

- Radici pubbliche dalla crate `webpki-root-certs`, sorella di `webpki-roots` scelta nell'intervista: `webpki-roots` contiene solo trust anchor, che non si possono scrivere in un bundle PEM.
- `x509-parser` per riconoscere le CA; i duplicati sono certificati con gli stessi byte DER.
- Il frammento `tech/omp` superava 12 KB: le sezioni `job` e `general-chat` passano, senza cambiare testo, al nuovo frammento `tech/acp`.
- I banner si chiudono in tutte le finestre insieme; quello dell'accensione non torna dopo l'uscita da Weaver, quello del fallimento sparisce con un'esportazione riuscita o spegnendo la modalità.
- Nella finestra di benvenuto i banner non hanno `Open Settings`, perché lì l'interruttore è già sotto il riquadro di omp.
