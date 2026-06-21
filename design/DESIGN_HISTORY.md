---
artifact: DESIGN_HISTORY
layer: memory         # the human-readable evolution of the design
status: active
updated: "2026-06-21"
---

# Design History — the evolution

> The narrative of how the design changed over time, and *why* — the story a new
> team member reads to understand the current state. Reverse-chronological.
> Token-level churn lives in git; this is the human-readable arc that ties
> changes back to the decisions (`D-NNN`) that drove them.

## 2026-06-21 — The view layer

The bridge gained its second WHAT-layer slice (after navigation): per-table
**record layouts**. A developer now declares how each table's records read —
which field leads, which support, which fold to the detail screen, which hide,
and which compose — in `[views.<table>]`, authored visually in the Adaptive View
Editor (**D-001**). `build` freezes each table to a `*.view.json` spec and
validates field names against the model's real columns.

For an operator, this means a list that leads with what matters for *their*
domain — a salon's appointment time, a shop's product name + price — instead of
every column in declaration order.

Consumption follows the navigation precedent rather than a framework change: a
generated per-model `list.html` override served via `RUSTIO_TEMPLATE_DIR`
reshapes only the columns and keeps all of rustio-admin's chrome (**D-002**) —
chosen over a runtime hook because the framework forbids schema-driven runtime
metadata. Verified live against the seeded `shop` admin.

<!-- ## YYYY-MM-DD — <milestone title>
     What changed at the design level, which decisions drove it (D-NNN),
     and what it means for the operators who live in this admin. -->
