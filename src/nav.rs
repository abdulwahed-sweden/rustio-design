//! The navigation layer — the first compilation of a WHAT-layer decision into a
//! generated artifact (tracking: rustio-design#1).
//!
//! `rustio-admin`'s `_sidebar.html` renders a flat "Models" list from the
//! framework-provided `entries` (each `{ admin_name, display_name }`); there is
//! no grouping/ordering/hiding seam in the builder. So the recompile-free seam is
//! a **template override**: we generate a `_sidebar.html` that buckets the
//! framework's own `entries` into the groups declared in `[navigation]` — reusing
//! the framework's URLs and labels (so links are always correct), hiding the
//! buried models, and preserving the Home / Auth / Developer sections verbatim.
//!
//! Consumed via `RUSTIO_TEMPLATE_DIR` exactly like the framework's own overrides.

use crate::spec::Navigation;

/// The outcome of linting a `[navigation]` block.
pub struct NavLint {
    /// Fatal problems — `build` refuses while any exist.
    pub errors: Vec<String>,
    /// Advisories — surfaced but non-blocking (model coverage is best-effort).
    pub warnings: Vec<String>,
}

/// Lint a navigation block: structural errors, plus best-effort coverage against
/// the project's registered models (when discoverable).
pub fn lint(nav: &Navigation, models: Option<&[String]>) -> NavLint {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // --- structural: every item appears exactly once across groups + hidden ---
    let mut seen: Vec<(String, String)> = Vec::new(); // (item, where)
    let mut record = |item: &str, whence: &str, errors: &mut Vec<String>| {
        if let Some((_, prev)) = seen.iter().find(|(i, _)| i == item) {
            errors.push(format!(
                "[navigation] `{item}` appears in both `{prev}` and `{whence}` — list it once"
            ));
        } else {
            seen.push((item.to_string(), whence.to_string()));
        }
    };
    for g in &nav.groups {
        if g.label.starts_with('_') {
            errors.push(format!(
                "[navigation] group label `{}` may not start with `_` (reserved)",
                g.label
            ));
        }
        if g.items.is_empty() {
            errors.push(format!("[navigation] group `{}` has no items", g.label));
        }
        for item in &g.items {
            record(item, &g.label, &mut errors);
        }
    }
    for item in &nav.hidden {
        record(item, "_hidden", &mut errors);
    }

    // --- best-effort model coverage (warnings only; plural rules are irregular) -
    if let Some(models) = models {
        let model_keys: Vec<(String, String)> =
            models.iter().map(|m| (m.clone(), key(m))).collect();

        // declared items that match no registered model
        for (item, _) in &seen {
            if !model_keys.iter().any(|(_, k)| *k == key(item)) {
                let hint = closest(&key(item), &model_keys)
                    .map(|m| format!(" — did you mean `{m}`?"))
                    .unwrap_or_default();
                warnings.push(format!(
                    "[navigation] `{item}` matches no registered model{hint}"
                ));
            }
        }
        // registered models neither grouped nor hidden (would silently vanish)
        for (model, mk) in &model_keys {
            if !seen.iter().any(|(i, _)| key(i) == *mk) {
                warnings.push(format!(
                    "[navigation] model `{model}` is neither grouped nor hidden — it will not \
                     appear in the sidebar; add it to a group or `_hidden`"
                ));
            }
        }
    } else {
        warnings.push(
            "[navigation] could not read the project's models (src/main.rs) — \
             coverage not verified"
                .to_string(),
        );
    }

    NavLint { errors, warnings }
}

/// Parse a project `src/main.rs` for `.model::<Type>()` registrations.
///
/// Line comments are stripped first, so commented examples (e.g. the
/// `.model::<YourModel>()` marker in a scaffold) are not mistaken for real
/// registrations.
pub fn registered_models(main_rs: &str) -> Vec<String> {
    let code: String = main_rs
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let mut out = Vec::new();
    let mut rest = code.as_str();
    while let Some(pos) = rest.find(".model::<") {
        rest = &rest[pos + ".model::<".len()..];
        if let Some(end) = rest.find('>') {
            let ty = rest[..end].trim();
            // keep only a bare type path's last segment, e.g. `crate::Product` → Product
            let ty = ty.rsplit("::").next().unwrap_or(ty).trim();
            if !ty.is_empty() && ty.chars().all(|c| c.is_alphanumeric() || c == '_') {
                out.push(ty.to_string());
            }
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }
    out
}

/// Generate the `_sidebar.html` override that groups the framework's `entries`.
///
/// One `{% for entry in entries %}` pass per declared item preserves the declared
/// order and matches on `display_name`, so URLs/labels/icons stay the framework's.
pub fn build_sidebar_html(project: &str, nav: &Navigation) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "{{# ============================================================\n\
         \x20# GENERATED by rustio-design — DO NOT EDIT THIS FILE.\n\
         \x20#\n\
         \x20#   rustio-admin sidebar (navigation layer)\n\
         \x20#   project: {project}\n\
         \x20#\n\
         \x20# Source of truth: the [navigation] section of rustio.design.toml,\n\
         \x20# decided in design/DESIGN_ARCHITECTURE.md (see DESIGN_REASONING).\n\
         \x20# Edit the spec, then run `rustio-design build`. Hand edits here are\n\
         \x20# reverted on the next build and flagged by `rustio-design check`.\n\
         \x20# ============================================================ #}}\n"
    ));
    // Wrap in a uniquely-named block so the framework's override-completeness
    // check (looks for {% extends %}, {% block %}, or <html>) does not warn — this
    // is an *included* partial, so the block renders its body verbatim.
    s.push_str("{% block rio_sidebar %}\n");
    s.push_str("<aside class=\"rio-sidebar\" aria-label=\"Admin navigation\">\n");
    s.push_str("  <nav>\n");
    s.push_str("    <ul class=\"rio-sidebar-list\">\n");
    s.push_str(
        "      <li><a href=\"/admin\" class=\"rio-sidebar-link\">{{ icon(\"home\", class=\"rio-icon\") }} Home</a></li>\n",
    );
    s.push_str("      {% if entries %}\n");

    for g in &nav.groups {
        s.push_str(&format!(
            "      <li class=\"rio-sidebar-section\">{}</li>\n",
            jinja_escape(&g.label)
        ));
        for item in &g.items {
            // One pass per item → declared order; match on the framework's label.
            s.push_str(&format!(
                "      {{% for entry in entries %}}{{% if entry.display_name == \"{}\" %}}\
                 <li><a href=\"/admin/{{{{ entry.admin_name }}}}\" class=\"rio-sidebar-link\">\
                 {{{{ icon(\"table\", class=\"rio-icon\") }}}} {{{{ entry.display_name }}}}</a></li>\
                 {{% endif %}}{{% endfor %}}\n",
                jinja_escape(item)
            ));
        }
    }

    s.push_str("      {% endif %}\n");
    // Auth + Developer sections, preserved verbatim from the framework template.
    s.push_str(
        "      {% if identity and identity.is_admin %}\n\
         \x20     <li class=\"rio-sidebar-section\">Auth</li>\n\
         \x20     <li><a href=\"/admin/users\" class=\"rio-sidebar-link\">{{ icon(\"users\", class=\"rio-icon\") }} Users</a></li>\n\
         \x20     <li><a href=\"/admin/groups\" class=\"rio-sidebar-link\">{{ icon(\"users-2\", class=\"rio-icon\") }} Groups</a></li>\n\
         \x20     <li><a href=\"/admin/history\" class=\"rio-sidebar-link\">{{ icon(\"clock\", class=\"rio-icon\") }} History</a></li>\n\
         \x20     {% endif %}\n\
         \x20     {% if identity and identity.is_developer %}\n\
         \x20     <li class=\"rio-sidebar-section\">Developer</li>\n\
         \x20     <li><a href=\"/admin/db\" class=\"rio-sidebar-link\">{{ icon(\"database\", class=\"rio-icon\") }} Database</a></li>\n\
         \x20     {% endif %}\n",
    );
    s.push_str("    </ul>\n");
    s.push_str("  </nav>\n");
    s.push_str("</aside>\n");
    s.push_str("{% endblock %}\n");
    s
}

/// Defensive escaping for a label embedded in a Jinja string literal / markup.
/// Labels are author-controlled, but never let a `"` or `<` break the template.
fn jinja_escape(s: &str) -> String {
    s.replace(['\\', '"', '<', '>'], "")
        .replace("{{", "")
        .replace("{%", "")
}

/// Plural-insensitive match key: lowercase, alphanumerics only, with a regular
/// plural ending folded (`ies`→`y`, trailing `s` dropped). Applied to both sides,
/// so same-base names that differ only by a regular plural compare equal.
fn key(s: &str) -> String {
    let norm: String = s
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect();
    if let Some(stem) = norm.strip_suffix("ies") {
        format!("{stem}y")
    } else if let Some(stem) = norm.strip_suffix('s') {
        stem.to_string()
    } else {
        norm
    }
}

/// Closest registered model to a key, by shared-prefix length (cheap, good enough
/// for a "did you mean" on short identifiers).
fn closest(target: &str, models: &[(String, String)]) -> Option<String> {
    models
        .iter()
        .map(|(name, k)| (shared_prefix(target, k), name))
        .filter(|(n, _)| *n >= 3)
        .max_by_key(|(n, _)| *n)
        .map(|(_, name)| name.clone())
}

fn shared_prefix(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_registered_models() {
        let src = "    .model::<Product>()\n.model::<crate::Category>()\n.model::<Order>();";
        assert_eq!(registered_models(src), vec!["Product", "Category", "Order"]);
    }

    #[test]
    fn ignores_commented_model_calls() {
        let src = "// insertion point for .model::<YourModel>() calls\n.model::<Product>()";
        assert_eq!(registered_models(src), vec!["Product"]);
    }

    #[test]
    fn key_folds_regular_plurals() {
        assert_eq!(key("Products"), key("Product"));
        assert_eq!(key("Categories"), key("Category"));
        assert_eq!(key("Order items"), key("OrderItem"));
        assert_eq!(key("Address"), key("Address"));
    }
}
