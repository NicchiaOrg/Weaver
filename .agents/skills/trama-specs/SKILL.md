---
name: trama-specs
description: Use when choosing, adding or using the project's languages, frameworks, libraries, data stores, tests, CI, repository, deployment, cloud resources or infrastructure as code.
---
# Project specs

The technologies this project is built, tested and released with, from the developer's Specs. An entry marked as chosen by the agent was picked by the agent itself, not named by the developer.

## Languages

- Rust stable (edition 2021): the core crate shared by the Tauri backend and the `trama` command line, the command line itself and the app backend
- TypeScript 5: the interface only; strict mode, ES modules, `noUncheckedIndexedAccess`; type-checked with `tsc --noEmit`

## Frameworks and libraries

- Tauri 2: the desktop shell for macOS, Windows and Linux; one process for every window
- Tauri plugins: single-instance, window-state, notification, dialog, opener, updater (chosen by the agent: the official plugins for the single process, window memory, system notifications, folder picking, opening files in the developer's editor and updates)
- Agent Client Protocol: the `agent-client-protocol` Rust crate, client side of ACP towards `omp acp`
- tokio (chosen by the agent: async runtime required by Tauri and the ACP client)
- clap (chosen by the agent: argument parsing of the `trama` command line, driven by one command registry)
- serde: with `serde_json` and `serde_yaml`, for front matter, Specs, settings and command line JSON (chosen by the agent: the standard serialisation of Rust)
- schemars (chosen by the agent: produces the JSON Schemas printed by `--help --json` from the same input types)
- notify (chosen by the agent: cross-platform file watching of each open project)
- pulldown-cmark (chosen by the agent: Markdown parsing of Model and Requirement files in the core)
- fd-lock (chosen by the agent: OS file locks that serialise writes of the app and of command line processes)
- Vite (chosen by the agent: the bundler Tauri recommends for a React interface)
- React 19: the interface, with `react-dom`
- shadcn/ui: components copied into the interface source, zinc base colour with a blue primary, light and dark
- Radix UI: the primitives under the shadcn/ui components
- Tailwind CSS 4
- class-variance-authority (chosen by the agent: used by the shadcn/ui components for their variants)
- clsx (chosen by the agent: used by the shadcn/ui components to join class names)
- tailwind-merge (chosen by the agent: used by the shadcn/ui components to merge Tailwind classes)
- lucide-react: the icons of the interface
- cytoscape: draws the Diagram
- elkjs: lays out the Diagram
- mermaid: draws `mermaid` blocks of Markdown, loaded on demand
- react-markdown: renders Markdown with `remark-gfm`; raw HTML disabled
- remark-gfm: tables, strikethrough, task lists and autolinks in Markdown

## Data and persistence

- Project files: Markdown and YAML under `.trama/` in the developer's repository; no database
- OS configuration folder: settings, recent projects and the registry of jobs to resume, outside every repository
- OS keychain: the only place for secrets; none are kept in the first version

## Testing

- cargo test: the core, the command line and the backend
- Vitest: the interface
- Testing Library: `@testing-library/react`, interactions as the user does them
- tauri-driver: end-to-end tests of the packaged app through WebDriver, on Linux and Windows runners of GitHub Actions
- WebdriverIO (chosen by the agent: the WebDriver client documented by Tauri for tauri-driver)

## CI

- GitHub Actions: push to `dev` and pull requests: build and test on macOS, Windows and Linux; push to `main`: the same, then a release when the tag `v<version>` does not exist yet

## Repository

- GitHub: `nicchia-code/weaver`, branches `dev` and `main`

## Deployment

- GitHub Releases: unsigned builds for now; macOS `.dmg`, Windows NSIS installer, Linux AppImage and deb, with the updater's `latest.json`
- Tauri updater: updates from GitHub Releases, signed with the updater key kept in GitHub Actions secrets

## Infrastructure as code

- omp: the only coding agent Weaver starts, as an ACP server, and the only source of model names; installed with its official installer from the system terminal when missing, updated by Weaver

## Notes

Code signing and notarisation on macOS and signing on Windows are deliberately left out for now and will come with a dedicated Proposal.

