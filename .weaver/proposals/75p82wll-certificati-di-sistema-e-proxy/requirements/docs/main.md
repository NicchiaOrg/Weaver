---
description: "Use when writing or updating README.md or CHANGELOG.md, including what they say about System Certificates."
---
# README and CHANGELOG

## General
- `CHANGELOG.md` follows Keep a Changelog 1.1.0: a short header naming the format, then `## [Unreleased]` on top and one `## [<version>] - <YYYY-MM-DD>` section per released version, newest first, each with the groups `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security` that have entries.
- Every change a user of the app can see adds at least one line under `## [Unreleased]`, in the same change as the code: what changed for the user, in one sentence, without Task or Proposal ids. Changes visible only to contributors (tests, CI, refactoring) add none.
- When a version is released, the lines under `## [Unreleased]` move to that version's section.
- `README.md` keeps its sections for installing and for contributors; user-facing behaviour that someone needs to get Weaver working goes in its own section after `## Install`.

## system-certificates
- `README.md` has a section `## Behind a corporate proxy`, after `## Install`, saying:
  - why omp fails to install or to reach its provider behind a proxy that inspects HTTPS traffic;
  - what System Certificates does, that it is off by default and where it is (Settings → omp, the omp setup box of Get Started and the welcome window);
  - which certificates it uses: those the system trusts (on macOS the user, admin and system trust settings; on Windows the user's and the machine's root stores; on Linux the system bundle), plus Mozilla's public roots; that it refreshes them at start when older than a day, every day while Weaver is open, and with `Refresh`;
  - the variables it sets, overwriting inherited values, including `PI_REQUEST_COMPRESSION=0`, and that they reach every omp process Weaver starts and its terminals, so the agent's commands inherit them;
  - the one-time turn-on when Zscaler is installed, on macOS and Windows only, never repeated after it is turned off;
  - the explanations shown for certificate errors, HTTP errors and a failed installer, and their suggestion while the setting is off;
  - that Java's tools and every program that reads a truststore instead of a PEM bundle are not covered.
- `README.md` `### Where Weaver keeps files` names the bundle `system-certificates.pem` and the installer markers `omp-install/` in the local data folder.
- `CHANGELOG.md` is created with this change; its first `## [Unreleased]` entries describe System Certificates, the Zscaler turn-on, the new error explanations and the installer that now reports its failure.
