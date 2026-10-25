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
    checked(path, src, is_script, None)
}

/// The same, with every file's text handed over by its path: Lust doesn't open any files.
fn front_from(entry: &str, files: &[(String, String)]) -> Result<Checked, Vec<Problem>> {
    let mem: project::Memory = files
        .iter()
        .map(|(p, text)| (project::tidy(Path::new(&p.replace('\\', "/"))), text.clone()))
        .collect();
    let path = project::tidy(Path::new(&entry.replace('\\', "/")));
    let is_script = kind_of(&path, entry)?;
    let Some(src) = mem.get(&path).cloned() else {
        return Err(whole(entry, format!("there's no file called {entry} among the files handed over.")));
    };
    checked(&path, src.replace("\r\n", "\n"), is_script, Some(&mem))
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

fn checked(path: &Path, src: String, is_script: bool, mem: Option<&project::Memory>) -> Result<Checked, Vec<Problem>> {
    let mut diags = diag::Diags::default();
    let project = project::load(path, src, is_script, &mut diags, mem);
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
    Ok(Checked {
        prog,
        info,
        files,
        is_script,
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

/// Checks a program: `entry` is its first file's path, and `files` every file's path and text,
/// with / between folders (\\ works too). Gives back every mistake found, each with its file,
/// line and column, in order.
pub fn check(entry: &str, files: &[(String, String)]) -> Result<(), Vec<Problem>> {
    on_own_stack(|| front_from(entry, files).map(|_| ()))
}

/// Lua for an engine to load (Lua 5.1, LuaJIT or Luau): Lust's runtime, the program, and at the
/// end a table the engine uses to make, run and swap components. See LUST.md, section 21.
pub fn to_lua(entry: &str, files: &[(String, String)]) -> Result<LuaChunk, Vec<Problem>> {
    on_own_stack(|| lua_chunk(entry, files))
}

fn lua_chunk(entry: &str, files: &[(String, String)]) -> Result<LuaChunk, Vec<Problem>> {
    let c = front_from(entry, files)?;
    let out = lua::write_for_engine(&c.prog, &c.info, !c.is_script);
    Ok(LuaChunk {
        code: out.code,
        files: c.files,
        lines: out.lines,
        cols: out.cols,
    })
}

/// A Rust library made from a .lst program: what `lust build --library` puts in its src/lib.rs.
/// Its Cargo.toml needs no dependencies, unless the Lust uses databases, the web or windows.
pub fn to_rust_library(entry: &str, files: &[(String, String)]) -> Result<String, Vec<Problem>> {
    on_own_stack(|| rust_library(entry, files))
}

fn rust_library(entry: &str, files: &[(String, String)]) -> Result<String, Vec<Problem>> {
    let c = front_from(entry, files)?;
    if c.is_script {
        return Err(whole(
            entry,
            "LustScript turns into Lua. Only .lst files can be built as Rust libraries.".into(),
        ));
    }
    Ok(rust::write(&c.prog, &c.info, &c.files, true))
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
