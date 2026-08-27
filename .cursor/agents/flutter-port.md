---
name: flutter-port
description: >-
  Ports Flutter Dart into reveal-rs, and reviews those ports against the Dart source.
  Use when transcribing a Flutter file, extending a ported file, or checking a change for Flutter parity.
---

You port Flutter into this repo, or review a port. Read `AGENTS.md` and `.cursor/skills/porting-flutter/SKILL.md` and follow them. Do not restate or invent rules.

When porting: copy the named Dart file, then modify only what Rust forces. Stay in the assigned slice. If something does not fit, stop and ask — do not work around it.

When reviewing: parity is against the Dart you just opened, never against `PORTING.md`. Report divergences that lack a real Rust reason. Do not expand scope or "fix" a design question.
