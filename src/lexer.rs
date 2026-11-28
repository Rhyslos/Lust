//! Turns Lust source text into tokens.

use crate::ast::Pos;
use crate::diag::Diags;

#[derive(Clone, Debug, PartialEq)]
pub enum Tk {
    Ident(String),
    Int(i64),
    Float(f64, String),
    Str(String),
    Sym(&'static str),
    /// The raw Lua inside `lua { … }`.
    LuaCode(String),
    /// Text with values in it: "hp: {hp}". Text with none is a Str.
    Text(Vec<TextPart>),
    Eof,
}

/// A piece of text with values in it: plain text, or a value's tokens (ending with Eof).
#[derive(Clone, Debug, PartialEq)]
pub enum TextPart {
    Plain(String),
    Value(Vec<Token>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: Tk,
    pub pos: Pos,
    /// Where the token ends: the spot right after its last character.
    pub end: Pos,
}

impl Token {
    pub fn is_word(&self, w: &str) -> bool {
        matches!(&self.kind, Tk::Ident(s) if s == w)
    }
    pub fn is_sym(&self, s: &str) -> bool {
        matches!(&self.kind, Tk::Sym(x) if *x == s)
    }
    pub fn describe(&self) -> String {
        match &self.kind {
            Tk::Ident(s) => format!("the word {s}"),
            Tk::Int(_) | Tk::Float(..) => "a number".into(),
            Tk::Str(_) | Tk::Text(_) => "some text".into(),
            Tk::Sym(s) => s.to_string(),
            Tk::LuaCode(_) => "a lua block".into(),
            Tk::Eof => "the end of the file".into(),
        }
    }
}

// Longest first, so === wins over == and =.
const SYMBOLS: &[&str] = &[
    "===", "!==", "<<=", ">>=", "==", "!=", "<<", ">>", "<=", ">=", "&&", "||", "=>", "+=", "-=", "*=", "/=", "&=", "|=", "^=", ";", ",",
    ".", ":", "(", ")", "{", "}", "[", "]", "?", "#", "+", "-", "*", "/", "%", "<", ">", "=", "&", "|", "^", "~",
];

pub fn lex(src: &str, file: usize, diags: &mut Diags) -> Vec<Token> {
    let chars: Vec<char> = src.chars().collect();
    let mut out: Vec<Token> = Vec::new();
    let (mut i, mut line, mut col) = (0usize, ((file as u32) << crate::ast::FILE_SHIFT) | 1, 1u32);
    let at = |i: usize| chars.get(i).copied().unwrap_or('\0');

    macro_rules! advance {
        ($n:expr) => {
            for _ in 0..$n {
                if at(i) == '\n' {
                    line += 1;
                    col = 1;
                } else {
                    col += 1;
                }
                i += 1;
            }
        };
    }

    while i < chars.len() {
        let c = at(i);
        let pos = Pos { line, col };
        if c.is_whitespace() {
            advance!(1);
            continue;
        }
        // Comments: --[[ block ]] and -- line. // was a comment until alpha 75.
        if c == '-' && at(i + 1) == '-' {
            if at(i + 2) == '[' && at(i + 3) == '[' {
                advance!(4);
                while i < chars.len() && !(at(i) == ']' && at(i + 1) == ']') {
                    advance!(1);
                }
                if i >= chars.len() {
                    diags.add(pos, "this --[[ block comment is never closed. End it with ]].");
                } else {
                    advance!(2);
                }
            } else {
                while i < chars.len() && at(i) != '\n' {
                    advance!(1);
                }
            }
            continue;
        }
        if c == '/' && at(i + 1) == '/' {
            diags.add(pos, "comments start with --, so write -- here instead of //.");
            while i < chars.len() && at(i) != '\n' {
                advance!(1);
            }
            continue;
        }
        if c == '/' && at(i + 1) == '*' {
            diags.add(pos, "Lust's block comments are written --[[ like this ]].");
            advance!(2);
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while at(i).is_alphanumeric() || at(i) == '_' {
                advance!(1);
            }
            let word: String = chars[start..i].iter().collect();
            if word.starts_with('_') {
                diags.add(
                    pos,
                    format!("names start with a letter, not _. Try {}.", word.trim_start_matches('_')),
                );
            }
            let end = Pos { line, col };
            out.push(Token {
                kind: Tk::Ident(word.clone()),
                pos,
                end,
            });
            // lua { … }: everything inside the braces is Lua, kept as it is.
            if word == "lua" {
                let mut j = i;
                while at(j) == ' ' || at(j) == '\t' || at(j) == '\r' || at(j) == '\n' {
                    j += 1;
                }
                if at(j) == '{' {
                    advance!(j - i);
                    let open = Pos { line, col };
                    advance!(1);
                    let start = i;
                    let mut depth = 1;
                    while i < chars.len() {
                        match at(i) {
                            '{' => depth += 1,
                            '}' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        advance!(1);
                    }
                    if i >= chars.len() {
                        diags.add(open, "this lua { is never closed. Add a } where the Lua ends.");
                        break;
                    }
                    let code: String = chars[start..i].iter().collect();
                    advance!(1);
                    out.push(Token {
                        kind: Tk::LuaCode(code),
                        pos: open,
                        end: Pos { line, col },
                    });
                }
            }
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while at(i).is_ascii_digit() {
                advance!(1);
            }
            let mut is_float = false;
            if at(i) == '.' && at(i + 1).is_ascii_digit() {
                is_float = true;
                advance!(1);
                while at(i).is_ascii_digit() {
                    advance!(1);
                }
            }
            // 1e30, 1.5e-7, 2E+5: a power of ten after the number, as Lust prints very big and
            // very small numbers. Always a float.
            let exp_digit = |k: usize| at(k).is_ascii_digit();
            if (at(i) == 'e' || at(i) == 'E') && (exp_digit(i + 1) || (matches!(at(i + 1), '+' | '-') && exp_digit(i + 2))) {
                is_float = true;
                advance!(if exp_digit(i + 1) { 1 } else { 2 });
                while at(i).is_ascii_digit() {
                    advance!(1);
                }
            }
            let text: String = chars[start..i].iter().collect();
            if at(i).is_alphabetic() || at(i) == '_' {
                diags.add(pos, "a name can't start with a number.");
            }
            let end = Pos { line, col };
            if is_float && text.parse::<f64>().map(|v| v.is_infinite()).unwrap_or(false) {
                diags.add(pos, format!("{text} is too big for a float, which goes up to about 1.8e308."));
            }
            if is_float {
                out.push(Token {
                    kind: Tk::Float(text.parse().unwrap_or(0.0), text),
                    pos,
                    end,
                });
            } else {
                match text.parse::<i64>() {
                    Ok(v) => out.push(Token {
                        kind: Tk::Int(v),
                        pos,
                        end,
                    }),
                    Err(_) => {
                        diags.add(pos, format!("{text} is too big for an int."));
                        out.push(Token {
                            kind: Tk::Int(0),
                            pos,
                            end,
                        });
                    }
                }
            }
            continue;
        }
        if c == '"' {
            advance!(1);
            let mut s = String::new();
            let mut parts: Vec<TextPart> = Vec::new();
            let mut closed = false;
            while i < chars.len() && at(i) != '\n' {
                // Plain letters go straight in: a data file's big map is millions of them.
                while i < chars.len() && !matches!(chars[i], '"' | '\\' | '{' | '\n') {
                    s.push(chars[i]);
                    i += 1;
                    col += 1;
                }
                if i >= chars.len() || chars[i] == '\n' {
                    break;
                }
                let ch = at(i);
                if ch == '"' {
                    closed = true;
                    advance!(1);
                    break;
                }
                if ch == '\\' {
                    let e = at(i + 1);
                    match e {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        '"' => s.push('"'),
                        '\\' => s.push('\\'),
                        '{' => s.push('{'),
                        '}' => s.push('}'),
                        _ => {
                            diags.add(
                                Pos { line, col },
                                format!("\\{e} isn't a known escape. Use \\n, \\t, \\\", \\\\, or \\{{ for a plain {{."),
                            );
                            s.push(e);
                        }
                    }
                    advance!(2);
                    continue;
                }
                // "hp: {hp}": a { straight before a name, a bracket, # or text starts a value, and
                // so does one before a number, [, - or ~, as in "{1 + lives}", unless it's a
                // pattern's count like [0-9]{4} or {1,3}. Any other { is plain text, so JSON and
                // patterns need nothing special.
                let next = at(i + 1);
                let sure = next.is_alphabetic() || matches!(next, '(' | '#' | '"');
                let maybe = next.is_ascii_digit()
                    || next == '['
                    || (matches!(next, '-' | '~') && {
                        let n = at(i + 2);
                        n.is_alphanumeric() || n == '('
                    });
                if ch == '{' && (sure || maybe) {
                    // Find the } that ends the value: past brackets and text inside it.
                    let (mut j, mut depth) = (i + 1, 1);
                    while j < chars.len() && at(j) != '\n' {
                        match at(j) {
                            '"' => {
                                j += 1;
                                while j < chars.len() && at(j) != '"' && at(j) != '\n' {
                                    if at(j) == '\\' {
                                        j += 1;
                                    }
                                    j += 1;
                                }
                            }
                            '{' => depth += 1,
                            '}' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        j += 1;
                    }
                    let count = || chars[i + 1..j].iter().all(|c| c.is_ascii_digit() || *c == ',');
                    if !sure && (at(j) != '}' || count()) {
                        s.push(ch);
                        advance!(1);
                        continue;
                    }
                    if at(j) != '}' {
                        diags.add(
                            Pos { line, col },
                            "this { in the text is never closed. Add a } after the value, or write \\{ for a plain {.",
                        );
                        s.push(ch);
                        advance!(1);
                        continue;
                    }
                    let code: String = chars[i + 1..j].iter().collect();
                    let at_col = col + 1;
                    // The value's tokens, where they are in the file.
                    let mut inner = Diags::default();
                    let mut toks = lex(&code, 0, &mut inner);
                    for t in toks.iter_mut() {
                        t.pos = Pos {
                            line,
                            col: at_col + t.pos.col - 1,
                        };
                        t.end = Pos {
                            line,
                            col: at_col + t.end.col - 1,
                        };
                    }
                    for (p, m) in inner.list {
                        diags.add(
                            Pos {
                                line,
                                col: at_col + p.col - 1,
                            },
                            m,
                        );
                    }
                    if !s.is_empty() {
                        parts.push(TextPart::Plain(std::mem::take(&mut s)));
                    }
                    parts.push(TextPart::Value(toks));
                    advance!(j - i + 1);
                    continue;
                }
                s.push(ch);
                advance!(1);
            }
            if !closed {
                diags.add(pos, "this text is never closed. Add a \" at its end, on the same line.");
            }
            let kind = if parts.is_empty() {
                Tk::Str(s)
            } else {
                if !s.is_empty() {
                    parts.push(TextPart::Plain(s));
                }
                Tk::Text(parts)
            };
            out.push(Token {
                kind,
                pos,
                end: Pos { line, col },
            });
            continue;
        }
        if c == '\'' {
            diags.add(pos, "text goes in double quotes: \"like this\".");
            advance!(1);
            continue;
        }
        // a ~= b, from Lua: Lust writes "not the same" as !=.
        if c == '~' && at(i + 1) == '=' {
            diags.add(pos, "Lust writes \"not the same\" as !=: is a != b?");
            advance!(2);
            out.push(Token {
                kind: Tk::Sym("!="),
                pos,
                end: Pos { line, col },
            });
            continue;
        }
        let mut matched = None;
        // Symbols are plain ASCII, so they're matched in place, without making anything.
        for s in SYMBOLS {
            if s.bytes().enumerate().all(|(k, b)| chars.get(i + k) == Some(&(b as char))) {
                matched = Some(*s);
                break;
            }
        }
        match matched {
            Some(s) => {
                advance!(s.len());
                out.push(Token {
                    kind: Tk::Sym(s),
                    pos,
                    end: Pos { line, col },
                });
            }
            None => {
                let hint = match c {
                    '!' => "Lust writes not, not !: is not alive?".to_string(),
                    _ => format!("Lust doesn't use the symbol {c}."),
                };
                diags.add(pos, hint);
                advance!(1);
            }
        }
    }
    out.push(Token {
        kind: Tk::Eof,
        pos: Pos { line, col },
        end: Pos { line, col },
    });
    out
}

#[cfg(test)]
mod text_values {
    //! Which { in text starts a value, and which is just a brace.

    use super::*;

    /// How many values the text holds, and how many mistakes it gives.
    fn values(src: &str) -> (usize, usize) {
        let mut diags = Diags::default();
        let toks = lex(src, 0, &mut diags);
        let n = match &toks[0].kind {
            Tk::Text(parts) => parts.iter().filter(|p| matches!(p, TextPart::Value(_))).count(),
            _ => 0,
        };
        (n, diags.list.len())
    }

    #[test]
    fn numbers_and_signs_start_values() {
        for s in [
            r#""{1 + a}""#,
            r#""{2.5}""#,
            r#""{-a}""#,
            r#""{~a}""#,
            r#""{[1, 2]}""#,
            r#""{-(a)}""#,
            r#""{12 * a}""#,
        ] {
            assert_eq!(values(s), (1, 0), "{s}");
        }
    }

    #[test]
    fn counts_json_and_lone_signs_stay_plain() {
        for s in [
            r#""[0-9]{4}""#,
            r#""{1,3}""#,
            r#""{2,}""#,
            r#""{0} items""#,
            r#""{}""#,
            r#""{ a }""#,
            r#""{-}""#,
            r#""{- a}""#,
            r#""{\"x\": 1}""#,
            r#""{1 + 2""#,
            r#""{[1""#,
        ] {
            assert_eq!(values(s), (0, 0), "{s}");
        }
    }

    #[test]
    fn a_name_left_open_is_still_a_mistake() {
        assert_eq!(values(r#""{a + 1""#), (0, 1));
    }

    #[test]
    fn odd_text_never_panics() {
        let bits = ["{", "}", "1", ",", "-", "~", "[", "]", "(", "a", "\\{", " ", "#", "\\\""];
        let mut seed = 7u64;
        for _ in 0..3000 {
            let mut s = String::from("\"");
            for _ in 0..(seed % 12) {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                s.push_str(bits[(seed >> 33) as usize % bits.len()]);
            }
            s.push('"');
            let _ = values(&s);
        }
    }
}
