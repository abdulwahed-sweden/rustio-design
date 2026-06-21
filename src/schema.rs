//! Schema extraction — the "what the data is" input layer.
//!
//! Best-effort discovery of a table's columns by parsing its model `struct` from
//! the project's Rust source, then mapping each Rust field type to the view
//! editor's vocabulary (text / number / timestamp / uuid / enum / …). This feeds
//! two consumers: the Adaptive View Editor (so it lists real fields) and
//! `views::lint` (so field names are validated against real columns).
//!
//! It is intentionally heuristic — like `nav::registered_models`, it reads source
//! rather than compiling it, so it stays zero-dependency and never needs to build
//! the target project. A model it cannot parse degrades to "columns unknown".

/// A discovered column: a field name and its inferred view type.
pub struct Column {
    /// The struct field name.
    pub name: String,
    /// The view type: text | number | timestamp | uuid | enum | boolean | email | phone.
    pub ty: String,
}

/// Parse `src` for `struct <model> { … }` and return its fields as columns.
///
/// Line comments and attribute lines (`#[…]`) are skipped; `pub` is stripped;
/// `Option<T>` is unwrapped to `T`. Returns empty if the struct is not found.
pub fn model_columns(src: &str, model: &str) -> Vec<Column> {
    // Strip line comments so commented fields/examples are not parsed.
    let code: String = src
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");

    // Find `struct <model>` then its opening brace.
    let needle = format!("struct {model}");
    let Some(start) = code.find(&needle) else {
        return Vec::new();
    };
    let after = &code[start + needle.len()..];
    let Some(open_rel) = after.find('{') else {
        return Vec::new();
    };
    let body_start = start + needle.len() + open_rel + 1;

    // Walk to the matching close brace (fields hold no nested braces in practice,
    // but count depth to be safe).
    let mut depth = 1usize;
    let mut end = body_start;
    for (i, c) in code[body_start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = body_start + i;
                    break;
                }
            }
            _ => {}
        }
    }
    // Strip `#[…]` attributes first — they can carry commas (e.g.
    // `#[rustio(choices = ["a", "b"])]`) that would otherwise split mid-attribute
    // and swallow the field that follows.
    let body = strip_attributes(&code[body_start..end]);

    let mut out = Vec::new();
    for raw in body.split(',') {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // A field is `name: Type` (possibly `pub name: Type`); take the last such.
        let Some(colon) = line.find(':') else {
            continue;
        };
        let name = line[..colon]
            .trim()
            .trim_start_matches("pub")
            .trim()
            .trim_start_matches("pub(crate)")
            .trim();
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let ty = line[colon + 1..].trim();
        out.push(Column {
            name: name.to_string(),
            ty: map_rust_type(ty).to_string(),
        });
    }
    out
}

/// Remove `#[…]` attribute spans from a struct body, with balanced-bracket
/// tracking so commas inside an attribute (`choices = ["a", "b"]`) don't leak
/// into field parsing.
fn strip_attributes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '#' && chars.peek() == Some(&'[') {
            chars.next(); // consume the opening '['
            let mut depth = 1usize;
            for d in chars.by_ref() {
                match d {
                    '[' => depth += 1,
                    ']' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Map a Rust type to the view editor's type vocabulary (best-effort).
pub fn map_rust_type(ty: &str) -> &'static str {
    // Unwrap Option<…> / Vec<…> and references to inspect the inner type.
    let inner = ty
        .trim()
        .trim_start_matches('&')
        .trim()
        .strip_prefix("Option<")
        .or_else(|| ty.trim().strip_prefix("Vec<"))
        .map(|s| s.trim_end_matches('>').trim())
        .unwrap_or(ty)
        .trim();
    let last = inner.rsplit("::").next().unwrap_or(inner);
    let lower = last.to_ascii_lowercase();

    if lower.contains("datetime")
        || lower.contains("timestamp")
        || lower == "date"
        || lower == "time"
        || lower == "naivedatetime"
        || lower == "naivedate"
    {
        "timestamp"
    } else if lower.contains("uuid") {
        "uuid"
    } else if last == "bool" {
        "boolean"
    } else if matches!(
        last,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
            | "Decimal"
    ) {
        "number"
    } else if lower.contains("email") {
        "email"
    } else if lower.contains("phone") {
        "phone"
    } else {
        // String, &str, and unknown user types (incl. enums) — the editor lets
        // the developer refine these.
        "text"
    }
}

/// Render columns as a view-editor schema file (JSON), ready to drop into the
/// editor's `data/schemas/<project>/<table>.json`.
pub fn columns_json(project: &str, table: &str, cols: &[Column]) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"project\": {},\n", j(project)));
    s.push_str(&format!("  \"table\": {},\n", j(table)));
    s.push_str("  \"columns\": [\n");
    for (i, c) in cols.iter().enumerate() {
        let comma = if i + 1 < cols.len() { "," } else { "" };
        s.push_str(&format!(
            "    {{ \"name\": {}, \"type\": {} }}{comma}\n",
            j(&c.name),
            j(&c.ty)
        ));
    }
    s.push_str("  ],\n");
    s.push_str("  \"sample\": []\n");
    s.push_str("}\n");
    s
}

/// Minimal JSON string literal.
fn j(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"
        #[derive(Model)]
        pub struct Booking {
            pub id: Uuid,
            pub booked_at: DateTime<Utc>,
            pub customer: String,
            pub fill_pct: i32,
            pub active: bool,
            pub email: Option<String>,
        }
    "#;

    #[test]
    fn extracts_fields_with_types() {
        let cols = model_columns(SRC, "Booking");
        let pairs: Vec<(&str, &str)> = cols
            .iter()
            .map(|c| (c.name.as_str(), c.ty.as_str()))
            .collect();
        assert_eq!(
            pairs,
            vec![
                ("id", "uuid"),
                ("booked_at", "timestamp"),
                ("customer", "text"),
                ("fill_pct", "number"),
                ("active", "boolean"),
                ("email", "text"),
            ]
        );
    }

    #[test]
    fn unknown_struct_yields_nothing() {
        assert!(model_columns(SRC, "Missing").is_empty());
    }

    #[test]
    fn handles_attribute_with_inner_commas() {
        // A `choices` attribute carries commas that must not split the field.
        let src = "pub struct Order {\n\
                   pub id: i64,\n\
                   pub total: Decimal,\n\
                   #[rustio(choices = [\"pending\", \"paid\", \"shipped\"])]\n\
                   pub status: String,\n\
                   }";
        let cols = model_columns(src, "Order");
        let names: Vec<&str> = cols.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["id", "total", "status"]);
    }

    #[test]
    fn maps_common_types() {
        assert_eq!(map_rust_type("Option<chrono::NaiveDateTime>"), "timestamp");
        assert_eq!(map_rust_type("uuid::Uuid"), "uuid");
        assert_eq!(map_rust_type("f64"), "number");
        assert_eq!(map_rust_type("String"), "text");
    }
}
