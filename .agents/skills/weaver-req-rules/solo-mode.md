# Solo mode

## solo-mode

- **Modes:** `off`, `folders` (default when turning on without a choice of mode), `files`. The same three values in the app, the command line and `solo.json` (`requirements/data#solo-mode`).
- **Exclusions file:** the path git prints for `git rev-parse --git-path info/exclude`, run in the project root and taken relative to it when relative. In a linked worktree it is the main repository's file, as git answers. A missing file, or missing `info/` folder, is created.
- **Prefix:** `git rev-parse --show-prefix` in the project root: empty at the repository's root, otherwise the project folder relative to it, ending with `/`. Characters special to git patterns in it (`*`, `?`, `[`, `\`, a leading `#` or `!`, trailing spaces) are escaped with `\`.
- **Block** of one working copy, `<root>` being the canonical project root with `/` separators and `<p>` the prefix:

  ```
  # >>> weaver solo mode: <root>
  <lines>
  # <<< weaver solo mode: <root>
  ```

  - `folders`: `/<p>.weaver/`, `/<p>.agents/skills/`, `/<p>.omp/`, in this order.
  - `files`: `/<p>.weaver/`, `/<p>.agents/skills/weaver-*/`, `/<p>.omp/agents/weaver-*.md`, `/<p>.omp/WATCHDOG.yml`, in this order.
  - Every line ends with LF.
- **Writing:** the block of this working copy is every run from its exact begin line to the next end line with the same `<root>`. The first such run is replaced in place and any further one removed; without one, the block is appended at the end of the file, after an LF when the file does not end with one. A begin line of this working copy with no end line after it is removed as a single line. Every other byte of the file, blocks of other working copies included, stays as it was. When the result equals the file, nothing is written; otherwise it is written atomically.
- **Removing:** every run of this working copy, begin and end lines included, and nothing else; a file left empty stays, empty.
- **Expected block:** the one the remembered mode and the current prefix give, byte for byte. At the opening, a missing or different block is written again (`solo-mode/Excluded`).
- **Changes:** turning on writes the block, then `solo.json`; turning off removes the block, then the `solo/` folder; changing mode rewrites the block, then `solo.json`. When the block step fails, nothing else is done and the earlier state stays.
- **Tracked files:** the files git tracks that the block's lines match, as relative paths from the project root, with `/`, sorted by path; only these, never other ignored tracked files.
- **Not available:** Solo mode cannot be changed, and an initialisation with a mode is refused, when one of these reasons holds, shown as the `<reason>` of the texts: `git was not found`, `this folder is not in a git repository`, `git did not answer within 10 s`, `git failed: <first line of its error output>`. A failed write of the exclusions file gives `could not write <path>: <system error>`.
