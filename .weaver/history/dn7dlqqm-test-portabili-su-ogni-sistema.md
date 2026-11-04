---
created: 2026-11-04T15:12:32.831Z
updated: 2026-11-04T15:12:32.831Z
summary: 'I test di Solo mode, delle note e della vista File non dipendono più dal sistema: l''assenza di git è simulata in modo uguale ovunque e i marker e i nomi seguono le regole del file system.'
tasks:
- f2fmxug1
- w3qfjfb0
- 2otzxz6z
cleared:
  2otzxz6z:
    title: Test nomi note su Linux
    refs:
    - requirements/tech#notes-view
    - requirements/quality#notes-view
    technical: true
    proposal: erbeawnn
    label: Test multipiattaforma
    finished: 2026-11-04T15:07:31.704Z
    body: |-
      Il test `notes::images::tests::an_invalid_own_name_gives_the_date_and_time` (`crates/core/src/notes/images.rs`) fallisce su Linux: per il nome proprio `"NOTES.MD"` si aspetta il nome con data e ora (`pasted-2026-11-04-090507.png`) e riceve `"NOTES.MD"`.

      Il codice è corretto: `check_name` confronta il nome con il foglio (`notes.md`) tramite `same_name`, che confronta i nomi come fa il file system, senza distinguere maiuscole e minuscole solo su macOS e Windows. Su Linux `NOTES.MD` è un file diverso dal foglio, quindi è un nome valido.

      Quando il Task è fatto:

      - il caso `"NOTES.MD"` non è più nell'elenco comune del test e si trova in un test, o in un ramo, valido solo dove il file system ignora le maiuscole (`#[cfg(any(windows, target_os = "macos"))]`), come fa già `names_that_differ_by_case_are_taken_where_the_file_system_ignores_case`;
      - su Linux un test verifica che `NOTES.MD` sia tenuto come nome proprio;
      - il codice di `check_name` e `same_name` non cambia;
      - `cargo test -p weaver-core` passa su macOS, Linux e Windows.
  f2fmxug1:
    title: Assenza di git su Windows
    refs:
    - requirements/tech#solo-mode
    - requirements/quality#General
    technical: true
    proposal: erbeawnn
    label: Test multipiattaforma
    finished: 2026-11-04T15:08:46.019Z
    body: |-
      I test simulano l'assenza di git con `Git::system().with_search_path("")` (`crates/core/src/solo_mode.rs`), che imposta `PATH=""` al processo `git`. Su Windows questo non basta: `std::process::Command`, quando non trova il programma nel `PATH` del processo figlio, lo cerca anche nella cartella dell'eseguibile, nelle cartelle di sistema e nel `PATH` del processo padre. Così git viene trovato e la condizione "git was not found" non si verifica mai.

      Su Windows falliscono per questo, almeno in apparenza, tre test di `src-tauri/src/project_setup.rs`:

      - `without_git_solo_mode_stays_on_with_its_problem_and_the_check_is_untouched` riceve `unavailable: None, problem: None` invece di `Some("git was not found")`;
      - `a_change_that_fails_changes_nothing_says_why_and_marks_solo_mode_unavailable` va in timeout su `the status never came`;
      - `a_folder_that_is_not_a_project_says_whether_solo_mode_can_be_chosen` va in timeout su `the status never came`.

      La stessa simulazione è usata in `crates/core/src/solo_mode/tests.rs`, `crates/core/src/project_files/initialise.rs` e in altri test di `weaver-core`. Su Windows quei test non sono ancora stati eseguiti in CI, perché `cargo test --workspace` si è fermato prima.

      Quando il Task è fatto:

      - `Git` ha un modo, usato dai test, di indicare dove cercare `git` che dia lo stesso risultato su tutti i sistemi. Per esempio, `Git` risolve da sé l'eseguibile nelle cartelle indicate (con le estensioni di `PATHEXT` su Windows) e, se non lo trova, restituisce `Reason::NoGit` senza avviare alcun processo. L'implementazione è libera, ma il `git` di sistema non deve mai essere trovato quando il test indica una ricerca vuota;
      - i test con un `git` finto (script con `with_search_path(fake)`) funzionano allo stesso modo, oppure sono limitati a Unix con un motivo scritto nel test, come chiede `requirements/quality#General`;
      - il comportamento con `Git::system()` resta quello di oggi: `git` cercato nel `PATH`, con il limite di 10 s per chiamata;
      - se i due timeout di `project_setup` hanno un'altra causa, va trovata e corretta;
      - su Windows passano `cargo test -p weaver-core` e `cargo test -p weaver`, compresi i test di `solo_mode` e `project_setup`.
  w3qfjfb0:
    title: Marker Solo mode nei test watch
    refs:
    - requirements/tech#solo-mode
    technical: true
    proposal: erbeawnn
    label: Test multipiattaforma
    finished: 2026-11-04T15:08:46.188Z
    body: |-
      Su Windows falliscono due test di `src-tauri/src/windows/watch.rs`:

      - `a_weaver_folder_that_vanishes_takes_the_block_of_solo_mode_with_it_before_anything_else`;
      - `the_block_goes_with_the_weaver_folder_even_when_its_state_file_is_already_gone`.

      Tutti e due falliscono su `assert!(read_to_string(&exclude).contains(&marker))`, subito dopo `set_mode`.

      Il test costruisce il marker atteso con `root.to_string_lossy()`, quindi su Windows con le barre `\`. `solo_mode` scrive invece il marker del blocco con le barre `/`, come mostra l'helper `marker()` di `crates/core/src/solo_mode/tests.rs` (`replace('\\', "/")`).

      Quando il Task è fatto:

      - i due test costruiscono il marker come lo scrive `solo_mode`, senza duplicare a mano la regola se `weaver-core` può esporla, per esempio una funzione pubblica per il marker di una radice;
      - il formato del blocco in `.git/info/exclude` non cambia;
      - i due test passano su macOS, Linux e Windows. Se dopo la correzione del marker su Windows resta un altro fallimento (per esempio il blocco non rimosso quando `.weaver/` sparisce), va trovata e corretta la causa nel codice.
---
# Test portabili su ogni sistema

L'assenza di git nei test è ora simulata da Git stesso, che cerca l'eseguibile solo nelle cartelle indicate (con PATHEXT su Windows) e senza processo restituisce che git non c'è. I test della vista File costruiscono il marker del blocco di Solo mode come lo scrive weaver-core. Il caso NOTES.MD del test sui nomi delle immagini delle note vale solo dove il file system ignora le maiuscole, e su Linux si verifica che sia un nome proprio valido. Il comportamento dell'app non cambia.
