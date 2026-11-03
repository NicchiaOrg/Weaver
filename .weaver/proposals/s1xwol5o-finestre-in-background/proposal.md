---
status: approved
kind: change
base:
  model/actors/developer.md: 7368cbba8c42e8ec187b644f431c5c90bdde86b4b4366cdb92f2416c2bc039c2
  model/app/window.md: dd1ce49464a693dc2584ede5d02c5d0be9aa78dfa0b47c6761db092a9660b81f
  requirements/tech/windows.md: e5e9efc0a13a78a39374ee6ba9d9eeb7eb963834aec1600cba3d65e718d124c6
  requirements/ui/look.md: f6e4497bb7c959e4008a567f0f280d20414227f520e9ef59586f6bf3ed2232a0
created: 2026-11-03T09:47:31.197Z
label: Run in background
---
# Finestre in background

Lo sviluppatore vuole mandare in background la finestra di un progetto senza sospendere i job, e farla tornare dal Dock, rilanciando l'app o dall'icona nella tray.

## Decisioni

- Il tasto `Run in background` (icona `EyeOff`) sta nella barra laterale, sopra Usage, ed è sempre attivo.
- Il tasto nasconde solo la finestra del progetto da cui viene premuto; ogni finestra di progetto può essere nascosta, anche tutte.
- Una finestra in background resta aperta: job, General, Ask more e question round continuano; le notifiche la trattano come in background.
- Riaprire Weaver dal Dock (macOS), rilanciandolo senza cartella o con `Show Weaver` dalla tray fa tornare tutte le finestre in background, con il focus su quella usata per ultima.
- Icona nella tray con il menu `Show Weaver` / `Quit`.
- La chiusura con la X e conferma sospende ancora i job, ma la conferma con soli job in corso offre anche `Keep working in background`.

## Assunzioni da verificare

- La tray è solo su Windows e Linux: su macOS il Dock resta il modo per far tornare le finestre.
- L'icona nella tray è sempre presente mentre Weaver è aperto; su Windows il clic sinistro equivale a `Show Weaver`, su Linux c'è solo il menu.
- `Keep working in background` compare solo nella conferma con job in corso e senza modifiche non salvate: il dialogo nativo ha al massimo tre pulsanti, e con modifiche non salvate restano `Save all` / `Discard all` / `Cancel`.
- Un rilancio con una cartella o il clic su una notifica fanno tornare solo la finestra di quel progetto.
- La chiusura dall'app (`Quit`) resta come ora e non offre il background.
- Alla partenza successiva, le finestre che erano in background si riaprono visibili.
