---
description: "Use when building or changing the `weaver` command line: commands, inputs, outputs, help and exit codes."
---
# Command line

`weaver` is the command built from the CLI crate, as the binary `weaver-cli`, on top of the same Rust core as the app. It ships inside the app bundle as `weaver-cli`; Weaver copies it as `weaver` into the folder it puts first on the PATH of every omp it starts, and that is how the agent reaches Weaver. It also works without the app for every command not bound to a job.
