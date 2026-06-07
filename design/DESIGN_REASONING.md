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
