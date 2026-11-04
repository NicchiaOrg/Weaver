---
technical: false
refs:
  - files
  - file-tab
  - requirements/ui#file-tab
  - requirements/rules#file-tab
  - requirements/tech#file-tab
  - requirements/quality#file-tab
---

# Anteprima immagini e SVG

Nella File view (Feature `files`) una scheda su un'immagine mostra l'immagine, e una scheda su un file SVG alterna tra il disegno e il testo. Alla fine del lavoro deve valere quanto segue per la Machine `file-tab`.

## Apertura (`Closed` su `developer.OPEN_FILE`)

- → `Image`: quando il file è un'immagine che Weaver sa mostrare; la scheda mostra l'immagine, con dimensioni in pixel e peso, e nessun testo.
- → `Binary`: quando il file è binario e non è un'immagine mostrabile; la scheda lo dice e non mostra testo; se il file è un'immagine troppo grande, dice anche questo.
- Le altre uscite (`Read-only`, `Locked`, `Saved`) restano invariate.

## Regole (`requirements/rules#file-tab`)

- Binario: NUL nei primi 8.192 byte, oppure byte non UTF-8 validi dopo un eventuale BOM UTF-8 (regola già esistente).
- Immagine mostrabile: file binario i cui primi byte sono la firma di PNG (`89 50 4E 47 0D 0A 1A 0A`), JPEG (`FF D8 FF`), GIF (`GIF87a`/`GIF89a`), WebP (`RIFF`, quattro byte, `WEBP`), BMP (`BM`) o ICO (`00 00 01 00`), riconosciuta solo dalla firma, mai dal nome o dall'estensione, e di al massimo 10 MB (10.485.760 byte). Un file più grande con una di queste firme resta binario e la scheda dice che l'immagine è troppo grande. Ogni altro binario non è mai mostrato come immagine.
- SVG: file non binario il cui nome finisce in `.svg` (confronto case-insensitive); il contenuto non decide mai. Restano tutte le regole dei file di testo, limite di 2 MB e `.weaver/` compresi.

## State `Image`

Nulla nella scheda può cambiare il file; un agente che lo modifica si vede solo dai cambiamenti sul disco (nessuna linea di lock, come per i binari). Quando l'immagine non si può disegnare, la scheda dice invece che il file è binario e che l'immagine non può essere mostrata.
- `workspace.OPEN_FILE_CHANGED` → `Image`: se il file è ancora un'immagine mostrabile, viene riletto e la nuova immagine si vede subito (aggiornamento automatico); se il file ricompare dopo una cancellazione, la barra sparisce.
- `workspace.OPEN_FILE_CHANGED` → `Binary`: se non è più un'immagine mostrabile; la scheda dice che è binario, e che l'immagine è troppo grande quando il motivo è quello.
- `workspace.OPEN_FILE_DELETED` → `Image`: l'immagine resta; barra `This file was deleted on disk.` senza `Save again`; nulla si può salvare.

## State `Binary` (modificato)

Il file è binario e non se ne può mostrare un'immagine.
- `workspace.OPEN_FILE_DELETED` → `Binary`: la scheda dice anche che il file è stato cancellato.
- `workspace.OPEN_FILE_CHANGED` → `Image`: se il file ora è un'immagine mostrabile, viene letto e l'immagine si vede; la scheda non dice più che il file è cancellato.
- `workspace.OPEN_FILE_CHANGED` → `Binary`: altrimenti; se il file ricompare, la scheda non dice più che è cancellato.

## SVG

Una scheda SVG mostra il disegno del testo della scheda (modifiche non salvate incluse) oppure il testo, e lo sviluppatore alterna in ogni momento. Si apre sul disegno (`Preview`); la scelta vale per quella scheda finché resta aperta. L'interruttore c'è in ogni State di una scheda con testo (sola lettura, lock, cancellato) e non cambia mai il file. Se il testo non si può disegnare, al posto del disegno compare in testo attenuato `This SVG cannot be drawn.`

## Interfaccia (`requirements/ui#file-tab`)

- Barra in alto: percorso relativo (monospace) e `Open in editor`. Su un'immagine, o su un SVG che mostra il disegno, dopo il percorso in testo attenuato: `<width> × <height> px` (appare quando l'immagine è disegnata) e il peso in `B`, `KB` o `MB` con 1 KB = 1.024 byte e una cifra decimale da 1 KB in su (`12.4 KB`). `Open in editor` su un'immagine apre il file senza riga.
- Su una scheda SVG: shadcn `ToggleGroup` a scelta singola `Code` / `Preview` accanto a `Open in editor`; Left e Right si muovono tra le due voci.
- Vista immagine: centrata, rimpicciolita per stare nel pannello e mai ingrandita, alt = nome del file. Sotto l'immagine, e solo sotto, scacchiera di quadrati chiari e attenuati dai token del tema, in entrambi i temi. Clic, oppure `Enter`/`Space` con il focus, alterna tra adattata e dimensione reale (un pixel immagine per pixel CSS), dove il pannello scorre in entrambe le direzioni; cursore `zoom-in` da adattata e `zoom-out` a dimensione reale; nome accessibile `Show actual size` o `Fit to panel`. La scheda parte adattata e mantiene la scelta quando l'immagine viene riletta.
- Anteprima SVG: stessa vista, dal testo della scheda. Linee di sola lettura, linea del lock e barre restano sopra come sopra il testo; `Cmd+S`/`Ctrl+S` salva ancora; cerca e sostituisci solo su `Code`. Il passaggio non perde cursore, selezione né cronologia di undo.
- Pannello `Binary file` con `Open in editor` e nessun testo; per un'immagine oltre 10 MB con la riga `This image is larger than 10 MB: no preview.`; per un'immagine non disegnabile con la riga `This image cannot be shown.`

## Tecnica (`requirements/tech#file-tab`)

- Dalla stessa unica lettura, un'immagine mostrabile arriva alla UI nei risultati di `file_open` e `file_reload` come `data:<mime>;base64,<…>` con la dimensione in byte, senza testo. MIME dalla firma: `image/png`, `image/jpeg`, `image/gif`, `image/webp`, `image/bmp`, `image/x-icon`. Riconoscimento e base64 riusano `ImageKind` e `base64_encode` di `crates/core/src/project/requests/image.rs`, estesi con BMP e ICO senza allargare i formati accettati per le richieste di design. Un binario con firma d'immagine oltre 10 MB arriva come binario con quel motivo, senza byte.
- Il backend non tiene byte d'immagine nello stato della scheda; la UI tiene il `data:` URL solo nello stato di quella scheda e lo scarta alla chiusura. Niente `blob:`, asset protocol o `convertFileSrc`: la CSP resta `img-src 'self' data:`.
- Un cambiamento `file-tab-changed` per una scheda che mostra un'immagine o un binario fa chiamare alla UI `file_reload`, il cui risultato dà la nuova immagine o dice che ora è binario; una cancellazione mantiene l'immagine e imposta solo la barra. L'immagine si disegna in un `<img>`: `naturalWidth`/`naturalHeight` dopo `load` danno le dimensioni; l'evento `error` mostra il pannello binario con la riga dell'immagine non mostrabile.
- SVG: la UI costruisce `data:image/svg+xml;charset=utf-8,<encodeURIComponent(text)>` dal testo corrente e lo disegna solo in un `<img>` (niente script, niente risorse esterne); mai SVG inline nel DOM, mai `dangerouslySetInnerHTML`. Si ricostruisce quando il disegno è mostrato e il testo è cambiato da quando era stato costruito. Peso = lunghezza in byte UTF-8 del testo. Il backend non viene chiamato per disegnarlo.

## Qualità (`requirements/quality#file-tab`)

- Prestazioni: aprire un file lo legge una volta, immagine compresa; una scheda immagine rilegge solo dopo un cambiamento del proprio watch; chiudere una scheda libera subito editor, testo, immagine e watch. Digitare non avvia processi né lavoro su disco e, in una scheda SVG, non disegna nulla finché il disegno non è mostrato.
- Log: mai il contenuto di un file, i byte di un'immagine o il suo `data:` URL; i percorsi sì.
- Test: come funzioni pure, le firme per ogni formato e il limite di 10 MB, la regola del nome SVG (oltre a quelle già esistenti); un'immagine riletta dopo un cambiamento, che diventa binaria, cancellata e di nuovo presente; la vista immagine (adattata e dimensione reale con clic e tastiera, immagine non disegnabile) e l'interruttore SVG `Code`/`Preview` che conserva la cronologia di undo, in Vitest con percorsi da tastiera e nomi accessibili.

## Dove nel codice

- `crates/core/src/file_tab/read.rs`: `BINARY_SCAN_BYTES` (r. 15), `is_binary` (r. 74), `MAX_EDITABLE_BYTES` (r. 18), `from_bytes` (r. 133) e `read` (r. 164), che legge tutto il file senza limite per i binari.
- `crates/core/src/project/requests/image.rs`: `MAX_IMAGE_BYTES` (r. 5), `ImageKind` con `mime`, `from_extension`, `detect` (r. 9-48), `base64_encode` (r. 99).
- `src-tauri/src/file_tabs/mod.rs`: `FileTabs::open` (r. 149), `reload` (r. 216), `check_tab` (r. 410), `read_file` (r. 508), `content_of` (r. 575, oggi scarta i byte); `crates/core/src/file_tab/limit.rs` `disk_event` (r. 53).
- `src-tauri/src/file_tabs/wire.rs`: `FileKind` (r. 10), `FileContent` (r. 29), `FileOpenResult`, `TabChange` (r. 90), `FileTabChanged` (r. 101); binding in `ui/src/bindings/FileOpenResult.ts`, `FileReloadResult.ts`, `FileContent.ts`, `FileKind.ts`. Comandi `file_open` (r. 34) e `file_reload` (r. 53) in `src-tauri/src/file_tabs/commands.rs`.
- CSP in `src-tauri/tauri.conf.json` (r. 16-17), da non cambiare.
- UI: `ui/src/components/files/file-tabs.tsx` (`EditorHost` r. 160, `BinaryPanel` r. 175, scelta binaria r. 197-219); `ui/src/components/files/tab-model.ts` (`TabState` r. 9, `kind` r. 16, `diskChanged` r. 88 che oggi non ricarica i binari); `ui/src/components/files/file-tabs-context.tsx`; `ui/src/lib/backend.ts`; `ui/src/components/open-in-editor.tsx`.
- Test: `ui/src/components/files/file-tabs.test.tsx`, `src-tauri/src/file_tabs/tests.rs`.
