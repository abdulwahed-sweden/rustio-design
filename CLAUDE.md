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
cargo run -- init       # scaffold rustio.design.toml (once)
# edit rustio.design.toml
cargo run -- build      # validate against doctrine + generate
cargo run -- check      # read-only: valid? in sync? no drift? (CI gate)
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

## How generation maps to the framework

There is exactly one runtime seam this tool targets: **`RUSTIO_TOKENS_CSS`**.
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
