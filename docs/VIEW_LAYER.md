# The view layer — how each table's records are laid out

A 5-minute guide to going from a rustio-admin table to a frozen, runtime-read
record layout. Same renderer for every table; importance is **per table** (a
salon leads with the appointment time; container ops leads with status + site).

You author the layout once, at build time, in the **Adaptive View Editor**.
`rustio-design` validates it and freezes it to a JSON file your renderer reads.
Nothing recompiles; the framework's source is never touched.

```
  models (Rust)  ──schema──▶  Adaptive View Editor  ──publish──▶  [views.<table>]
                                                                       │
                                                              rustio-design build
                                                                       ▼
                                                     generated/views/<table>.view.json
                                                                       │
                                                              read at runtime by
                                                              the record renderer
```

---

## 1. Extract your tables' fields (one command)

The editor needs to know each table's columns. Generate them from your models:

```sh
# every registered .model::<T>() at once
rustio-design schema --all --source src/main.rs --out-dir ./schemas

# …or a single model whose struct lives elsewhere
rustio-design schema --model Booking --source src/models.rs --out ./schemas/bookings.json
```

Each file looks like this (types are inferred from the Rust field types; tweak
any in the editor):

```json
{
  "project": "Salon Admin",
  "table": "bookings",
  "columns": [
    { "name": "booked_at", "type": "timestamp" },
    { "name": "customer",  "type": "text" },
    { "name": "status",    "type": "enum" }
  ],
  "sample": []
}
```

> Best-effort, zero-dependency: it parses your source, it does not compile it.
> A model defined in another file is reported — point `--source` at it.

## 2. Lay it out in the editor

Open `view_editor.html` (just double-click — no build, no server). Drop your
schema file(s) in, then:

- set each field's **role** — `primary` (leads) · `secondary` (supports) ·
  `detail` (detail screen only) · `hidden`,
- **compose** related fields (`customer + phone`) and pick a style
  (`stacked` / `inline` / `badge`),
- choose the default **view mode** (list / cards / gallery),
- click **Publish** — it gives you the `[views.<table>]` block.

## 3. Put the layout in your spec

Paste the published block into `rustio.design.toml`. Add `model` (and `source`
if the struct isn't in `src/main.rs`) so field names are validated against the
real columns:

```toml
[views.bookings]
model     = "Booking"
mode      = "list"
primary   = "booked_at"
secondary = "customer + phone (inline), status (badge), assigned_to"
detail    = "address, notes"
hidden    = "id, internal_uuid"
```

> A WHAT-layer decision: reason in `DESIGN_ARCHITECTURE.md` /
> `DESIGN_REASONING.md` first (`/design-reason`), exactly like `[navigation]`.

## 4. Build & verify

```sh
rustio-design build     # validates field names, freezes generated/views/bookings.view.json
rustio-design check     # CI gate: spec valid · output in sync · no drift
```

`build` refuses on structural errors (bad mode, a field placed twice) and warns
on typos with a suggestion (`customer` ← `custmer`) and on any column you left
unplaced. The generated JSON is fingerprinted in the manifest, so a hand-edit is
caught as drift.

The frozen artifact:

```json
{
  "table": "bookings",
  "default_mode": "list",
  "modes": ["list", "cards", "gallery"],
  "cells": [
    { "members": ["booked_at"], "role": "primary", "style": "stacked" },
    { "members": ["customer", "phone"], "role": "secondary", "style": "inline" },
    { "members": ["status"], "role": "secondary", "style": "badge" }
  ]
}
```

## 5. Serve it (no framework change)

`build` also emits a per-model template override:

```
generated/templates/admin/<table>/list.html
```

Point the running admin at it with the same seam navigation uses — no recompile,
no edit to rustio-admin:

```sh
export RUSTIO_TEMPLATE_DIR="$PWD/generated/templates"
cargo run            # the reshaped list is now live
```

Or, if your app already serves a hand-written `templates/` dir (and points
`RUSTIO_TEMPLATE_DIR` at it — like `examples/shop` does for its `_sidebar.html`),
set `_out` so `build` writes the override straight there, exactly like
`[navigation]._out`:

```toml
[views.products]
model     = "Product"
mode      = "list"
primary   = "name"
secondary = "price + in_stock (badge)"
_out      = "templates/admin/products/list.html"   # served dir
```

The override reproduces the framework's list chrome (search, filters, sort, bulk
actions, pagination) **verbatim** and drives only the columns from your view-spec
— so nothing else changes. It mirrors a specific framework `list.html`; if you
upgrade rustio-admin and its list chrome changes, re-run `build` (and bump
`LIST_TEMPLATE_BASED_ON` in `src/list_tpl.rs`) to re-sync. The durable
`*.view.json` is emitted alongside as the source of truth and for any future
renderer (cards/gallery).

> Why a template, not a runtime reader: rustio-admin forbids schema-driven
> runtime metadata (a "second runtime"). Reshaping at build time via the template
> seam honours that — see `DESIGN_REASONING.md` R-002.

---

### Field-type → display reference

| Rust type (inferred)                | view type   | shown as            |
| ----------------------------------- | ----------- | ------------------- |
| `String`, `&str`                    | `text`      | plain text          |
| `i32`/`u64`/`f64`/`Decimal`         | `number`    | right-aligned value |
| `bool`                              | `boolean`   | badge               |
| `DateTime`/`NaiveDate`/`Timestamp`  | `timestamp` | time/date           |
| `Uuid`                              | `uuid`      | (usually hidden)    |
| enum-like / unknown                 | `text`      | refine in editor    |

`Option<T>` and `Vec<T>` are unwrapped to `T`. Anything the extractor can't
place becomes `text` — adjust it in the editor or the schema file.

### Command reference

| Command | What it does |
| --- | --- |
| `rustio-design schema --all --out-dir <dir>` | extract every model's columns |
| `rustio-design schema --model <T> [--source <f>] [--out <f>]` | extract one model |
| `rustio-design build` | validate + freeze `*.view.json` (+ tokens, sidebar) |
| `rustio-design check` | read-only CI gate: valid · in sync · no drift |
