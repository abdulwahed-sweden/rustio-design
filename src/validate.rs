//! The doctrine validator — the bridge's safety brain.
//!
//! It refuses to generate anything that would violate rustio-admin's design
//! contract or quietly fail. Each check maps to a real rule from the framework's
//! `CLAUDE.md` / `docs/design/`:
//!
//! * **Token allowlist** — typos and invented tokens are rejected (with a "did
//!   you mean" hint) instead of silently doing nothing.
//! * **Contrast** — literal color overrides for text tokens are held to WCAG; a
//!   ratio below 3.0 is an error, 3.0–4.5 a warning. Mirrors `rio-theme`'s job.
//! * **No build step / no external fetch** — `custom_css` may not `@import`,
//!   pull a remote `url(http…)`, or inject `<script>`.
//! * **No second runtime** — the forbidden Tier-2 symbols rustio-admin's CI
//!   bans must never appear, even in a CSS comment.

use crate::allowlist;
use crate::color;
use crate::spec::Spec;

/// The outcome of validating a spec.
pub struct Report {
    /// Fatal problems — `build` refuses to run while any exist.
    pub errors: Vec<String>,
    /// Advisories — surfaced but non-blocking.
    pub warnings: Vec<String>,
}

impl Report {
    /// True when there are no fatal errors.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Tier-2 symbols rustio-admin's CI forbids anywhere in the repo. The bridge
/// refuses to smuggle them in through a CSS comment.
const FORBIDDEN_SYMBOLS: &[&str] = &[
    "HasSchema",
    "ModelSchema",
    "SchemaOps",
    "from_schema",
    "contract_validator",
    "contract_doctor",
    "RustioModel",
];

/// Validate a spec against the rustio-admin design doctrine.
pub fn validate(spec: &Spec) -> Report {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // --- brand color must be a real hex color ---
    if let Some(c) = spec.brand_color() {
        if color::parse_hex(c).is_none() {
            errors.push(format!(
                "[brand].color `{c}` is not a valid #rgb / #rrggbb color"
            ));
        }
    }
    if spec.brand_derive() && spec.rustio_admin_path().is_none() {
        warnings.push(
            "[brand].derive = true but no [project].rustio_admin_path is set — the WCAG-safe \
             ramp cannot be generated; the literal brand color will be used instead"
                .to_string(),
        );
    }

    // --- token overrides: resolve every key, contrast-check literal colors ---
    for t in spec.raw_tokens() {
        match allowlist::resolve(&t.section, &t.key) {
            Err(e) => errors.push(e),
            Ok(token) => {
                if let Some(rgb) = color::parse_hex(&t.value) {
                    // Only foreground "text" tokens get a contrast gate, measured
                    // against the lightest surface the framework ships (#ffffff).
                    if token.contains("text") {
                        let white = color::Rgb {
                            r: 255,
                            g: 255,
                            b: 255,
                        };
                        let ratio = color::contrast_ratio(rgb, white);
                        if ratio < 3.0 {
                            errors.push(format!(
                                "[{}].{} = {} fails contrast on white ({:.2}:1, need ≥ 4.5) — unreadable",
                                t.section, t.key, t.value, ratio
                            ));
                        } else if ratio < 4.5 {
                            warnings.push(format!(
                                "[{}].{} = {} is borderline on white ({:.2}:1, WCAG AA wants ≥ 4.5)",
                                t.section, t.key, t.value, ratio
                            ));
                        }
                    }
                }
            }
        }
    }

    // --- custom_css escape hatch: keep the framework's hard rules ---
    if let Some(css) = spec.custom_css() {
        let lower = css.to_ascii_lowercase();
        if lower.contains("@import") {
            errors.push(
                "[custom_css] contains `@import` — forbidden (no build step / single bundle)"
                    .to_string(),
            );
        }
        if lower.contains("url(http") || lower.contains("url('http") || lower.contains("url(\"http")
        {
            errors.push(
                "[custom_css] references a remote `url(http…)` — assets must be local/offline"
                    .to_string(),
            );
        }
        if lower.contains("<script") {
            errors.push("[custom_css] contains `<script` — CSS only, no markup".to_string());
        }
        for sym in FORBIDDEN_SYMBOLS {
            if css.contains(sym) {
                errors.push(format!(
                    "[custom_css] contains forbidden Tier-2 symbol `{sym}` (rustio-admin CI bans it)"
                ));
            }
        }
    }

    Report { errors, warnings }
}
