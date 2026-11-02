---
status: approved
kind: change
base:
  model/agent-work/general-chat.md: 33830490ec61278c270272986bfc92abd9ef0b7d90446b434a09803262faa092
  requirements/tech/omp.md: e4fabeee190c30e8bc3e956ab4919a4733ff7e7392570d5718add503c6031d3e
  requirements/ui/conversations.md: f01462cb67f00455b26474f1a4f4e36e3d5e6976774ef5c0b6d0350c879ed036
created: 2026-11-02T15:57:47.043Z
reworks: 4jb3qm6l
label: General ottimistico
job: 5feakb5r
---
# General: avvio ottimistico

Dopo un riavvio di Weaver, la vista Chat (General) mostra il testo vuoto finché il developer non scrive il primo messaggio, anche quando c'è una conversazione conservata: Weaver tiene solo l'identità della sessione e omp restituisce la cronologia solo quando parte, cioè al primo messaggio. La modifica mostra subito la cronologia, senza inviare nulla a omp prima del primo messaggio.

Rifacimento della Proposal respinta `4jb3qm6l` (feedback: «Ri-update»): stesse decisioni, riscritte sul Model e sui Requirements attuali, che nel frattempo hanno rinominato General in `Chat` nell'interfaccia e ordinato `session/load` con gli aggiornamenti riprodotti.

## Decisioni

- Meccanismo: all'apertura Weaver avvia omp e ricarica la sessione conservata senza inviare alcun messaggio (come per i job sospesi); Weaver continua a non conservare il testo.
- Momento: all'apertura della finestra del progetto, non alla prima apertura della vista Chat.
- Progetti senza conversazione conservata: omp parte solo al primo messaggio, come oggi.
- Durante il caricamento la vista mostra il testo secondario centrato `Loading conversation…`.
- Sessione non ricaricabile: si apre subito una conversazione vuota con l'avviso; omp resta pronto per il primo messaggio.
- omp che non parte, termina o supera il tempo limite durante il caricamento: `omp stopped: <reason>.` e `Your next message starts omp again.`; il primo messaggio riavvia omp e ricarica la sessione.
- omp non installato o non pronto all'apertura: con omp mancante si mostra subito `omp is not installed.` con `Install omp`; appena omp è pronto Weaver ricarica la conversazione da solo.
- Messaggio inviato durante il caricamento: compare subito con `Starting omp…` sotto, la cronologia si mette sopra di esso e il messaggio parte appena il caricamento finisce.
- `Clear` funziona durante il caricamento: interrompe il caricamento, cancella la conversazione e ne inizia una vuota.
- `Clear` durante un turno dell'agente: lo interrompe, cancella la conversazione e ne inizia una vuota; il Model si allinea al codice e all'interfaccia, che già lo fanno.
- `Stop` mentre omp sta ancora partendo per il primo messaggio: termina anche omp, e il messaggio successivo lo riavvia e ricarica la conversazione; il Model descrive ciò che il codice già fa.

## Assunzioni dell'agente

- Mentre omp è solo in verifica, in aggiornamento o senza provider connesso, la vista mostra soltanto `Loading conversation…`: la riga `omp is not installed.` compare solo quando omp manca davvero.
- Il dialogo di `Clear` durante il caricamento usa il testo normale (`Clear Chat?` / `The conversation is deleted and a new one starts.`), perché il caricamento non è un turno dell'agente.
- Chiudere la finestra durante il caricamento termina omp e conserva la conversazione, come oggi durante un turno.
- Il caricamento all'apertura usa lo stesso limite di 30 s dell'avvio di omp e lo stesso ordinamento di `session/load` con gli aggiornamenti riprodotti già richiesto per la ripresa: la cronologia arriva in un unico blocco e nessun aggiornamento riprodotto entra nella conversazione dal vivo. L'ordinamento stesso resta alla Task già aperta `fvci4zh6`, e i testi `Chat` dell'interfaccia alle Task già aperte `f2ws7caj`, `h6spvmt3` e `lyktflgx`.
- Il feedback «Ri-update» è stato letto come richiesta di rifare la stessa modifica sui file aggiornati, senza cambiare le decisioni già prese.
