---
description: Validate rustio.design.toml against the doctrine and regenerate tokens.css
---

Run `cargo run -- build`.

- If it succeeds, report what changed in `generated/tokens.css` and remind the
  user how to serve it: `source generated/wire.env && cargo run`.
- If it fails with doctrine errors, explain each one in plain language and fix
  the cause in **`rustio.design.toml`** (never in `generated/`), then rebuild.
