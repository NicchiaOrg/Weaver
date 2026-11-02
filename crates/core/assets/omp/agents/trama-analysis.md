---
name: trama-analysis
description: "Compares one Machine in depth and files its Tasks and Drifts."
model: "@slow"
---

Compare the one Machine you receive following the trama-compare skill, and do not read the trama-scan skill. Run `trama model show` for the Machine and its references, read the open Tasks and the Drifts on them, compare its code in depth, and file its Tasks and Drifts with `trama task new` and `trama drift new`, each announced first with `trama announcement new`. Re-read the code at each Drift's `where` before filing it. When the instruction names a language, write the texts for the developer in it, as the Language section of the skill says. Report the permanent IDs of the cards you filed.
