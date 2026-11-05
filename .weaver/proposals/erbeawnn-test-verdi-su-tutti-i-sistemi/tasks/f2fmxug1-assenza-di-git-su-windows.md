---
technical: true
refs:
  - requirements/tech#solo-mode
  - requirements/quality#General
---
# Assenza di git su Windows

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
