---
name: trama-req-ui-views
description: "Use when building or changing the Requirements, Specs, History, focus, overlay or welcome window."
---

# Interface

Every text of the interface is in English, whatever the language setting says; that setting only changes the language of the agent's texts. The interface only displays what the Rust backend holds and sends the developer's requests to it: it keeps no state of the project, the cards or the jobs of its own, and never reads or writes project files itself.

## app

### Overlay of a Proposal
- The Requirements and Specs share one overlay picker per window: `Current Model` first, then every open Proposal not attached to a Drift, by id and title. Never a Draft, a Proposal being refined or an attached Proposal. When the chosen Proposal stops being open, the views go back to `Current Model` and a notice says why.
- The overlay shows the Requirements and the Specs with the Proposal applied on top of the Proposals it requires.
- Change colours (Tailwind 600 light / 400 dark): added `green`, changed `amber`, removed `red`. Every change also has a text label (`added`, `changed`, `removed`) and an accessible name.

### Focus
- Requirements and Specs each have their own focus, opened by `View requirements` (a Proposal's scope or a Task's refs) or by a chip (a single ref). Steps: sections, Areas or fields in page order.
- A chip opens the first of Requirements and Specs where its focus has at least one step. The focus of a Model element on Requirements holds the sections titled with that element or with an element containing it, up to its Feature. A chip whose focus has no step in either view is plain text, not a control.
- Focus bar, full width under the view's toolbar: what it is about (`Proposal P3 – <title>`, `Task T4 – <title>`, or the ref), the step name (section, Area or field), `<n> of <N>`, `Previous` / `Next` (icon-only; disabled at the ends), `<M> not found` (muted, tooltip lists the missing refs one per line), on Requirements an `Only these` toggle (off at start), and `Clear`. With one step there is no counter nor arrows.
- Focused sections and fields get a cyan (or change-colour) left border and tint; going to a step scrolls it to the top of the view.
- When files change, the focus is recomputed and stays on the same step if it still exists. Esc clears the focus. `Clear` keeps the overlay. Step changes are announced to assistive technologies.

### Requirements view
- Areas in folder-name order, each with its title and prose; inside, fragments in file-name order (`<fragment>.md` in muted text next to the title); inside, sections with `General` first, then in file order.
- Each section heading shows its title and `Open in editor` at the heading's line; it carries no reference chip.
- Overlay labels: `new fragment`, `fragment removed`, `added`, `removed`, `changed`, `moved from <fragment>`. A removed section is shown struck through in muted text; a changed one shows the new text with a `Show before` toggle that shows the old text.

### Specs view
- Fields in this fixed order: languages, frameworks, data, testing, CI, repository, deployment, infrastructure, notes. Empty fields are hidden.
- Max width 1100 px; entries in a grid of columns at least 300 px wide. An entry shows name, version and note; one the agent chose carries the `Sparkles` mark `agent's choice` whose tooltip is its reason.
- Overlay: entries labelled `added`, `changed`, `removed` as on Requirements.

### Errors and the editor
- An invalid Model or Requirements shows, at the top of the Requirements and Specs, a red `Alert` `The Model has <n> errors` with an expandable list: each error with its message and file, its line when known, and `Open in editor`. Unreadable files are listed the same way.
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
