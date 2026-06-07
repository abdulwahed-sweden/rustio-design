//! Assemble the whole design stack into one canonical context stream.
//!
//! `rustio-design context` emits a single document that Claude Design and Claude
//! Code read **before** touching the token layer. It stitches the narrative
//! design memory (the WHY and WHAT layers, the reasoning trail, the decision
//! ledger, and the history) together with the current token spec (the HOW), in
//! the canonical order:
//!
//!   WHY  → Reasoning → WHAT → Decisions → History → HOW
//!
//! The higher layers are **not** compiled into output — that is deliberate. This
//! command exists so the reasoning happens over one coherent narrative, not so
//! the narrative is generated from. Missing artifacts render as explicit
//! `(reserved)` placeholders rather than vanishing, which keeps them *active*
//! memory: their absence is always visible.

use crate::allowlist;
use crate::spec::Spec;
use std::path::Path;

/// Build the canonical design context for a spec rooted at `root`.
pub fn build_context(spec: &Spec, root: &Path) -> String {
    let mut out = String::new();
    out.push_str(&header(spec));

    let p = spec.design_paths();
    out.push_str(&layer("LAYER 1 · WHY — DESIGN BRIEF", &root.join(&p.brief)));
    out.push_str(&layer(
        "REASONING — DESIGN REASONING (ADR trail)",
        &root.join(&p.reasoning),
    ));
    out.push_str(&layer(
        "LAYER 2 · WHAT — DESIGN ARCHITECTURE",
        &root.join(&p.architecture),
    ));
    out.push_str(&layer(
        "DESIGN DECISIONS (ledger)",
        &root.join(&p.decisions),
    ));
    out.push_str(&layer("DESIGN HISTORY (evolution)", &root.join(&p.history)));
    out.push_str(&token_spec(spec));
    out.push_str(&footer());
    out
}

fn banner(title: &str) -> String {
    let bar = "=".repeat(78);
    format!("\n{bar}\n {title}\n{bar}\n\n")
}

fn header(spec: &Spec) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "# rustio-design — canonical design context for `{}`\n\n",
        spec.project_name()
    ));
    s.push_str(
        "This is the single, coherent design narrative for this admin. Read it top\n\
         to bottom BEFORE proposing or making any change to the token layer.\n\n\
         Pipeline:  Brief → Reasoning → Architecture → Spec → Generated\n\
         Direction: reason TOP-DOWN (WHY → WHAT → HOW); generate BOTTOM-UP.\n\n\
         Rule: the WHY and WHAT layers are design memory, not generated output.\n\
         Justify changes in DESIGN_REASONING.md first (see `/design-reason`).\n",
    );
    s
}

/// Render one narrative layer from a file, or a `(reserved)` placeholder.
fn layer(title: &str, path: &Path) -> String {
    let mut s = banner(title);
    match std::fs::read_to_string(path) {
        Ok(body) => {
            let body = strip_frontmatter(&body);
            let trimmed = body.trim();
            if trimmed.is_empty() {
                s.push_str(&placeholder(path));
            } else {
                s.push_str(trimmed);
                s.push('\n');
            }
        }
        Err(_) => s.push_str(&placeholder(path)),
    }
    s
}

fn placeholder(path: &Path) -> String {
    format!(
        "(reserved — `{}` is not authored yet. Create it, or run `rustio-design init` \
         to scaffold the design memory, then capture the design intent here.)\n",
        path.display()
    )
}

/// Strip a leading YAML-style `--- … ---` frontmatter block for the context view
/// (the metadata is for tooling, not for the reasoning narrative).
fn strip_frontmatter(body: &str) -> String {
    let t = body.trim_start();
    if let Some(rest) = t.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            // Skip past the closing delimiter line.
            let after = &rest[end + 4..];
            return after.trim_start_matches('\n').to_string();
        }
    }
    body.to_string()
}

/// Render the HOW layer: the current, resolved token specification.
fn token_spec(spec: &Spec) -> String {
    let mut s = banner("LAYER 3 · HOW — TOKEN SPECIFICATION (rustio.design.toml)");

    match spec.brand_color() {
        Some(c) if spec.brand_derive() => {
            s.push_str(&format!(
                "brand: {c}  (derive = true → WCAG-safe ramp via rustio-admin rio-theme)\n"
            ));
        }
        Some(c) => s.push_str(&format!("brand: {c}  (literal)\n")),
        None => s.push_str("brand: (none set)\n"),
    }

    let tokens = spec.raw_tokens();
    if tokens.is_empty() {
        s.push_str("token overrides: (none)\n");
    } else {
        s.push_str("token overrides:\n");
        for t in tokens {
            match allowlist::resolve(&t.section, &t.key) {
                Ok(name) => s.push_str(&format!("  {name}: {}\n", t.value)),
                Err(_) => s.push_str(&format!(
                    "  [{}].{} = {}  (UNRESOLVED — run `rustio-design check`)\n",
                    t.section, t.key, t.value
                )),
            }
        }
    }

    match spec.custom_css() {
        Some(_) => s.push_str("custom_css: present (validated escape hatch)\n"),
        None => s.push_str("custom_css: none\n"),
    }

    // Navigation (WHAT compiled to a sidebar override) — shown here as the
    // concrete projection of the architecture's navigation decision.
    match spec.navigation() {
        Some(nav) => {
            s.push_str("\nnavigation (compiled → ");
            s.push_str(&nav.out);
            s.push_str("):\n");
            for g in &nav.groups {
                s.push_str(&format!("  {}: {}\n", g.label, g.items.join(", ")));
            }
            if !nav.hidden.is_empty() {
                s.push_str(&format!("  (hidden: {})\n", nav.hidden.join(", ")));
            }
        }
        None => s.push_str("navigation: (none declared — flat model list)\n"),
    }
    s
}

fn footer() -> String {
    let bar = "=".repeat(78);
    format!(
        "\n{bar}\n end of design context — reason from the above, then justify any token\n\
         \x20change in DESIGN_REASONING.md before editing rustio.design.toml.\n{bar}\n"
    )
}
