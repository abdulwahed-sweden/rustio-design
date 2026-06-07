# CLAUDE.md

Guidance for Claude Code (and other AI assistants) working in this repository.
Read this before making changes.

## What this repo is

`rustio-design` is the **design bridge** between a developer's brand/visual intent
and [`rustio-admin`](../rustio-admin), a deliberately strict, security-first Rust
admin framework. It compiles one declarative spec (`rustio.design.toml`) into a
`generated/tokens.css` that the running admin serves via its `RUSTIO_TOKENS_CSS`
seam — **without** recompiling the framework or hand-editing its lock-step CSS.

It is a pure-`std` Rust CLI with **zero external dependencies** (intentionally —
mirroring rustio-admin's "minimal deps, hand-rolled, no magic" ethos).

## The design stack (this bridge is design *memory*, not a token compiler)

The token spec is the bottom of a stack. Above it sit the layers a designer — or
Claude Design — actually reasons about first. The whole point is to **preserve
design intent, rationale, and evolution over time**, so a future developer
understands not only what the design is, but *why it became that way*.

```
WHY        design/DESIGN_BRIEF.md         business context · design intent · visual direction
Reasoning  design/DESIGN_REASONING.md     the ADR trail — justify BEFORE the spec
WHAT       design/DESIGN_ARCHITECTURE.md  information architecture · navigation · UX hierarchy
Memory     design/DESIGN_DECISIONS.md     the durable ledger of accepted decisions
Memory     design/DESIGN_HISTORY.md       how (and why) the design evolved
HOW        rustio.design.toml             the validated token spec
Output     generated/                     machine-owned; never hand-edited
```

`rustio-design context` assembles all of this into one canonical stream. The
`[design]` section of the spec is the manifest pointing at these files.

**Reason TOP-DOWN (WHY → WHAT → HOW); generate BOTTOM-UP.** Today only the HOW
layer compiles to output — the higher layers are *active design memory*, not yet
generated from. Humans, Claude Design, and Claude Code share them as one
narrative source of truth.

## The reasoning contract (Plan → Review → Approve → Apply)

The pipeline is **Brief → Reasoning → Architecture → Spec → Generated** — never
Brief → Spec. Before you change a single token:

1. **Plan** — read `rustio-design context`; append an ADR-style entry to the top
   of `DESIGN_REASONING.md` (context, ≥2 options with trade-offs, decision,
   rationale, *why alternatives were rejected*, spec/architecture impact;
   status `proposed`).
2. **Review** — show the entry + the proposed token diff; wait for approval.
3. **Approve** — set status `accepted`; add a row to `DESIGN_DECISIONS.md` and a
   line to `DESIGN_HISTORY.md`.
4. **Apply** — only now edit `rustio.design.toml`, then `build` + `check`.

`/design-reason` drives this; `/design-context` loads the stack. The reasoning
trail must exist from day one — even a trivial change gets a one-line entry.

## The single most important rule

> **Edit `rustio.design.toml`. Then run `rustio-design build`.**
> **Never edit anything under `generated/`.**

`generated/` is machine output. Hand edits there are (a) reverted on the next
build and (b) flagged as *drift* by `rustio-design check`, which fails CI. If a
user asks you to "change the admin's colors / spacing / radius / fonts," the
answer is always: edit the spec, then build. Do not reach into `rustio-admin`'s
CSS, and do not edit generated files.

## Workflow

```sh
cargo run -- init       # scaffold spec + design-memory artifacts (once)
cargo run -- context    # assemble WHY→WHAT→HOW into one stream (read before changing)
# reason in design/DESIGN_REASONING.md, then edit rustio.design.toml
cargo run -- build      # validate against doctrine + generate
cargo run -- check      # read-only: valid? in sync? no drift? memory present? (CI gate)
cargo run -- wire       # print the RUSTIO_TOKENS_CSS export
```

After editing the spec, **always run `build` then `check`** before finishing.

## The spec format

Sections and what they control:

- `[project]` — `name`, `out_dir` (default `generated`), and optional
  `rustio_admin_path` (a path to a rustio-admin checkout; unlocks the WCAG-safe
  brand ramp).
- `[brand]` — `color` (`#rrggbb`), and `derive = true` to delegate the full,
  contrast-safe color ramp to `rustio-admin theme generate` (requires
  `rustio_admin_path`).
- `[colors]` / `[spacing]` / `[radius]` / `[typography]` — token overrides by
  **short key**. The key is resolved to a real `--rio-*` custom property:
  - `[colors] text-strong` → `--rio-text-strong`
  - `[radius] default` → `--rio-radius`; `[radius] sm` → `--rio-radius-sm`
  - `[spacing] content-max` → `--rio-content-max`
  - `[typography] font-sans` → `--rio-font-sans`
- `[custom_css] rules = """ … """` — raw CSS escape hatch, validated (no
  `@import`, no remote `url(http…)`, no markup, no forbidden symbols). Prefer
  tokens; reach for this only when no token exists.

The canonical token vocabulary lives in `src/allowlist.rs` and mirrors
`rustio-admin/crates/rustio-admin/assets/static/admin/tokens/*.css`. An unknown
key is **rejected with a suggestion** — so if `build` says "did you mean
`--rio-accent`?", fix the key in the spec; don't try to force it.

## Navigation layer (`[navigation]`)

The first WHAT-layer slice that compiles to output. `Group = "Item, Item"`
(ordered, matched against rustio-admin's `entry.display_name`); reserved
`_hidden` (models reached via their parent, kept out of nav) and `_out` (the
generated `_sidebar.html` path, relative to the spec root). `build` emits a
`_sidebar.html` override consumed via `RUSTIO_TEMPLATE_DIR`. It is a WHAT-layer
decision: reason in `DESIGN_ARCHITECTURE.md` / `DESIGN_REASONING.md` (`/design-reason`)
before changing it. Coverage is validated best-effort against the project's
`src/main.rs` models. Tracking issue: rustio-design#1.

## How generation maps to the framework

This tool targets two recompile-free seams: **`RUSTIO_TOKENS_CSS`** (tokens) and
**`RUSTIO_TEMPLATE_DIR`** (the generated `_sidebar.html`). For tokens:
rustio-admin appends the file after its baked CSS bundle, and later `:root`
declarations win — so every token override here takes effect with no recompile
and no source change. This is the same mechanism the framework's own
`rio-theme`/`theme generate` is built around. We do **not** generate template
overrides or touch the multi-file CSS tree; that strictness stays the
framework's, and is precisely what we're protecting the developer from.

## When you change the Rust code

- Keep it **zero-dependency** (`std` only). Do not add crates.
- Match the existing module layout: `toml_lite` (parser) → `spec` (typed view) →
  `validate` (doctrine) → `tokens` (emit) → `manifest` (drift) → `main` (CLI).
- Every public item has a doc comment (as in rustio-admin).
- If you add a new `--rio-*` token to the allowlist, note in the PR that it
  mirrors a real framework token.
- Run `cargo test` and `cargo build` before finishing.

## What NOT to do

- Do not edit files under `generated/`.
- Do not edit, or write into, the sibling `rustio-admin` repo from here. The
  only interaction is read-only delegation to `rustio-admin theme generate` for
  the brand ramp.
- Do not add a build step, bundler, or external dependency.
- Do not invent `--rio-*` tokens that the framework does not define.
