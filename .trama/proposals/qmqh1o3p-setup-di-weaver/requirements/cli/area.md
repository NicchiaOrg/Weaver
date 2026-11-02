---
description: "Use when building or changing the `trama` command line: commands, inputs, outputs, help and exit codes."
---
# Command line

`trama` is the binary built from the CLI crate on top of the same Rust core as the app. It ships inside the app bundle and is put first on the PATH of every omp Weaver starts; it is how the agent reaches Weaver. It also works without the app for every command not bound to a job.
