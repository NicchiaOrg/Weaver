---
status: approved
kind: change
base:
  requirements/data/project-files.md: 3dbdab19ff493d0a6ab1c7c42355aead9cc961c1775eb8021f45ebf0b1c95c97
  requirements/rules/instructions.md: new
  requirements/rules/skills.md: 762bf61a5ce9c98b6a4361d53965c5b2dae4e7e256b321218d8c037f86a21be7
  requirements/tech/acp.md: 90c32cddca14d1bc4c19a0573e96cf5e7ffff6c4d22ae12d9a7c207103a88f0b
  requirements/tech/architecture.md: e7dcf2f8f7c57698016e17ed51fe098e9caec7d2dfdba14dafda4c15a482a7e5
  requirements/tech/ask-more.md: fde8ae5ea59109302ee936e1d4a3704477c7f86bd775d8e7108130e227636ae8
created: 2026-11-03T08:05:40.296Z
reworks: 3l0wlhhd
label: Prefisso weaver
job: 6r5sru3s
---
# Prefisso weaver per skill

Le skill, i sub-agent e le skill dei Requirement che Weaver installa o genera hanno ancora il prefisso `trama` del vecchio nome del prodotto: passano al prefisso `weaver`. Rifà la Proposal rifiutata `3l0wlhhd`, che non aveva feedback, sui Requirements attuali.

## Decisioni

- Passano a `weaver-…` tutti i nomi: le sette skill installate (`weaver-setup`, `weaver-design` con `FORMAT.md` e `COMMANDS.md`, `weaver-import`, `weaver-scan`, `weaver-compare`, `weaver-implement`, `weaver-resolve-drifts`), i sub-agent `.omp/agents/weaver-analysis.md` e `.omp/agents/weaver-implementer.md` (anche il loro `name:` e la voce `agentAdvisor` passata a omp), le skill dei Requirement `weaver-req-<area>-<fragment>` / `weaver-req-<area>`, le frasi delle istruzioni di ogni tipo di lavoro, il testo dell'Ask more sui Requirements e ogni riferimento nei testi installati, nel codice, nei test e nel `README.md`.
- Niente codice di migrazione: Weaver non conosce i vecchi nomi e non riconosce, rinomina o rimuove `.agents/skills/trama-*` e `.omp/agents/trama-*` nei progetti già inizializzati; l'aggiornamento delle skill installa e rimuove solo i nomi `weaver-*`.
- Il budget del fragment mode conta il nome nuovo (+1 carattere per fragment; con il fragment nuovo qui sotto questo repository arriva a circa 3.771 su 4.000 e resta in fragment mode).
- Restano come sono: la cartella `.trama` ignorata, l'eseguibile `trama` / `trama.exe` cancellato da `bin/`, il test che vieta la parola "Trama" nei testi installati e la frase vietata `trama-merge`.
- Il Requirement su `--slow` descrive ciò che fa già il codice: si passa solo per uno Scan, la cui skill affida le Machine a `weaver-analysis`, con il modello di scansione impostato.
- Una riga in `Changed` del `CHANGELOG.md` dice che skill e sub-agent si chiamano `weaver-…` e che i vecchi `trama-*` di Weaver si possono eliminare a mano. Nessuna sezione nuova nel `README.md`.
- Migrazione una tantum di questo repository nell'ultimo Task, dopo il rinomino nel codice: elimina ogni `.agents/skills/trama-*` e `.omp/agents/trama-*`, comprese `trama-handoff` e `trama-merge` (funzioni del vecchio prodotto fuori ambito, non nominate da nessun file) e `trama-specs` (le tecnologie stanno nelle Specs); esegue l'aggiornamento delle skill con la riga di comando compilata da questo repository forzando l'esecuzione locale; controlla che non resti alcun `trama-*` in `.agents/skills/` e `.omp/agents/`.
- La History e le Proposal già chiuse in `.weaver/` restano come sono.

## Cambiamenti rispetto alla Proposal rifiutata

- `requirements/tech/architecture.md` cambia solo nell'elenco dei file incorporati: la descrizione del fragment e le sezioni `window` e `notification`, spostate nel frattempo in `requirements/tech/windows.md`, restano come sono ora.
- Seguono i nomi nuovi anche i due riferimenti a `trama-req-*` comparsi dopo, nell'istruzione dell'Ask more sui Requirements (`requirements/rules#ask-conversation`, `requirements/tech#ask-conversation`).
- Scelta dell'agente, da controllare: con i nomi nuovi `requirements/rules/skills.md` supererebbe i 12 KB (12.331 byte), quindi le sezioni `job` e `ask-conversation` passano senza cambiare testo nel fragment nuovo `requirements/rules/instructions.md` ("Agent instructions"); `skills.md` tiene `project` e diventa "Agent skills", con la descrizione aggiornata.
