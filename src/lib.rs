//! Lust's compiler, for programs that use it: an engine checking a game's Lust and turning it
//! into Lua while the game runs, or into Rust for a release.
//!
//! ```no_run
//! // The engine reads the files itself, and hands over each one's path and text.
//! let files = vec![
//!     ("main.lst".to_string(), std::fs::read_to_string("game/main.lst").unwrap()),
//!     ("parts/gun.lst".to_string(), std::fs::read_to_string("game/parts/gun.lst").unwrap()),
//! ];
//! match lust::to_lua("main.lst", &files) {
//!     // Load chunk.code into Lua (or Luau) with the chunk name lust::LuaChunk::NAME.
//!     Ok(chunk) => {}
//!     // Every mistake, each with its file, line and column.
//!     Err(problems) => {
//!         for p in &problems {
//!             eprintln!("{p}"); // parts/gun.lst:12:5  ammo is an int, but this is text.
//!         }
//!     }
//! }
//! ```
//!
//! With `default-features = false` it needs nothing but Rust's own library. The `cli` feature,
//! on by default, is the lust command itself.

mod ast;
mod check;
mod diag;
mod lexer;
mod lua;
mod parser;
mod project;
mod rust;
mod stdlib;
mod types;
mod values;
mod words;
/// The built-in icons: a subset of Lucide, drawn by views and checked by name before running.
#[allow(dead_code)]
mod icons {
    include!("icons.rs");
    /// An icon's shapes, by its name or another name for it.
    pub fn find(name: &str) -> Option<&'static str> {
        let name = ICON_ALIASES.iter().find(|(a, _)| *a == name).map(|(_, to)| *to).unwrap_or(name);
        ICONS.iter().find(|(n, _)| *n == name).map(|(_, d)| *d)
    }
}

// The runtime that goes into built programs. lust uses its file functions too, so
// LustScript run by lust says exactly the same things.
#[cfg(feature = "cli")]
#[allow(dead_code, non_snake_case)]
mod native_rt {
    // lust itself runs LustScript views on its own main thread.
    pub const UI_PROGRAM: bool = false;
    include!("native_rt.rs");
    include!("native_localtime.rs");
}
// Databases, for built programs that use them, and for LustScript run by lust.
#[cfg(feature = "cli")]
#[allow(dead_code)]
mod native_db {
    pub use crate::native_rt::*;
    use std::rc::Rc;
    include!("native_db.rs");
}
// Serving routes, for built programs that call Web:serve, and for LustScript run by lust.
#[cfg(feature = "cli")]
#[allow(dead_code)]
mod native_web {
    pub use crate::native_rt::*;
    include!("native_web.rs");
}
// Asking other websites, and text patterns: for built programs that use them, and for LustScript.
#[cfg(feature = "cli")]
#[allow(dead_code)]
mod native_net {
    pub use crate::native_rt::*;
    include!("native_net.rs");
}
#[cfg(feature = "cli")]
#[allow(dead_code)]
mod native_text {
    pub use crate::native_rt::*;
    include!("native_text.rs");
}
// The computer's own boxes for choosing files.
#[cfg(feature = "cli")]
#[allow(dead_code)]
mod native_dlg {
    include!("native_dlg.rs");
}
// Screens: views drawn by Lust, for built programs with a view and for LustScript run by lust.
#[cfg(feature = "cli")]
#[allow(dead_code)]
mod native_ui {
    use crate::icons::{ICONS, ICON_ALIASES};
    include!("native_ui.rs");
    include!("native_ui_pieces.rs");
    include!("native_ui_table.rs");
}
#[cfg(feature = "cli")]
mod run;
#[cfg(feature = "cli")]
mod tests;

#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "cli")]
mod update;
#[cfg(feature = "cli")]
pub(crate) use cli::{build_host, build_native, compile};

/// The lust command: lust run, lust build, lust test and the rest.
#[cfg(feature = "cli")]
#[doc(hidden)]
pub fn cli_main() -> std::process::ExitCode {
    cli::main()
}

use std::path::Path;

/// This compiler's version. Lua and Rust it wrote work with the same version's runtime only.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A mistake found before anything runs: where it is, and what's wrong, in plain words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The file, as the program's first file sees it: `main.lst`, `parts/gun.lst`.
    pub file: String,
    /// From 1. 0 when the mistake is about the file as a whole, like a file that isn't there.
    pub line: u32,
    /// From 1. 0 with `line`.
    pub column: u32,
    pub message: String,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}:{}:{}  {}", self.file, self.line, self.column, self.message)
        }
    }
}

impl std::error::Error for Problem {}

/// A program read and checked, with no mistakes.
pub(crate) struct Checked {
    pub prog: ast::Program,
    pub info: check::Info,
    pub files: Vec<String>,
    pub is_script: bool,
    /// Notes: not mistakes (me, which is now this).
    pub notes: Vec<Problem>,
}

/// Reads a .lst or .lsc file from the disk, and every file it brings in, and checks them.
#[cfg(feature = "cli")]
pub(crate) fn front(path: &Path) -> Result<Checked, Vec<Problem>> {
    let display = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    let is_script = kind_of(path, &display)?;
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s.replace("\r\n", "\n"),
        Err(_) => return Err(whole(&display, format!("there's no file called {}.", path.display()))),
    };
    checked(path, src, is_script, None, &[])
}

/// The same, with every file's text handed over by its path: Lust doesn't open any files.
fn front_from(entry: &str, files: &[(String, String)], engine: &Engine) -> Result<Checked, Vec<Problem>> {
    let mem: project::Memory = files
        .iter()
        .map(|(p, text)| (project::tidy(Path::new(&p.replace('\\', "/"))), text.clone()))
        .collect();
    let path = project::tidy(Path::new(&entry.replace('\\', "/")));
    let is_script = kind_of(&path, entry)?;
    let Some(src) = mem.get(&path).cloned() else {
        return Err(whole(entry, format!("there's no file called {entry} among the files handed over.")));
    };
    checked(&path, src.replace("\r\n", "\n"), is_script, Some(&mem), &engine.folders)
}

fn whole(file: &str, message: String) -> Vec<Problem> {
    vec![Problem {
        file: file.to_string(),
        line: 0,
        column: 0,
        message,
    }]
}

/// Whether a file is LustScript (.lsc) or Lust (.lst).
fn kind_of(path: &Path, display: &str) -> Result<bool, Vec<Problem>> {
    let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    let name = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    if ext == "lust" {
        return Err(whole(
            display,
            format!(
                "Lust programs now end in .lst. Rename {name} to {}.lst and try again.",
                path.file_stem().unwrap_or_default().to_string_lossy()
            ),
        ));
    }
    if ext != "lst" && ext != "lsc" {
        return Err(whole(
            display,
            format!("{name} isn't a Lust file. Lust files end in .lst, or .lsc for LustScript."),
        ));
    }
    Ok(ext == "lsc")
}

fn checked(path: &Path, src: String, is_script: bool, mem: Option<&project::Memory>, host: &[String]) -> Result<Checked, Vec<Problem>> {
    let mut diags = diag::Diags::default();
    let project = project::load(path, src, is_script, &mut diags, mem, host);
    let prog = project.program;
    let files = project.files;
    let info = check::check(&prog, &mut diags, is_script);
    if !diags.is_empty() {
        return Err(diags
            .sorted()
            .into_iter()
            .map(|(pos, message)| Problem {
                file: files.get(pos.file()).cloned().unwrap_or_default(),
                line: pos.row(),
                column: pos.col,
                message,
            })
            .collect());
    }
    let notes = {
        let mut n = diags.notes.clone();
        n.sort_by_key(|(p, _)| (p.line, p.col));
        n.into_iter()
            .map(|(pos, message)| Problem {
                file: files.get(pos.file()).cloned().unwrap_or_default(),
                line: pos.row(),
                column: pos.col,
                message,
            })
            .collect()
    };
    Ok(Checked {
        prog,
        info,
        files,
        is_script,
        notes,
    })
}

/// How much stack the compiler gets: it runs on a thread of its own, so the program using it
/// needs none to spare. Code nested as deeply as Lust allows needs well under this, even in a
/// debug build; the room is only reserved, not used, until it's needed.
const STACK: usize = 64 << 20;

/// Runs the compiler on its own thread with STACK room, or here if a thread can't be made.
fn on_own_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    let mut f = Some(f);
    let result = std::thread::scope(|s| {
        let run = &mut f;
        std::thread::Builder::new()
            .name("lust compiler".into())
            .stack_size(STACK)
            .spawn_scoped(s, move || run.take().map(|f| f()))
            .ok()
            .map(|h| h.join().expect("Lust's compiler stopped unexpectedly"))
    });
    match result {
        Some(Some(r)) => r,
        _ => (f.take().expect("the compiler runs once"))(),
    }
}

pub use values::{read_values, read_values_with, Fixed, Named, Values};

/// What Lust knows of the engine it's inside.
#[derive(Debug, Clone, Default)]
pub struct Engine {
    /// The folders holding the engine's own files, from the project's folder (`"solis"`). Those
    /// files are part of every program, and only they may give `entity` methods
    /// (`method vec3 entity:position() native;`).
    pub folders: Vec<String>,
}

/// Checks a program: `entry` is its first file's path, and `files` every file's path and text,
/// with / between folders (\\ works too). Gives back every mistake found, each with its file,
/// line and column, in order.
pub fn check(entry: &str, files: &[(String, String)]) -> Result<(), Vec<Problem>> {
    check_with(&Engine::default(), entry, files)
}

/// [`check`], inside an engine: its own folders' files come with every program.
pub fn check_with(engine: &Engine, entry: &str, files: &[(String, String)]) -> Result<(), Vec<Problem>> {
    on_own_stack(|| front_from(entry, files, engine).map(|_| ()))
}

/// Lua for an engine to load (Lua 5.1, LuaJIT or Luau): Lust's runtime, the program, and at the
/// end a table the engine uses to make, run and swap components. See LUST.md, section 21.
pub fn to_lua(entry: &str, files: &[(String, String)]) -> Result<LuaChunk, Vec<Problem>> {
    to_lua_with(&Engine::default(), entry, files)
}

/// [`to_lua`], inside an engine: its own folders' files come with every program.
pub fn to_lua_with(engine: &Engine, entry: &str, files: &[(String, String)]) -> Result<LuaChunk, Vec<Problem>> {
    on_own_stack(|| lua_chunk(entry, files, engine))
}

fn lua_chunk(entry: &str, files: &[(String, String)], engine: &Engine) -> Result<LuaChunk, Vec<Problem>> {
    let c = front_from(entry, files, engine)?;
    engine_objects(&c)?;
    let out = lua::write_for_engine(&c.prog, &c.info, !c.is_script);
    Ok(LuaChunk {
        code: out.code,
        files: c.files,
        lines: out.lines,
        cols: out.cols,
        notes: c.notes,
    })
}

/// A Rust library made from a .lst program: what `lust build --library` puts in its src/lib.rs.
/// Its Cargo.toml needs no dependencies, unless the Lust uses databases, the web or windows.
pub fn to_rust_library(entry: &str, files: &[(String, String)]) -> Result<String, Vec<Problem>> {
    to_rust_library_with(&Engine::default(), entry, files)
}

/// [`to_rust_library`], inside an engine: its own folders' files come with every program.
pub fn to_rust_library_with(engine: &Engine, entry: &str, files: &[(String, String)]) -> Result<String, Vec<Problem>> {
    on_own_stack(|| rust_library(entry, files, engine))
}

fn rust_library(entry: &str, files: &[(String, String)], engine: &Engine) -> Result<String, Vec<Problem>> {
    let c = front_from(entry, files, engine)?;
    if c.is_script {
        return Err(whole(
            entry,
            "LustScript turns into Lua. Only .lst files can be built as Rust libraries.".into(),
        ));
    }
    engine_objects(&c)?;
    Ok(rust::write(&c.prog, &c.info, &c.files, true))
}

/// For a program an engine runs: an object it passes, with an `update` the engine can't call,
/// is a mistake, as it is for a component (the engine would never tick it, and say nothing).
fn engine_objects(c: &Checked) -> Result<(), Vec<Problem>> {
    let mut problems = Vec::new();
    for item in &c.prog.items {
        let ast::Item::Class(d) = item else { continue };
        if !d.is_object || d.is_node || !c.info.shared.contains(&d.name) {
            continue;
        }
        let Some(m) = d.methods.iter().find(|m| m.name == "update") else {
            continue;
        };
        let Some(&ci) = c.info.class_index.get(&d.name) else { continue };
        let sig = &c.info.classes[ci].methods["update"];
        if sig.ret != types::Ty::Void || sig.params.len() != 1 || sig.params[0].1 != types::Ty::Float {
            problems.push(Problem {
                file: c.files.get(m.pos.file()).cloned().unwrap_or_default(),
                line: m.pos.row(),
                column: m.pos.col,
                message: format!(
                    "{} is passed, so the engine runs its update once a tick, with how many seconds a tick is: method update(float dt) {{ … }}",
                    ast::shown(&d.name)
                ),
            });
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}

/// Lua code an engine loads, and what it needs to say where a failure came from.
#[derive(Debug, Clone)]
pub struct LuaChunk {
    pub code: String,
    /// The program's files, by number, as people see them.
    pub files: Vec<String>,
    /// For each line of `code` (from 1), the Lust line it came from: file number in the top
    /// bits, line in the low 20. 0 for the runtime.
    lines: Vec<u32>,
    /// The column of the statement each line came from, or 0.
    cols: Vec<u32>,
    /// Notes on the code: not mistakes, so it runs, but worth showing (`me`, which is now
    /// `this`), each with its file, line and column.
    pub notes: Vec<Problem>,
}

impl LuaChunk {
    /// The name to load the chunk with (mlua: `.set_name(LuaChunk::NAME)`), so a failure's
    /// message says where in the chunk it came from, and `locate` can find it.
    pub const NAME: &'static str = "=lust";

    /// Where a Lua failure came from in the Lust, from its message (an mlua error's text, for
    /// one): `lust:812: ammo can't be below 0` becomes main.lst, line 14, and the message.
    pub fn locate(&self, lua_error: &str) -> RunError {
        let mut line = None;
        let mut column = 0;
        let mut message = lua_error.lines().next().unwrap_or("").trim().to_string();
        if let Some(i) = message.find("lust:") {
            let rest = &message[i + 5..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let (Ok(n), Some(after)) = (digits.parse::<usize>(), rest[digits.len()..].strip_prefix(": ")) {
                let packed = self.lines.get(n.saturating_sub(1)).copied().unwrap_or(0);
                if packed > 0 {
                    line = Some(packed);
                    column = self.cols.get(n.saturating_sub(1)).copied().unwrap_or(0);
                }
                message = after.to_string();
            }
        }
        let message = ast::unmark(&lua::friendly(&message));
        match line {
            Some(p) => {
                let (f, row) = ast::unpack(p);
                RunError {
                    file: self.files.get(f).cloned().unwrap_or_default(),
                    line: Some(row),
                    column: (column > 0).then_some(column),
                    message,
                }
            }
            None => RunError {
                file: self.files.first().cloned().unwrap_or_default(),
                line: None,
                column: None,
                message,
            },
        }
    }
}

/// A failure while Lust code runs: the file and line it stopped at, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunError {
    pub file: String,
    /// None when Lust can't tell the line, like a failure inside the engine's own Lua.
    pub line: Option<u32>,
    /// From 1: the column of the statement that failed (for `int each = 12 / ways;`, where
    /// `each` is). None with `line`.
    pub column: Option<u32>,
    pub message: String,
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.line {
            Some(l) => write!(f, "{}:{l}  {}", self.file, self.message),
            None => write!(f, "{}  {}", self.file, self.message),
        }
    }
}

impl std::error::Error for RunError {}

#[cfg(test)]
mod deep_code {
    //! Code nested very deeply is a mistake, not a crash: the compiler must never run out of stack.

    /// Checks one file from a thread with a tiny stack (128 KB): the compiler makes its own room,
    /// so the program using it needs none. Gives back the messages.
    fn messages(src: String) -> Vec<String> {
        std::thread::Builder::new()
            .stack_size(128 << 10)
            .spawn(move || {
                let files = vec![("main.lst".to_string(), src)];
                let mut out = Vec::new();
                match crate::check("main.lst", &files) {
                    Ok(()) => {}
                    Err(problems) => out.extend(problems.into_iter().map(|p| p.message)),
                }
                // The outputs walk the same trees, so they're tried too when there's nothing wrong.
                if out.is_empty() {
                    crate::to_lua("main.lst", &files).unwrap();
                    crate::to_rust_library("main.lst", &files).unwrap();
                }
                out
            })
            .unwrap()
            .join()
            .expect("the compiler ran out of stack")
    }

    fn too_deep(src: String) {
        let m = messages(src);
        assert!(m.iter().any(|m| m.contains("nested too deeply")), "{m:?}");
    }

    fn fine(src: String) {
        let m = messages(src);
        assert!(m.is_empty(), "{m:?}");
    }

    fn method(body: &str) -> String {
        format!("method int f(int a) {{\n{body}\n}}\nend\n")
    }

    #[test]
    fn an_engine_s_folders_give_entity_methods() {
        let files = vec![
            (
                "main.lst".to_string(),
                "entity e = nothing;\nvec3 at = e:position();\nend\n".to_string(),
            ),
            (
                "solis/entity.lst".to_string(),
                "method vec3 entity:position() native;\nend\n".to_string(),
            ),
        ];
        let engine = crate::Engine {
            folders: vec!["solis".into()],
        };
        crate::check_with(&engine, "main.lst", &files).unwrap();
        let chunk = crate::to_lua_with(&engine, "main.lst", &files).unwrap();
        assert!(chunk.code.contains("entity_position"));
        let rust = crate::to_rust_library_with(&engine, "main.lst", &files).unwrap();
        assert!(rust.contains("fn entity_position(&self, this: Option<u64>) -> Vec3"));
        // Without the engine's folders, the file isn't the engine's: it's not even part of the
        // program, and its declaration is refused when it is.
        let problems = crate::check("main.lst", &files).unwrap_err();
        assert!(problems[0].message.contains("no method called position"), "{}", problems[0]);
        let problems = crate::check("solis/entity.lst", &files).unwrap_err();
        assert!(problems[0].message.contains("only the engine's own files"), "{}", problems[0]);
    }

    #[test]
    fn an_engine_s_components_join_only_when_fetched() {
        let kit = "component Walker {\n    has { float speed = 4; }\n    method update(float dt) { }\n}\n\
            object Weather {\n    has { float rain = 0; }\n    method update(float dt) { }\n}\npass Walker, Weather;\nend\n";
        let mut files = vec![
            (
                "main.lst".to_string(),
                "component Walker {\n    has { int steps = 0; }\n}\nend\n".to_string(),
            ),
            (
                "solis/entity.lst".to_string(),
                "method vec3 entity:position() native;\nend\n".to_string(),
            ),
            ("solis/kit/walker.lst".to_string(), kit.to_string()),
        ];
        let engine = crate::Engine {
            folders: vec!["solis".into()],
        };
        // Not fetched: the game's own Walker is the only one, with no clash.
        crate::check_with(&engine, "main.lst", &files).unwrap();
        let chunk = crate::to_lua_with(&engine, "main.lst", &files).unwrap();
        assert!(chunk.code.contains("steps"), "{}", chunk.code);
        assert!(!chunk.code.contains("speed"), "{}", chunk.code);
        assert!(!chunk.code.contains("Weather"), "{}", chunk.code);
        let rust = crate::to_rust_library_with(&engine, "main.lst", &files).unwrap();
        assert!(!rust.contains("Weather"));
        // Fetched: the engine's Walker and Weather are the game's.
        files[0].1 = "fetch \"solis/kit/walker\":[Walker, Weather];\nend\n".to_string();
        let chunk = crate::to_lua_with(&engine, "main.lst", &files).unwrap();
        assert!(chunk.code.contains("speed"), "{}", chunk.code);
        assert!(chunk.code.contains("[\"Weather\"] = true"), "{}", chunk.code);
    }

    #[test]
    fn an_engine_ticks_and_saves_only_the_objects_files_pass() {
        let game = "object Clock {\n    has { float hour = 0; }\n    method update(float dt) { hour += dt; }\n}\n\
            object Notes {\n    has { int seen = 0; }\n    method update(float dt) { seen += 1; }\n}\npass Clock;\nend\n";
        let files = vec![("main.lst".to_string(), game.to_string())];
        let chunk = crate::to_lua("main.lst", &files).unwrap();
        assert!(chunk.code.contains("[\"Clock\"] = true"), "{}", chunk.code);
        assert!(!chunk.code.contains("[\"Notes\"]"), "{}", chunk.code);
        let rust = crate::to_rust_library("main.lst", &files).unwrap();
        assert!(rust.contains("Clock::get().update(dt)"));
        assert!(!rust.contains("Notes::get().update(dt)"));
        // A passed object's update the engine can't call is a mistake, as for a component.
        for wrong in [
            "method update(int dt) { }",
            "method int update(float dt) { return 1; }",
            "method update() { }",
        ] {
            let text = format!("object Clock {{\n    has {{ float hour = 0; }}\n    {wrong}\n}}\npass Clock;\nend\n");
            let files = vec![("main.lst".to_string(), text)];
            for problems in [
                crate::to_lua("main.lst", &files).unwrap_err(),
                crate::to_rust_library("main.lst", &files).unwrap_err(),
            ] {
                assert_eq!(problems[0].line, 3, "{wrong}");
                assert!(problems[0].message.contains("method update(float dt)"), "{}", problems[0]);
            }
            // A program that isn't an engine's may have an update of its own.
            crate::check("main.lst", &files).unwrap();
        }
    }

    #[test]
    fn brackets() {
        too_deep(method(&format!("return {}1{};", "(".repeat(100_000), ")".repeat(100_000))));
        fine(method(&format!("return {}1{};", "(".repeat(140), ")".repeat(140))));
    }

    #[test]
    fn operators_in_a_row() {
        too_deep(method(&format!("return {};", vec!["a"; 100_000].join(" + "))));
        too_deep(method(&format!("return {};", vec!["a"; 100_000].join(" * "))));
        too_deep(method(&format!("bool b = {}true;\nreturn a;", "not ".repeat(100_000))));
        too_deep(method(&format!("return {}a;", "- ".repeat(100_000))));
        too_deep(method(&format!("bool b = {};\nreturn a;", vec!["true"; 100_000].join(" && "))));
        too_deep(method(&format!("bool b = {};\nreturn a;", vec!["true"; 100_000].join(" || "))));
        fine(method(&format!("return {};", vec!["a"; 140].join(" + "))));
        fine(method(&format!("bool b = {}true;\nreturn a;", "not ".repeat(70))));
        // The bitwise operators, each level of them, and ~.
        for op in [" | ", " ^ ", " & ", " << ", " >> "] {
            too_deep(method(&format!("return {};", vec!["a"; 100_000].join(op))));
            fine(method(&format!("return {};", vec!["a"; 140].join(op))));
        }
        too_deep(method(&format!("return {}a;", "~ ".repeat(100_000))));
        fine(method(&format!("return {}a;", "~ ".repeat(40))));
        fine(method(&format!("return {}a;", "a | a ^ a & a << a + ".repeat(25))));
    }

    #[test]
    fn blocks_and_lists() {
        let ifs = |n: usize| {
            method(&format!(
                "{}a = a + 1;{}\nreturn a;",
                "is a > 0? then {\n".repeat(n),
                "\n}".repeat(n)
            ))
        };
        too_deep(ifs(50_000));
        fine(ifs(140));
        too_deep(format!("int array x = {}{};\nend\n", "[".repeat(100_000), "]".repeat(100_000)));
        too_deep(format!("int array array a = [];\nint x = a{};\nend\n", "[0]".repeat(100_000)));
        too_deep(format!("{} x = [];\nend\n", format!("int{}", " array".repeat(100_000))));
    }
}

#[cfg(test)]
mod no_unsafe {
    //! A library for an engine has no unsafe code, and says so to Rust, which checks it.

    fn library(src: &str) -> String {
        crate::to_rust_library("game.lst", &[("game.lst".to_string(), src.to_string())]).unwrap()
    }

    fn safe(rust: &str) {
        assert!(rust.contains("#![forbid(unsafe_code)]"));
        assert!(!rust.contains("unsafe {"), "unsafe code in the library");
        assert!(!rust.contains("extern \"C\""), "C functions in the library");
    }

    #[test]
    fn plain_library() {
        safe(&library("method int twice(int n) {\n    return n * 2;\n}\npass twice;\nend\n"));
    }

    #[test]
    fn library_with_local_time() {
        let rust = library("method string today() {\n    return Time:date() + \" \" + Time:time();\n}\npass today;\nend\n");
        safe(&rust);
        assert!(rust.contains("fn local_time_offset(&self, unix_seconds: i64) -> i64;"));
    }

    #[test]
    fn components_and_events() {
        safe(&library(
            "event Hit(float d);\ncomponent Health {\n    has {\n        float hp = 1;\n    }\n    on Hit(float d) {\n        hp = hp - d;\n        send Hit(d) to me;\n    }\n}\nend\n",
        ));
    }
}

#[cfg(all(test, feature = "cli"))]
mod maths_helpers {
    //! The runtime's number helpers and random tools, tried straight from Rust.
    use crate::native_rt as rt;

    #[test]
    fn noise_matches_ken_perlins_own() {
        // Ken Perlin's reference Java (ImprovedNoise) gives this for (3.14, 42, 7).
        assert_eq!(rt::maths_noise3(3.14, 42.0, 7.0), 0.13691995878400012);
        // Whole numbers sit where the noise is always 0.
        assert_eq!(rt::maths_noise3(1.0, 2.0, 3.0), 0.0);
        // The same everywhere it's asked, and inside -1 to 1, even far out and below zero.
        for i in 0..5000 {
            let (x, y, z) = (i as f64 * 0.731 - 1800.0, i as f64 * -0.377, i as f64 * 0.019);
            let n = rt::maths_noise3(x, y, z);
            assert!((-1.0..=1.0).contains(&n));
            assert_eq!(n, rt::maths_noise3(x, y, z));
        }
        assert_eq!(rt::maths_noise(1.3, 4.7), rt::maths_noise3(1.3, 4.7, 0.0));
    }

    #[test]
    fn numbers() {
        assert_eq!(rt::maths_fraction(-2.75), -0.75);
        assert!(rt::maths_fraction(-2.0).is_sign_positive());
        assert_eq!(rt::maths_trunc(-2.75), -2);
        assert_eq!(rt::maths_sign_int(-9), -1);
        assert_eq!(rt::maths_sign_float(-0.0).to_bits(), 0.0f64.to_bits());
        assert!(rt::maths_sign_float(f64::NAN).is_nan());
        assert_eq!(rt::maths_moveTowards(0.0, 1.0, 5.0), 1.0);
        assert_eq!(rt::maths_moveTowards(0.0, -1.0, 0.25), -0.25);
        assert_eq!(rt::maths_smoothstep(0.0, 1.0, 0.5), 0.5);
        assert!(!rt::maths_isFinite(f64::NAN) && !rt::maths_isFinite(f64::INFINITY) && rt::maths_isFinite(0.0));
        assert_eq!(rt::maths_log(rt::maths_e()), 1.0);
    }

    #[test]
    fn seeded_rolls_repeat() {
        rt::random_seed(42);
        let a: Vec<i64> = (0..20).map(|_| rt::random(1, 100)).collect();
        let d = rt::random_direction();
        rt::random_seed(42);
        let b: Vec<i64> = (0..20).map(|_| rt::random(1, 100)).collect();
        assert_eq!(a, b);
        assert_eq!(rt::random_direction(), d);
        let len = ((d.x as f64).powi(2) + (d.y as f64).powi(2) + (d.z as f64).powi(2)).sqrt();
        assert!((len - 1.0).abs() < 1e-6);
    }

    #[test]
    fn shuffle_keeps_every_item() {
        rt::random_seed(7);
        for n in 0..40 {
            let a = rt::arr((0..n).collect::<Vec<i64>>());
            rt::array_shuffle(a.clone());
            let mut v = a.borrow().clone();
            v.sort();
            assert_eq!(v, (0..n).collect::<Vec<i64>>());
        }
    }
}

#[cfg(all(test, feature = "cli"))]
mod rotation_helpers {
    //! Rotation:toward, Rotation:around, rotation lerp and vec3 angle, tried straight from Rust.
    use crate::native_rt as rt;

    fn close(a: rt::Vec3, b: (f32, f32, f32)) -> bool {
        (a.x - b.0).abs() < 1e-6 && (a.y - b.1).abs() < 1e-6 && (a.z - b.2).abs() < 1e-6
    }

    #[test]
    fn toward_faces_the_direction() {
        let x = rt::vec3(1.0, 0.0, 0.0);
        for d in [
            (1.0, 2.0, 3.0),
            (-4.0, 0.5, -1.0),
            (0.0, 0.0, 1.0),
            (0.0, 0.0, -7.0),
            (0.0, -1.0, 0.0),
        ] {
            let r = rt::rotation_toward(rt::vec3(d.0, d.1, d.2));
            let l = (d.0 * d.0 + d.1 * d.1 + d.2 * d.2).sqrt();
            let f = rt::qmulv(r, x);
            assert!(close(f, ((d.0 / l) as f32, (d.1 / l) as f32, (d.2 / l) as f32)), "{d:?} {f:?}");
            // No roll: the turn's own right side (its y) stays level.
            assert!(rt::qmulv(r, rt::vec3(0.0, 1.0, 0.0)).z.abs() < 1e-6);
        }
        // Straight up turns no way round z.
        assert_eq!(rt::qdegrees(rt::rotation_toward(rt::vec3(0.0, 0.0, 1.0))).z, 0.0);
    }

    #[test]
    fn around_and_lerp() {
        let q = rt::rotation_around(rt::vec3(0.0, 0.0, 2.0), 90.0);
        assert_eq!(q, rt::rot(0.0, 0.0, 90.0));
        let a = rt::rot(0.0, 0.0, 0.0);
        assert_eq!(rt::qlerp(a, q, 0.0), a);
        assert_eq!(rt::qlerp(a, q, 1.0), q);
        assert!(close(rt::qdegrees(rt::qlerp(a, q, 0.5)), (0.0, 0.0, 45.0)));
        // The same turn written with every number's sign flipped is the shortest way: no turn.
        let neg = rt::Rotation {
            x: -q.x,
            y: -q.y,
            z: -q.z,
            w: -q.w,
        };
        assert!(close(rt::qmulv(rt::qlerp(q, neg, 0.5), rt::vec3(1.0, 0.0, 0.0)), (0.0, 1.0, 0.0)));
    }

    #[test]
    fn angles_in_degrees() {
        let x = rt::vec3(1.0, 0.0, 0.0);
        assert_eq!(rt::vangle(x, rt::vec3(0.0, 2.0, 0.0)), 90.0);
        assert_eq!(rt::vangle(x, rt::vec3(-3.0, 0.0, 0.0)), 180.0);
        assert_eq!(rt::vangle(x, x), 0.0);
    }
}

#[cfg(all(test, feature = "cli"))]
mod vec2_colour {
    //! vec2 and colour maths in runtime.lua (plain Lua's f32 rounding) against the Rust runtime,
    //! bit for bit, over many numbers; and hex text read the same by the checker and both runtimes.
    use crate::native_rt as rt;
    use mlua::{Function, Lua, Table};

    fn runtime(lua: &Lua) -> Table {
        lua.load(crate::lua::RUNTIME).eval().unwrap()
    }

    /// Numbers to try: edges, then a repeatable spread of others, big and small.
    fn numbers() -> Vec<f64> {
        let mut v = vec![
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            0.1,
            1.0 / 3.0,
            90.0,
            180.0,
            -270.0,
            720.0,
            1e-40,
            3e38,
            -3e38,
            255.0,
        ];
        let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
        for _ in 0..300 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let m = (x >> 11) as f64 / (1u64 << 53) as f64;
            let e = (x % 13) as i32 - 6;
            v.push((m - 0.5) * 10f64.powi(e));
        }
        v
    }

    fn v2(t: &Table) -> (f64, f64) {
        (t.get("x").unwrap(), t.get("y").unwrap())
    }
    fn same(a: f64, b: f64) -> bool {
        a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
    }

    #[test]
    fn vec2_maths_matches_rust() {
        let lua = Lua::new();
        let l = runtime(&lua);
        let make: Function = l.get("vec2").unwrap();
        let f = |n: &str| -> Function { l.get(n).unwrap() };
        let (rotated, lerp, cross, length, normalized) = (f("v2rotated"), f("v2lerp"), f("v2cross"), f("v2length"), f("v2normalized"));
        let ns = numbers();
        for (i, &a) in ns.iter().enumerate() {
            let b = ns[(i * 7 + 3) % ns.len()];
            let c = ns[(i * 13 + 5) % ns.len()];
            let lv: Table = make.call((a, b)).unwrap();
            let lw: Table = make.call((c, a)).unwrap();
            let (rv, rw) = (rt::vec2(a, b), rt::vec2(c, a));
            assert_eq!(v2(&lv), (rv.x as f64, rv.y as f64), "vec2({a}, {b})");
            let (x, y) = v2(&rotated.call::<Table>((lv.clone(), c)).unwrap());
            let r = rt::v2rotated(rv, c);
            assert!(same(x, r.x as f64) && same(y, r.y as f64), "[{a}, {b}]:rotated({c})");
            let (x, y) = v2(&lerp.call::<Table>((lv.clone(), lw.clone(), b)).unwrap());
            let r = rt::v2lerp(rv, rw, b);
            assert!(same(x, r.x as f64) && same(y, r.y as f64), "lerp {a} {b} {c}");
            assert!(
                same(cross.call::<f64>((lv.clone(), lw.clone())).unwrap(), rt::v2cross(rv, rw)),
                "cross"
            );
            assert!(same(length.call::<f64>(lv.clone()).unwrap(), rt::v2length(rv)), "length");
            let (x, y) = v2(&normalized.call::<Table>(lv.clone()).unwrap());
            let r = rt::v2normalized(rv);
            assert!(same(x, r.x as f64) && same(y, r.y as f64), "normalized [{a}, {b}]");
        }
    }

    #[test]
    fn quarter_turns_are_exact() {
        let east = rt::vec2(1.0, 0.0);
        for (d, want) in [
            (90.0, (0.0, 1.0)),
            (180.0, (-1.0, 0.0)),
            (270.0, (0.0, -1.0)),
            (-90.0, (0.0, -1.0)),
            (720.0, (1.0, 0.0)),
        ] {
            assert_eq!(rt::v2rotated(east, d), rt::vec2(want.0, want.1), "{d}");
        }
        assert_eq!(rt::v2cross(east, rt::vec2(0.0, 1.0)), 1.0);
        assert_eq!(rt::v2angle(east, rt::vec2(0.0, -3.0)), 90.0);
    }

    #[test]
    fn colours_match_rust() {
        let lua = Lua::new();
        let l = runtime(&lua);
        let (make, lerp, hex): (Function, Function, Function) =
            (l.get("colour").unwrap(), l.get("collerp").unwrap(), l.get("colhex").unwrap());
        let parts = |t: &Table| -> [f64; 4] { ["r", "g", "b", "a"].map(|k| t.get::<f64>(k).unwrap()) };
        let ns = numbers();
        for (i, &a) in ns.iter().enumerate() {
            let (b, c, d) = (
                ns[(i * 3 + 1) % ns.len()],
                ns[(i * 5 + 2) % ns.len()].abs() * 1e3,
                ns[(i * 11 + 4) % ns.len()],
            );
            let lc: Table = make.call((a, b, c, d)).unwrap();
            let rc = rt::colour(a, b, c, d);
            assert_eq!(parts(&lc), rc.to_array().map(|x| x as f64), "colour({a}, {b}, {c}, {d})");
            assert!(rc.to_array().iter().all(|x| (0.0..=1.0).contains(x)));
            assert_eq!(hex.call::<String>(lc.clone()).unwrap(), rt::colhex(rc));
            let lo: Table = make.call((d, c, b, 0.5)).unwrap();
            let ro = rt::colour(d, c, b, 0.5);
            assert_eq!(
                parts(&lerp.call::<Table>((lc, lo, a)).unwrap()),
                rt::collerp(rc, ro, a).to_array().map(|x| x as f64)
            );
        }
        // Not a number becomes 0, and infinity 1.
        assert_eq!(
            rt::colour(f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.0).to_array(),
            [0.0, 1.0, 0.0, 1.0]
        );
    }

    #[test]
    fn hex_text_read_alike() {
        let lua = Lua::new();
        let l = runtime(&lua);
        let read: Function = l.get("colour_hex").unwrap();
        for text in [
            "#3a8f2a",
            "#3A8F2A80",
            "#00000000",
            "#ffffff",
            "3a8f2a",
            "#3a8f2",
            "#3a8f2a8",
            "#3a8f2g",
            "",
            "#",
            "#é3a8f2",
        ] {
            let checked = crate::stdlib::colour_hex(text);
            let lua_says = read.call::<Table>(text);
            match checked {
                Ok(bytes) => {
                    let c = rt::rgb(bytes[0] as f64, bytes[1] as f64, bytes[2] as f64, bytes[3] as f64);
                    let t = lua_says.unwrap();
                    assert_eq!(
                        ["r", "g", "b", "a"].map(|k| t.get::<f64>(k).unwrap()),
                        c.to_array().map(|x| x as f64),
                        "{text}"
                    );
                    assert_eq!(rt::colour_hex(text.to_string()), c, "{text}");
                    // And back to the same text: bytes survive the trip through 0 to 1.
                    assert_eq!(rt::colhex(c), text.to_lowercase(), "{text}");
                }
                Err(msg) => {
                    let e = lua_says.unwrap_err().to_string();
                    assert!(e.contains(&msg), "{text}: {e} / {msg}");
                }
            }
        }
        // Every colour word is exactly its palette bytes.
        for (word, rgb) in crate::words::PALETTE {
            let c = rt::rgb((rgb >> 16) as f64, ((rgb >> 8) & 255) as f64, (rgb & 255) as f64, 255.0);
            assert_eq!(rt::colhex(c), format!("#{rgb:06x}"), "{word}");
        }
    }
}

#[cfg(all(test, feature = "cli"))]
mod bitwise {
    //! lust run's bitwise operators, made from plain arithmetic in runtime.lua, against Rust's own
    //! 64-bit ones, for numbers all over the range lust run holds exactly.
    use crate::native_rt as rt;
    use mlua::{Function, Lua, Table};

    const EXACT: i64 = 9_007_199_254_740_991;

    fn runtime(lua: &Lua) -> Table {
        lua.load(crate::lua::RUNTIME).eval().unwrap()
    }

    /// Numbers to try: the edges, then a spread of others (a fixed sequence, so it's repeatable).
    fn numbers() -> Vec<i64> {
        let mut v = vec![
            0,
            1,
            -1,
            2,
            -2,
            255,
            -256,
            1 << 26,
            (1 << 27) - 1,
            1 << 27,
            -(1 << 27),
            EXACT,
            -EXACT,
            EXACT - 1,
            1 << 52,
            -(1 << 52),
        ];
        let mut x: u64 = 0x2545_f491_4f6c_dd1d;
        for _ in 0..400 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            // Some big, some small: the top bits cut off by a varying amount.
            let n = ((x >> 11) as i64) % (EXACT + 1);
            let n = n >> (x % 50);
            v.push(if x & 1 == 0 { n } else { -n });
        }
        v
    }

    #[test]
    fn and_or_xor_not_match_rust() {
        let lua = Lua::new();
        let l = runtime(&lua);
        let (band, bor, bxor, bnot): (Function, Function, Function, Function) = (
            l.get("band").unwrap(),
            l.get("bor").unwrap(),
            l.get("bxor").unwrap(),
            l.get("bnot").unwrap(),
        );
        let ns = numbers();
        for (i, &a) in ns.iter().enumerate() {
            let want = !a;
            match bnot.call::<f64>(a as f64) {
                Ok(got) => assert_eq!(got, want as f64, "~{a}"),
                Err(e) => assert!(want < -EXACT && e.to_string().contains("too big for lust run"), "~{a}: {e}"),
            }
            for &b in ns.iter().skip(i % 7).step_by(7) {
                for (f, want, op) in [(&band, a & b, "&"), (&bor, a | b, "|"), (&bxor, a ^ b, "^")] {
                    match f.call::<f64>((a as f64, b as f64)) {
                        Ok(got) => assert_eq!(got, want as f64, "{a} {op} {b}"),
                        Err(e) => assert!(
                            want.abs() > EXACT && e.to_string().contains("too big for lust run"),
                            "{a} {op} {b}: {e}"
                        ),
                    }
                }
            }
        }
        // A number lust run can't hold exactly stops, rather than giving a wrong answer.
        let e = band.call::<f64>((2f64.powi(60), 1.0)).unwrap_err().to_string();
        assert!(e.contains("too big for lust run"), "{e}");
    }

    #[test]
    fn shifts_match_rust() {
        let lua = Lua::new();
        let l = runtime(&lua);
        let (shl, shr): (Function, Function) = (l.get("shl").unwrap(), l.get("shr").unwrap());
        for &a in &numbers() {
            for n in 0..64i64 {
                assert_eq!(shr.call::<f64>((a as f64, n as f64)).unwrap(), (a >> n) as f64, "{a} >> {n}");
                let want = (a as i128) << n;
                match shl.call::<f64>((a as f64, n as f64)) {
                    Ok(got) => assert_eq!(got, want as f64, "{a} << {n}"),
                    Err(e) if want.abs() > EXACT as i128 && (i64::MIN as i128..=i64::MAX as i128).contains(&want) => {
                        assert!(e.to_string().contains("too big for lust run"), "{a} << {n}: {e}")
                    }
                    Err(e) => assert!(e.to_string().contains("too big for an int"), "{a} << {n}: {e}"),
                }
            }
        }
        for bad in [-1.0, 64.0, 1000.0] {
            let e = shl.call::<f64>((1.0, bad)).unwrap_err().to_string();
            assert!(e.contains("<< moves bits by 0 to 63 places"), "{e}");
            let e = shr.call::<f64>((1.0, bad)).unwrap_err().to_string();
            assert!(e.contains(">> moves bits by 0 to 63 places"), "{e}");
        }
    }

    fn stops(f: impl FnOnce() -> i64 + std::panic::UnwindSafe) -> bool {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let r = std::panic::catch_unwind(f).is_err();
        std::panic::set_hook(hook);
        r
    }

    #[test]
    fn built_shifts() {
        assert_eq!(rt::shl(1, 62), 1 << 62);
        assert_eq!(rt::shl(-1, 63), i64::MIN);
        assert_eq!(rt::shr(-8, 1), -4);
        assert_eq!(rt::shr(-1, 63), -1);
        assert_eq!(rt::shr(i64::MAX, 63), 0);
        // Bits pushed out of the top stop the program, as * does when it goes past an int.
        assert!(stops(|| rt::shl(1, 63)));
        assert!(stops(|| rt::shl(3, 62)));
        assert!(stops(|| rt::shl(i64::MIN, 1)));
        assert!(stops(|| rt::shl(1, 64)));
        assert!(stops(|| rt::shr(1, -1)));
    }

    #[test]
    fn many_quickly() {
        // Not a bit at a time: 100,000 of each takes well under a second.
        let lua = Lua::new();
        lua.globals().set("L", runtime(&lua)).unwrap();
        let start = std::time::Instant::now();
        lua.load("local x = 0 for i = 1, 100000 do x = L.bxor(L.band(i * 7919, 8191) + L.bor(x, i), i) end return x")
            .exec()
            .unwrap();
        assert!(start.elapsed().as_secs_f64() < 5.0, "{:?}", start.elapsed());
    }
}

#[cfg(all(test, feature = "cli"))]
mod vector_json {
    //! vec3s, vec2s, rotations and colours as JSON: each 32-bit number written as the shortest
    //! text that reads back as the same bits, the same in runtime.lua and the Rust runtime.
    use crate::native_rt as rt;
    use mlua::{Function, Lua, Table};

    /// f32s to try: edges, then a repeatable spread over every bit pattern that's a number.
    fn floats() -> Vec<f32> {
        let mut v = vec![
            1.0,
            -1.0,
            0.1,
            1.0 / 3.0,
            16777216.0,
            16777217.0,
            1e-45,
            f32::MIN_POSITIVE,
            f32::MAX,
            -f32::MAX,
            1e21,
            9.999999e20,
            1e-7,
            9.99e-8,
            0.5,
            123456789.0,
        ];
        let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
        while v.len() < 20000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let f = f32::from_bits(x as u32);
            if f.is_finite() {
                v.push(f);
            }
        }
        v
    }

    #[test]
    fn numbers_are_shortest_exact_and_alike() {
        let lua = Lua::new();
        let l: Table = lua.load(crate::lua::RUNTIME).eval().unwrap();
        let text: Function = l.get("f32json").unwrap();
        for f in floats() {
            let r = rt::f32json(f);
            assert_eq!(text.call::<String>(f as f64).unwrap(), r, "{f:e}");
            // It reads back as the same bits, through an f64, as Json:read does.
            let back = r.parse::<f64>().unwrap() as f32;
            assert_eq!(back.to_bits(), f.to_bits(), "{r}");
            // And no shorter text would: one digit fewer doesn't read back.
            let digits = r.trim_start_matches('-').split('e').next().unwrap().replace('.', "");
            let digits = digits.trim_start_matches('0').trim_end_matches('0').len();
            if digits > 1 {
                let fewer = format!("{:.*e}", digits - 2, f as f64);
                assert_ne!((fewer.parse::<f64>().unwrap() as f32).to_bits(), f.to_bits(), "{r} {fewer}");
            }
        }
        assert_eq!(rt::f32json(0.0), "0.0");
        assert_eq!(rt::f32json(-0.0), "-0.0");
        assert_eq!(rt::f32json(1e-45), "1e-45");
        assert_eq!(rt::f32json(f32::MAX), "3.4028235e38");
        assert_eq!(rt::f32json(0.1), "0.1");
        assert_eq!(rt::f32json(2.0), "2.0");
        assert_eq!(rt::f32json(1e-7), "0.0000001");
        assert_eq!(rt::f32json(1e21), "1e21");
    }

    #[test]
    fn colours_are_hex_only_when_that_is_exact() {
        assert_eq!(rt::jw_colour(&rt::rgb(58.0, 143.0, 42.0, 255.0), None), "\"#3a8f2a\"");
        assert_eq!(rt::jw_colour(&rt::rgb(58.0, 143.0, 42.0, 128.0), None), "\"#3a8f2a80\"");
        assert_eq!(rt::jw_colour(&rt::colour(0.4, 0.7, 1.0, 1.0), None), "[0.4,0.7,1.0,1.0]");
        assert_eq!(rt::jw_colour(&rt::colour(0.4, 0.7, 1.0, 1.0), Some(0)), "[0.4, 0.7, 1.0, 1.0]");
        // Every byte reads back from its hex as itself.
        for b in 0..=255u32 {
            let c = rt::rgb(b as f64, 255.0 - b as f64, 0.0, b as f64);
            let j = rt::json_parse(&rt::jw_colour(&c, None));
            assert_eq!(rt::jr_colour(&j, ""), c, "{b}");
        }
    }

    #[test]
    fn rotations_are_written_as_their_quaternion() {
        for i in 0..500 {
            let f = i as f64;
            let q = rt::rot(f * 7.3, f * 1.9 - 90.0, f * 13.1);
            let back = rt::jr_rotation(&rt::json_parse(&rt::jw_rotation(&q, None)), "");
            assert_eq!(back, q);
        }
        // Three numbers are degrees.
        assert_eq!(rt::jr_rotation(&rt::json_parse("[0, 0, 90]"), ""), rt::rot(0.0, 0.0, 90.0));
    }

    #[test]
    fn every_ease_has_its_own_curve_in_both_runtimes() {
        let lua = include_str!("runtime.lua");
        for &e in crate::stdlib::EASES {
            // Each starts near 0 and ends at 1 (OutBack's formula gives 2e-16 at 0).
            assert!(rt::ease_apply(e, 0.0).abs() < 1e-12, "{e} at 0");
            assert!((rt::ease_apply(e, 1.0) - 1.0).abs() < 1e-12, "{e} at 1");
            // A name the Rust doesn't know would fall through to Linear's straight line.
            if e != "Linear" {
                assert_ne!(rt::ease(e, 0.3), 0.3, "{e} isn't in native_rt.rs's ease");
            }
            let named = format!("\n  {e} = ");
            assert!(
                lua.contains(&named) || e == "OutBounce" && lua.contains("OutBounce = out_bounce"),
                "{e} isn't in runtime.lua"
            );
        }
        // Outside 0 to 1, t is kept to 0 or 1.
        assert_eq!(rt::ease_apply("InQuad", -2.0), 0.0);
        assert_eq!(rt::ease_apply("InQuad", 3.0), 1.0);
        // The standard values from easings.net.
        assert_eq!(rt::ease("OutCubic", 0.5), 0.875);
        assert_eq!(rt::ease("InOutQuad", 0.25), 0.125);
        assert_eq!(rt::ease("OutBounce", 0.5), 0.765625);
    }

    #[test]
    fn tweens_step_and_replace_in_order() {
        let place = std::rc::Rc::new(std::cell::RefCell::new(0.0f64));
        let done = std::rc::Rc::new(std::cell::Cell::new(0));
        let (p, d) = (place.clone(), done.clone());
        let key = rt::tween_key(&place, "x");
        let finished: std::rc::Rc<dyn Fn()> = std::rc::Rc::new(move || d.set(d.get() + 1));
        rt::tween(
            key,
            0.0,
            10.0,
            0.5,
            "Linear",
            Box::new(move |v| *p.borrow_mut() = v),
            Some(finished.clone()),
        );
        for _ in 0..15 {
            assert!(rt::tween_step(1.0 / 60.0));
        }
        // 15 steps of 1/60 s are exactly a quarter second.
        assert_eq!(*place.borrow(), 5.0);
        // Replacing it: the first one's done never runs.
        let p = place.clone();
        rt::tween(key, 5.0, 1.0, 0.1, "Linear", Box::new(move |v| *p.borrow_mut() = v), Some(finished));
        assert!(rt::tween_step(0.05));
        assert!(!rt::tween_step(0.05));
        assert_eq!(*place.borrow(), 1.0);
        assert_eq!(done.get(), 1);
        // Asking with 0 moves nothing.
        assert!(!rt::tween_step(0.0));
    }
}
