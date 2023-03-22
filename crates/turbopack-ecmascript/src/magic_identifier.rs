use std::{borrow::Cow, fmt::Write};

use once_cell::sync::Lazy;
use regex::{Captures, Regex};

pub fn encode(content: &str) -> String {
    let mut r = "__TURBOPACK__".to_string();
    let mut hex_mode = false;
    for c in content.chars() {
        if matches!(c, '0'..='9' | 'A'..='Z' | 'a'..='z' | ' ') {
            if hex_mode {
                r.push('$');
                hex_mode = false;
            }
            if c == ' ' {
                r += "__";
            } else {
                r.push(c);
            }
        } else if c == '_' && (!r.ends_with('_') || hex_mode) {
            if hex_mode {
                r.push('$');
                hex_mode = false;
            }
            r += "_";
        } else if c == '$' && !hex_mode {
            r += "$$";
        } else if matches!(c, '\0'..='\u{ff}') {
            if !hex_mode {
                r.push('$');
                hex_mode = true;
            }
            write!(r, "{0:2x}", c as u8).unwrap();
        } else {
            if !hex_mode {
                r.push('$');
            }
            write!(r, "_{:x}$", c as u32).unwrap();
            hex_mode = false;
        }
    }
    if hex_mode {
        r.push('$');
    }
    r += "__";
    r
}

/// Decodes a magic identifier into a string.
pub fn decode(identifier: &str) -> String {
    static DECODE_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^__TURBOPACK__([a-zA-Z0-9_$]+)__$").unwrap());

    let Some(captures) = DECODE_REGEX.captures(identifier) else {
        return identifier.to_string();
    };

    let content = captures.get(1).unwrap().as_str();

    enum Mode {
        Text,
        Underscore,
        Hex,
        LongHex,
    }
    let mut mode = Mode::Text;
    let mut output = String::new();
    let mut buffer = String::with_capacity(2);
    for char in content.chars() {
        match mode {
            Mode::Text => match char {
                '_' => mode = Mode::Underscore,
                '$' => mode = Mode::Hex,
                c => output.push(c),
            },
            Mode::Underscore => match char {
                '_' => {
                    output.push(' ');
                    mode = Mode::Text;
                }
                '$' => {
                    output.push('_');
                    mode = Mode::Hex;
                }
                c => {
                    output.push('_');
                    output.push(c);
                    mode = Mode::Text;
                }
            },
            Mode::Hex => {
                if buffer.len() == 2 {
                    if let Ok(byte) = u8::from_str_radix(&buffer, 16) {
                        output.push(byte as char);
                    }
                    buffer.clear();
                }
                match char {
                    '_' => {
                        debug_assert!(buffer.len() == 0);
                        mode = Mode::LongHex;
                    }
                    '$' => {
                        debug_assert!(buffer.len() == 0);
                        mode = Mode::Text;
                    }
                    c => {
                        buffer.push(c);
                    }
                }
            }
            Mode::LongHex => {
                debug_assert!(char != '_');
                match char {
                    '$' => {
                        if let Ok(code) = u32::from_str_radix(&buffer, 16) {
                            output.push(std::char::from_u32(code).unwrap());
                        }
                        buffer.clear();
                        mode = Mode::Text;
                    }
                    c => {
                        buffer.push(c);
                    }
                }
            }
        }
    }
    debug_assert!(matches!(mode, Mode::Text));
    output
}

/// Decode all magic identifiers in a string.
pub fn decode_identifiers<T: AsRef<str>>(text: &str, magic: impl Fn(String) -> T) -> Cow<'_, str> {
    static IDENTIFIER_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"__TURBOPACK__[a-zA-Z0-9_$]+__").unwrap());

    IDENTIFIER_REGEX.replace_all(text, |captures: &Captures| {
        magic(decode(captures.get(0).unwrap().as_str()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        assert_eq!(encode("Hello World"), "__TURBOPACK__Hello__World__");
        assert_eq!(encode("Hello_World"), "__TURBOPACK__Hello_World__");
        assert_eq!(encode("Hello__World"), "__TURBOPACK__Hello_$5f$World__");
        assert_eq!(encode("Hello___World"), "__TURBOPACK__Hello_$5f$_World__");
        assert_eq!(encode("Hello/World"), "__TURBOPACK__Hello$2f$World__");
        assert_eq!(encode("Hello///World"), "__TURBOPACK__Hello$2f2f2f$World__");
        assert_eq!(encode("Hello/_World"), "__TURBOPACK__Hello$2f$_World__");
        assert_eq!(encode("Hello_/_World"), "__TURBOPACK__Hello_$2f$_World__");
        assert_eq!(encode("Hello😀World"), "__TURBOPACK__Hello$_1f600$World__");
        assert_eq!(
            encode("Hello/😀/World"),
            "__TURBOPACK__Hello$2f_1f600$$2f$World__"
        );
        assert_eq!(
            encode("Hello😀😀World"),
            "__TURBOPACK__Hello$_1f600$$_1f600$World__"
        );
    }

    #[test]
    fn test_decode() {
        assert_eq!(decode("__TURBOPACK__Hello__World__"), "Hello World");
        assert_eq!(decode("__TURBOPACK__Hello_World__"), "Hello_World");
        assert_eq!(decode("__TURBOPACK__Hello_$5f$World__"), "Hello__World");
        assert_eq!(decode("__TURBOPACK__Hello_$5f$_World__"), "Hello___World");
        assert_eq!(decode("__TURBOPACK__Hello$2f$World__"), "Hello/World");
        assert_eq!(decode("__TURBOPACK__Hello$2f2f2f$World__"), "Hello///World");
        assert_eq!(decode("__TURBOPACK__Hello$2f$_World__"), "Hello/_World");
        assert_eq!(decode("__TURBOPACK__Hello_$2f$_World__"), "Hello_/_World");
        assert_eq!(decode("__TURBOPACK__Hello$_1f600$World__"), "Hello😀World");
        assert_eq!(
            decode("__TURBOPACK__Hello$2f_1f600$$2f$World__"),
            "Hello/😀/World"
        );
        assert_eq!(
            decode("__TURBOPACK__Hello$_1f600$$_1f600$World__"),
            "Hello😀😀World"
        );
    }

    #[test]
    fn test_decode_identifiers() {
        assert_eq!(
            decode_identifiers(
                "Hello __TURBOPACK__Hello__World__ __TURBOPACK__Hello_World__",
                |s| format!("{{{s}}}")
            ),
            "Hello {Hello World} {Hello_World}"
        );
    }
}
