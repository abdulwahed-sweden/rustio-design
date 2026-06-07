//! The build manifest — content fingerprints that make drift and staleness
//! observable.
//!
//! After every `build`, the manifest records the SHA-256 of the spec and of each
//! generated file. `check` re-reads it to answer two questions deterministically:
//!
//! * **stale?** — the spec's current hash differs from the recorded one, so the
//!   generated output no longer reflects the source of truth (`build` is owed).
//! * **drifted?** — a generated file's current hash differs from the recorded
//!   one, i.e. someone hand-edited an output file (exactly the "dizziness" we
//!   exist to prevent).
//!
//! Format is `sha256sum`-style lines so it is greppable and diff-friendly:
//!
//! ```text
//! # rustio-design manifest — generated, do not edit
//! <64-hex>  rustio.design.toml
//! <64-hex>  tokens.css
//! ```

/// A parsed manifest: ordered `(relative_path, sha256_hex)` entries.
pub struct Manifest {
    /// Entries in write order.
    pub entries: Vec<(String, String)>,
}

impl Manifest {
    /// Build an empty manifest.
    pub fn new() -> Self {
        Manifest {
            entries: Vec::new(),
        }
    }

    /// Record (or replace) the hash for a path.
    pub fn set(&mut self, path: &str, hash: &str) {
        if let Some(e) = self.entries.iter_mut().find(|(p, _)| p == path) {
            e.1 = hash.to_string();
        } else {
            self.entries.push((path.to_string(), hash.to_string()));
        }
    }

    /// Look up the recorded hash for a path.
    pub fn get(&self, path: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, h)| h.as_str())
    }

    /// Serialize to the on-disk text format.
    pub fn to_text(&self) -> String {
        let mut s = String::from("# rustio-design manifest — generated, do not edit\n");
        for (path, hash) in &self.entries {
            s.push_str(&format!("{hash}  {path}\n"));
        }
        s
    }

    /// Parse the on-disk text format. Unknown / malformed lines are skipped.
    pub fn parse(text: &str) -> Self {
        let mut entries = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((hash, path)) = line.split_once("  ") {
                entries.push((path.trim().to_string(), hash.trim().to_string()));
            }
        }
        Manifest { entries }
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self::new()
    }
}
