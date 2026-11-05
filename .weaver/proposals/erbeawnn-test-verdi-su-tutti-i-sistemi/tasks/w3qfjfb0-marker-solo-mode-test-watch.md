---
technical: true
refs:
  - requirements/tech#solo-mode
---
# Marker Solo mode nei test watch

Su Windows falliscono due test di `src-tauri/src/windows/watch.rs`:

- `a_weaver_folder_that_vanishes_takes_the_block_of_solo_mode_with_it_before_anything_else`;
- `the_block_goes_with_the_weaver_folder_even_when_its_state_file_is_already_gone`.

Tutti e due falliscono su `assert!(read_to_string(&exclude).contains(&marker))`, subito dopo `set_mode`.

Il test costruisce il marker atteso con `root.to_string_lossy()`, quindi su Windows con le barre `\`. `solo_mode` scrive invece il marker del blocco con le barre `/`, come mostra l'helper `marker()` di `crates/core/src/solo_mode/tests.rs` (`replace('\\', "/")`).

Quando il Task è fatto:

- i due test costruiscono il marker come lo scrive `solo_mode`, senza duplicare a mano la regola se `weaver-core` può esporla, per esempio una funzione pubblica per il marker di una radice;
- il formato del blocco in `.git/info/exclude` non cambia;
- i due test passano su macOS, Linux e Windows. Se dopo la correzione del marker su Windows resta un altro fallimento (per esempio il blocco non rimosso quando `.weaver/` sparisce), va trovata e corretta la causa nel codice.
