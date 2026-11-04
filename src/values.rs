//! Data files: an ordinary .lst whose top level is only `lock` values of fixed values, read by
//! the program Lust is part of without running anything (`lust::read_values`). An engine keeps
//! what its editor makes in them (tile maps, areas), so a project is all Lust:
//!
//! ```text
//! lock int version = 2;
//! lock TileMap farm = new TileMap {
//!     tileset = Tileset.Farm,
//!     size = [40, 36],
//!     rows = ["....tttt", "....tttt"],
//! };
//! end
//! ```
//!
//! Fixed values are numbers, text, bools, `nothing`, a state's option (`Tileset.Farm`), lists of
//! fixed values (`[40, 36]`, a vec3 too), and things made with `new` from fixed values. Anything
//! that would need running (a call, a sum, a variable) is a mistake with its line.

use crate::ast::{ExprKind, Expr, Stmt};
use crate::Problem;

/// A fixed value in a data file.
#[derive(Debug, Clone, PartialEq)]
pub enum Fixed {
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Nothing,
    /// A state's option, by its full name with the state's: `"Tileset.Farm"`.
    Option(String),
    /// `[a, b, c]`: a list, or a vec3's three numbers.
    List(Vec<Fixed>),
    /// `new TileMap { … }`: a thing, its class's name and its fields in the order written.
    Thing { class: String, fields: Vec<(String, Fixed)> },
}

impl Fixed {
    /// The field `name` of a thing.
    pub fn field(&self, name: &str) -> Option<&Fixed> {
        match self {
            Fixed::Thing { fields, .. } => fields.iter().find(|(n, _)| n == name).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Fixed::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Fixed::Float(f) => Some(*f),
            Fixed::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Fixed::Text(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Fixed::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Fixed]> {
        match self {
            Fixed::List(l) => Some(l),
            _ => None,
        }
    }
}

/// One `lock` value of a data file.
#[derive(Debug, Clone, PartialEq)]
pub struct Named {
    /// Its name: `farm`.
    pub name: String,
    /// Its type as written: `TileMap`, `int`, `string array`.
    pub ty: String,
    pub value: Fixed,
    /// The line it starts on.
    pub line: u32,
}

/// What a data file holds: its values in the order written, and what it passes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Values {
    pub values: Vec<Named>,
    pub passes: Vec<String>,
}

impl Values {
    /// The value called `name`.
    pub fn get(&self, name: &str) -> Option<&Fixed> {
        self.values.iter().find(|v| v.name == name).map(|v| &v.value)
    }
}

/// Reads a data file (`path` only names it in mistakes): every value, without running anything.
/// A file with anything but `lock` values of fixed values (and `fetch`, `get` and `pass` lines)
/// gives back every mistake, with its line.
pub fn read_values(path: &str, text: &str) -> Result<Values, Vec<Problem>> {
    let mut diags = crate::diag::Diags::default();
    let toks = crate::lexer::lex(&text.replace("\r\n", "\n"), 0, &mut diags);
    let prog = crate::parser::parse(toks, &mut diags, false, 0);
    let problem = |pos: crate::ast::Pos, message: String| Problem {
        file: path.to_string(),
        line: pos.row(),
        column: pos.col,
        message,
    };
    let mut problems: Vec<Problem> = diags.sorted().into_iter().map(|(p, m)| problem(p, m)).collect();
    if !problems.is_empty() {
        return Err(problems);
    }
    for item in &prog.items {
        let pos = item_pos(item);
        problems.push(problem(
            pos,
            "a data file holds only lock values: classes, methods and the rest go in a file of their own.".into(),
        ));
    }
    let mut out = Values {
        values: Vec::new(),
        passes: prog.passes.iter().map(|(n, _)| n.clone()).collect(),
    };
    for s in &prog.body {
        match s {
            Stmt::Var {
                ty,
                name,
                pos,
                value,
                locked: true,
            } => match fixed(value) {
                Ok(v) => out.values.push(Named {
                    name: name.clone(),
                    ty: ty.to_string(),
                    value: v,
                    line: pos.row(),
                }),
                Err((at, why)) => problems.push(problem(at, why)),
            },
            Stmt::Var { name, pos, .. } => problems.push(problem(
                *pos,
                format!("a data file's values are locked, since nothing runs to change them: lock {name} = …;"),
            )),
            other => problems.push(problem(
                other.pos(),
                "a data file holds only lock values, and nothing in it runs.".into(),
            )),
        }
    }
    if problems.is_empty() {
        Ok(out)
    } else {
        Err(problems)
    }
}

fn item_pos(item: &crate::ast::Item) -> crate::ast::Pos {
    use crate::ast::Item;
    match item {
        Item::State(s) => s.pos,
        Item::Class(c) => c.pos,
        Item::Method(m) => m.pos,
        Item::Assembly(a) => a.pos,
        Item::Database(d) => d.pos,
        Item::Route(r) => r.pos,
        Item::Style(s) => s.pos,
        Item::Event(e) => e.pos,
    }
}

/// A fixed value, or where and why it isn't one. Deep lists are walked without the stack (a big
/// map is millions of values, but never deep).
fn fixed(e: &Expr) -> Result<Fixed, (crate::ast::Pos, String)> {
    Ok(match &e.kind {
        ExprKind::Int(i) => Fixed::Int(*i),
        ExprKind::Float(f) => Fixed::Float(*f),
        ExprKind::Str(s) => Fixed::Text(s.clone()),
        ExprKind::Bool(b) => Fixed::Bool(*b),
        ExprKind::Nothing => Fixed::Nothing,
        ExprKind::Neg(inner) => match &inner.kind {
            ExprKind::Int(i) => Fixed::Int(-i),
            ExprKind::Float(f) => Fixed::Float(-f),
            _ => return Err((e.pos, "only a number can be made negative in a data file: -3, not -x.".into())),
        },
        ExprKind::List { items, keys: None } => Fixed::List(items.iter().map(fixed).collect::<Result<_, _>>()?),
        ExprKind::List { keys: Some(_), .. } => {
            return Err((e.pos, "a data file's lists are plain: [1, 2, 3]. Use new with fields for named values.".into()))
        }
        ExprKind::New { class, inits, .. } => Fixed::Thing {
            class: crate::ast::shown(class).to_string(),
            fields: inits
                .iter()
                .map(|(n, _, v)| Ok((n.clone(), fixed(v)?)))
                .collect::<Result<_, _>>()?,
        },
        // Tileset.Farm, Clip.Farmer.Walk: a state's option, by its full name.
        ExprKind::Field(..) => match option_path(e) {
            Some(path) => Fixed::Option(path),
            None => return Err((e.pos, "a data file's values are fixed: numbers, text, options like Tileset.Farm, lists and new things.".into())),
        },
        ExprKind::Template(_) => {
            return Err((e.pos, "text in a data file is plain: it can't have values in { }.".into()))
        }
        _ => {
            return Err((
                e.pos,
                "a data file's values are fixed, since nothing in it runs: numbers, text, bools, nothing, options like Tileset.Farm, lists and new things.".into(),
            ))
        }
    })
}

/// `State.Option.Inner` as one name, if it's only names joined with dots.
fn option_path(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Name(n) => Some(crate::ast::shown(n).to_string()),
        ExprKind::Field(t, f, _) => Some(format!("{}.{f}", option_path(t)?)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_data_file_s_values_are_read_without_running_anything() {
        let text = "-- Written by the editor.\n\
            fetch \"solis/tiles\":TileMap;\n\
            lock int version = 2;\n\
            lock TileMap farm = new TileMap {\n\
                tileset = Tileset.Farm,\n\
                size = [40, -36],\n\
                scale = 1.5,\n\
                shown = true,\n\
                owner = nothing,\n\
                layers = [new Layer { name = \"Ground\", rows = [\"..tt\", \"..tt\"] }],\n\
            };\n\
            end\n";
        let v = read_values("maps/farm.lst", text).unwrap();
        assert_eq!(v.get("version"), Some(&Fixed::Int(2)));
        assert!(v.passes.is_empty());
        let farm = v.get("farm").unwrap();
        assert_eq!(farm.field("tileset"), Some(&Fixed::Option("Tileset.Farm".into())));
        assert_eq!(farm.field("size"), Some(&Fixed::List(vec![Fixed::Int(40), Fixed::Int(-36)])));
        assert_eq!(farm.field("scale").and_then(Fixed::as_float), Some(1.5));
        assert_eq!(farm.field("shown").and_then(Fixed::as_bool), Some(true));
        assert_eq!(farm.field("owner"), Some(&Fixed::Nothing));
        let layer = &farm.field("layers").and_then(Fixed::as_list).unwrap()[0];
        assert_eq!(layer.field("name").and_then(Fixed::as_text), Some("Ground"));
        assert_eq!(v.values[1].ty, "TileMap");
        assert_eq!(v.values[1].line, 4);
    }

    #[test]
    fn anything_that_would_run_is_a_mistake_with_its_line() {
        let text = "class Thing {\n    has {\n        int a = 1;\n    }\n}\n\
            int loose = 3;\n\
            lock int sum = 1 + 2;\n\
            lock int called = make();\n\
            lock string said = \"hp {loose}\";\n\
            print(\"hi\");\n\
            end\n";
        let problems = read_values("bad.lst", text).unwrap_err();
        let lines: Vec<u32> = problems.iter().map(|p| p.line).collect();
        assert_eq!(lines, [1, 6, 7, 8, 9, 10], "{problems:?}");
        assert!(problems[1].message.contains("locked"), "{}", problems[1]);
        assert_eq!(problems[2].file, "bad.lst");
        // A file Lust can't read at all says so as it would anywhere.
        let problems = read_values("broken.lst", "lock int a = ;\n").unwrap_err();
        assert!(!problems.is_empty());
    }

    #[test]
    fn a_thousand_by_thousand_map_reads_quickly() {
        // A million cells as rows of one-character codes, in chunks of 32 × 32 as Solis writes
        // them: 1,024 chunks of 32 rows.
        let row = |x: usize, y: usize| -> String {
            (0..32).map(|i| if (x + y + i) % 7 == 0 { 't' } else { '.' }).collect()
        };
        let mut text = String::from("lock int version = 2;\nlock TileMap farm = new TileMap {\n    size = [1024, 1024],\n    chunks = [\n");
        for cy in 0..32 {
            for cx in 0..32 {
                let rows: Vec<String> = (0..32).map(|y| format!("\"{}\"", row(cx, cy * 32 + y))).collect();
                text.push_str(&format!("        new Chunk {{ at = [{cx}, {cy}], rows = [{}] }},\n", rows.join(", ")));
            }
        }
        text.push_str("    ],\n};\nend\n");
        let started = std::time::Instant::now();
        let v = read_values("big.lst", &text).unwrap();
        let took = started.elapsed();
        let chunks = v.get("farm").and_then(|f| f.field("chunks")).and_then(Fixed::as_list).unwrap();
        assert_eq!(chunks.len(), 1024);
        let cells: usize = chunks
            .iter()
            .map(|c| c.field("rows").and_then(Fixed::as_list).unwrap().iter().map(|r| r.as_text().unwrap().len()).sum::<usize>())
            .sum();
        assert_eq!(cells, 1 << 20);
        eprintln!("read a {} KB map in {took:?}", text.len() / 1024);
        // Under a second in a release build, as asked; debug builds are far slower.
        if !cfg!(debug_assertions) {
            assert!(took.as_secs_f64() < 1.0, "{took:?}");
        }
    }
}
