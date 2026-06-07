---
description: Load the whole design stack (WHY→WHAT→HOW) as one canonical context before reasoning
---

Run `rustio-design context` and read the entire output — Brief (WHY), the
reasoning trail, Architecture (WHAT), the decisions ledger, the history, and the
current token spec (HOW).

This is the canonical, single source of design truth for this admin. Use it to
ground any design question or change **before** touching the token layer. If a
layer prints as `(reserved)`, it has not been authored yet — say so rather than
inventing intent; offer to capture it (e.g. via `/design-reason` or by editing
`design/DESIGN_BRIEF.md`).

Do not edit anything under `generated/`.
