---
artifact: DESIGN_DECISIONS
layer: memory         # the durable ledger of accepted decisions
status: active
updated: "2026-06-21"
---

# Design Decisions — the ledger

> The durable record of **accepted** design decisions — the approved outcomes of
> `DESIGN_REASONING.md`. One row per decision. **Never delete a row**: supersede
> it, so the trail of *why the design became what it is* stays intact for the
> next developer.

| ID    | Date       | Decision                                                                 | Status   | Reasoning |
|-------|------------|--------------------------------------------------------------------------|----------|-----------|
| D-001 | 2026-06-21 | View layer: per-table record layouts via `[views.<table>]` → frozen `*.view.json` | accepted | R-001     |
| D-002 | 2026-06-21 | View consumption via a generated per-model `list.html` override (no runtime hook) | accepted | R-002     |
