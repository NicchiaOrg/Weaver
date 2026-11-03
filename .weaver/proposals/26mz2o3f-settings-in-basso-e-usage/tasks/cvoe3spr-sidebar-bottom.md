---
technical: true
refs:
  - requirements/ui#General
---
# Settings in fondo alla barra

Nella finestra di un progetto la barra laterale occupa tutta l'altezza della finestra e ha due gruppi di voci:

- in alto, in quest'ordine: Board, Chat, Jobs, Requirements, Specs, History;
- in basso a sinistra, staccato dal gruppo in alto dallo spazio libero tra i due: Settings, che resta l'ultima voce della barra.

Settings continua ad aprire la sua vista a destra come oggi, con la stessa icona `Settings`, e il cambio di voce conserva lo stato di ogni vista (focus, overlay, scroll). Larghezza 208 px; sotto i 1024 px di larghezza della finestra la barra mostra solo le icone, 56 px, e ogni voce, Settings compreso, ha il suo nome come tooltip. I numeri sulle voci Board e Jobs restano come sono.

Il gruppo in basso è pensato per contenere anche la voce Usage, sopra Settings: quella voce, con l'icona `Gauge` e il Dialog che apre, è il Task della finestra Usage. Qui basta che il gruppo in basso possa ospitare voci che non sono viste, cioè che aprono qualcosa senza diventare la voce selezionata e senza `aria-current`.

Dove nel codice: l'elenco `VIEWS` in `ui/src/views.ts` (ordine e icone, Settings in fondo), il componente `Sidebar` in `ui/src/components/sidebar.tsx` (oggi un'unica `<ul>` senza parte in basso), il layout in `ui/src/components/project-window.tsx`. Aggiornare i test di `ui/src/components/sidebar.test.tsx` che controllano l'ordine delle voci, e aggiungere una riga sotto `## [Unreleased]` di `CHANGELOG.md`.
