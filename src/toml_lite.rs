//! A tiny, strict TOML *subset* parser — just enough for `rustio.design.toml`.
//!
//! We deliberately do not pull a full TOML crate. The spec grammar the bridge
//! needs is small and fully under our control:
//!
//! * `# comment` lines and blank lines are ignored.
//! * `[section]` headers open a section.
//! * `key = "string"` — a double-quoted string (supports `\"` and `\\`).
//! * `key = """ ... """` — a triple-quoted multi-line string (for raw CSS).
//! * `key = true` / `key = false` — a boolean.
//!
//! Anything outside this grammar is a hard parse error with a line number, so a
//! malformed spec fails loudly instead of silently dropping a setting.

/// A parsed scalar value.
#[derive(Debug, Clone)]
pub enum Value {
    /// A string value (single- or triple-quoted in the source).
    Str(String),
    /// A boolean value.
    Bool(bool),
}

impl Value {
    /// Borrow this value as a string slice, if it is a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            Value::Bool(_) => None,
        }
    }

    /// Read this value as a boolean, if it is one.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            Value::Str(_) => None,
        }
    }
}

/// One `[section]` and its ordered key/value entries.
#[derive(Debug, Clone)]
pub struct Section {
    /// The section name (the top-level section before any header is `""`).
    pub name: String,
    /// Entries in source order.
    pub entries: Vec<(String, Value)>,
}

impl Section {
    /// Look up a key within this section.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Convenience: read a key as a string slice.
    pub fn str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Value::as_str)
    }

    /// Convenience: read a key as a boolean.
    pub fn boolean(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(Value::as_bool)
    }
}

/// A parsed document: an ordered list of sections.
#[derive(Debug, Clone)]
pub struct Document {
    /// Sections in source order.
    pub sections: Vec<Section>,
}

impl Document {
    /// Find the first section with the given name.
    pub fn section(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.name == name)
    }

    /// Parse a `rustio.design.toml` document from source text.
    pub fn parse(input: &str) -> Result<Document, String> {
        let lines: Vec<&str> = input.lines().collect();
        let mut sections: Vec<Section> = Vec::new();
        let mut current = Section {
            name: String::new(),
            entries: Vec::new(),
        };

        let mut i = 0;
        while i < lines.len() {
            let raw = lines[i];
            let line = raw.trim();

            if line.is_empty() || line.starts_with('#') {
                i += 1;
                continue;
            }

            // Section header: [name]
            if let Some(stripped) = line.strip_prefix('[') {
                let end = stripped
                    .find(']')
                    .ok_or_else(|| format!("line {}: missing `]` in section header", i + 1))?;
                let name = stripped[..end].trim().to_string();
                sections.push(std::mem::replace(
                    &mut current,
                    Section {
                        name,
                        entries: Vec::new(),
                    },
                ));
                i += 1;
                continue;
            }

            // key = value
            let eq = line
                .find('=')
                .ok_or_else(|| format!("line {}: expected `key = value`", i + 1))?;
            let key = line[..eq].trim().to_string();
            if key.is_empty() {
                return Err(format!("line {}: empty key", i + 1));
            }
            let rhs = line[eq + 1..].trim();

            // Triple-quoted multi-line string.
            if let Some(after) = rhs.strip_prefix("\"\"\"") {
                let mut content = String::new();
                if let Some(close) = after.find("\"\"\"") {
                    content.push_str(&after[..close]);
                } else {
                    content.push_str(after);
                    content.push('\n');
                    i += 1;
                    loop {
                        if i >= lines.len() {
                            return Err(format!("line {}: unterminated `\"\"\"` string", i));
                        }
                        let l = lines[i];
                        if let Some(close) = l.find("\"\"\"") {
                            content.push_str(&l[..close]);
                            break;
                        }
                        content.push_str(l);
                        content.push('\n');
                        i += 1;
                    }
                }
                // TOML convention: a newline immediately after the opening
                // delimiter is trimmed.
                let content = content.strip_prefix('\n').unwrap_or(&content).to_string();
                current.entries.push((key, Value::Str(content)));
                i += 1;
                continue;
            }

            // Single-line double-quoted string.
            if let Some(rest) = rhs.strip_prefix('"') {
                let close = rest
                    .find('"')
                    .ok_or_else(|| format!("line {}: unterminated string", i + 1))?;
                let val = unescape(&rest[..close]);
                current.entries.push((key, Value::Str(val)));
                i += 1;
                continue;
            }

            // Boolean (allow a trailing inline comment).
            let bare = rhs.split('#').next().unwrap_or("").trim();
            match bare {
                "true" => current.entries.push((key, Value::Bool(true))),
                "false" => current.entries.push((key, Value::Bool(false))),
                other => {
                    return Err(format!(
                        "line {}: unsupported value `{other}` — strings must be quoted",
                        i + 1
                    ))
                }
            }
            i += 1;
        }

        sections.push(current);
        Ok(Document { sections })
    }
}

/// Minimal escape handling for double-quoted strings: `\"` and `\\`.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sections_and_values() {
        let doc = Document::parse(
            r##"
# a comment
[project]
name = "Acme"
[brand]
color = "#2563eb"
derive = true
[custom_css]
rules = """
.x { color: red; }
"""
"##,
        )
        .unwrap();
        assert_eq!(doc.section("project").unwrap().str("name"), Some("Acme"));
        assert_eq!(doc.section("brand").unwrap().str("color"), Some("#2563eb"));
        assert_eq!(doc.section("brand").unwrap().boolean("derive"), Some(true));
        assert!(doc
            .section("custom_css")
            .unwrap()
            .str("rules")
            .unwrap()
            .contains("color: red"));
    }

    #[test]
    fn rejects_unquoted_string() {
        assert!(Document::parse("[a]\nk = bare").is_err());
    }
}
