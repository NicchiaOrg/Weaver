---
status: approved
kind: change
base:
  model/actors/developer.md: 7312605ff64f86ae2da0812a5b225af53e683a5708ef8ffdcd64c9eca9fa598c
  model/actors/omp.md: efda8fc003fdaeb0aa6b79321de20e1341502ee85170167f768021ca32d52316
  model/project-setup/feature.md: 7600e5ba2896879f2cf377c60bf56ea54d6a5017c8f4eb8a4256967a88c6e24c
  model/project-setup/usage.md: new
  requirements/tech/omp.md: c21d0b1ca0636a89c77c1f5df65836ecb1330fb51f54ba613d2c284f98081e37
  requirements/ui/look.md: bdcc6a31f2a224978feca3d6009c0a3fb1eccbbe28428aa901fe2d968ce32ec5
  requirements/ui/views.md: d8e4523b42efbfde672ca3b7f85f6044534920df041664d3efb7414119daa9b4
created: 2026-11-02T21:12:56.638Z
request: 2026-11-02T21-59-53
label: Usage
job: i2rjgj2s
---
# Settings in basso e Usage

Settings si stacca dalle altre voci della barra laterale e va in fondo; una nuova voce Usage, sopra Settings, apre un piccolo Dialog con i limiti d'uso dei provider collegati a omp, letti con `omp usage --json` senza chiamare alcun modello.

## Decisioni

- In fondo alla barra laterale, separati dalle altre voci dallo spazio vuoto: Usage, poi Settings come ultima voce.
- Usage apre un Dialog piccolo centrato (480 px), non una vista.
- Mostra solo i limiti degli account (quanto è usato e quando si azzera), non i token consumati.
- Un blocco per provider, nell'ordine della lista fissa; un solo account per provider, quindi nessun identificativo (email, account, organizzazione) è mai mostrato.
- Solo i 7 provider noti a Weaver; gli altri account di omp non compaiono.
- I dati si leggono solo a ogni apertura del Dialog: nessun pulsante di aggiornamento, nessuna lettura in background, nessun segnale sulla voce.
- Con omp non pronto il Dialog si apre, dice perché e offre Open Settings; quando omp diventa pronto legge da solo.
- Ogni limite: nome, barra, valore secondo l'unità e tempo al reset; ambra con `Almost used up`, rosso con `Used up`.
- In caso di errore: avviso con Retry; i dati letti prima restano visibili con la loro ora.
- Un provider collegato senza limiti compare con il motivo; se nessun provider ha limiti, `No usage limits to show.` sopra le righe.
- Mentre legge, i dati precedenti restano visibili con `Updating…`; i dati letti valgono per tutta l'app finché Weaver resta aperto.
- La lettura fallisce dopo 30 secondi.

## Assunzioni dell'agent

- Un reset entro 24 ore si scrive `Resets in <durata>`, uno più lontano `Resets on <data>`.
- Se omp riporta comunque più account di uno stesso provider, i loro limiti si elencano uno dopo l'altro nello stesso blocco, senza identificativi.
- Una lettura ancora in corso quando il Dialog si chiude continua; il suo risultato vale per la prossima apertura.
- Usage non compare nella finestra di benvenuto, che non ha barra laterale.
