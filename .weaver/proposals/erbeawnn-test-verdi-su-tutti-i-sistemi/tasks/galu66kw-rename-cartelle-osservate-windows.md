---
technical: true
refs:
  - requirements/tech#file-view
  - requirements/rules#file-view
---
# Rename cartelle osservate su Windows

Su Windows, rinominare da Files una cartella espansa fallisce con `Could not rename "src": Access is denied. (os error 5)`. Lo mostra il test `file_view::tests::renaming_moves_the_tabs_the_tree_and_the_watches_of_a_folder` (`src-tauri/src/file_view/tests.rs`). Per l'utente di Windows il rename è rotto, non solo il test.

La causa probabile è il watcher `notify` di Files: tiene una watch non ricorsiva per ogni cartella espansa (`requirements/tech#file-view`), e su Windows ogni watch tiene aperto un handle sulla cartella (`ReadDirectoryChangesW`), che impedisce di rinominarla. Il rename passa da `FileViewService::rename` (`src-tauri/src/file_view/mod.rs`) a `ops::rename` e `rename_no_replace` (`crates/core/src/file_view/ops.rs`, `rename.rs`).

Quando il Task è fatto:

- prima di rinominare o spostare una cartella, Files rilascia le watch della cartella e di ogni cartella espansa al suo interno. Dopo l'operazione le ricrea con i nomi nuovi se è riuscita, con i nomi vecchi se è fallita o è stata rifiutata. Su macOS e Linux il risultato visibile non cambia;
- si controllano con la stessa causa anche lo spostamento di una cartella espansa e l'eliminazione nel cestino di una cartella espansa, e si correggono se falliscono su Windows;
- un rifiuto continua a non cambiare niente: il test `a_refused_rename_changes_nothing_and_tells_the_tabs_nothing` resta valido;
- il test del rename passa su Windows senza essere saltato, e le cartelle spostate restano osservate con il nome nuovo (l'evento su `lib/deep/fresh.rs` arriva).
