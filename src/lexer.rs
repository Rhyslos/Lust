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
    "===", "!==", "==", "!=", "<=", ">=", "&&", "||", "=>", ";", ",", ".", ":", "(", ")", "{", "}", "[", "]", "?", "#", "+", "-", "*", "/",
    "%", "<", ">", "=",
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
        // Comments: --[[ block ]], -- line, // line.
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
            let text: String = chars[start..i].iter().collect();
            if at(i) == 'e' || at(i) == 'E' {
                diags.add(pos, "Lust writes numbers out in full, without e.");
            } else if at(i).is_alphabetic() || at(i) == '_' {
                diags.add(pos, "a name can't start with a number.");
            }
            let end = Pos { line, col };
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
                // "hp: {hp}": a { straight before a name, a bracket, # or text starts a value. Any
                // other { is plain text, so JSON and patterns like [0-9]{4} need nothing special.
                let next = at(i + 1);
                if ch == '{' && (next.is_alphabetic() || matches!(next, '(' | '#' | '"')) {
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
        let mut matched = None;
        for s in SYMBOLS {
            let sc: Vec<char> = s.chars().collect();
            if chars[i..].starts_with(&sc) {
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
                    '&' => "Lust joins checks with &&.".to_string(),
                    '|' => "Lust joins checks with ||.".to_string(),
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
