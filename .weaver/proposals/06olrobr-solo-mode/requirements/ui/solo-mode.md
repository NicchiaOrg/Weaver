---
description: Use when building the Solo mode control in Settings and Get Started, its notes, warnings and the list of tracked files.
---
# Solo mode

## solo-mode

- **Control:** title `Solo mode`, a segmented control `Off` / `Whole folders` / `Weaver's files only` (the same component as `Theme`, a radio group for assistive technology), with the note `Applies only to this working copy: other clones start with it off.` under it. It is not in the settings schema and is never stored in `settings.json`: it shows the project's state and sends changes to the backend.
- **Places:** first in the `Project` group of Settings, before `Update skills`; in Get Started between the list of agent files and `Initialise`.
- **Settings, saving:** at once on choice, no confirmation, also when turning it off. While a change runs the control is disabled. A failed change shows `Could not change Solo mode: <reason>.` under it in the error colour and puts back the project's value.
- **Settings, not initialised:** disabled on `Off`, with `Choose Solo mode when you initialise the project.` under the note.
- **No git:** disabled on `Off` in Settings and in Get Started, with `Solo mode needs git: git was not found.` or `Solo mode needs git: this folder is not in a git repository.` (the other reasons of `requirements/rules#solo-mode` as `Solo mode needs git: <reason>.`).
- **On:** under the note, in the warning colour with `TriangleAlert`: `Weaver's files are not versioned anywhere: losing this working copy loses them.`
- **Not excluded:** in the warning colour with `TriangleAlert`, in Settings only: `Solo mode is on but cannot exclude Weaver's files: <reason>.` when git is missing or the folder is no longer in a repository, otherwise `Weaver's files may not be excluded from git: <reason>.` The control stays usable.
- **Tracked files:** in Settings, while Solo mode is on and the list is not empty, an `Alert` (warning variant) with `Git still tracks <N> of Weaver's files: their changes still go into commits.` and below it the relative paths, monospace, scrolling past 8 lines. No action to stop tracking them. The list is asked for each time the Settings view is shown and replaced by the result of every change; while it loads, the earlier list stays.
- **Get Started:** `Off` is chosen at first and the choice lives only in the screen. With `Whole folders` or `Weaver's files only` chosen, the text becomes `Weaver will create a .weaver folder that describes this project. With Solo mode its files stay only in this working copy and never go into a commit.` and the warning of **On** shows under the control. `Initialise` and `Overwrite and initialise` both send the choice. A Solo mode failure shows as `Could not initialise: <reason>.`
- **Accessibility:** the note, the warnings and the reason a control is disabled are its description for assistive technology (`aria-describedby`); the list of tracked files is a labelled region; a change of the warnings is announced (`aria-live="polite"`).
