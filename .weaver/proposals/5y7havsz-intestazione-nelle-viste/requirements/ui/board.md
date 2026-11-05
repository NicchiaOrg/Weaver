---
description: "Use when building or changing the Board: columns, cards, ghosts, badges, card controls and details dialogs."
---
# Board

## work
- Two columns of equal width share the whole view, in this order: `Backlog` (open Proposals and open Drifts in one creation order, oldest first) and `Active` (Tasks in To do and In progress, blocked ones included, oldest first). No Done column, collapsed or not, and no other column.
- Each column title is followed by the number of its real cards; ghosts never count, a resolution card counts once. Empty texts: `Nothing to decide` (Backlog), `Nothing active` (Active), shown also above ghosts.
- The Backlog grouping (which Drifts sit in a resolution, which Proposals are hidden) comes computed from the backend; the interface never regroups.
- No card can be dragged, moved, taken back or removed from the Board. Clicking a card outside its controls, or Enter on its title, opens its details dialog.
- Each column scrolls on its own; its footer stays visible at its bottom.

### Footers and header
- Bottom of Backlog, full column width: `+ Work Item`, `outline` Button with a dashed border, always enabled; it opens a new design request (see `design-request`). Tooltip `Write a work item in Markdown and send it to an agent`.
- Bottom of Active, full column width: `Run all`, primary. Tooltip `Run all the Tasks in To do`. Disabled reasons, in order: `Drifts are being resolved`, `A run is in progress`, `No Task is in To do`, `All the Tasks in To do are blocked`.
- Board header (the view header of `requirements/ui#General`, title `Board`), on its right: `Scan System` (outline), followed by a muted `· <n> done since last scan`: the Tasks finished (not closed unchanged) since the last Scan System job started, deleted and forgotten ones included; hidden when 0. It confirms first (`Compare the whole System with the code?`, `Scan System` / `Cancel`); cancelling leaves the count unchanged.

### Ghosts
- A ghost sits at the bottom of its target column (Backlog for a Proposal or Drift, Active for a Task), after the real cards and before the footer, in announcement order.
- Skeleton card: dashed border, 70% opacity, shimmer on its text lines. It shows the kind (`Proposal`, `Task`, `Drift`), the title, a phase line, then the job's latest note in muted text on one line cut with `…` (full text as tooltip).
- Phase line: `Preparing` until the Draft exists, `Writing` once it exists, `Waiting for your answers` while the job waits for a round or a permission, `Being refined` for a refinement.
- The whole ghost is one button that opens its job's conversation in Jobs. Accessible name `<kind> being prepared by the agent: <title>, <phase>`.
- When the card arrives, in the same picture the ghost disappears and the card takes its place in creation order.

### Details dialogs and notices
- A details dialog follows its card: each new picture updates it in place without moving the focus. When the card leaves the Board (decided, Done, deleted, gathered into a resolution) the dialog closes by itself and the focus goes to the column the card was in.
- Notices sit at the top of the Board, above both columns, newest first, each an `Alert` with a close button, until closed or the window closes: decisions refused when applied (`Could not delete T4: the agent that filed it is still working.`) and Proposals sent back to Draft while the developer was deciding them.
- History warnings sit under the notices, above both columns: one amber `Alert` with `TriangleAlert` for each cited Task whose copy in History could not be made (`Could not record T3 in History: <reason>. The Task stays until its copy can be made.`), each with a close button (`X`, accessible name and tooltip `Close`). A closed warning stays hidden while the window stays open, even when the Board changes; it shows again at the next opening of the window if it is still true. A new warning, or one whose text changed, shows at once.

## proposal
- Card, top to bottom: id, title (at most 3 lines, then `…`) and kind mark; badges; files line; Tasks line; controls.
- Kind mark: outline badge `setup` or `import`; an ordinary change has none; a resolution is the yellow card below.
- Badges, in order: `stale` (amber), `refining` (blue, a refinement job works on it), `refinement interrupted`, `blocked` (red), the chosen model (outline, icon `Cpu`). `blocked` is a Popover button listing every blocking reason on its own line; a reason about a required Proposal names it and says `is still being written`, `was rejected`, `was discarded`, `is missing`, `is not approved yet` or `forms a cycle`.
- Files line: `<n> files` with a muted `+a ~c −r` (added, changed, removed); paths, relative to `.weaver/`, are in the details.
- Tasks line, only when it carries Tasks: `Creates <n> Tasks` (code-removal Tasks included) and its label as a badge in the label colour (see `task`).
- Controls: `Approve` (primary), or `Refine` in its place (see below); `Reject`, a split button whose menu has one item, `Rework with feedback…` (absent once reworked); `View requirements`, only when its focus would have at least one step; last, the model control (icon `Settings`, only when it carries Tasks), pushed to the card's right edge, also when the row wraps; the other controls stay on the left.
- `Approve` disabled reasons, in order: `The agent is still finishing`, `A run is in progress`, `Being refined`, then each blocking reason. When staleness is its only blocker, `Refine` replaces `Approve`, enabled also during a Run, with the tooltip `Refine on the current Model, then approve`. When staleness comes with any other reason, the control stays a disabled `Approve` listing every reason, `stale` included. `Reject` and `Rework with feedback…` are disabled only by `The agent is still finishing` and `Being refined`.
- `Refine` tells that a click starts the agent: text and accessible name `Refine`, icon `Sparkles`, solid yellow (background `yellow-500` in light and `yellow-400` in dark, one shade darker on hover, text `zinc-950`), no confirmation. It shows wherever the card's controls show, the details dialog included.
- `Reject` opens a Dialog with an optional `Reason` Textarea and `Reject` (destructive) / `Cancel`. `Rework with feedback…` opens a Dialog with a `Feedback` Textarea and `Rework` / `Cancel`; blank feedback asks `Rework without feedback?` first.
- The model control opens a Popover with a Select of omp's models (the Settings list), `Use the Settings models` first; a choice applies at once.
- A Draft left half-way by a refinement keeps its card with the badge `refinement interrupted` and only `Refine` (tooltip `Continue refining, then approve`) and `Discard` (destructive, confirmed). No other Draft has a card.
- Resolution card: same layout, 3 px yellow left border and light yellow tint, mark `resolution`, then one row per listed Drift: id, title and decision (`Fix the code: option <n>` with the note muted, `Change the Model`, `Let it be`). Rows open the Drift's details. Lines with nothing to say are omitted.
- Details dialog: id, title, kind, badges, rendered body, `Requires` (each Proposal with its state), files grouped as added, changed and removed, `Tasks it creates` and `Code-removal Tasks`, then the card's controls followed by `Ask more` (see `ask-conversation`), which the card itself never shows.
- Task lists in the details dialog: one compact row per Task holding only its title; no ref chips and no `technical` badge. Each row of `Tasks it creates` is a button that opens a nested Dialog over the details dialog with the Task's title and its rendered body (`No description.` when blank), no refs and no badges; `Esc` or `Close` closes only the nested Dialog and the focus returns to the row. Code-removal rows use the same list style but are plain text, not controls, and render the reference in their title as inline code, not with literal backticks. When the details dialog closes, the nested Dialog closes with it.

## drift
- Card: compact, 3 px red left border; id, title (at most 3 lines), badges; then `Resolve` (primary) and `Dismiss Drift` (outline).
- Badges: `recurring` (tooltip names the earlier Drift), `resolving` (a resolve job works on it or a resolution Draft lists it; tooltip `An agent is resolving this Drift`), `prepared` (valid prepared question; tooltip `Resolve asks the prepared question at once`).
- `Resolve` disabled reasons, in order: `The agent is still finishing`, `A run is in progress`, `This Drift is being resolved`, `Another resolution is being written`, `Proposal <id> resolving Drifts awaits a decision`.
- `Dismiss Drift` confirms with `Dismiss <id>? Nothing changes in the code or in the Model.`, `Dismiss` (destructive) / `Cancel`. Disabled reasons: `The agent is still finishing`, `This Drift is in a resolution`.
- Details dialog, in order: id, title, badges; referenced elements and sections as chips (each opens a single-ref focus); `What happens` and `What should happen` side by side (stacked below 640 px of dialog width); `Fix options`, numbered, the first marked `recommended`; `Where in the code`, one row per file with its line when given and `Open in editor`; the origin Task and the repeated Drift, when known; the attached Proposal, if any, with title, rendered body and files.
- Fix decision, in the details: a RadioGroup of the fix options (recommended preselected), an optional `Note` Textarea and `Fix the code`. Without options the RadioGroup is replaced by `No code change can respect both the Model and the Requirements.` and there is no `Fix the code`. With an attached Proposal, `Accept Proposal`; disabled with that Proposal's blocking reasons when it cannot be approved. `Resolve`, `Dismiss Drift` and then `Ask more` (see `ask-conversation`) are in the footer. `The agent is still finishing` comes first on every decision control.

## task
- Card: status dot, id, title (at most 3 lines), badges; under them the Proposal's label badge when it has one; then `View requirements` (only when its focus would have at least one step) and `Run` (outline).
- Status dot, 8 px, never animated, with tooltip and accessible name: To do = hollow ring in `muted-foreground`, `To do`; In progress = filled amber, `In progress`, or `Agent working` while a job holds it.
- Badges: `technical` (outline), `blocked` (red), `waiting <id>` (amber); both may show. `blocked` and `waiting` are Popover buttons: `Blocked by` lists the blocking Drifts (id, title) in creation order, `Waiting for` lists the fix Task; a row closes the popover, scrolls its card (or its row in a resolution card) into view and focuses it. Tooltips: `Blocked by Drift D3, D7`, `Waiting for Task T4`.
- Left border 3 px in its label colour: the FNV-1a 32-bit hash of the label's UTF-8 bytes modulo 6 picks, in order, blue, green, amber, violet, cyan, pink. A Task outside any design uses the `border` token.
- While a job holds it: a 2 px conic border rotating blue → cyan (CSS only, one turn every 3 s), static cyan under reduced motion. No other card has it.
- `Run` disabled reasons, in order: `The agent is still finishing`, `Drifts are being resolved`, `A run is in progress`, `The agent is working on it`, `Blocked by Drift <ids>`, `Waiting for Task <id>`. Another Task held by the agent never disables it.
- Details dialog: id, status, badges, label, refs as chips (single-ref focus), rendered body, for a fix Task its Drift's two sides, chosen option and note; then `View requirements`, `Run`, `Ask more` (see `ask-conversation`; not on the card) and, only in To do, `Delete` (destructive), confirmed with `Delete <id> for good?` / `Delete` / `Cancel`. `Delete` disabled reasons, in order: `The agent is still finishing`, `Origin of open Drift <id>`, `Fix Task of Drift <id>, whose code is to be fixed`.
