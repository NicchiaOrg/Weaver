---
refs:
  - developer.CHANGE_SOLO_MODE
  - developer.INITIALISE_PROJECT
  - project/Not initialised/developer.INITIALISE_PROJECT
  - requirements/ui#solo-mode
  - requirements/ui#developer.CHANGE_SETTINGS
  - requirements/ui#project
---
# Solo mode nell'interfaccia

Al termine, lo sviluppatore sceglie Solo mode dall'interfaccia: in Settings per la copia di lavoro di un progetto già inizializzato (`developer.CHANGE_SOLO_MODE`) e in Get Started al momento dell'inizializzazione (`developer.INITIALISE_PROJECT`).

## Dipendenza

Questa Task si appoggia al modulo `crates/core/src/solo_mode.rs` di `weaver-core` e al backend Tauri creati dalla Task "Solo mode: core e CLI", che va completata prima: i comandi `set_solo_mode` e `solo_tracked_files`, il campo `ProjectStatus.solo` (stato, modo, motivo per cui è disabilitato o non escluso) e il parametro `solo` di `initialise_project`. Non va reimplementata alcuna logica di git, del blocco di esclusione o di stato: l'interfaccia mostra lo stato del progetto e invia le scelte al backend. Se un binding manca o ha forma diversa, si usa quello che quella Task ha prodotto.

## Controllo (requirements/ui#solo-mode)

- Titolo `Solo mode`, controllo segmentato `Off` / `Whole folders` / `Weaver's files only`, lo stesso componente di `Theme` (radio group per le tecnologie assistive), con sotto la nota `Applies only to this working copy: other clones start with it off.`
- Non sta nello schema delle impostazioni e non viene mai salvato in `settings.json`.
- Accessibilità: nota, avvisi e motivo di disabilitazione sono la descrizione del controllo (`aria-describedby`); l'elenco dei file tracciati è una regione con etichetta; i cambi degli avvisi sono annunciati (`aria-live="polite"`).

## Settings (requirements/ui#developer.CHANGE_SETTINGS, requirements/ui#project)

- Primo elemento del gruppo `Project`, prima di `Update skills`.
- Salvataggio immediato alla scelta, senza conferma, anche spegnendo; cambio di modo immediato. Durante il cambio il controllo è disabilitato. Un cambio fallito (git assente, esclusioni non scrivibili) non cambia nulla: mostra sotto il controllo `Could not change Solo mode: <reason>.` nel colore di errore e ripristina il valore del progetto.
- Cartella non inizializzata: disabilitato su `Off`, con `Choose Solo mode when you initialise the project.` sotto la nota.
- Senza git o fuori da un repository: disabilitato su `Off` con `Solo mode needs git: git was not found.` o `Solo mode needs git: this folder is not in a git repository.` (gli altri motivi di `requirements/rules#solo-mode` come `Solo mode needs git: <reason>.`).
- Acceso: sotto la nota, colore di avviso con `TriangleAlert`, `Weaver's files are not versioned anywhere: losing this working copy loses them.`
- Acceso ma non escluso (riparazione all'apertura fallita; lo dice solo Settings): colore di avviso con `TriangleAlert`, `Solo mode is on but cannot exclude Weaver's files: <reason>.` quando git manca o la cartella non è più in un repository, altrimenti `Weaver's files may not be excluded from git: <reason>.` Il controllo resta utilizzabile.
- File tracciati: con Solo mode acceso e l'elenco non vuoto, un `Alert` (variante warning) con `Git still tracks <N> of Weaver's files: their changes still go into commits.` e sotto i percorsi relativi in monospace, che scorrono oltre 8 righe. Nessuna azione per smettere di tracciarli. L'elenco si chiede a `solo_tracked_files` ogni volta che la vista Settings viene mostrata e si sostituisce col risultato di ogni cambio; durante il caricamento resta l'elenco precedente.

## Get Started (requirements/ui#project)

- Il controllo sta tra l'elenco dei file dell'agente e `Initialise`; `Off` scelto all'inizio, la scelta vive solo nella schermata.
- Con `Whole folders` o `Weaver's files only` il testo diventa `Weaver will create a .weaver folder that describes this project. With Solo mode its files stay only in this working copy and never go into a commit.` e sotto il controllo compare l'avviso di Solo mode acceso.
- Senza git o fuori da un repository: disabilitato su `Off` con lo stesso motivo `Solo mode needs git: ...` di Settings.
- `Initialise` e `Overwrite and initialise` inviano entrambi la scelta (`initialise_project` con `solo`). Come vuole il Model, con un modo scelto Weaver accende Solo mode prima di scrivere i suoi file; se non ci riesce il progetto non viene creato e il motivo compare nel riquadro esistente `Could not initialise: <reason>.`, con il pulsante che resta per riprovare.

## Punti del codice

- `requirements/ui#solo-mode` — nuovo componente in `ui/src/components/project/` (es. `solo-mode.tsx`), riusato da Settings e Get Started; chiamate in `ui/src/lib/backend.ts`.
- `requirements/ui#developer.CHANGE_SETTINGS` — `ui/src/components/project-content.tsx:85` (`projectSlot`: Solo mode prima di `<UpdateSkills />`), `ui/src/components/settings-view.tsx` (gruppo `Project`), stato da `ui/src/components/project/use-project-status.tsx`.
- `project/Not initialised/developer.INITIALISE_PROJECT` — `ui/src/components/project/get-started.tsx` (testo, controllo dopo `PathList`, `Initialise`, `ConfirmDialog` di sovrascrittura, riquadro d'errore).

## Test (requirements/quality#solo-mode)

In Vitest, con `vi.mock("@/lib/backend")` e query per ruolo: il controllo, le note, gli avvisi e l'elenco in Settings e in Get Started, con i percorsi da tastiera e i nomi accessibili. Aggiornare `ui/src/components/settings-view.test.tsx` (ordine del gruppo `Project`: Solo mode prima di `Update skills`; stati disabilitati, acceso, non escluso, elenco tracciato ricaricato a ogni visualizzazione, cambio fallito che ripristina il valore) e `ui/src/components/project/get-started.test.tsx` (`Off` iniziale, testo che segue la scelta, avviso, scelta inviata da `Initialise` e `Overwrite and initialise`, errore in `Could not initialise: <reason>.`, stato senza git).
