---
technical: true
refs:
- requirements/ui#General
---
# Voce Hide

Quando il Task è finito, la voce della barra laterale di ogni finestra di progetto che oggi si chiama `Run in background` si chiama `Hide`. Cambia solo il testo: comportamento, posizione e icona restano quelli di oggi.

**La voce `Hide`**
- Sta nel gruppo in basso a sinistra, prima di Usage e Settings (ordine: Hide, Usage, Settings), con l'icona `EyeOff`.
- Il testo `Hide` è anche il suo nome accessibile e, sotto una larghezza di finestra di 1024 px (barra di sole icone, 56 px), il suo tooltip.
- È sempre attiva, non è una vista, non è mai selezionata e non ha mai `aria-current`. Un clic manda subito in background la finestra (`developer.SEND_TO_BACKGROUND`, comando `send_to_background`), senza conferma; al ritorno la finestra mostra la stessa vista con lo stato di ogni vista com'era.
- Quando la finestra non si riesce a nascondere, il toast di errore dice `Could not hide the window.` (al posto di `Could not run the window in the background.`) e dà il motivo come descrizione, come oggi.

**Cosa non cambia**
- Il pulsante `Keep working in background` della conferma nativa di chiusura di una finestra con job in corso resta com'è.
- I nomi interni (comando `send_to_background`, Event `developer.SEND_TO_BACKGROUND`, State `window/In background`) restano invariati.

**Dove nel codice**
- `ui/src/views.ts` — `BACKGROUND_ENTRY`: `label` diventa `Hide`; aggiornare il commento che nomina `Run in background`.
- `ui/src/components/project-window.tsx` — il testo del `toast.error` in `onActivate` e il commento che nomina `Run in background`.
- `ui/src/components/sidebar.test.tsx` — i test che cercano la voce per nome e il testo del toast usano i nuovi testi.

**Documentazione**
- `CHANGELOG.md`, sotto `## [Unreleased]`, gruppo `Added`: la riga che introduce la voce della barra laterale la chiama `Hide` invece di `Run in background`; la riga su `Keep working in background` resta com'è.
