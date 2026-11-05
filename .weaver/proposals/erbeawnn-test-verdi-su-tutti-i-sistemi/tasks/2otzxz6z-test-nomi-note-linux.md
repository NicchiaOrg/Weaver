---
technical: true
refs:
  - requirements/tech#notes-view
  - requirements/quality#notes-view
---
# Test nomi note su Linux

Il test `notes::images::tests::an_invalid_own_name_gives_the_date_and_time` (`crates/core/src/notes/images.rs`) fallisce su Linux: per il nome proprio `"NOTES.MD"` si aspetta il nome con data e ora (`pasted-2026-11-04-090507.png`) e riceve `"NOTES.MD"`.

Il codice è corretto: `check_name` confronta il nome con il foglio (`notes.md`) tramite `same_name`, che confronta i nomi come fa il file system, senza distinguere maiuscole e minuscole solo su macOS e Windows. Su Linux `NOTES.MD` è un file diverso dal foglio, quindi è un nome valido.

Quando il Task è fatto:

- il caso `"NOTES.MD"` non è più nell'elenco comune del test e si trova in un test, o in un ramo, valido solo dove il file system ignora le maiuscole (`#[cfg(any(windows, target_os = "macos"))]`), come fa già `names_that_differ_by_case_are_taken_where_the_file_system_ignores_case`;
- su Linux un test verifica che `NOTES.MD` sia tenuto come nome proprio;
- il codice di `check_name` e `same_name` non cambia;
- `cargo test -p weaver-core` passa su macOS, Linux e Windows.
