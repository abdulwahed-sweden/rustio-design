//! The canonical `--rio-*` token allowlist and spec-key → token resolution.
//!
//! This is the heart of the "developer doesn't get dizzy" guarantee. rustio-admin
//! defines a fixed vocabulary of CSS custom properties across its `tokens/` files.
//! A hand-written `--rio-acccent` typo silently does nothing — the override never
//! lands and the developer chases a ghost. Here, every spec key is resolved to a
//! real token name; an unknown key is rejected *with the closest valid suggestion*
//! before a single byte is generated.
//!
//! The list mirrors `crates/rustio-admin/assets/static/admin/tokens/*.css`. If the
//! framework adds a token, add it here too (or point `[project].rustio_admin_path`
//! at a checkout and `check` will diff against the live files in a future version).

/// Every canonical `--rio-*` custom property the framework recognises.
pub const CANONICAL: &[&str] = &[
    // ---- colors / brand (engine output) ----
    "--rio-brand-light",
    "--rio-brand-dark",
    "--rio-brand-adaptive",
    "--rio-brand-surface",
    "--rio-brand-accent",
    "--rio-brand-secondary",
    "--rio-brand-hover",
    "--rio-brand-active",
    "--rio-brand-tint",
    "--rio-brand-text",
    "--rio-muted",
    // ---- colors / live admin aliases ----
    "--rio-accent",
    "--rio-accent-hover",
    "--rio-accent-rgb",
    "--rio-accent-soft",
    "--rio-accent-border",
    "--rio-bg",
    "--rio-surface",
    "--rio-surface-2",
    "--rio-surface-3",
    "--rio-surface-chrome",
    "--rio-surface-elevated",
    "--rio-text-strong",
    "--rio-text",
    "--rio-text-muted",
    "--rio-text-subtle",
    "--rio-border-soft",
    "--rio-border",
    "--rio-border-strong",
    "--rio-success",
    "--rio-warning",
    "--rio-danger",
    "--rio-success-bg",
    "--rio-warning-bg",
    "--rio-danger-bg",
    "--rio-info-bg",
    // ---- spacing ----
    "--rio-s1",
    "--rio-s2",
    "--rio-s3",
    "--rio-s4",
    "--rio-s5",
    "--rio-s6",
    "--rio-s7",
    "--rio-sidebar-w",
    "--rio-topbar-h",
    "--rio-content-max",
    "--rio-z-sidebar",
    "--rio-z-topbar",
    "--rio-z-dropdown",
    "--rio-z-modal",
    // ---- radius ----
    "--rio-radius",
    "--rio-radius-sm",
    "--rio-radius-lg",
    // ---- typography ----
    "--rio-font-sans",
    "--rio-font-arabic",
    "--rio-font-arabic-body",
    "--rio-font-mono",
    "--rio-font-japanese",
    "--rio-font-korean",
    "--rio-font-chinese",
    "--rio-font-thai",
    "--rio-font-devanagari",
    "--rio-font-size-base",
    "--rio-fs-xs",
    "--rio-fs-sm",
    "--rio-fs-md",
    "--rio-fs-base",
    "--rio-fs-lg",
    "--rio-fs-xl",
    "--rio-fs-h3",
    "--rio-fs-h2",
    "--rio-fs-h1",
    "--rio-fs-display",
    "--rio-lh-tight",
    "--rio-lh-ui",
    "--rio-lh-body",
    "--rio-lh-arabic",
    "--rio-fw-regular",
    "--rio-fw-medium",
    "--rio-fw-semibold",
    "--rio-fw-bold",
];

/// Resolve a `(section, key)` pair from the spec to a canonical `--rio-*` token.
///
/// Returns `Err` with a human-readable message (including the nearest valid
/// token, when one is close) if the key does not map to a real token.
pub fn resolve(section: &str, key: &str) -> Result<String, String> {
    let candidate = match section {
        "radius" if key == "default" => "--rio-radius".to_string(),
        "radius" => format!("--rio-radius-{key}"),
        "colors" | "spacing" | "typography" => format!("--rio-{key}"),
        other => {
            return Err(format!(
                "section [{other}] does not map to design tokens (use colors/spacing/radius/typography)"
            ))
        }
    };

    if CANONICAL.contains(&candidate.as_str()) {
        Ok(candidate)
    } else {
        let hint = suggest(&candidate)
            .map(|s| format!(" — did you mean `{s}`?"))
            .unwrap_or_default();
        Err(format!(
            "unknown token `{key}` in [{section}] → `{candidate}` is not a rustio-admin token{hint}"
        ))
    }
}

/// Return the canonical token closest to `name` (Levenshtein ≤ 4), if any.
pub fn suggest(name: &str) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    for &cand in CANONICAL {
        let d = levenshtein(name, cand);
        if best.map(|(bd, _)| d < bd).unwrap_or(true) {
            best = Some((d, cand));
        }
    }
    match best {
        Some((d, cand)) if d <= 4 => Some(cand.to_string()),
        _ => None,
    }
}

/// Classic Levenshtein edit distance, used for "did you mean" suggestions.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_keys() {
        assert_eq!(resolve("colors", "accent").unwrap(), "--rio-accent");
        assert_eq!(resolve("radius", "default").unwrap(), "--rio-radius");
        assert_eq!(resolve("radius", "sm").unwrap(), "--rio-radius-sm");
        assert_eq!(resolve("spacing", "sidebar-w").unwrap(), "--rio-sidebar-w");
    }

    #[test]
    fn typo_is_rejected_with_suggestion() {
        let err = resolve("colors", "acccent").unwrap_err();
        assert!(err.contains("--rio-accent"), "got: {err}");
    }
}
