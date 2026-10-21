//! The `lust` command.

mod ast;
mod check;
mod diag;
mod lexer;
mod lua;
mod parser;
mod project;
// The runtime that goes into built programs. lust uses its file functions too, so
// LustScript run by lust says exactly the same things.
#[path = "native_rt.rs"]
#[allow(dead_code, non_snake_case)]
mod native_rt;
// Databases, for built programs that use them, and for LustScript run by lust.
#[allow(dead_code)]
mod native_db {
    pub use crate::native_rt::*;
    use std::rc::Rc;
    include!("native_db.rs");
}
mod run;
mod rust;
mod stdlib;
mod tests;
mod types;

use std::path::Path;
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = "Lust 2 (alpha)

  lust run <file>      check a .lst or .lsc file, then run it
  lust check <file>    only check it, and list any mistakes
  lust lua <file>      show the Lua a file turns into
  lust rust <file>     show the Rust a .lst file turns into
  lust build <file>    .lst: build a real program next to it (needs Rust installed)
                       .lsc: write the Lua next to it, as file.lua
  lust new <name>      make a new project folder, with a main.lst
  lust test <folder>   run the test programs in a folder
  lust version         show which Lust this is

lust <file> is the same as lust run <file>.
lust run is quick to start, so it runs .lst files through Lua too.
lust build makes the real, fast program.";

pub struct Compiled {
    pub out: lua::Output,
    /// The Rust version, for .lst files.
    pub rust: Option<String>,
    /// The program's files, by number, as people see them.
    pub files: Vec<String>,
}

/// Reads, checks and writes one file. Mistakes are printed, and give None.
pub fn compile(path: &Path, quiet: bool) -> Option<Compiled> {
    let display = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    if ext == "lust" {
        eprintln!(
            "Lust programs now end in .lst. Rename {display} to {}.lst and try again.",
            path.file_stem().unwrap_or_default().to_string_lossy()
        );
        return None;
    }
    if ext != "lst" && ext != "lsc" {
        eprintln!("{display} isn't a Lust file. Lust files end in .lst, or .lsc for LustScript.");
        return None;
    }
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s.replace("\r\n", "\n"),
        Err(_) => {
            eprintln!("there's no file called {}.", path.display());
            return None;
        }
    };
    let is_script = ext == "lsc";
    let mut diags = diag::Diags::default();
    let project = project::load(path, src, is_script, &mut diags);
    let prog = project.program;
    let files = project.files;
    let info = check::check(&prog, &mut diags, is_script);
    if !diags.is_empty() {
        if !quiet {
            let list = diags.sorted();
            for (pos, msg) in &list {
                let file = files.get(pos.file()).cloned().unwrap_or_default();
                eprintln!("{file}:{}:{}  {msg}", pos.row(), pos.col);
            }
            eprintln!();
            eprintln!("{} mistake{}. Nothing was run.", list.len(), if list.len() == 1 { "" } else { "s" });
        }
        return None;
    }
    let rust = (!is_script).then(|| rust::write(&prog, &info, &files));
    Some(Compiled {
        out: lua::write(&prog, &info),
        rust,
        files,
    })
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, file) = match args.as_slice() {
        [] => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        [c] if c == "version" || c == "--version" => {
            println!("Lust {VERSION}");
            return ExitCode::SUCCESS;
        }
        [c] if c == "help" || c == "--help" => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        [f] => ("run", f.clone()),
        [c, f, ..] => (c.as_str(), f.clone()),
    };
    let path = Path::new(&file);
    let display = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    match cmd {
        "test" => tests::run_folder(path),
        "new" => new_project(&file),
        "check" => match compile(path, false) {
            Some(_) => {
                println!("{display} looks good.");
                ExitCode::SUCCESS
            }
            None => ExitCode::from(1),
        },
        "lua" => match compile(path, false) {
            Some(c) => {
                print!("{}", c.out.code);
                ExitCode::SUCCESS
            }
            None => ExitCode::from(1),
        },
        "rust" => match compile(path, false) {
            Some(Compiled { rust: Some(r), .. }) => {
                print!("{r}");
                ExitCode::SUCCESS
            }
            Some(_) => {
                eprintln!("{display} is LustScript, which turns into Lua. Try: lust lua {display}");
                ExitCode::from(1)
            }
            None => ExitCode::from(1),
        },
        "build" => match compile(path, false) {
            Some(Compiled { rust: Some(r), .. }) => match build_native(path, &r) {
                Ok(exe) => {
                    println!("Built {}.", exe.file_name().unwrap().to_string_lossy());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::from(1)
                }
            },
            Some(c) => {
                let out = path.with_extension("lua");
                if let Err(e) = std::fs::write(&out, &c.out.code) {
                    eprintln!("couldn't write {}: {e}", out.display());
                    return ExitCode::from(1);
                }
                println!("Wrote {}.", out.file_name().unwrap().to_string_lossy());
                ExitCode::SUCCESS
            }
            None => ExitCode::from(1),
        },
        "run" => match compile(path, false) {
            Some(c) => run::run(&c.out, &c.files),
            None => ExitCode::from(1),
        },
        other => {
            eprintln!("lust doesn't know the command {other}.\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Turns the Rust for a program into a real program next to the .lst file, with rustc.
/// Where Lust keeps what it builds with: the libraries built programs use, compiled once per computer.
fn cache_dir() -> std::path::PathBuf {
    if let Ok(d) = std::env::var("LUST_CACHE") {
        return d.into();
    }
    if cfg!(windows) {
        if let Ok(d) = std::env::var("LOCALAPPDATA") {
            return Path::new(&d).join("lust");
        }
    } else if let Ok(d) = std::env::var("XDG_CACHE_HOME") {
        return Path::new(&d).join("lust");
    } else if let Ok(h) = std::env::var("HOME") {
        return Path::new(&h).join(".cache").join("lust");
    }
    std::env::temp_dir().join("lust-cache")
}

const CARGO_TOML: &str = "[package]\nname = \"lust_program\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nrusqlite = { version = \"0.40\", features = [\"bundled\"] }\n\n[profile.release]\noverflow-checks = true\n\n[workspace]\n";

/// Builds a program that needs outside libraries with Cargo, in a folder of Lust's own. The
/// libraries are shared, so they're only compiled the first time on each computer.
fn build_with_cargo(path: &Path, rust: &str, exe: &Path) -> Result<(), String> {
    let cache = cache_dir();
    let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut hash: u64 = 1469598103934665603;
    for b in full.to_string_lossy().bytes() {
        hash = (hash ^ b as u64).wrapping_mul(1099511628211);
    }
    let project = cache.join("projects").join(format!("{stem}-{hash:016x}"));
    let target = cache.join("target");
    std::fs::create_dir_all(project.join("src")).map_err(|e| format!("couldn't make a build folder: {e}"))?;
    let toml = project.join("Cargo.toml");
    if std::fs::read_to_string(&toml).ok().as_deref() != Some(CARGO_TOML) {
        std::fs::write(&toml, CARGO_TOML).map_err(|e| format!("couldn't write {}: {e}", toml.display()))?;
    }
    std::fs::write(project.join("src").join("main.rs"), rust).map_err(|e| format!("couldn't write the program: {e}"))?;
    let first = !target
        .join("release")
        .join("deps")
        .read_dir()
        .map(|d| d.flatten().any(|e| e.file_name().to_string_lossy().contains("rusqlite")))
        .unwrap_or(false);
    if first {
        eprintln!("Getting SQLite ready for built programs. This happens once on this computer, and takes a minute or two.");
    }
    let out = std::process::Command::new("cargo")
        .args(["build", "--release", "--quiet", "--manifest-path"])
        .arg(&toml)
        .env("CARGO_TARGET_DIR", &target)
        .output()
        .map_err(|_| "building needs Rust, and it isn't installed. Get it from https://rustup.rs, then try again.".to_string())?;
    if !out.status.success() {
        return Err(format!(
            "Lust made Rust that doesn't build, or Cargo couldn't get SQLite. Rust said:\n{}\nThe Rust is in {}.",
            String::from_utf8_lossy(&out.stderr),
            project.join("src").join("main.rs").display()
        ));
    }
    let built = target.join("release").join(format!("lust_program{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(&built, exe).map_err(|e| format!("couldn't copy the program to {}: {e}", exe.display()))?;
    Ok(())
}

/// lust new name: a folder with a main.lst to start from, and the libraries made ready.
fn new_project(name: &str) -> ExitCode {
    let dir = Path::new(name);
    if dir.exists() {
        eprintln!("there's already something called {name} here.");
        return ExitCode::from(1);
    }
    let shown = dir
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| name.to_string());
    let made = std::fs::create_dir_all(dir)
        .and_then(|_| {
            std::fs::write(
                dir.join("main.lst"),
                format!("-- {shown}: where the program starts.\nprint(\"Hello from {shown}!\");\nend\n"),
            )
        })
        .and_then(|_| {
            std::fs::write(
                dir.join(".gitignore"),
                "# What lust build makes, and saves.\n/main\n/main.exe\n*.save\n*.db\n",
            )
        });
    if let Err(e) = made {
        eprintln!("couldn't make {name}: {e}");
        return ExitCode::from(1);
    }
    println!("Made {name}, with a main.lst to start from.");
    // A small program with a database, built once, so the first real build is quick.
    let warm = dir.join("lust-warm-up.lst");
    let src = "database Warm in \"warm.db\" {\n    cMap(int id, string name) things;\n}\nprint(#Warm.things);\nend\n";
    let ready = std::fs::write(&warm, src).is_ok()
        && compile(&warm, true)
            .and_then(|c| c.rust)
            .map(|r| build_native(&warm, &r).map(|exe| std::fs::remove_file(exe).is_ok()).is_ok())
            .unwrap_or(false);
    let _ = std::fs::remove_file(&warm);
    if ready {
        println!("Built programs are ready to go on this computer.");
    } else {
        println!("lust run works now. lust build needs Rust: get it from https://rustup.rs.");
    }
    println!();
    println!("  cd {name}");
    println!("  lust run main.lst");
    println!("  lust build main.lst");
    ExitCode::SUCCESS
}

pub fn build_native(path: &Path, rust: &str) -> Result<std::path::PathBuf, String> {
    if rust.contains("\npub mod db {\n") {
        let exe = path.with_extension(std::env::consts::EXE_EXTENSION);
        build_with_cargo(path, rust, &exe)?;
        return Ok(exe);
    }
    let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let dir = std::env::temp_dir().join("lust-build");
    std::fs::create_dir_all(&dir).map_err(|e| format!("couldn't make a build folder: {e}"))?;
    let src = dir.join(format!("{stem}-{}.rs", std::process::id()));
    std::fs::write(&src, rust).map_err(|e| format!("couldn't write {}: {e}", src.display()))?;
    let exe = path.with_extension(std::env::consts::EXE_EXTENSION);
    let result = std::process::Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "-O",
            "-C",
            "overflow-checks=on",
            "--crate-name",
            "lust_program",
            "-o",
        ])
        .arg(&exe)
        .arg(&src)
        .output();
    let _ = std::fs::remove_file(&src);
    match result {
        Err(_) => Err("building needs Rust, and it isn't installed. Get it from https://rustup.rs, then try again.".into()),
        Ok(o) if !o.status.success() => {
            let keep = dir.join(format!("{stem}.rs"));
            let _ = std::fs::write(&keep, rust);
            Err(format!(
                "Lust made Rust that doesn't build. That's a bug in Lust, not your program.\nThe Rust is in {}. Rust said:\n{}",
                keep.display(),
                String::from_utf8_lossy(&o.stderr)
            ))
        }
        Ok(_) => Ok(exe),
    }
}
