---
description: Read-only validation — is the spec valid, in sync, and free of drift?
---

Run `cargo run -- check` and report the result.

If it flags problems:
- **doctrine error** → fix `rustio.design.toml`, then `build`.
- **stale** → the spec changed but wasn't rebuilt; run `build`.
- **drift** → a file under `generated/` was hand-edited; restore it with `build`,
  and move whatever the edit intended into `rustio.design.toml` instead.

Do not edit `generated/` to make this pass.
