---
description: Apply a plain-English design change to rustio.design.toml, then build & check
---

The user wants to change the rustio-admin look: **$ARGUMENTS**

Do this, and only this:

1. Read `rustio.design.toml` (run `rustio-design init` first if it does not exist).
2. Translate the request into edits to **`rustio.design.toml` only**. Map intent
   to the right section/key (`[brand]`, `[colors]`, `[spacing]`, `[radius]`,
   `[typography]`, or — last resort — `[custom_css]`). Use the canonical token
   short-names; if unsure of a name, check `src/allowlist.rs`.
3. Run `cargo run -- build`. If it reports a doctrine error (unknown token,
   failed contrast, forbidden CSS), fix the **spec** and rebuild — do not work
   around the validator.
4. Run `cargo run -- check` to confirm the spec is valid and in sync.
5. Show the user the diff of `rustio.design.toml` and the resulting
   `generated/tokens.css`, and remind them they can preview with
   `source generated/wire.env && cargo run` in their rustio-admin app.

**Never** edit anything under `generated/`, and never touch the `rustio-admin`
repo's CSS.
