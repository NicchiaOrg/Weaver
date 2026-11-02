---
status: approved
kind: setup
base:
  system.md: 51dd86b090b68eab14e2dc45871bfebdf80febeb41154e3d7828a23d3f62155a
  actors/developer.md: new
  actors/agent.md: new
  actors/omp.md: new
  actors/workspace.md: new
  actors/desktop.md: new
  actors/time.md: new
  project-setup/feature.md: new
  project-setup/project.md: new
  project-setup/omp-setup.md: new
  model-changes/feature.md: new
  model-changes/proposal.md: new
  model-changes/design-request.md: new
  work/feature.md: new
  work/task.md: new
  work/drift.md: new
  work/history-entry.md: new
  agent-work/feature.md: new
  agent-work/job.md: new
  agent-work/question-round.md: new
  agent-work/general-chat.md: new
  app/feature.md: new
  app/window.md: new
  app/notification.md: new
  specs.yaml: new
  requirements/tech/area.md: 06aa9f00c99a631ad469c427f5e1d13d178fcbe4f810eb5d8642a864a7f70ba1
  requirements/tech/architecture.md: new
  requirements/tech/omp.md: new
  requirements/data/area.md: 97d5b7dae9c2306df25e3ec5e8c9ea60eb1325695e8b94593dccf410f640881f
  requirements/data/project-files.md: new
  requirements/data/settings.md: new
  requirements/cli/area.md: new
  requirements/cli/commands.md: new
  requirements/rules/area.md: 5405d436cbfb44de9ce3e6366028306c312a147f3ce9d9667683121491833cfb
  requirements/rules/cards.md: new
  requirements/rules/skills.md: new
  requirements/ui/area.md: 67fe3f632baa6c808ff82a6da2942decec9beb4618dcc8cbf70ffd586a10bfa3
  requirements/ui/look.md: new
  requirements/ui/board.md: new
  requirements/ui/views.md: new
  requirements/ui/conversations.md: new
  requirements/ui/setup.md: new
  requirements/quality/area.md: f12f9d6d15cb07d392b8cef89663cdda29e484f92885decffb60f2a223e74e1b
  requirements/quality/main.md: new
  requirements/release/area.md: new
  requirements/release/pipeline.md: new
delete: []
request: 2026-11-02T07-21-11
created: 2026-11-02T07:07:19.522Z
label: Weaver
---
# Setup di Weaver

Weaver è il rifacimento di Trama come app desktop: un Modello visivo e testuale degli stati di un sistema, più i Requisiti su come costruirlo, che il developer decide e un agente (omp) esplora, cambia, confronta con il codice e realizza. Questa Proposal porta il primo Modello, i Requisiti, le Specs e i Task che costruiscono la prima versione. `.trama.old/` è servita come fonte di comportamenti e decisioni: il comportamento è riscritto, le Proposal rifiutate o scartate non sono riproposte, i Drift risolti hanno guidato le regole contro i punti fragili (fine del lavoro dell'agente, rilascio dei Task, decisioni a metà, numeri brevi, file cancellati da fuori).

## Decisioni dell'intervista

- Piattaforma: Tauri 2, app desktop per macOS, Windows e Linux; un solo processo, una finestra per progetto.
- Logica: core Rust condiviso fra backend Tauri e CLI `trama`; la CLI funziona anche ad app chiusa.
- CLI `trama`: binario incluso nell'app, messo nel PATH di ogni omp che Weaver avvia; il developer usa solo l'app.
- Interfaccia: React 19, shadcn/ui (zinc con primario blu, chiaro e scuro), Tailwind CSS 4, lucide-react, cytoscape + elkjs, mermaid; l'interfaccia mostra soltanto, lo stato vive nel backend.
- Markdown: react-markdown + remark-gfm, HTML grezzo disattivato.
- Avvio: Weaver riapre i progetti dell'ultima sessione; senza progetti, una finestra di benvenuto con i recenti e Apri cartella.
- Editor: preset (VS Code, Cursor, Zed, JetBrains, Sublime Text) più un comando personalizzato con `{file}` e `{line}`.
- Agente: solo omp via ACP, in conversazioni dentro l'app; niente terminali, hook, Cursor o Copilot. Nel vecchio Trama «solo omp» era stato approvato, non rifiutato.
- Conversazione di un lavoro: il developer legge, scrive all'agente mentre lavora, lo ferma, chiude la conversazione.
- Chat General: inclusa; su richiesta esplicita del developer l'agente in General avvia lavori di Weaver (design, scan, Run, Resolve), che nascono come lavori normali.
- Permessi di omp: yolo predefinito, 'ask' come impostazione.
- Dati: file Markdown e YAML sotto `.trama/`, nessun database; impostazioni nella cartella di configurazione del sistema; segreti nel portachiavi.
- omp: installazione e login di un fornitore nel terminale del sistema, aperto da Weaver; Weaver ricontrolla omp da sé finché non risponde.
- Fornitore Cursor: compare solo con il login, nessuna chiave API.
- Repository: nuovo repository GitHub `nicchia-code/weaver`.
- Rilascio: GitHub Releases con l'updater di Tauri; per ora build non firmate (la soluzione più veloce); firma e notarizzazione su macOS e firma su Windows arriveranno con una Proposal dedicata.
- Linux: AppImage + deb.
- Test: `cargo test`, Vitest, end-to-end con tauri-driver eseguiti su GitHub Actions (Linux e Windows), visto che lo sviluppo avviene su macOS.
- CI: push su `dev` → test; push su `main` → test e rilascio; il rilascio parte solo se il tag `v<versione>` non esiste ancora, altrimenti il push fa solo i test.
- Sempre attive, senza interruttore: esecuzione parallela dei Task, Sub-agents Context Sharing, Advisor Scanner, Async Drift Resolver, modello per Proposal, raffinamento all'Approve.
- Rework with feedback: incluso.
- Niente pre-flight né scan di fine Run: l'Advisor Scanner, sempre attivo, controlla il codice durante i Run. Niente stima né barra di Run all.
- Richiesta di design: scritta in un editor Markdown dentro Weaver, salvata in `.trama/requests/`; parte solo con Send.
- Setup e import: la Proposal di setup e le Proposal di import (anche più d'una, a catena) scritte da un lavoro partito su un Modello vuoto sono approvate da sole alla consegna se nulla le blocca; dopo il setup parte Run all.
- Task finiti: lasciano subito la Board; restano disponibili finché una voce di History li cita (allora una copia resta nella voce) oppure finché nel progetto non finiscono altri 20 Task; poi spariscono.
- Chiusura di Weaver con lavori in corso: conferma, omp fermato, sessioni riprese alla riapertura; una sessione non ripresa fa fallire il lavoro.
- Notifiche di sistema: subito, solo se la finestra del progetto non è in primo piano; il clic porta alla finestra.
- Lavori: un elenco Lavori nella finestra con il loro stato; le schede annunciate restano ghost sulla Board.
- Progetti del vecchio Trama: non supportati, nessuna migrazione.
- Task: il developer non li sposta a mano; dall'app solo Run e Delete in To do; per il resto chiede all'agente in General.

## Scelte dell'agente da verificare

- Chiudere la finestra di un progetto con lavori in corso equivale a chiudere Weaver per quel progetto: conferma, lavori sospesi, ripresi alla riapertura.
- Quando un lavoro finisce, la sua conversazione resta leggibile ma non accetta più messaggi; per continuare si usa General.
- Un solo Run alla volta per progetto; Run e Resolve si escludono; scan, design e import possono lavorare insieme. Run all con modelli diversi per Proposal avvia un Run per modello, insieme, e conta come un solo Run.
- Approvare dall'app è bloccato mentre un Run lavora e finché il lavoro che ha consegnato la Proposal non ha finito il turno.
- Nessun segreto nella prima versione (Cursor solo con login): il portachiavi resta la sola sede ammessa per segreti futuri.
- La chiave di firma dell'updater di Tauri, necessaria all'updater anche senza firma del codice, sta nei segreti di GitHub Actions.
- Aree iniziali di un progetto nuovo: `ui`, `tech`, `data`, `rules`, `quality`.
- Niente avviso sui file di istruzioni dell'agente, niente barra d'uso di omp, niente registro dei file tra sotto-agenti (servivano hook).
- Le notifiche non hanno un'impostazione: seguono i permessi del sistema.
- I numeri brevi vivono in un file locale ignorato da git sotto `.trama/`.
- La lingua dei testi dell'agente resta un'impostazione; l'interfaccia è sempre in inglese.
- Scan System confronta sempre tutto il Sistema, dopo una conferma.
