---
refs:
  - general-chat
  - requirements/ui#general-chat
---

# Chat nella vista

La conversazione generale del progetto si chiama `Chat` in ogni testo dell'interfaccia, mai `General`. Quando il Task è concluso valgono queste condizioni:

- Il titolo della vista nell'intestazione è `Chat` (oggi `General`, in `ui/src/components/general/general-view.tsx`).
- La finestra di conferma del pulsante `Clear` (icona `Eraser`) ha il titolo `Clear Chat?` (oggi `Clear General?`); restano invariati il testo `The conversation is deleted and a new one starts.` (con la sua variante durante un turno), i pulsanti `Clear` (distruttivo) e `Cancel` e il focus su `Cancel`.
- Una richiesta rifiutata perché la Chat non è aperta riporta `Chat is not open for this project` (oggi `General is not open for this project` in `src-tauri/src/general/service.rs`, in tutti i punti in cui compare); il testo arriva nella vista tramite il `problem` di `use-general.tsx`.
- Nessun altro testo visibile nella vista nomina `General`: anche i motivi di errore che possono comparire come `omp stopped: <reason>.` (per esempio `cannot write the instruction of General: …` in `src-tauri/src/general/actor.rs`) nominano la conversazione come `Chat`.
- Restano `General` i testi destinati all'agente (istruzione della conversazione, riga di comando, skill), il Model, i Requirement `tech`, `data` e `cli` e gli identificatori interni (id di vista `general`, `WEAVER_GENERAL`, comandi `general_*`, nomi di file e cartelle). L'icona `MessageSquare` non cambia.
- La voce della barra laterale, il suo tooltip e l'ordine delle voci non fanno parte di questo Task.
- I test che fissano i vecchi testi (per esempio `ui/src/components/general/general-view.test.tsx` e i test Rust del servizio) sono aggiornati ai nuovi testi e passano.
