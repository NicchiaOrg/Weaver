---
status: approved
kind: change
base:
  model/app/feature.md: 2f904b91c6714b3ab85a315f89253155da709c34d404b8af28545b896838ff23
  requirements/quality/main.md: 6a8defbeff415a20a832ed2ad5e4e42a4ce77cc8d1a070717d55af9161cea884
  requirements/rules/skills.md: a0603fa659493f4b8e67f47a8d36a493fd67f4717ad6f642613e40731f360cae
  requirements/ui/board.md: 4c5878b847cf6f70fb23107d2000c1b93811dfdcecf913877a47cccc304f6f1c
  requirements/ui/look.md: 51e9fb44705c78bc869c38f18d08318602723fc5dc77c18815f18c397a322f12
  requirements/ui/views.md: a46ba0b89444742d3c5b45e85320365f05b9d7c6a8f2cf5ac8df6612d28e8ce1
  specs.yaml: b794808f03ecdb37e246445eef5e32afa83c9b05386ca789f8f5d8a9e2a209fa
created: 2026-11-02T14:40:00.054Z
request: 2026-11-02T15-18-52
label: Diagram rimosso
job: fwel7d26
---
# Rimuovere il Diagram

La vista Diagram esce dall'interfaccia di Weaver. Il Modello resta la fonte di verità e la logica che serve agli agenti (`weaver model show`, Task, confronto con il codice) non cambia: legge il Modello per una strada che non passa dal Diagram.

## Decisioni

- Codice: la vista Diagram viene **rimossa**, non solo nascosta: componenti, test, calcoli del backend usati solo dal Diagram (livelli, dettagli per elemento, ancore), cytoscape ed elkjs. Resta tutta la logica che serve all'agente.
- `View on diagram` sparisce dalle carte Proposal e Task e dai loro dettagli; resta `View requirements`.
- Le informazioni del pannello dei dettagli del Diagram (descrizione, Transition o Event, Drift e Task di un elemento) non compaiono in nessun altro posto.
- Un chip di un elemento del Modello nei dettagli di Task e Drift apre il focus sulle sezioni dei Requirements intitolate con l'elemento o con chi lo contiene; senza sezioni è testo semplice.
- Il chip del riferimento sul titolo delle sezioni della vista Requirements viene tolto; restano il titolo e `Open in editor`.
- Lo smoke test end-to-end controlla che si disegnino la Board e i Requirements.
- Le skill installate per gli agenti non parlano più di «livello del Diagram»: dicono «State, Transition ed Event del Modello».
- Di conseguenza: la descrizione della Feature `app` non nomina più il Diagram; barra laterale, overlay, focus, errori, accessibilità e Specs non lo citano più.

## Riallineamento al Modello attuale

- `requirements/quality/main.md`: riscritto sul testo attuale; lo smoke test end-to-end conserva il controllo, aggiunto nel frattempo, del Dialog dei dettagli su pixel CSS interi con la finestra di misure dispari, e disegna la Board e i Requirements invece del Diagram.
- `requirements/ui/look.md`: riscritto sul testo attuale; restano le regole aggiunte nel frattempo sui Dialog (centrati dal layout su pixel interi, apertura con sola dissolvenza di 150 ms) e sui toast (sezione `Toasts`, icone dei toast, toast di errore quando il browser non si apre) e il Diagram esce come prima da movimento ridotto, barra laterale e tabella delle icone.
- `requirements/ui/board.md`: riscritto sul testo attuale; resta la regola aggiunta nel frattempo sugli avvisi della History sopra le colonne, e `View on diagram` esce come prima dalle carte Proposal e Task e dai dettagli del Task.
- `requirements/ui/views.md`: riscritto sul testo attuale; `Open in editor` che fallisce mostra il toast di errore `Could not open the file in the editor.` come richiede ora il Modello, e il Diagram esce come prima da overlay, focus, titoli delle sezioni ed errori.
- Task «Rimuovere la vista Diagram»: lo smoke test mantiene il controllo del Dialog su pixel interi.
- Assunzione dell'agente: le regole aggiunte nel frattempo non nominano il Diagram e non contraddicono alcuna decisione, quindi nessuna domanda allo sviluppatore.
