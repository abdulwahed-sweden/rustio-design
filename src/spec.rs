//! A typed, convenient view over the parsed `rustio.design.toml` document.
//!
//! Every accessor here corresponds to something the generator actually emits, so
//! there are no dead settings: if a key exists in the spec, it has an effect.

use crate::toml_lite::Document;

/// The design spec — the single source of truth for a rustio-admin look.
pub struct Spec {
    doc: Document,
}

/// Resolved paths to the narrative design-memory artifacts (relative to the
/// directory that holds the spec). Defaults match what `init` scaffolds.
pub struct DesignPaths {
    /// WHY — business context, design intent, visual direction.
    pub brief: String,
    /// The ADR-style reasoning trail authored before any spec change.
    pub reasoning: String,
    /// WHAT — information architecture, navigation, UX hierarchy.
    pub architecture: String,
    /// The durable ledger of accepted decisions.
    pub decisions: String,
    /// The human-readable evolution of the design.
    pub history: String,
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

    /// Paths to the higher design-memory layers, from the `[design]` section
    /// (with conventional defaults when a key, or the section, is absent).
    pub fn design_paths(&self) -> DesignPaths {
        let d = self.doc.section("design");
        let get =
            |key: &str, default: &str| d.and_then(|s| s.str(key)).unwrap_or(default).to_string();
        DesignPaths {
            brief: get("brief", "design/DESIGN_BRIEF.md"),
            reasoning: get("reasoning", "design/DESIGN_REASONING.md"),
            architecture: get("architecture", "design/DESIGN_ARCHITECTURE.md"),
            decisions: get("decisions", "design/DESIGN_DECISIONS.md"),
            history: get("history", "design/DESIGN_HISTORY.md"),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toml_lite::Document;

    #[test]
    fn design_paths_default_when_absent() {
        let spec = Spec::new(Document::parse("[project]\nname = \"X\"\n").unwrap());
        let p = spec.design_paths();
        assert_eq!(p.brief, "design/DESIGN_BRIEF.md");
        assert_eq!(p.reasoning, "design/DESIGN_REASONING.md");
        assert_eq!(p.history, "design/DESIGN_HISTORY.md");
    }

    #[test]
    fn design_paths_honor_overrides() {
        let spec = Spec::new(Document::parse("[design]\nbrief = \"docs/brief.md\"\n").unwrap());
        assert_eq!(spec.design_paths().brief, "docs/brief.md");
        // unspecified keys still fall back to defaults
        assert_eq!(spec.design_paths().decisions, "design/DESIGN_DECISIONS.md");
    }
}
