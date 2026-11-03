---
status: approved
kind: change
base:
  model/actors/agent.md: bc9bb7b3910cd5eaed1632051614a263aee60c19d3406517f595e03a419fe31a
  model/actors/developer.md: 0b1d8543de65135f4a8ebe327aef00989538e0604e34a55df47bd321115bf108
  model/agent-work/ask-conversation.md: new
  model/agent-work/feature.md: 4b10763b09d99a6ecac2d8b00f9183aa386acab1f9d907eabb4712dd55b9c4ce
  model/agent-work/general-chat.md: 34f26396079caaa98d9386059238bb81ee42a4bc1924663593ac760ef8152ec0
  model/agent-work/job.md: 934677a12ed4ebf14077e4e6e2a0196812e6e2ad6bc3da2148b9efca077c0d83
  model/agent-work/question-round.md: 9c428a1622760c34d584d23945d73f5a3366e3ee0b1e221138e0a9e25b282e5c
  requirements/cli/commands.md: 212a7a1d29c764a59a7ea48dd197aa490ef3b5908866188fb738041b51449c15
  requirements/rules/skills.md: 9d50450a77142338d595f460bb73058cd0c8bfe58d0dbe05cf5c68af466dcf03
  requirements/tech/architecture.md: c56aa6f2fed05930068c71fa05e59b38b5d3a3290bbd68a24f457eb744d93e7d
  requirements/tech/ask-more.md: new
  requirements/ui/ask-more.md: new
  requirements/ui/board.md: d01314cb8b5a08269798e799ce15fb5415cbc42769013ff52ad81f22f200c969
  requirements/ui/conversations.md: 189e7308c59cbb3d223ccfc32eb480543e1ce671c548dea4b40b83702dc1d108
  requirements/ui/rounds.md: 5ed860d6f2292e9b283ba5ea11b8661e381f72886e0efe0e9db4c91d94ee3e06
  requirements/ui/views.md: 72f2e5506e83875f118724fa9168ea8e0159c6c4715d32fbdf7c3ea575748766
created: 2026-11-03T03:41:21.111Z
request: 2026-11-03T04-15-24
label: Ask more
job: 32c4xwlm
---
# Ask more

Riporta in Weaver il pulsante `Ask more` del vecchio Trama, adattato: dalla finestra dei dettagli di una Proposal, di un Task o di un Drift, dalla domanda in vista di un round, da ogni sezione della vista Requirements e da un Task copiato in una voce della History, il developer apre una conversazione nuova con omp, vuota, il cui primo messaggio parte con un'istruzione di Weaver che descrive la cosa scelta. Niente terminali, hook, ghost sulla Board né Unify Drifts.

## Decisioni

- La conversazione vive nella vista Jobs: una riga per conversazione, tipo `ask`, nome `Ask <cosa scelta>` (numero breve della scheda, `<area>#<sezione>`, `question`, o identità permanente per un Task della History); non è un job e non conta per nessuna esclusione.
- Distintivi della riga: `running` durante un turno, `waiting` mentre un permesso aspetta, `idle` (nuovo, outline) a riposo e prima del primo messaggio.
- Ordine: con i job, per stato; le `idle` dopo le suspended e prima dei job finiti, dalla più recente.
- La riga mostra solo nome, tipo, distintivo e ora di apertura: niente tempo di lavoro né note.
- Finisce con la finestra: chiudere la finestra o uscire da Weaver interrompe il turno e la conversazione sparisce; nulla si conserva su disco.
- Chiusura: `X` sempre presente sulla riga; mentre l'agente lavora chiede conferma (`Close <nome>?`), a riposo chiude subito.
- `Stop` come Chat: senza dialogo, annulla solo il turno.
- Come Chat, non conta per la conferma di chiusura della finestra o di uscita, e i suoi permessi non notificano né contano nella barra laterale.
- Nessuna barra dell'uso: nessuna conversazione di Weaver ne ha una.
- History: in Weaver un Task citato non esiste più come file, quindi `Ask more` sta sulla copia, nella parte espansa della riga del Task nella finestra della voce; l'istruzione dice la copia e la voce (identità, titolo, riassunto), con `weaver history show <voce>` e `weaver proposal show <id>` se la Proposal esiste ancora.
- Round: premere `Ask more` riduce il round alla barra in fondo, con scelte e testo intatti; la conversazione di Ask more mostra `Back to questions` finché quel round è aperto, che lo riapre sulla stessa domanda.
- L'istruzione su una domanda nomina anche il job che ha aperto il round (nome e identità); per la domanda pronta di un Drift, l'identità del Drift.
- Requirements: la sezione "non esiste più" quando la vista ricalcolata al clic con lo stesso overlay non la ha più, o la Proposal dell'overlay non è più aperta; per una sezione `added` l'istruzione elenca solo `weaver proposal show <id>`.
- Il messaggio del developer perde spazi e righe vuote iniziali e finali, come ogni altro messaggio.
- L'agente di Ask more ha una variabile sua, `WEAVER_ASK`, accettata da `weaver job new` mentre quella conversazione lavora; la riga `Started <job>` appare nella conversazione di Ask more.
- Se omp si ferma e la sessione non si ricarica, il messaggio che apre la sessione vuota riparte con l'istruzione davanti, riletta dai file in quel momento; se la cosa scelta non esiste più parte da solo.

## Scelte dell'agente

- Una Machine nuova `ask-conversation` nella Feature `agent-work`, sul modello di Chat, invece di un tipo di job: il ciclo di un job finisce al primo fine turno, Ask more no.
- L'esempio `Ask ui#board` della richiesta non è una sezione esistente: i nomi usano il riferimento reale della sezione (per esempio `Ask ui#app`).
- `Ask more` non va sulle intestazioni di Area e di frammento della vista Requirements, solo sulle sezioni, come dice la richiesta.
- Senza numero breve (una scheda che non ne ha), nome e nome accessibile usano l'identità permanente, come già fanno gli altri controlli.
- I moduli di domanda propri di omp sono rifiutati con un testo che chiede all'agente di rispondere nel messaggio, come in Chat.
- La sezione `## Language` del testo fisso non esiste: l'istruzione aggiunge solo la frase della lingua di Chat (`Write the texts for the developer in <language>.`).
- I testi dei pulsanti e delle conversazioni sono in inglese; nuovi frammenti `ui/ask-more` e `tech/ask-more` perché `ui/conversations` e `tech/acp` supererebbero 12 KB.
