//! The record-view layer — the second WHAT-layer slice compiled to output
//! (after navigation). A `[views.<table>]` block declares how that table's
//! records are laid out: which fields lead (`primary`), support (`secondary`),
//! fold to the detail screen (`detail`), or never show (`hidden`), plus how
//! composed fields render and the default view mode.
//!
//! `build` validates the block against the table's columns (best-effort, like
//! navigation coverage) and freezes it to a deterministic `*.view.json` — the
//! "frozen file the renderer reads at runtime". We emit data, not a template:
//! the bridge never guesses at rustio-admin's record-list markup (see R-001).

use crate::spec::TableView;

/// Supported view modes; the default mode must be one of these.
const MODES: [&str; 3] = ["list", "cards", "gallery"];

/// The outcome of linting a `[views.<table>]` block.
pub struct ViewLint {
    /// Fatal problems — `build` refuses while any exist.
    pub errors: Vec<String>,
    /// Advisories — surfaced but non-blocking (column coverage is best-effort).
    pub warnings: Vec<String>,
}

/// Lint one table view: structural errors, plus best-effort coverage against the
/// table's columns (when discoverable from its model).
pub fn lint(view: &TableView, columns: Option<&[String]>) -> ViewLint {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let table = &view.table;

    // --- mode must be a real view mode ---
    if !MODES.contains(&view.mode.as_str()) {
        errors.push(format!(
            "[views.{table}] mode `{}` is not one of {:?}",
            view.mode, MODES
        ));
    }

    // --- unrecognised keys (typo guard) ---
    for k in &view.unknown_keys {
        warnings.push(format!(
            "[views.{table}] unknown key `{k}` — expected mode / primary / secondary / \
             detail / hidden / model / source / _out"
        ));
    }

    // --- every field is placed in exactly one cell ---
    let mut seen: Vec<String> = Vec::new();
    for cell in &view.cells {
        for m in &cell.members {
            if seen.iter().any(|s| s == m) {
                errors.push(format!(
                    "[views.{table}] field `{m}` is placed more than once — list it in one role"
                ));
            } else {
                seen.push(m.clone());
            }
        }
    }

    // --- at least one primary field leads the record ---
    if !view.cells.iter().any(|c| c.role == "primary") {
        warnings.push(format!(
            "[views.{table}] no `primary` field — the record has nothing to lead with"
        ));
    }

    // --- best-effort column coverage (warnings only) ---
    match columns {
        Some(cols) => {
            for field in &seen {
                if !cols.iter().any(|c| c == field) {
                    let hint = closest(field, cols)
                        .map(|c| format!(" — did you mean `{c}`?"))
                        .unwrap_or_default();
                    warnings.push(format!(
                        "[views.{table}] `{field}` matches no column on the model{hint}"
                    ));
                }
            }
            for col in cols {
                if !seen.iter().any(|s| s == col) {
                    warnings.push(format!(
                        "[views.{table}] column `{col}` is unplaced — it will be hidden; \
                         add it to a role to show it, or to `hidden` to silence this"
                    ));
                }
            }
        }
        None => warnings.push(format!(
            "[views.{table}] could not read the table's columns — set `model` (and `source`) \
             to validate field names; coverage not verified"
        )),
    }

    ViewLint { errors, warnings }
}

/// Render the frozen view spec as deterministic JSON — the artifact the runtime
/// renderer reads. Field order is fixed so the output is reproducible.
pub fn build_view_json(project: &str, view: &TableView) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str("  \"_generated_by\": \"rustio-design\",\n");
    s.push_str(&format!("  \"project\": {},\n", json_str(project)));
    s.push_str(&format!("  \"table\": {},\n", json_str(&view.table)));
    s.push_str(&format!("  \"default_mode\": {},\n", json_str(&view.mode)));
    s.push_str("  \"modes\": [\"list\", \"cards\", \"gallery\"],\n");
    s.push_str("  \"cells\": [\n");
    for (i, cell) in view.cells.iter().enumerate() {
        let members = cell
            .members
            .iter()
            .map(|m| json_str(m))
            .collect::<Vec<_>>()
            .join(", ");
        let comma = if i + 1 < view.cells.len() { "," } else { "" };
        s.push_str(&format!(
            "    {{ \"members\": [{members}], \"role\": {}, \"style\": {} }}{comma}\n",
            json_str(&cell.role),
            json_str(&cell.style),
        ));
    }
    s.push_str("  ]\n");
    s.push_str("}\n");
    s
}

/// Minimal JSON string literal (quotes + escapes the few characters that matter).
fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Closest column to a field by shared-prefix length (cheap "did you mean").
fn closest(target: &str, columns: &[String]) -> Option<String> {
    columns
        .iter()
        .map(|c| (shared_prefix(target, c), c))
        .filter(|(n, _)| *n >= 3)
        .max_by_key(|(n, _)| *n)
        .map(|(_, c)| c.clone())
}

fn shared_prefix(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Spec;
    use crate::toml_lite::Document;

    fn view(toml: &str) -> TableView {
        Spec::new(Document::parse(toml).unwrap())
            .views()
            .pop()
            .expect("one view")
    }

    #[test]
    fn flags_duplicate_field_and_bad_mode() {
        let v = view("[views.bookings]\nmode = \"grid\"\nprimary = \"a\"\nsecondary = \"a\"\n");
        let l = lint(&v, None);
        assert!(l.errors.iter().any(|e| e.contains("placed more than once")));
        assert!(l.errors.iter().any(|e| e.contains("not one of")));
    }

    #[test]
    fn warns_on_unknown_column_with_hint() {
        let v = view("[views.bookings]\nprimary = \"custmer\"\n");
        let cols = vec!["customer".to_string(), "status".to_string()];
        let l = lint(&v, Some(&cols));
        assert!(l.errors.is_empty());
        assert!(l
            .warnings
            .iter()
            .any(|w| w.contains("did you mean `customer`")));
    }

    #[test]
    fn json_is_deterministic_and_composed() {
        let v = view(
            "[views.bookings]\nmode = \"list\"\nprimary = \"booked_at\"\n\
             secondary = \"customer + phone (inline), status (badge)\"\nhidden = \"id\"\n",
        );
        let j = build_view_json("salon", &v);
        assert!(j.contains("\"default_mode\": \"list\""));
        assert!(j.contains(
            "\"members\": [\"customer\", \"phone\"], \"role\": \"secondary\", \"style\": \"inline\""
        ));
        assert!(
            j.contains("\"members\": [\"status\"], \"role\": \"secondary\", \"style\": \"badge\"")
        );
        assert_eq!(build_view_json("salon", &v), j); // reproducible
    }
}
