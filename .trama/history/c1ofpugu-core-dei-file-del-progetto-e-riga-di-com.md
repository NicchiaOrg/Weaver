---
created: 2026-11-02T12:31:02.779Z
updated: 2026-11-02T12:31:02.779Z
summary: "Lettura, validazione e scrittura di .trama/ in trama-core come funzioni pure, e binario trama con cornice comune e regole dell'Actor agent: il solo canale con cui l'agente raggiunge Weaver."
tasks:
  - vxdhoylk
  - kmvi79rq
---
# Core dei file del progetto e riga di comando trama

Il core dei file del progetto è l'unico codice che tocca `.trama/`: layout, formato delle carte, validazione e scrittura come funzioni pure e testate, più l'immagine in memoria del progetto. App e CLI passano entrambe da qui, l'interfaccia mai.

La riga di comando `trama` (crates/cli, sopra trama-core) fornisce la cornice comune `trama <object> <verb> ...` con `--input`, `--root` e `--json`, e le regole dell'Actor agent. Su questa cornice si innestano i comandi dei singoli oggetti.
