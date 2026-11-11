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
    include!("native_ui.rs");
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
