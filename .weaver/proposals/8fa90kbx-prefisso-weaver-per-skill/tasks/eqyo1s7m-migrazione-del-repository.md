---
technical: true
refs:
  - requirements/rules#project
---
# Migrazione del repository

Migrazione una tantum di questo repository, e di nessun altro, ai nomi `weaver-*` delle skill e dei sub-agent. Non diventa codice di Weaver.

Prerequisito: il Task `3hv6kx17` (Prefisso weaver nel codice) è Done, cioè nel codice i sorgenti installati sono in `crates/core/assets/skills/weaver-*/` e `crates/core/assets/omp/agents/weaver-*.md` e le skill dei Requirement si chiamano `weaver-req-*`. Se non lo è, rilasciare questo Task senza cambiamenti.

A Task finito valgono tutte queste condizioni:

- Sono eliminati ogni `.agents/skills/trama-*` e ogni `.omp/agents/trama-*` di questo repository:
  - le skill di Weaver `trama-setup`, `trama-design`, `trama-import`, `trama-scan`, `trama-compare`, `trama-implement`, `trama-resolve-drifts`;
  - ogni skill dei Requirement `trama-req-*` (oggi 27, tutte in fragment mode);
  - `.omp/agents/trama-analysis.md` e `.omp/agents/trama-implementer.md`;
  - anche `trama-handoff` e `trama-merge`, skill del vecchio prodotto per i progetti `.trama/` (Team, handoff, merge in modalità Solo), fuori ambito per Weaver e non nominate da nessun file, e `trama-specs`, scritta dal vecchio prodotto e mai aggiornata da Weaver: le tecnologie stanno nelle Specs in `.weaver/specs.yaml`, che gli agenti leggono con `weaver model show specs`. Nessuna di queste tre viene rinominata.
- L'aggiornamento delle skill è eseguito con la riga di comando compilata da questo repository, non con quella installata con l'app: `cargo build -p weaver-cli`, poi `WEAVER_ENDPOINT=none target/debug/weaver-cli project edit --root .`. `project edit` è un comando che scrive: con l'app aperta la riga di comando lo passerebbe all'app, che installerebbe i testi vecchi; `WEAVER_ENDPOINT` con un valore che non corrisponde all'app la fa considerare assente, e il comando scrive da solo sotto il lock del progetto. Il risultato installa le skill `weaver-*`, i sub-agent `.omp/agents/weaver-analysis.md` e `.omp/agents/weaver-implementer.md`, il `WATCHDOG.yml` aggiornato, e rigenera le skill `weaver-req-*`: una per ognuno dei 28 fragment con contenuto, compresi `weaver-req-rules-skills` e `weaver-req-rules-instructions` nati dalla divisione del fragment `rules/skills` (budget atteso circa 3.771 su 4.000: fragment mode).
- In `.agents/skills/` e in `.omp/agents/` non resta alcun nome che inizia con `trama-`; ci sono le sette skill `weaver-*`, le skill `weaver-req-*` di ogni fragment con contenuto e i due sub-agent `weaver-*`.
- La History e le Proposal già chiuse in `.weaver/`, la cartella `.trama.old/` e ogni altro file restano come sono.
