//! A typed, convenient view over the parsed `rustio.design.toml` document.
//!
//! Every accessor here corresponds to something the generator actually emits, so
//! there are no dead settings: if a key exists in the spec, it has an effect.

use crate::toml_lite::Document;

/// The design spec — the single source of truth for a rustio-admin look.
pub struct Spec {
    doc: Document,
}

/// One token override pulled from the spec, before resolution.
pub struct RawToken {
    /// The originating section (`colors` / `spacing` / `radius` / `typography`).
    pub section: String,
    /// The key as written in the spec.
    pub key: String,
    /// The literal value (e.g. `#2563eb`, `12px`, `'Inter', sans-serif`).
    pub value: String,
}

impl Spec {
    /// Wrap a parsed document.
    pub fn new(doc: Document) -> Self {
        Spec { doc }
    }

    /// Human-facing project name, used in banners. Defaults to `rustio-admin`.
    pub fn project_name(&self) -> &str {
        self.doc
            .section("project")
            .and_then(|s| s.str("name"))
            .unwrap_or("rustio-admin")
    }

    /// Output directory for generated artifacts. Defaults to `generated`.
    pub fn out_dir(&self) -> &str {
        self.doc
            .section("project")
            .and_then(|s| s.str("out_dir"))
            .unwrap_or("generated")
    }

    /// Optional path to a rustio-admin checkout (enables brand-ramp derivation).
    pub fn rustio_admin_path(&self) -> Option<&str> {
        self.doc
            .section("project")
            .and_then(|s| s.str("rustio_admin_path"))
    }

    /// The brand color, if set.
    pub fn brand_color(&self) -> Option<&str> {
        self.doc.section("brand").and_then(|s| s.str("color"))
    }

    /// Whether to derive a WCAG-safe ramp via `rustio-admin theme generate`.
    pub fn brand_derive(&self) -> bool {
        self.doc
            .section("brand")
            .and_then(|s| s.boolean("derive"))
            .unwrap_or(false)
    }

    /// Raw token overrides from the four token sections, in source order.
    pub fn raw_tokens(&self) -> Vec<RawToken> {
        let mut out = Vec::new();
        for section in ["colors", "spacing", "radius", "typography"] {
            if let Some(s) = self.doc.section(section) {
                for (k, v) in &s.entries {
                    if let Some(val) = v.as_str() {
                        out.push(RawToken {
                            section: section.to_string(),
                            key: k.clone(),
                            value: val.to_string(),
                        });
                    }
                }
            }
        }
        out
    }

    /// Raw CSS from the `[custom_css].rules` escape hatch, if any.
    pub fn custom_css(&self) -> Option<&str> {
        self.doc
            .section("custom_css")
            .and_then(|s| s.str("rules"))
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }
}
