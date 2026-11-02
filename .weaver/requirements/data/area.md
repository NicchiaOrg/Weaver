---
description: "Use when reading or writing project files under `.weaver/`, Weaver's settings or the local state it keeps outside the project."
---
# Data

Everything Weaver keeps lives in files: the project's Model, Requirements, Specs and cards under `.weaver/` in the developer's repository, and the developer's settings and Weaver's own local state in the operating system's per-user folders. There is no database. The Rust core crate is the only code that reads or writes these files; the `weaver` CLI and the Tauri backend both go through it, and the UI never touches a file.
