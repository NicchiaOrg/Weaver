---
refs:
  - app
---
# README e CHANGELOG Notes

Weaver ora ha le note del progetto: la vista `Notes`, nella barra laterale dopo Assets. È un unico foglio Markdown per progetto, scritto in un editor WYSIWYG alla Notion e salvato da solo. Ha le scorciatoie Markdown durante la scrittura (`# ` per un titolo, `[ ] ` per una voce di checklist) e checklist con voci che, spuntate, restano al loro posto barrate. Le immagini si incollano o si trascinano dal Finder o da Explorer. Il foglio e le sue immagini stanno in `.weaver/notes/`, una cartella solo locale con un `.gitignore` `*`: non sono mai versionati e l'agente non li vede mai. Quando questo Task è fatto, la documentazione del repository lo dice, come chiede `requirements/docs#General`.

- `CHANGELOG.md`, sotto `## [Unreleased]` → `### Added`: una riga, in inglese e senza id di Task o Proposal, che dice all'utente cosa c'è di nuovo. Deve nominare:
  - la vista `Notes` dopo Assets;
  - il foglio unico in stile Notion, con scorciatoie Markdown e checklist;
  - le immagini incollate o trascinate (PNG, JPEG, GIF o WebP fino a 10 MB);
  - il salvataggio automatico;
  - che le note restano solo su questo computer, in `.weaver/notes/`, non versionate e mai mostrate all'agente.
- `README.md`:
  - il paragrafo iniziale nomina la vista Notes accanto al File view;
  - `### Where Weaver keeps files` ha un paragrafo su `.weaver/notes/`, accanto a quello su `.weaver/assets/`. Il paragrafo dice che la cartella contiene `notes.md` e le immagini del foglio; che è solo locale (`.gitignore` `*`), quindi un altro clone parte senza note; che viene creata alla prima nota o immagine; che è scritta solo dalla vista Notes; che git ne ignora i file, quindi il File view mostra la cartella vuota; che l'agente non riceve mai le note.
- La tabella dei moduli di `weaver-core` (`### weaver-core modules`) e quella dei moduli dell'app (`### weaver (app) modules`) elencano i moduli nuovi delle note, con una riga ciascuno, come fanno per gli asset.
