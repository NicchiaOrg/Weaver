---
refs:
  - files
  - requirements/tech#file-view
---
# Vista File e Solo mode

Quando il Task è concluso, l'albero della File view continua a mostrare ciò che un blocco di Solo mode esclude, di qualunque working copy sia il blocco, e continua a lasciare fuori `.git` e tutto ciò che le altre regole di git ignorano. Il Model (`files/file-view`) dice: `developer.OPEN_FILE_VIEW` → Open: Weaver legge solo nomi e tipi delle voci della radice del progetto, mai il contenuto di un file, lasciando fuori `.git` e ciò che git ignora, tranne ciò che Solo mode tiene fuori da git. Lo stesso vale per ogni cartella espansa e per ogni nuova lettura dopo un evento del watcher.

Cosa deve valere:
- Il listing segue `requirements/tech#file-view`: una cartella alla volta con `read_dir`, `.git` escluso per nome, ciò che git ignora escluso con una sola chiamata `git check-ignore` per cartella listata, con `GIT_OPTIONAL_LOCKS=0`. Un percorso ignorato solo da una riga di un blocco di Solo mode viene listato.
- Per saperlo si usa `git check-ignore -v -z --stdin`, che per ogni percorso indica file sorgente e riga della regola che lo ignora. Se la regola è una riga dentro un blocco di Solo mode del file delle esclusioni locali (formato dei marcatori `# >>> weaver solo mode: <root>` / `# <<< weaver solo mode: <root>` in `requirements/rules#solo-mode`), il percorso resta nell'albero; vale per i blocchi di ogni working copy, non solo per quello della radice corrente. Se la regola viene da qualunque altra parte (`.gitignore`, `core.excludesFile`, righe delle esclusioni fuori dai blocchi), il percorso resta fuori come oggi.
- Il file delle esclusioni viene letto una sola volta per listing per trovare gli intervalli di righe dentro i blocchi di Solo mode, e viene trovato con l'helper di `crates/core/src/solo_mode.rs`. Quel modulo e il suo backend sono creati dal Task "Solo mode: core e CLI", che va fatto prima; questo Task li usa e non li re-implementa.
- Resta una sola chiamata git per cartella listata (performance della File view in `requirements/quality`, e `requirements/quality#solo-mode`: mai sul thread della UI o dell'IPC, mai oltre 10 s per chiamata). Senza git, o fuori da un repository, non si ignora nulla, come oggi.
- Il comportamento dell'albero di Import from Code non cambia: continua a lasciare fuori tutto ciò che git ignora, blocchi di Solo mode compresi.

Punti del codice:
- `files` — `crates/core/src/file_view/listing.rs`: listing della File view, dove si filtrano i risultati di `git check-ignore`.
- `requirements/tech#file-view` — `crates/core/src/project_files/folder_tree.rs`: la chiamata `git check-ignore` condivisa; aggiungere la variante con `-v` per la File view lasciando invariata quella usata da Import from Code.
- `crates/core/src/solo_mode.rs`: helper per trovare il file delle esclusioni e il formato dei marcatori (solo uso).

Test da aggiungere, secondo `requirements/quality#solo-mode`, contro repository git temporanei: la File view lista ciò che un blocco di Solo mode ignora (anche un blocco di un'altra working copy) e continua a lasciare fuori ciò che le altre regole ignorano, incluso un percorso coperto sia dal blocco sia da `.gitignore`; l'albero di Import from Code continua a escludere ciò che il blocco ignora.
