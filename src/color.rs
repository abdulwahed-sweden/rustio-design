//! Hex color parsing and WCAG relative-luminance / contrast math.
//!
//! The validator uses this to refuse a spec that would ship unreadable text —
//! the same concern rustio-admin's `rio-theme` engine encodes, but here applied
//! to the *literal* color overrides a developer (or Claude) writes by hand.

/// An 8-bit-per-channel sRGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    /// Red channel (0–255).
    pub r: u8,
    /// Green channel (0–255).
    pub g: u8,
    /// Blue channel (0–255).
    pub b: u8,
}

/// Parse a `#rgb` or `#rrggbb` color. Returns `None` for anything else.
pub fn parse_hex(s: &str) -> Option<Rgb> {
    let h = s.strip_prefix('#')?;
    let bytes = h.as_bytes();
    match bytes.len() {
        3 => {
            let r = hex_nibble(bytes[0])?;
            let g = hex_nibble(bytes[1])?;
            let b = hex_nibble(bytes[2])?;
            Some(Rgb {
                r: r * 17,
                g: g * 17,
                b: b * 17,
            })
        }
        6 => Some(Rgb {
            r: hex_byte(bytes[0], bytes[1])?,
            g: hex_byte(bytes[2], bytes[3])?,
            b: hex_byte(bytes[4], bytes[5])?,
        }),
        _ => None,
    }
}

fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn hex_byte(hi: u8, lo: u8) -> Option<u8> {
    Some(hex_nibble(hi)? * 16 + hex_nibble(lo)?)
}

/// WCAG 2.x relative luminance of an sRGB color (0.0–1.0).
pub fn relative_luminance(c: Rgb) -> f64 {
    fn lin(channel: u8) -> f64 {
        let s = channel as f64 / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

/// WCAG contrast ratio between two colors (1.0–21.0).
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let la = relative_luminance(a);
    let lb = relative_luminance(b);
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_forms() {
        assert_eq!(
            parse_hex("#fff"),
            Some(Rgb {
                r: 255,
                g: 255,
                b: 255
            })
        );
        assert_eq!(parse_hex("#000000"), Some(Rgb { r: 0, g: 0, b: 0 }));
        assert_eq!(parse_hex("nope"), None);
    }

    #[test]
    fn black_on_white_is_max_contrast() {
        let ratio = contrast_ratio(
            Rgb { r: 0, g: 0, b: 0 },
            Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
        );
        assert!(ratio > 20.9);
    }
}
