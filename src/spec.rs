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

/// One sidebar group: a label and its ordered member items (by display name).
pub struct NavGroup {
    /// The section label shown in the sidebar (e.g. "Catalogue").
    pub label: String,
    /// Member items, in declared order, matched against `entry.display_name`.
    pub items: Vec<String>,
}

/// The navigation layer parsed from `[navigation]` (the WHAT layer's compiled
/// projection). Reserved keys: `_hidden` (omitted from the sidebar) and `_out`
/// (the generated template's path, relative to the spec root).
pub struct Navigation {
    /// Ordered groups.
    pub groups: Vec<NavGroup>,
    /// Items deliberately kept out of the sidebar (reached via their parent).
    pub hidden: Vec<String>,
    /// Output path for the generated `_sidebar.html`, relative to the spec root.
    pub out: String,
}

/// One cell of a record view: one or more composed members under a lead field,
/// carrying an importance role and a compose style.
pub struct ViewCell {
    /// Member field names; the first is the lead. More than one ⇒ composed.
    pub members: Vec<String>,
    /// Importance role: `primary` | `secondary` | `detail` | `hidden`.
    pub role: String,
    /// Compose style: `stacked` | `inline` | `badge`.
    pub style: String,
}

/// The record-layout for one table, parsed from `[views.<table>]` (the WHAT
/// layer's record-view projection). Roles are authored as comma-separated cell
/// lists; a cell may compose members with `+` and hint a style in parens, e.g.
/// `secondary = "customer + phone (inline), status (badge)"`.
pub struct TableView {
    /// Table name (the suffix after `views.` in the section header).
    pub table: String,
    /// Default view mode: `list` | `cards` | `gallery`.
    pub mode: String,
    /// Cells in declared order, each tagged with its role.
    pub cells: Vec<ViewCell>,
    /// Optional model type name, used to resolve the table's columns for
    /// schema-aware validation (best-effort; see `schema::model_columns`).
    pub model: Option<String>,
    /// Optional source file holding the model struct (defaults to `src/main.rs`).
    pub source: Option<String>,
    /// Output path for the generated `list.html` override (`_out`), relative to
    /// the spec root. Like `[navigation]._out`, point it at the directory the
    /// framework serves via `RUSTIO_TEMPLATE_DIR` (e.g.
    /// `templates/admin/<table>/list.html`). Defaults under `generated/`.
    pub out: String,
    /// Unrecognised keys in the section (surfaced as warnings by `views::lint`).
    pub unknown_keys: Vec<String>,
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

    /// The navigation layer from `[navigation]`, if declared (and non-empty).
    ///
    /// `Group = "Item, Item"` (ordered); reserved `_hidden = "Item, Item"` and
    /// `_out = "path"`. Returns `None` when the section is absent or has no groups.
    pub fn navigation(&self) -> Option<Navigation> {
        let s = self.doc.section("navigation")?;
        let mut groups = Vec::new();
        let mut hidden = Vec::new();
        let mut out = "generated/templates/admin/_sidebar.html".to_string();
        for (k, v) in &s.entries {
            let Some(val) = v.as_str() else { continue };
            match k.as_str() {
                "_hidden" => hidden = split_items(val),
                "_out" => out = val.to_string(),
                _ => groups.push(NavGroup {
                    label: k.clone(),
                    items: split_items(val),
                }),
            }
        }
        if groups.is_empty() {
            return None;
        }
        Some(Navigation {
            groups,
            hidden,
            out,
        })
    }

    /// The record-view layer: every `[views.<table>]` section with at least one
    /// placed field, in document order.
    ///
    /// Section keys: `mode`, the four role lists (`primary` / `secondary` /
    /// `detail` / `hidden`), and reserved `model` / `source` / `_out`. Unknown
    /// keys are retained on `TableView::unknown_keys` for `views::lint` to flag.
    pub fn views(&self) -> Vec<TableView> {
        const ROLES: [&str; 4] = ["primary", "secondary", "detail", "hidden"];
        let mut out = Vec::new();
        for section in &self.doc.sections {
            let Some(table) = section.name.strip_prefix("views.") else {
                continue;
            };
            if table.is_empty() {
                continue;
            }
            let mut mode = "list".to_string();
            let mut cells = Vec::new();
            let mut model = None;
            let mut source = None;
            let mut out_path = format!("generated/templates/admin/{table}/list.html");
            let mut unknown_keys = Vec::new();
            for (k, v) in &section.entries {
                let Some(val) = v.as_str() else { continue };
                match k.as_str() {
                    "mode" => mode = val.trim().to_string(),
                    "model" => model = Some(val.trim().to_string()),
                    "source" => source = Some(val.trim().to_string()),
                    "_out" => out_path = val.trim().to_string(),
                    role if ROLES.contains(&role) => {
                        for (members, style) in parse_cells(val) {
                            cells.push(ViewCell {
                                members,
                                role: role.to_string(),
                                style,
                            });
                        }
                    }
                    other => unknown_keys.push(other.to_string()),
                }
            }
            if cells.is_empty() {
                continue;
            }
            out.push(TableView {
                table: table.to_string(),
                mode,
                cells,
                model,
                source,
                out: out_path,
                unknown_keys,
            });
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

/// Split a comma-separated item list, trimming and dropping empties.
fn split_items(s: &str) -> Vec<String> {
    s.split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

/// Parse a role's value into `(members, style)` cells.
///
/// `"customer + phone (inline), status (badge), notes"` ⇒
/// `[(["customer","phone"], "inline"), (["status"], "badge"), (["notes"], "stacked")]`.
/// An unrecognised style falls back to `stacked`.
fn parse_cells(value: &str) -> Vec<(Vec<String>, String)> {
    value
        .split(',')
        .filter_map(|raw| {
            let raw = raw.trim();
            if raw.is_empty() {
                return None;
            }
            let (body, style) = match raw.rfind('(') {
                Some(i) if raw.ends_with(')') => (raw[..i].trim(), &raw[i + 1..raw.len() - 1]),
                _ => (raw, "stacked"),
            };
            let members: Vec<String> = body
                .split('+')
                .map(|m| m.trim().to_string())
                .filter(|m| !m.is_empty())
                .collect();
            if members.is_empty() {
                None
            } else {
                Some((members, normalize_style(style.trim())))
            }
        })
        .collect()
}

/// Clamp a style hint to the supported set, defaulting to `stacked`.
fn normalize_style(style: &str) -> String {
    match style {
        "inline" | "badge" | "stacked" => style.to_string(),
        _ => "stacked".to_string(),
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
