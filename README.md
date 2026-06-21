# rustio-design

**The Claude-friendly design bridge for [rustio-admin](../rustio-admin).**
One declarative spec → a safe `tokens.css`, with a doctrine validator and drift
detection. Edit one file; never hand-touch the generated CSS.

> **TL;DR** — `rustio-admin`'s design layer is deliberately strict: hand-written
> multi-file CSS whose `@import` list and `include_str!` block must stay in
> lock-step, a fixed `--rio-*` token vocabulary, WCAG rules, and a hard "no build
> step" rule. Editing that by hand is how a developer gets *dizzy* — one typo or
> one broken lock-step list silently breaks the bundle. `rustio-design` removes
> the dizziness: you (and Claude) edit a single `rustio.design.toml`; the tool
> validates it against the framework's doctrine and compiles it to a `tokens.css`
> the running admin serves through its existing `RUSTIO_TOKENS_CSS` seam — no
> recompile, no source edits.

---

## Why this exists

`rustio-admin` is intentionally narrow. Its `CLAUDE.md` spells out the rules its
design layer refuses to break:

- **No build step** — hand-written CSS/JS, no Tailwind/PostCSS/Sass/bundler.
- **No second runtime** — `ConcreteOps<M>` is *the* runtime; forbidden "Tier-2"
  symbols are banned by CI.
- The CSS is a Primer/Carbon-style multi-file tree (`tokens/ → base/ → layout/ →
  components/ → pages/ → print/`) whose `@import` order **must** match the
  `concat!(include_str!(…))` order in `routes.rs`.
- Colors live in a fixed `--rio-*` token vocabulary and must pass WCAG contrast.

All of that is *correct* for the framework — and exactly why customizing the look
by hand is error-prone. A spec→code bridge cannot live *inside* `rustio-admin`
(that would be the build step the framework forbids), so it lives **here**, as a
developer-time tool that emits a plain artifact the runtime merely serves —
mirroring how the framework's own `rio-theme` engine produces `tokens.css`.

## The model

The token spec is the *bottom* of a stack. This bridge is **design memory** —
it preserves design intent, rationale, and evolution, so a future developer
understands not only what the design is, but *why it became that way*.

```
  WHY        design/DESIGN_BRIEF.md         business context · intent · visual direction
  Reasoning  design/DESIGN_REASONING.md     the ADR trail — justify BEFORE the spec
  WHAT       design/DESIGN_ARCHITECTURE.md  information architecture · navigation · hierarchy
  Memory     design/DESIGN_DECISIONS.md     the durable ledger of accepted decisions
  Memory     design/DESIGN_HISTORY.md       how (and why) the design evolved
  HOW        rustio.design.toml             the validated token spec  ← compiles today
          │
          │   rustio-design build   (validate against doctrine, then compile)
          ▼
  generated/tokens.css                       machine-owned; never hand-edited
          │
          │   RUSTIO_TOKENS_CSS=…/tokens.css cargo run
          ▼
  rustio-admin appends it after its baked CSS — later :root wins, no recompile
```

**Reason top-down (WHY → WHAT → HOW); generate bottom-up.** Today only the HOW
layer compiles to output — the higher layers are *active design memory*, surfaced
together by `rustio-design context` so Claude Design and Claude Code reason over
one coherent narrative before touching a single token. The pipeline is
**Brief → Reasoning → Architecture → Spec → Generated** (mirroring RustIO's
Plan → Review → Approve → Apply); `/design-reason` enforces it.

## Install

```sh
git clone <this repo> rustio-design && cd rustio-design
cargo build --release
# optionally: cp target/release/rustio-design ~/.cargo/bin/
```

Zero external dependencies — pure `std`. It builds offline, anywhere.

## Quickstart

```sh
rustio-design init          # scaffold rustio.design.toml
$EDITOR rustio.design.toml   # change a few tokens
rustio-design build          # validate + generate generated/tokens.css
rustio-design check          # CI gate: valid? in sync? no drift?

# serve it on your rustio-admin app without recompiling:
source generated/wire.env && cargo run
```

## Commands

| Command   | What it does |
|-----------|--------------|
| `init`    | Scaffold the spec **and** the design-memory artifacts (`design/DESIGN_*.md`). Non-destructive: never overwrites an existing spec or authored memory — safe to run in an existing project to reserve the layers. |
| `build`   | Validate the spec against the doctrine, then generate `tokens.css`, `wire.env`, a guard-rail `README.md`, and a `.manifest` of content hashes. |
| `check`   | **Read-only.** Re-validate; detect **staleness** (spec changed, not rebuilt), **drift** (a generated file was hand-edited), and **missing design memory** (a higher layer went absent). Non-zero exit on any problem — drop it into CI. |
| `schema`  | Extract a table's columns from its model struct into a view-editor schema — `--model <T>` for one, or `--all` for every registered model. Best-effort, zero-dependency (parses source, never compiles). |
| `context` | Assemble the whole stack — Brief, Reasoning, Architecture, Decisions, History, and the token spec — into one canonical stream to read **before** changing anything. |
| `wire`    | Print the `RUSTIO_TOKENS_CSS` export that serves the generated output. |
| `explain` | Print the stack, the workflow, and the iron rules. |

### Design memory & the reasoning pass

`init` scaffolds five first-class artifacts under `design/` (referenced from the
spec's `[design]` section): **DESIGN_BRIEF** (WHY), **DESIGN_REASONING** (the
ADR trail), **DESIGN_ARCHITECTURE** (WHAT), **DESIGN_DECISIONS** (the ledger),
and **DESIGN_HISTORY** (the evolution). They are Markdown + frontmatter — prose
for the narrative Claude reasons over, structure for the machine-actionable bits.

The reasoning layer is enforced, not optional: `/design-reason` makes Claude
justify a change (context, alternatives considered, *why the rejected ones lost*,
spec/architecture impact) and get approval **before** editing tokens — then
record it in `DESIGN_DECISIONS.md` and `DESIGN_HISTORY.md`. The trail exists from
day one so the *why* behind every token is auditable.

### Navigation layer (first WHAT → output)

Beyond tokens, the bridge can compile a slice of the **WHAT** layer. Declare the
admin sidebar by domain in `[navigation]`:

```toml
[navigation]
Catalogue = "Products, Categories"   # Group = "Item, Item" (ordered)
Customers = "Customers"
Sales     = "Orders, Payments"
_hidden   = "Order items, Cart items"     # reached via their parent, kept out of nav
_out      = "generated/templates/admin/_sidebar.html"
```

`build` generates a `_sidebar.html` override that buckets rustio-admin's own
`entries` into these groups — reusing the framework's URLs and labels, hiding the
buried models, and preserving the Home / Auth / Developer sections. It's served
via `RUSTIO_TEMPLATE_DIR`, the recompile-free template seam. The validator catches
duplicate entries and (best-effort, by reading the project's `src/main.rs`) warns
when a registered model is left unplaced or a nav item matches no model. The
generated file is manifest-tracked and drift-detected like any other output.

### View layer (per-table record layouts)

The second WHAT-layer slice. The same renderer draws every table, but importance
is **per table** — declare how each table's records are laid out in
`[views.<table>]`:

```toml
[views.bookings]
model     = "Booking"                               # validates field names
mode      = "list"                                  # list | cards | gallery
primary   = "booked_at"                             # leads the record
secondary = "customer + phone (inline), status (badge), assigned_to"
detail    = "address, notes"                        # detail screen only
hidden    = "id, internal_uuid"                     # never shown
```

`build` validates field names against the table's columns (typo → suggestion;
unplaced column → warning) and emits two artifacts per table: the durable
`generated/views/<table>.view.json` spec **and** a per-model
`generated/templates/admin/<table>/list.html` override the framework serves via
`RUSTIO_TEMPLATE_DIR` — the same recompile-free seam navigation uses, so
**rustio-admin is never modified**. The override reproduces the framework's list
chrome (search/filter/sort/bulk/pagination) verbatim and drives only the columns
from your spec. Author it visually with the **Adaptive View Editor**
(`view_editor.html`); seed the editor's fields with `rustio-design schema --all`.
Full walkthrough: [`docs/VIEW_LAYER.md`](docs/VIEW_LAYER.md).

All commands accept `--spec <path>` (default `rustio.design.toml`).

## The spec

```toml
[project]
name = "My Admin"
out_dir = "generated"
# rustio_admin_path = "../rustio-admin"   # unlocks the WCAG-safe brand ramp

[brand]
color = "#2563eb"
# derive = true        # delegate the ramp to rustio-admin's rio-theme engine

[colors]               # any --rio-* color token, by short name
# text-strong = "#0f172a"

[radius]               # default → --rio-radius, sm/lg → --rio-radius-{sm,lg}
default = "8px"
sm      = "5px"
lg      = "12px"

[spacing]              # s1..s7, sidebar-w, topbar-h, content-max, z-*
# content-max = "1440px"

[typography]           # font-sans, fs-*, lh-*, fw-*, …
# font-sans = "'Inter', system-ui, sans-serif"

[custom_css]           # escape hatch; validated (no @import / remote url / markup)
# rules = """
# .rio-sidebar { letter-spacing: 0.01em; }
# """
```

Every key under `[colors]`/`[spacing]`/`[radius]`/`[typography]` is resolved to a
real `--rio-*` token. The vocabulary mirrors
`rustio-admin/crates/rustio-admin/assets/static/admin/tokens/*.css`.

## The guarantees ("no dizziness")

1. **One file to edit.** The framework's lock-step CSS is never touched.
2. **Typos can't silently fail.** Unknown tokens are rejected *with a "did you
   mean"* suggestion before anything is written.
3. **Unreadable colors can't ship.** Literal text colors are held to WCAG
   contrast (< 3.0:1 → error, 3.0–4.5 → warning).
4. **Generated files are tamper-evident.** `check` fingerprints them, so a stray
   hand-edit (drift) or a forgotten rebuild (staleness) fails CI.
5. **No recompile to preview.** `source generated/wire.env && cargo run`.

For a WCAG-safe brand ramp, set `[project].rustio_admin_path` and
`[brand].derive = true`; the OKLab/WCAG color math is delegated to the
framework's own `rio-theme` engine (`rustio-admin theme generate`) — never
reimplemented here.

## Working with Claude

This repo ships first-class Claude Code support (see [`CLAUDE.md`](CLAUDE.md) and
[`.claude/commands/`](.claude/commands)). The contract is one sentence:

> **Claude edits `rustio.design.toml` and runs `rustio-design build`. Claude
> never edits anything under `generated/`.**

Slash commands: `/design-context` (load the stack), `/design-reason` (justify a
change before touching tokens), `/design-edit`, `/design-build`, `/design-check`,
`/design-explain`.

## CI

```yaml
# .github/workflows/design.yml
- run: cargo run -- check     # fails on invalid spec, drift, or staleness
```

## License

MIT © 2026 Abdulwahed Mansour.
