---
description: Reason about a design change BEFORE touching tokens — record an ADR-style justification, get approval, then apply
---

The user wants a design change: **$ARGUMENTS**

This is the **reasoning pass**. In this bridge the reasoning layer matters more
than the token layer: no change reaches `rustio.design.toml` until it is
justified first. Follow RustIO's **Plan → Review → Approve → Apply**, and never
collapse it to "just edit the tokens."

## 1. Plan (read, then reason)

1. Run `rustio-design context` and read the whole stack — Brief (WHY),
   Architecture (WHAT), the existing reasoning/decisions/history, and the current
   token spec (HOW). Reason **top-down**: does this request serve the Brief? Does
   it fit the Architecture?
2. Append a new entry to the **top** of `design/DESIGN_REASONING.md` using the
   template in that file. Fill in every field honestly:
   - **Context** — what prompted this.
   - **Options considered** — at least two real alternatives, with trade-offs.
   - **Decision** + **Rationale** — why this serves the Brief better than the others.
   - **Rejected because** — why each alternative lost (this is the point: a future
     developer must understand why the design became what it is).
   - **Spec impact** — the exact `--rio-*` tokens / `[sections]` you will change.
   - **Architecture impact** — IA / navigation / hierarchy effects, if any.
   Set **Status: proposed**. Give it the next `R-NNN` id.

## 2. Review

Show the user the reasoning entry and the precise token diff you propose. Do
**not** edit `rustio.design.toml` yet. Wait for explicit approval.

## 3. Approve

On approval: set the entry's **Status: accepted**, add a row to
`design/DESIGN_DECISIONS.md` (`D-NNN`, date, decision, `accepted`, → `R-NNN`),
and add a line under **Unreleased** in `design/DESIGN_HISTORY.md`.

## 4. Apply

Only now edit `rustio.design.toml`, then run `rustio-design build` and
`rustio-design check`. If the validator rejects the change (unknown token, failed
contrast, forbidden CSS), fix the **spec** — and note in the reasoning entry if
the constraint changed the decision.

**Hard rule:** never edit anything under `generated/`, and never change tokens
without a corresponding accepted reasoning entry. If the change is trivial and
the user insists on skipping the trail, record a one-line entry anyway — the
reasoning trail must exist from day one.
