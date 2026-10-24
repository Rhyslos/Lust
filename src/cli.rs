//! The `lust` command: everything it does beyond the compiler itself.

use crate::{lua, native_ui, run, rust, tests};
use std::path::{Path, PathBuf};
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
  lust new <name> app  the same, starting with a window and a piece in ui/
  lust build <file> --library
                       .lst: make a Rust library from it, for a Rust program to use
  lust test <folder>   run the test programs in a folder (or one test file)
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
    /// Whether it has native methods, whose bodies are outside Lust.
    pub natives: bool,
}

/// Reads, checks and writes one file. Mistakes are printed, and give None.
pub fn compile(path: &Path, quiet: bool) -> Option<Compiled> {
    compile_as(path, quiet, false)
}

/// The same, with `library` writing the Rust as a library for another Rust program.
pub fn compile_as(path: &Path, quiet: bool, library: bool) -> Option<Compiled> {
    let c = match crate::front(path) {
        Ok(c) => c,
        Err(problems) => {
            if !quiet {
                for p in &problems {
                    eprintln!("{p}");
                }
                if problems.iter().any(|p| p.line > 0) {
                    eprintln!();
                    eprintln!(
                        "{} mistake{}. Nothing was run.",
                        problems.len(),
                        if problems.len() == 1 { "" } else { "s" }
                    );
                }
            }
            return None;
        }
    };
    let rust = (!c.is_script).then(|| rust::write(&c.prog, &c.info, &c.files, library));
    Some(Compiled {
        out: lua::write(&c.prog, &c.info, !c.is_script),
        rust,
        natives: !c.info.natives.is_empty() || !c.info.sends.is_empty(),
        files: c.files,
    })
}

pub fn main() -> ExitCode {
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
        "new" => new_project(&file, args.get(2).map(|s| s == "app").unwrap_or(false)),
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
        "build" if args.iter().any(|a| a == "--library") => match compile_as(path, false, true) {
            Some(Compiled { rust: Some(r), .. }) => match build_library(path, &r) {
                Ok(dir) => {
                    let name = crate_name(path);
                    println!("Made {}, a Rust library.", dir.display());
                    println!("A Rust program uses it with this line under [dependencies] in its Cargo.toml:");
                    println!();
                    println!("  {name} = {{ path = {:?} }}", dir.display().to_string().replace('\\', "/"));
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::from(1)
                }
            },
            Some(_) => {
                eprintln!("{display} is LustScript, which turns into Lua. Only .lst files can be built as Rust libraries.");
                ExitCode::from(1)
            }
            None => ExitCode::from(1),
        },
        "build" => match compile(path, false) {
            Some(Compiled {
                natives: true,
                rust: Some(_),
                ..
            }) => {
                eprintln!(
                    "{display} has native methods or sends events, which need the Rust program that uses it, so it can't be a program of its own. Build it as a library: lust build {display} --library"
                );
                ExitCode::from(1)
            }
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

const CARGO_TOML: &str = "[package]\nname = \"lust_program\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nrusqlite = { version = \"0.40\", features = [\"bundled\"] }\ntiny_http = \"0.12\"\nureq = \"3\"\nregex = \"1\"\nwinit = \"0.30\"\nsoftbuffer = \"0.4\"\nab_glyph = \"0.2\"\nrfd = \"0.16\"\narboard = { version = \"3\", default-features = false }\npng = \"0.17\"\njpeg-decoder = { version = \"0.3\", default-features = false }\n\n[profile.release]\noverflow-checks = true\n\n[workspace]\n";

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
    // Each project's program has a name of its own in the shared target folder, so two builds at
    // once can't copy out each other's program.
    let bin = format!("lust_{hash:016x}");
    let manifest = format!("{CARGO_TOML}\n[[bin]]\nname = \"{bin}\"\npath = \"src/main.rs\"\n");
    if std::fs::read_to_string(&toml).ok().as_deref() != Some(manifest.as_str()) {
        std::fs::write(&toml, &manifest).map_err(|e| format!("couldn't write {}: {e}", toml.display()))?;
    }
    std::fs::write(project.join("src").join("main.rs"), rust).map_err(|e| format!("couldn't write the program: {e}"))?;
    // Views draw with Lust's font, which goes into the program.
    let fonts = project.join("assets").join("fonts");
    std::fs::create_dir_all(&fonts).map_err(|e| format!("couldn't make a build folder: {e}"))?;
    for (name, bytes) in native_ui::FONT_FILES.iter().zip(native_ui::FONTS.iter()) {
        let f = fonts.join(name);
        if std::fs::metadata(&f).map(|m| m.len() as usize != bytes.len()).unwrap_or(true) {
            std::fs::write(&f, bytes).map_err(|e| format!("couldn't write {}: {e}", f.display()))?;
        }
    }
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
    let built = target.join("release").join(format!("{bin}{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(&built, exe).map_err(|e| format!("couldn't copy the program to {}: {e}", exe.display()))?;
    Ok(())
}

/// lust new name: a folder with a main.lst to start from, and the libraries made ready.
/// The window a new app starts with.
const APP_MAIN: &str = r#"-- NAME: where the program starts. It opens a window.
fetch "ui/counter":Counter;

assembly App {
    has {
        string name = "";
    }

    view [title "NAME", width 480, height 360] {
        column [gap 16, pad 24] {
            text "NAME" [huge, bold];
            input name [placeholder "What's your name?", width fill];
            is name != ""? then {
                text "Hello, " + name + "!" [large];
            }
            Counter { label = "Clicks" };
        }
    }
}

App:open();
end
"#;

/// A piece of the window, in its own file.
const APP_COUNTER: &str = r#"-- A piece of the screen: it counts clicks, and remembers its count between redraws.
assembly Counter {
    has {
        string label = "";
        int count = 0;
    }

    view {
        row [gap 8, pad 12, round, background gray1] {
            text label + ": " + count.text();
            space;
            button "Add one" [blue] {
                count = count + 1;
            }
            button "Reset" [subtle, disabled count == 0] {
                count = 0;
            }
        }
    }
}

pass Counter;
end
"#;

fn new_project(name: &str, app: bool) -> ExitCode {
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
            if app {
                std::fs::create_dir_all(dir.join("ui"))?;
                std::fs::write(dir.join("ui").join("counter.lst"), APP_COUNTER)?;
                std::fs::write(dir.join("main.lst"), APP_MAIN.replace("NAME", &shown))
            } else {
                std::fs::write(
                    dir.join("main.lst"),
                    format!("-- {shown}: where the program starts.\nprint(\"Hello from {shown}!\");\nend\n"),
                )
            }
        })
        .and_then(|_| {
            std::fs::write(
                dir.join(".gitignore"),
                "# What lust build makes, and saves.\n/main\n/main.exe\n*_library/\n*.save\n*.db\n",
            )
        });
    if let Err(e) = made {
        eprintln!("couldn't make {name}: {e}");
        return ExitCode::from(1);
    }
    if app {
        println!("Made {name}, with a main.lst that opens a window, and a piece of it in ui/counter.lst.");
    } else {
        println!("Made {name}, with a main.lst to start from.");
    }
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

/// The name a Lust file's library has in Rust: game.lst is the crate game.
fn crate_name(path: &Path) -> String {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let mut name = String::new();
    for (i, ch) in stem.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 {
                name.push('_');
            }
            name.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_alphanumeric() {
            name.push(ch);
        } else {
            name.push('_');
        }
    }
    if name.starts_with(|c: char| c.is_ascii_digit()) || name.is_empty() {
        name.insert_str(0, "lust_");
    }
    name
}

/// lust build file.lst --library: a Rust library in a folder next to the file (file_library), for a
/// Rust program to use. It's checked with Cargo, so a mistake in it shows up here.
pub fn build_library(path: &Path, rust: &str) -> Result<PathBuf, String> {
    let name = crate_name(path);
    let dir = path.parent().unwrap_or(Path::new(".")).join(format!("{name}_library"));
    build_library_in(path, rust, &dir)?;
    Ok(dir)
}

/// The same, into a folder of its own choosing (lust test keeps them in the cache).
pub fn build_library_in(path: &Path, rust: &str, dir: &Path) -> Result<(), String> {
    let name = crate_name(path);
    std::fs::create_dir_all(dir.join("src")).map_err(|e| format!("couldn't make {}: {e}", dir.display()))?;
    // Only the libraries the Lust code uses.
    let has = |m: &str| rust.contains(&format!("\npub mod {m} {{\n"));
    let mut deps = String::new();
    if has("db") {
        deps.push_str("rusqlite = { version = \"0.40\", features = [\"bundled\"] }\n");
    }
    if has("web") {
        deps.push_str("tiny_http = \"0.12\"\n");
    }
    if has("net") {
        deps.push_str("ureq = \"3\"\n");
    }
    if has("pat") {
        deps.push_str("regex = \"1\"\n");
    }
    if has("dlg") {
        deps.push_str("rfd = \"0.16\"\n");
    }
    if has("ui") {
        deps.push_str("winit = \"0.30\"\nsoftbuffer = \"0.4\"\nab_glyph = \"0.2\"\narboard = { version = \"3\", default-features = false }\npng = \"0.17\"\njpeg-decoder = { version = \"0.3\", default-features = false }\n");
        let fonts = dir.join("assets").join("fonts");
        std::fs::create_dir_all(&fonts).map_err(|e| format!("couldn't make {}: {e}", fonts.display()))?;
        for (file, bytes) in native_ui::FONT_FILES.iter().zip(native_ui::FONTS.iter()) {
            std::fs::write(fonts.join(file), bytes).map_err(|e| format!("couldn't write the font {file}: {e}"))?;
        }
    }
    let toml = format!(
        "# Made by Lust from {}. lust build --library writes it again, so don't edit it.\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n\n[dependencies]\n{deps}",
        path.file_name().unwrap_or_default().to_string_lossy()
    );
    std::fs::write(dir.join("Cargo.toml"), toml).map_err(|e| format!("couldn't write Cargo.toml: {e}"))?;
    std::fs::write(dir.join("src").join("lib.rs"), rust).map_err(|e| format!("couldn't write lib.rs: {e}"))?;
    let out = std::process::Command::new("cargo")
        .args(["check", "--quiet", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", cache_dir().join("target"))
        .output()
        .map_err(|_| "building needs Rust, and it isn't installed. Get it from https://rustup.rs, then try again.".to_string())?;
    if !out.status.success() {
        return Err(format!(
            "Lust made Rust that doesn't build. That's a bug in Lust, not your program. Rust said:\n{}\nThe Rust is in {}.",
            String::from_utf8_lossy(&out.stderr),
            dir.join("src").join("lib.rs").display()
        ));
    }
    Ok(())
}

/// Builds a Rust program (lust test's name.host.rs) that uses a Lust file as a library.
pub fn build_host(path: &Path, host: &Path) -> Result<PathBuf, String> {
    let rust = compile_as(path, true, true).and_then(|c| c.rust).ok_or("it has mistakes")?;
    let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut hash: u64 = 1469598103934665603;
    for b in full.to_string_lossy().bytes() {
        hash = (hash ^ b as u64).wrapping_mul(1099511628211);
    }
    let cache = cache_dir();
    let lib = cache.join("projects").join(format!("lib-{hash:016x}"));
    build_library_in(path, &rust, &lib)?;
    let proj = cache.join("projects").join(format!("host-{hash:016x}"));
    std::fs::create_dir_all(proj.join("src")).map_err(|e| e.to_string())?;
    let bin = format!("host_{hash:016x}");
    std::fs::write(
        proj.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{bin}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{} = {{ path = {:?} }}\n\n[workspace]\n",
            crate_name(path),
            lib.display().to_string().replace('\\', "/")
        ),
    )
    .map_err(|e| e.to_string())?;
    let src = std::fs::read_to_string(host).map_err(|e| e.to_string())?;
    std::fs::write(proj.join("src").join("main.rs"), src).map_err(|e| e.to_string())?;
    let target = cache.join("target");
    let out = std::process::Command::new("cargo")
        .args(["build", "--release", "--quiet", "--manifest-path"])
        .arg(proj.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target)
        .output()
        .map_err(|_| "building needs Rust, and it isn't installed.".to_string())?;
    if !out.status.success() {
        return Err(format!(
            "the Rust program using it didn't build:\n{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(target.join("release").join(format!("{bin}{}", std::env::consts::EXE_SUFFIX)))
}

pub fn build_native(path: &Path, rust: &str) -> Result<std::path::PathBuf, String> {
    if ["db", "web", "net", "pat", "ui"]
        .iter()
        .any(|m| rust.contains(&format!("\npub mod {m} {{\n")))
    {
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
