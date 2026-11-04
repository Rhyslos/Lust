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
//! that would need running (a call, a sum, a variable) is a mistake with its line. The file is
//! checked like any other too, with the files it fetches, so types, classes and fields are right.

use crate::ast::{Expr, ExprKind, Stmt};
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
    Thing {
        class: String,
        fields: Vec<(String, Fixed)>,
    },
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

/// Reads a data file without running anything: every value, checked like any Lust file (types,
/// classes, fields and options), with the files it fetches handed over as for [`crate::check`].
/// A file with anything but `lock` values of fixed values (and `fetch`, `get` and `pass` lines)
/// gives back every mistake, with its line.
pub fn read_values(entry: &str, files: &[(String, String)]) -> Result<Values, Vec<Problem>> {
    read_values_with(&crate::Engine::default(), entry, files)
}

/// [`read_values`], inside an engine: its own folders' files come with the data file, so it can
/// use the engine's classes and the project's states.
pub fn read_values_with(engine: &crate::Engine, entry: &str, files: &[(String, String)]) -> Result<Values, Vec<Problem>> {
    let Some((_, text)) = files.iter().find(|(p, _)| p.replace('\\', "/") == entry.replace('\\', "/")) else {
        return Err(vec![Problem {
            file: entry.to_string(),
            line: 0,
            column: 0,
            message: format!("there's no file called {entry} among the files handed over."),
        }]);
    };
    crate::on_own_stack(|| {
        // The whole check, as for any file: a wrong type, an unknown class or field, or a name
        // that isn't an option is a mistake.
        let checked = match crate::front_from(entry, files, engine) {
            Ok(c) => c,
            // When something in it would run, that's said plainly instead of the check's own
            // words about it (a call to nothing, a variable it can't see).
            Err(problems) => {
                let mut diags = crate::diag::Diags::default();
                let toks = crate::lexer::lex(&text.replace("\r\n", "\n"), 0, &mut diags);
                let prog = crate::parser::parse(toks, &mut diags, false, 0);
                if diags.is_empty() {
                    shape(entry, &prog, None)?;
                }
                return Err(problems);
            }
        };
        let values = shape(entry, &checked.prog, Some(&checked.info))?;
        Ok(Values {
            values,
            passes: checked
                .prog
                .passes
                .iter()
                .filter(|(_, p)| p.file() == 0)
                .map(|(n, _)| n.clone())
                .collect(),
        })
    })
}

/// The data file's values (the program's first file), or every place it's more than `lock`
/// values of fixed values.
fn shape(path: &str, prog: &crate::ast::Program, info: Option<&crate::check::Info>) -> Result<Vec<Named>, Vec<Problem>> {
    let problem = |pos: crate::ast::Pos, message: String| Problem {
        file: path.to_string(),
        line: pos.row(),
        column: pos.col,
        message,
    };
    let mut problems = Vec::new();
    let mut values = Vec::new();
    for item in prog.items.iter().filter(|i| item_pos(i).file() == 0) {
        problems.push(problem(
            item_pos(item),
            "a data file holds only lock values: classes, methods and the rest go in a file of their own.".into(),
        ));
    }
    for s in prog.body.iter().filter(|s| s.pos().file() == 0) {
        match s {
            Stmt::Var {
                ty,
                name,
                pos,
                value,
                locked: true,
            } => match fixed(value, info) {
                Ok(v) => values.push(Named {
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
    problems.sort_by_key(|p| (p.line, p.column));
    if problems.is_empty() {
        Ok(values)
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

/// A fixed value, or where and why it isn't one. With the check's findings, a dotted name must be
/// a state's option: `Tileset.Farm`, never a field of something (`other.hp`).
fn fixed(e: &Expr, info: Option<&crate::check::Info>) -> Result<Fixed, (crate::ast::Pos, String)> {
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
        ExprKind::List { items, keys: None } => Fixed::List(items.iter().map(|i| fixed(i, info)).collect::<Result<_, _>>()?),
        ExprKind::List { keys: Some(_), .. } => {
            return Err((e.pos, "a data file's lists are plain: [1, 2, 3]. Use new with fields for named values.".into()))
        }
        ExprKind::New { class, inits, .. } => Fixed::Thing {
            class: crate::ast::shown(class).to_string(),
            fields: inits
                .iter()
                .map(|(n, _, v)| Ok((n.clone(), fixed(v, info)?)))
                .collect::<Result<_, _>>()?,
        },
        // Tileset.Farm, Clip.Farmer.Walk: a state's option, by its full name.
        ExprKind::Field(..) => match option_path(e) {
            // The check marks each name of an option, so a field that holds a state (farm.tileset)
            // isn't taken for one.
            Some(path) if info.is_none_or(|i| i.options.contains_key(&e.id)) => Fixed::Option(path),
            _ => return Err((e.pos, "a data file's values are fixed: numbers, text, options like Tileset.Farm, lists and new things.".into())),
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

    /// The classes and states a farm's data file uses, as an engine's files would give them.
    const TILES: &str = "state Tileset = [Farm, Town];\n\
        class Layer {\n    has {\n        string name = \"\";\n        string array rows = [];\n    }\n}\n\
        class Chunk {\n    has {\n        int array at = [];\n        string array rows = [];\n    }\n}\n\
        class TileMap {\n    has {\n        Tileset tileset = Tileset.Farm;\n        int array size = [];\n        float scale = 1.0;\n        bool shown = false;\n        vec3 offset = [0, 0, 0];\n        Layer array layers = [];\n        Chunk array chunks = [];\n    }\n}\n\
        pass Tileset, Layer, Chunk, TileMap;\nend\n";

    fn read(path: &str, text: &str) -> Result<Values, Vec<Problem>> {
        read_values(path, &[(path.to_string(), text.to_string()), ("tiles.lst".into(), TILES.into())])
    }

    #[test]
    fn a_data_file_s_values_are_read_without_running_anything() {
        let text = "-- Written by the editor.\n\
            fetch \"tiles\":[TileMap, Layer, Tileset];\n\
            lock int version = 2;\n\
            lock TileMap farm = new TileMap {\n\
                tileset = Tileset.Town,\n\
                size = [40, -36],\n\
                scale = 1.5,\n\
                shown = true,\n\
                offset = [1.0, 2.0, 3.0],\n\
                layers = [new Layer { name = \"Ground\", rows = [\"..tt\", \"..tt\"] }, new Layer { name = \"Empty\" }],\n\
                chunks = [],\n\
            };\n\
            end\n";
        let v = read("farm.lst", text).unwrap();
        assert_eq!(v.get("version"), Some(&Fixed::Int(2)));
        assert!(v.passes.is_empty());
        let farm = v.get("farm").unwrap();
        assert_eq!(farm.field("tileset"), Some(&Fixed::Option("Tileset.Town".into())));
        assert_eq!(farm.field("size"), Some(&Fixed::List(vec![Fixed::Int(40), Fixed::Int(-36)])));
        assert_eq!(farm.field("scale").and_then(Fixed::as_float), Some(1.5));
        assert_eq!(farm.field("shown").and_then(Fixed::as_bool), Some(true));
        assert_eq!(farm.field("offset").and_then(Fixed::as_list).map(<[Fixed]>::len), Some(3));
        assert_eq!(farm.field("chunks"), Some(&Fixed::List(vec![])));
        let layers = farm.field("layers").and_then(Fixed::as_list).unwrap();
        assert_eq!(layers[0].field("name").and_then(Fixed::as_text), Some("Ground"));
        // A field left out keeps its class's starting value, so it isn't in the file's value.
        assert_eq!(layers[1].field("rows"), None);
        assert_eq!(v.values[1].ty, "TileMap");
        assert_eq!(v.values[1].line, 4);
        // Windows line ends read the same.
        let v = read("farm.lst", &text.replace('\n', "\r\n")).unwrap();
        assert_eq!(v.values.len(), 2);
    }

    #[test]
    fn anything_that_would_run_is_a_mistake_with_its_line() {
        let text = "class Thing {\n    has {\n        int a = 1;\n    }\n}\n\
            int loose = 3;\n\
            lock int sum = 1 + 2;\n\
            lock int called = make();\n\
            lock string said = \"hp {loose}\";\n\
            print(\"hi\");\n\
            lock float roll = Random[1, 5];\n\
            lock int other = -loose;\n\
            end\n";
        let problems = read("bad.lst", text).unwrap_err();
        let lines: Vec<u32> = problems.iter().map(|p| p.line).collect();
        assert_eq!(lines, [1, 6, 7, 8, 9, 10, 11, 12], "{problems:?}");
        assert!(problems[1].message.contains("locked"), "{}", problems[1]);
        assert_eq!(problems[2].file, "bad.lst");
        // A mistake deep inside a value says where it is.
        let deep = "fetch \"tiles\":[TileMap, Layer];\nlock TileMap m = new TileMap {\n    layers = [\n        new Layer { name = 1 + 1 },\n    ],\n};\nend\n";
        let problems = read("deep.lst", deep).unwrap_err();
        assert_eq!(problems.iter().map(|p| p.line).collect::<Vec<_>>(), [4], "{problems:?}");
        // A file Lust can't read at all says so as it would anywhere.
        let problems = read("broken.lst", "lock int a = ;\n").unwrap_err();
        assert!(!problems.is_empty());
        // So does one that isn't there.
        assert!(read_values("gone.lst", &[]).is_err());
    }

    #[test]
    fn a_data_file_is_checked_like_any_file() {
        let wrong = |body: &str| {
            let text = format!("fetch \"tiles\":[TileMap, Tileset];\n{body}\nend\n");
            let problems = read("check.lst", &text).unwrap_err();
            assert_eq!(problems[0].line, 2, "{body}: {problems:?}");
            problems[0].message.clone()
        };
        // A value that isn't its type.
        wrong("lock int n = \"hi\";");
        // A class Lust doesn't know, and a field the class doesn't have.
        wrong("lock Nope n = new Nope { };");
        wrong("lock TileMap m = new TileMap { colour = 3 };");
        // A field of the wrong type.
        wrong("lock TileMap m = new TileMap { scale = \"big\" };");
        // An option the state doesn't have.
        wrong("lock Tileset t = Tileset.Moon;");
        // A dotted name that isn't an option is a value read from something, so it's refused.
        let text = "fetch \"tiles\":[TileMap];\nlock TileMap farm = new TileMap { };\nlock float s = farm.scale;\nend\n";
        let problems = read("dots.lst", text).unwrap_err();
        assert_eq!(problems.iter().map(|p| p.line).collect::<Vec<_>>(), [3], "{problems:?}");
        assert!(problems[0].message.contains("fixed"), "{}", problems[0]);
        // The same for a field that holds a state: it reads from something, so it isn't fixed.
        let text = "fetch \"tiles\":[TileMap, Tileset];\nlock TileMap farm = new TileMap { };\nlock Tileset t = farm.tileset;\nend\n";
        let problems = read("dots.lst", text).unwrap_err();
        assert_eq!(problems.iter().map(|p| p.line).collect::<Vec<_>>(), [3], "{problems:?}");
        // Options inside options are fixed.
        let text = "fetch \"clips\":Clip;\nlock Clip c = Clip.Farmer.Walk;\nend\n";
        let files = [
            ("c.lst".to_string(), text.to_string()),
            (
                "clips.lst".to_string(),
                "state Clip = [Farmer [Idle, Walk], Cow];\npass Clip;\nend\n".to_string(),
            ),
        ];
        let v = read_values("c.lst", &files).unwrap();
        assert_eq!(v.get("c"), Some(&Fixed::Option("Clip.Farmer.Walk".into())));
    }

    /// A map `side` cells wide and high, in chunks of 32 × 32 rows of one-character codes, as
    /// Solis writes them.
    fn map(side: usize) -> String {
        let row = |x: usize, y: usize| -> String { (0..32).map(|i| if (x + y + i) % 7 == 0 { 't' } else { '.' }).collect() };
        let n = side / 32;
        let mut text = format!("fetch \"tiles\":[TileMap, Chunk];\nlock int version = 2;\nlock TileMap farm = new TileMap {{\n    size = [{side}, {side}],\n    chunks = [\n");
        for cy in 0..n {
            for cx in 0..n {
                let rows: Vec<String> = (0..32).map(|y| format!("\"{}\"", row(cx, cy * 32 + y))).collect();
                text.push_str(&format!(
                    "        new Chunk {{ at = [{cx}, {cy}], rows = [{}] }},\n",
                    rows.join(", ")
                ));
            }
        }
        text.push_str("    ],\n};\nend\n");
        text
    }

    fn read_map(side: usize) -> (usize, std::time::Duration) {
        let text = map(side);
        let started = std::time::Instant::now();
        let v = read("big.lst", &text).unwrap();
        let took = started.elapsed();
        let chunks = v.get("farm").and_then(|f| f.field("chunks")).and_then(Fixed::as_list).unwrap();
        let cells: usize = chunks
            .iter()
            .map(|c| {
                c.field("rows")
                    .and_then(Fixed::as_list)
                    .unwrap()
                    .iter()
                    .map(|r| r.as_text().unwrap().len())
                    .sum::<usize>()
            })
            .sum();
        assert_eq!(cells, side * side);
        eprintln!("read a {} KB map in {took:?}", text.len() / 1024);
        (text.len(), took)
    }

    #[test]
    fn a_thousand_by_thousand_map_reads_quickly() {
        let (_, took) = read_map(1024);
        // Under a second in a release build, as asked; debug builds are far slower.
        if !cfg!(debug_assertions) {
            assert!(took.as_secs_f64() < 1.0, "{took:?}");
        }
    }

    /// Tens of megabytes: a 4,096 × 4,096 map, about 19 MB, in about 0.7 s on a laptop. It takes a
    /// while to build, so it's run by hand: `cargo test --release -- --ignored big_files`.
    #[test]
    #[ignore]
    fn big_files_of_tens_of_megabytes_read_in_under_a_second() {
        let (bytes, took) = read_map(4096);
        assert!(bytes > 16 << 20, "{bytes}");
        assert!(took.as_secs_f64() < 1.0, "{took:?}");
    }
}
