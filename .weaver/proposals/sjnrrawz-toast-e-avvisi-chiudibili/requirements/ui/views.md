---
description: "Use when building or changing the Diagram, Requirements, Specs, History, focus, overlay or welcome window."
---
# Views

## app

### Diagram
- Drawn with cytoscape, laid out with elkjs (`layered`, direction `RIGHT`); the root level uses a grid with no edges.
- Levels: System (root) → Whole System or an Actor → Feature → Machine → State. The root shows, in this order, a `Whole System` node, the Actors in Model order, then the Features no Actor reaches. Whole System shows every Actor and Feature with edges labelled by their number of Events; an Actor level shows the Actor, the Features receiving its Events and the edges between them. A Feature level shows its Machines and the Event edges between them, plus the outside Actors and Machines sending them, as compact dashed nodes. A Machine level shows its States (initial with a filled dot, final with a double border) and Transitions labelled `<sender>.<EVENT>`. A State level shows the State with the Transitions entering and leaving it and their other ends.
- Subtitles: `<n> features` on Actors, `<n> machines` on Features (singular for 1). An Actor whose Events reach no Machine shows `No events received`, a dashed border, muted text, and cannot be opened.
- Click selects an element and shows its details in a right panel (360 px): title, rendered description, Transitions or Events, the open Drifts and the Tasks that concern it, `Open in editor`. Double-click, or Enter on a selected element, opens its level. On a Feature level a Machine node can be expanded in place to show its States inside it, and collapsed again.
- Breadcrumb above the canvas: `System › <Actor | Whole System> › <Feature> › <Machine> › <State>`, every segment clickable; a level opened from a link starts at `System › <Feature>`.
- Hover box on a node or edge: the title and the first sentence of its description (of the Transition's text for an edge), never more; just the title when there is no description.
- An element with open Drifts carries a red badge with their count.
- When the Model or the overlay changes, the Diagram keeps its level and selection; when the level no longer exists it falls back to the nearest container that does, up to the root.
- `Fit` fits the whole level. The viewport is kept per level while the window is open.

### Overlay of a Proposal
- The Diagram, Requirements and Specs share one overlay picker per window: `Current Model` first, then every open Proposal not attached to a Drift, by id and title. Never a Draft, a Proposal being refined or an attached Proposal. When the chosen Proposal stops being open, the views go back to `Current Model` and a notice says why.
- The overlay draws the Model with the Proposal applied on top of the Proposals it requires.
- Change colours (Tailwind 600 light / 400 dark): added `green`, changed `amber`, removed `red`; a removed element stays drawn with a dashed border. Every change also has a text label (`added`, `changed`, `removed`) in the details, the hover box and the accessible name.

### Focus
- Each of Diagram, Requirements and Specs has its own focus, opened by `View on diagram` / `View requirements` (a Proposal's scope or a Task's refs) or by a chip (a single ref). Steps: the levels holding the focused elements, in Model order, on the Diagram; sections, Areas or fields in page order elsewhere.
- Focus bar, full width under the view's toolbar: what it is about (`Proposal P3 – <title>`, `Task T4 – <title>`, or the ref), the step name (breadcrumb, section, Area or field), `<n> of <N>`, `Previous` / `Next` (icon-only; disabled at the ends), `<M> not found` (muted, tooltip lists the missing refs one per line), on Requirements an `Only these` toggle (off at start), and `Clear`. With one step there is no counter nor arrows.
- On the Diagram, focused elements get a thick border and light tint in `cyan`, or their change colour under an overlay; every other node, edge and label drops to 30% opacity and stays clickable. Going to a step moves the viewport to contain the step with a margin, never zooming beyond `Fit`. On Requirements and Specs, focused sections and fields get a cyan (or change-colour) left border and tint; going to a step scrolls it to the top of the view.
- When files change, the focus is recomputed and stays on the same step if it still exists. Esc first clears a selection, then the focus. `Clear` keeps the level and the overlay. Step changes are announced to assistive technologies.

### Requirements view
- Areas in folder-name order, each with its title and prose; inside, fragments in file-name order (`<fragment>.md` in muted text next to the title); inside, sections with `General` first, then in file order.
- Each section heading carries its reference as a chip that opens the element on the Diagram, and `Open in editor` at the heading's line.
- Overlay labels: `new fragment`, `fragment removed`, `added`, `removed`, `changed`, `moved from <fragment>`. A removed section is shown struck through in muted text; a changed one shows the new text with a `Show before` toggle that shows the old text.

### Specs view
- Fields in this fixed order: languages, frameworks, data, testing, CI, repository, deployment, infrastructure, notes. Empty fields are hidden.
- Max width 1100 px; entries in a grid of columns at least 300 px wide. An entry shows name, version and note; one the agent chose carries the `Sparkles` mark `agent's choice` whose tooltip is its reason.
- Overlay: entries labelled `added`, `changed`, `removed` as on Requirements.

### Errors and the editor
- An invalid Model or Requirements shows, at the top of the Diagram, Requirements and Specs, a red `Alert` `The Model has <n> errors` with an expandable list: each error with its message and file, its line when known, and `Open in editor`. Unreadable files are listed the same way.
- `Open in editor` is shown wherever a file and a line are known and opens them in the editor chosen in the Settings; when it fails, an error toast `Could not open the file in the editor.` gives the reason as its description.

## history-entry
- The History view lists entries newest first by their date (written or last updated), grouped by local day. Group headers: `Today · <n>`, `Yesterday · <n>`, then the long date (`Monday, 2 November 2026 · <n>`).
- An entry row: title, one-line summary (cut with `…`, full text as tooltip), the time `HH:MM` in Today and Yesterday, the number of Tasks.
- Period filter, a Select: `Today`, `Last 7 days` (default at each opening of the window), `Last 30 days`, `All`. A period has both bounds: from local midnight of its first day (today, 6 days ago, 29 days ago) to now; an entry dated in the future is shown only under `All`. Empty: `Nothing done in this period`.
- Clicking an entry opens a Dialog: title, date, summary, rendered text, then its Tasks, from the copies kept in the entry, in the order cited: id, title, label badge, finished time; a row expands to show the Task's rendered body and refs, read-only.

## window
- With no project window open, Weaver shows the welcome window, 640 × 480 px, not resizable: the app name and version, the recent projects (up to 10, most recent first; folder name, full path muted; a folder that no longer exists is marked `Not found` and can be removed from the list), `Open Folder` (system folder picker), and the omp status line linking to the omp setup (see `omp-setup`).
- Clicking a recent project or choosing a folder opens it as described by `developer.OPEN_PROJECT`; the welcome window closes once a project window is open.
- A project window's title is the project folder's name.
