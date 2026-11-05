---
status: rejected
kind: change
base:
  model/agent-work/general-chat.md: 33830490ec61278c270272986bfc92abd9ef0b7d90446b434a09803262faa092
  requirements/tech/omp.md: fce66d57a0e883f00c1b552d0a8bb70d937d263d4127d4291bf3e87dce1e1f4e
  requirements/ui/conversations.md: 1a5bced9b4a056b71a6f67116e9f9e1a5f01b7409606bd55c1be85e36da9e3ef
created: 2026-11-02T15:40:32.624Z
request: 2026-11-02T16-33-03
reworkRequested: true
label: General ottimistico
job: 4tixby14
---
# General: avvio ottimistico

Dopo un riavvio di Weaver, General mostra il testo vuoto finché il developer non scrive il primo messaggio, anche quando c'è una conversazione conservata: Weaver tiene solo l'identità della sessione e omp restituisce la cronologia solo quando parte, cioè al primo messaggio. La richiesta chiede di mostrare subito la cronologia, senza inviare nulla a omp prima del primo messaggio.

## Decisioni

- Meccanismo: all'apertura Weaver avvia omp e ricarica la sessione conservata senza inviare alcun messaggio (come per i job sospesi); Weaver continua a non conservare il testo.
- Momento: all'apertura della finestra del progetto, non alla prima apertura della vista General.
- Progetti senza conversazione conservata: omp parte solo al primo messaggio, come oggi.
- Durante il caricamento la vista mostra il testo secondario centrato `Loading conversation…`.
- Sessione non ricaricabile: si apre subito una conversazione vuota con l'avviso; omp resta pronto per il primo messaggio.
- omp che non parte, termina o supera il tempo limite durante il caricamento: `omp stopped: <reason>.` e `Your next message starts omp again.`; il primo messaggio riavvia omp e ricarica la sessione.
- omp non installato o non pronto all'apertura: con omp mancante si mostra subito `omp is not installed.` con `Install omp`; appena omp è pronto Weaver ricarica la conversazione da solo.
- Messaggio inviato durante il caricamento: compare subito con `Starting omp…` sotto, la cronologia si mette sopra di esso e il messaggio parte appena il caricamento finisce.
- `Clear` funziona durante il caricamento: interrompe il caricamento, cancella la conversazione e ne inizia una vuota.

## Assunzioni dell'agente

- Mentre omp è solo in verifica, in aggiornamento o senza provider connesso, la vista mostra soltanto `Loading conversation…`: la riga `omp is not installed.` compare solo quando omp manca davvero.
- Il dialogo di `Clear` durante il caricamento usa il testo normale (`The conversation is deleted and a new one starts.`), perché il caricamento non è un turno dell'agente.
- Chiudere la finestra durante il caricamento termina omp e conserva la conversazione, come oggi durante un turno.
- Il caricamento all'apertura usa lo stesso limite di 30 s dell'avvio di omp.

## Feedback

Ri-update
