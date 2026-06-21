---
artifact: DESIGN_REASONING
layer: reasoning      # Brief → [Reasoning] → Architecture → Spec → Generated
status: reserved
updated: ""           # YYYY-MM-DD
---

# Design Reasoning — the pass before the spec

> **Brief → Reasoning → Architecture → Spec → Generated.**
> No change reaches the token layer (`rustio.design.toml`) until it is justified
> here first. This file is the design equivalent of an **ADR** (Architecture
> Decision Record): it records *why*, *what else was considered*, and *why the
> alternatives were rejected* — so a future developer understands not only what
> the design is, but how it became that way.
>
> Workflow — mirrors RustIO's **Plan → Review → Approve → Apply**:
>
> 1. **Plan** — append a reasoning entry below (status: `proposed`).
> 2. **Review** — a human reads the rationale and the rejected options.
> 3. **Approve** — set status `accepted`; record it in `DESIGN_DECISIONS.md`.
> 4. **Apply** — only now edit `rustio.design.toml`, then `rustio-design build`,
>    and add a line to `DESIGN_HISTORY.md`.
>
> Drive this with `/design-reason`. Newest entry on top.

<!-- ───────────────────────────────────────────────────────────────────
     TEMPLATE — copy for each new reasoning pass.

## R-NNN · <short title>

- **Date:** YYYY-MM-DD
- **Status:** proposed | accepted | superseded by R-MMM
- **Serves:** <the Brief intent / Architecture goal this advances>
- **Context:** <what prompted this — the request or the problem>
- **Options considered:**
  1. <option A> — <trade-offs>
  2. <option B> — <trade-offs>
- **Decision:** <the chosen option>
- **Rationale:** <why this serves the Brief better than the others>
- **Rejected because:** <why each alternative lost>
- **Spec impact:** <which `--rio-*` tokens / `[sections]` will change>
- **Architecture impact:** <IA / navigation / hierarchy effects, if any>
─────────────────────────────────────────────────────────────────── -->

_No reasoning entries yet. Run `/design-reason` to record the first._

## R-001 · The view layer — per-table record layouts compiled to a frozen spec

- **Date:** 2026-06-21
- **Status:** proposed
- **Serves:** the WHAT layer's "adaptive view" goal — let a developer decide how
  each table's records are laid out (which fields lead, which fold to the detail
  screen, which hide, which compose), authored once at build time and executed by
  a fast, dumb renderer at runtime.
- **Context:** `[navigation]` compiled the first WHAT-layer decision (sidebar
  grouping) into a recompile-free artifact. The natural next slice is the
  *record view*: rustio-admin renders every table the same way, but importance is
  per-table (a salon leads with time; container ops leads with status + site).
  We built an authoring UI for this (the Adaptive View Editor). It now needs a
  home in the spec + a generated artifact, exactly as navigation has.
- **Options considered:**
  1. **`[views.<table>]` in the spec → generated `*.view.json` per table**, read
     by a runtime renderer. Mirrors `[navigation]` (flat role lists, `_out`),
     reuses the manifest/drift machinery, framework-agnostic artifact.
     Trade-off: needs a small renderer on the rustio-admin side to consume JSON.
  2. **Generate a Jinja template override per table** (like `_sidebar.html`),
     consumed via `RUSTIO_TEMPLATE_DIR`. Trade-off: requires reproducing
     rustio-admin's record-list template contract (search/filter/sort/bulk/
     pagination) verbatim — large and fragile, and couples the bridge to template
     internals.
  3. **Bake layout into `tokens.css`** (e.g. ordering via CSS). Trade-off:
     CSS can't express field selection/compose/mode switching; wrong layer.
- **Decision:** Option 1 — `[views.<table>]` → a deterministic `*.view.json`
  artifact, with schema-aware validation. The editor publishes the `[views.*]`
  block; `build` validates field names against the table's columns (best-effort,
  like nav coverage) and freezes the layout to JSON; the renderer reads it.
- **Rationale:** It is the smallest, most honest step that keeps the bridge's
  contract intact: one file to edit, validated keys, tamper-evident output, no
  recompile, and **no guessing at rustio-admin's internals**. JSON is the
  "frozen file the renderer reads at runtime" the design discussion called for,
  and decouples authoring from the framework's template tree.
- **Rejected because:** (2) hard-couples to the framework's record-list template
  and re-introduces exactly the multi-file fragility the bridge exists to avoid;
  (3) is the wrong layer — CSS cannot select or compose fields.
- **Spec impact:** new `[views.<table>]` sections (`mode`, `primary`,
  `secondary`, `detail`, `hidden`, optional `model`/`source`/`_out`). No token
  changes. A commented example is added to the starter spec.
- **Architecture impact:** introduces the record-view slice of the WHAT layer
  (to be documented in DESIGN_ARCHITECTURE.md on accept). Adds a `schema`
  command that extracts a table's columns from its model struct (best-effort,
  mirroring the `[navigation]` model-discovery heuristic) to feed both the editor
  and view validation. Runtime consumption of `*.view.json` is a follow-up that
  lands in the rustio-admin repo (a small renderer or template partial).
