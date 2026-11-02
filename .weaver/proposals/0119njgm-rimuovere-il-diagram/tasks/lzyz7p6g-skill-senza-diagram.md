---
technical: true
refs:
  - requirements/rules#project
---
# Skill senza Diagram

L'app non ha più una vista Diagram, quindi i testi che Weaver installa per l'agente non devono più nominarla. Le skill e le definizioni dei sub-agent installate misurano il dettaglio di un confronto con gli State, le Transition e gli Event del Modello; non citano mai un Diagram. Il significato delle regole non cambia: cambia solo come sono scritte.

## Cosa deve valere alla fine

- In `crates/core/assets/skills/` nessun testo installato contiene «Diagram». Le frasi da riscrivere oggi sono:
  - `trama-compare/SKILL.md:25` — «something extra that would change the Diagram» → qualcosa in più che aggiungerebbe o cambierebbe uno State, una Transition o un Event del Modello.
  - `trama-compare/SKILL.md:28` e `trama-scan/SKILL.md:64` — «Anything below Diagram level…» → sotto il livello di State, Transition ed Event del Modello.
  - `trama-compare/SKILL.md:31` e `trama-scan/SKILL.md:83` — «would drawing it change the Diagram?» → aggiungerebbe o cambierebbe uno State, una Transition o un Event del Modello?
  - `trama-scan/SKILL.md:22` — «Compare behaviour at the level of the **Diagram**: States, Transitions, Events…» → al livello di State, Transition ed Event del Modello, chi reagisce a cosa.
  - `trama-scan/SKILL.md:61` — «…a State or Transition in the Diagram» → nel Modello.
  - `trama-implement/SKILL.md:99` — «Small choices that stay below Diagram level…» → sotto il livello di State, Transition ed Event del Modello.
- Controllare anche le definizioni dei sub-agent e le istruzioni dell'advisor installate (`.omp/agents/trama-analysis.md`, `.omp/agents/trama-implementer.md`, `.omp/WATCHDOG.yml`, sorgenti in `crates/core/assets/`): nessuna cita il Diagram.
- I testi restano in inglese, con fine riga LF, senza prima persona; ogni definizione di sub-agent resta sotto i 6 KB. Le copie installate nei progetti si aggiornano da sole al prossimo sync, perché il loro contenuto cambia.
- Eventuali test che confrontano il contenuto delle skill installate vanno aggiornati.
