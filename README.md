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

```
  rustio.design.toml          ← the ONE file you and Claude edit
          │
          │   rustio-design build   (validate against doctrine, then compile)
          ▼
  generated/tokens.css        ← generated; never hand-edited (tamper-evident)
          │
          │   RUSTIO_TOKENS_CSS=…/tokens.css cargo run
          ▼
  rustio-admin appends it after its baked CSS — later :root wins, no recompile
```

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
| `init`    | Scaffold a starter `rustio.design.toml` (won't overwrite an existing one). |
| `build`   | Validate the spec against the doctrine, then generate `tokens.css`, `wire.env`, a guard-rail `README.md`, and a `.manifest` of content hashes. |
| `check`   | **Read-only.** Re-validate, and detect **staleness** (spec changed, not rebuilt) and **drift** (a generated file was hand-edited). Non-zero exit on any problem — drop it into CI. |
| `wire`    | Print the `RUSTIO_TOKENS_CSS` export that serves the generated output. |
| `explain` | Print the workflow and the iron rules. |

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

Slash commands: `/design-edit`, `/design-build`, `/design-check`, `/design-explain`.

## CI

```yaml
# .github/workflows/design.yml
- run: cargo run -- check     # fails on invalid spec, drift, or staleness
```

## License

MIT © 2026 Abdulwahed Mansour.
