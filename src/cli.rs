//! The `lust` command: everything it does beyond the compiler itself.

use crate::{lua, native_ui, run, rust, tests};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = "Lust (alpha)

  lust run <file>      check a .lst or .lsc file, then run it
  lust check <file>    only check it, and list any mistakes
  lust lua <file>      show the Lua a file turns into
  lust rust <file>     show the Rust a .lst file turns into
  lust build <file>    .lst: build a real program next to it (needs Rust installed)
                       .lsc: write the Lua next to it, as file.lua
  lust new <name>      make a new project folder, with a main.lst
  lust new <name> app  the same, starting with a window and a piece in ui/
  lust build <file> --release
                       .lst: the same, taking longer to make the fastest program, for sharing
  lust build <file> --library
                       .lst: make a Rust library from it, for a Rust program to use
  lust test <folder>   run the test programs in a folder (or one test file)
  lust version         show which Lust this is
  lust update          get the newest Lust (lust update check only looks)
  lust update off      stop the note saying a newer Lust is out (lust update on: back on)

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
    // Notes aren't mistakes (me, which is now this): said, and everything goes on.
    if !quiet {
        for n in &c.notes {
            eprintln!("{n}");
        }
    }
    let rust = (!c.is_script).then(|| rust::write(&c.prog, &c.info, &c.files, library));
    Some(Compiled {
        out: lua::write(&c.prog, &c.info, !c.is_script),
        rust,
        natives: !c.info.natives.is_empty() || !c.info.sends.is_empty(),
        files: c.files,
    })
}

pub fn main() -> ExitCode {
    crate::update::tidy();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("update") {
        return crate::update::command(args.get(1).map(String::as_str));
    }
    // A person running a program, building or checking may hear that a newer Lust is out.
    let first = args.first().map(String::as_str).unwrap_or("");
    let watched = matches!(first, "run" | "build" | "check" | "new") || first.ends_with(".lst") || first.ends_with(".lsc");
    let note = if watched {
        crate::update::start()
    } else {
        crate::update::Note::none()
    };
    let code = command(args);
    crate::update::finish(note);
    code
}

fn command(args: Vec<String>) -> ExitCode {
    let (cmd, file) = match args.as_slice() {
        [] => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        [c] if c == "warm-up" => return warm_up(),
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
            Some(Compiled { rust: Some(r), .. }) => match build_native_with(path, &r, args.iter().any(|a| a == "--release")) {
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

/// The tray icon and notifications: per system, for the shared runtime and a library with views.
const UI_TARGET_DEPS: &str = "\n[target.'cfg(windows)'.dependencies]\ntray-icon = { version = \"0.25\", default-features = false }\ntauri-winrt-notification = \"0.8\"\n\n[target.'cfg(target_os = \"linux\")'.dependencies]\nzbus = { version = \"5\", default-features = false, features = [\"async-io\", \"blocking-api\"] }\n";

/// What lust build says when Rust isn't on the computer.
const NEEDS_RUST: &str = "building a real program needs Rust, which isn't on this computer yet. lust run works without it.
To get Rust: download rustup-init.exe from https://rustup.rs and run it. On Windows it also offers the Visual Studio build tools Rust needs: say yes. It's a few GB, and takes a while.
Then open a new terminal, and build again.";

/// What lust build says when Rust is there, but Windows' build tools (its linker) aren't.
const NEEDS_TOOLS: &str = "Rust is here, but Windows' build tools aren't, and Rust needs them to make a program.
Get them by running rustup-init.exe from https://rustup.rs again and saying yes to the Visual Studio build tools, or install \"Build Tools for Visual Studio\" with \"Desktop development with C++\" from https://visualstudio.microsoft.com/downloads/
Then open a new terminal, and build again.";

/// The libraries the shared runtime is built with, as Cargo.toml writes them.
const RUNTIME_DEPS: &str = "rusqlite = { version = \"0.40\", features = [\"bundled\"] }\ntiny_http = \"0.12\"\nureq = \"3\"\nregex = \"1\"\nwinit = \"0.30\"\naccesskit = \"0.25\"\naccesskit_winit = \"0.34\"\nsoftbuffer = \"0.4\"\nab_glyph = \"0.2\"\nrfd = \"0.16\"\narboard = { version = \"3\", default-features = false }\npng = \"0.17\"\njpeg-decoder = { version = \"0.3\", default-features = false }\nresvg = { version = \"0.49\", default-features = false }\n";

/// Changes when the way the runtime is compiled or described (ready.txt) changes, so a runtime
/// made the old way is made again.
const RUNTIME_FORMAT: &str = "lust-runtime 3\n";

/// The shared runtime, compiled: the library a program is built against, and where Rust finds
/// the libraries it uses.
struct Runtime {
    dir: PathBuf,
    rlib: PathBuf,
    deps: PathBuf,
    search: Vec<String>,
    /// The libraries the runtime uses directly, by the names Rust code calls them, for a release
    /// build, which compiles the program and its own copy of the runtime as one.
    externs: Vec<(String, PathBuf)>,
}

/// The runtime's name: a hash of everything that goes into it (its code, the libraries it uses,
/// and which Rust compiles it, since pieces compiled by one version of Rust can't be used by
/// another). A new Lust whose runtime is the same uses the one already compiled.
fn runtime_key() -> Result<String, String> {
    let out = std::process::Command::new("rustc")
        .arg("-vV")
        .output()
        .map_err(|_| NEEDS_RUST.to_string())?;
    let text = format!(
        "{RUNTIME_FORMAT}{}{}{}",
        String::from_utf8_lossy(&out.stdout),
        runtime_toml(),
        rust::runtime_library()
    );
    let mut hash: u64 = 1469598103934665603;
    for b in text.bytes() {
        hash = (hash ^ b as u64).wrapping_mul(1099511628211);
    }
    Ok(format!("{hash:016x}"))
}

/// The folder for this runtime, compiled by this Rust.
fn runtime_dir() -> Result<PathBuf, String> {
    Ok(cache_dir().join("runtime").join(runtime_key()?))
}

/// The runtime's Cargo.toml.
fn runtime_toml() -> String {
    format!(
        "[package]\nname = \"lust_rt\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n\n[dependencies]\n{RUNTIME_DEPS}{UI_TARGET_DEPS}\n[profile.release]\noverflow-checks = true\n\n[workspace]\n"
    )
}

/// What a finished runtime folder says about itself, in ready.txt: the runtime's library, then
/// a line `extern name=file` for each library the runtime uses directly, then one line per place
/// Rust looks for the libraries it links (kind=folder). Every path is written from the runtime's
/// own folder, which holds everything a build needs (lib/ and native/), so the folder can be
/// moved or downloaded whole.
fn read_runtime(dir: &Path) -> Option<Runtime> {
    let text = std::fs::read_to_string(dir.join("ready.txt")).ok()?;
    let mut lines = text.lines();
    let rlib = dir.join(lines.next()?);
    let deps = dir.join("lib");
    let mut search = Vec::new();
    let mut externs = Vec::new();
    for l in lines {
        if let Some((name, file)) = l.strip_prefix("extern ").and_then(|e| e.split_once('=')) {
            externs.push((name.to_string(), dir.join(file)));
        } else if let Some((kind, at)) = l.split_once('=') {
            search.push(format!("{kind}={}", dir.join(at).display()));
        }
    }
    rlib.exists().then(|| Runtime {
        dir: dir.to_path_buf(),
        rlib,
        deps,
        search,
        externs,
    })
}

/// The libraries named in Cargo.toml's text, with the version each asks for: ("jpeg_decoder", "0.3").
fn direct_deps() -> Vec<(String, String)> {
    format!("{RUNTIME_DEPS}{UI_TARGET_DEPS}")
        .lines()
        .filter_map(|l| {
            let (name, rest) = l.split_once(" = ")?;
            let version = rest.split('"').nth(1)?;
            (!name.starts_with('[')).then(|| (name.trim().to_string(), version.to_string()))
        })
        .collect()
}

/// What Cargo said it made (its JSON messages, one a line): every library compiled, as
/// (package id, .rlib file), and every folder build scripts link libraries from (kind=folder).
#[derive(Default, Debug, PartialEq)]
struct CargoMade {
    rlibs: Vec<(String, String)>,
    link_paths: Vec<String>,
}

fn cargo_made(messages: &str) -> CargoMade {
    // JSON text as plain text: only \\ and \" are escaped in paths.
    let unescape = |s: &str| s.replace("\\\\", "\\").replace("\\\"", "\"");
    let text_field = |line: &str, key: &str| -> Option<String> {
        let at = line.find(&format!("\"{key}\":\""))? + key.len() + 4;
        let end = line[at..].find('"')? + at;
        Some(unescape(&line[at..end]))
    };
    let list_field = |line: &str, key: &str| -> Vec<String> {
        let Some(at) = line.find(&format!("\"{key}\":[")).map(|a| a + key.len() + 4) else {
            return Vec::new();
        };
        let end = line[at..].find(']').map_or(line.len(), |e| e + at);
        line[at..end].split('"').skip(1).step_by(2).map(unescape).collect()
    };
    let mut made = CargoMade::default();
    for line in messages.lines() {
        if line.contains("\"reason\":\"compiler-artifact\"") {
            let Some(id) = text_field(line, "package_id") else { continue };
            // Libraries (.rlib), and macros that run while compiling (a .dll, .so or .dylib),
            // which Rust needs again to read the libraries that use them.
            for file in list_field(line, "filenames")
                .into_iter()
                .filter(|f| [".rlib", ".dll", ".so", ".dylib"].iter().any(|e| f.ends_with(e)))
            {
                made.rlibs.push((id.clone(), file));
            }
        } else if line.contains("\"reason\":\"build-script-executed\"") {
            made.link_paths.extend(list_field(line, "linked_paths"));
        }
    }
    made
}

/// Which of Cargo's libraries is each one in `wanted`: ("jpeg_decoder", file). A library Cargo
/// compiled in two versions is told apart by the version Cargo.toml asks for.
fn rlibs_wanted(made: &CargoMade, wanted: &[(String, String)]) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = Vec::new();
    for (name, version) in wanted {
        let crate_name = name.replace('-', "_");
        // registry+https://…#jpeg-decoder@0.3.1, or (older Cargo) jpeg-decoder 0.3.1 (registry+…)
        let file = made.rlibs.iter().find(|(id, f)| {
            f.ends_with(".rlib") && (id.contains(&format!("#{name}@{version}")) || id.starts_with(&format!("{name} {version}")))
        });
        if let Some((_, file)) = file {
            found.push((crate_name, file.clone()));
        }
    }
    found
}

/// The runtime for this Lust and this Rust, compiled now if it isn't yet. Compiling it takes a
/// few minutes, once per computer and version; a second build at the same time waits for it.
fn ensure_runtime() -> Result<Runtime, String> {
    let dir = runtime_dir()?;
    if let Some(r) = read_runtime(&dir) {
        return Ok(r);
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("couldn't make {}: {e}", dir.display()))?;
    let lock = dir.join("building.lock");
    loop {
        if std::fs::OpenOptions::new().write(true).create_new(true).open(&lock).is_ok() {
            // Another build may have finished it while this one waited.
            if let Some(r) = read_runtime(&dir) {
                let _ = std::fs::remove_file(&lock);
                return Ok(r);
            }
            break;
        }
        // Another build is compiling it: wait, unless its lock is old (one that was stopped).
        let old = std::fs::metadata(&lock)
            .and_then(|m| m.modified())
            .map(|t| t.elapsed().map(|e| e.as_secs() > 1800).unwrap_or(false))
            .unwrap_or(true);
        if old {
            let _ = std::fs::remove_file(&lock);
            continue;
        }
        if let Some(r) = read_runtime(&dir) {
            return Ok(r);
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    let made = compile_runtime(&dir);
    let _ = std::fs::remove_file(&lock);
    made?;
    read_runtime(&dir).ok_or_else(|| format!("Lust's runtime was built, but {} can't be read.", dir.join("ready.txt").display()))
}

/// Compiles the runtime with Cargo into `dir`, then writes ready.txt.
fn compile_runtime(dir: &Path) -> Result<(), String> {
    eprintln!("Getting Lust's libraries ready for built programs. This happens once on this computer, and takes a few minutes.");
    std::fs::create_dir_all(dir.join("src")).map_err(|e| format!("couldn't make a build folder: {e}"))?;
    let toml = runtime_toml();
    // Written only when they change, so Cargo doesn't compile the runtime again for nothing.
    let write = |file: PathBuf, text: String| -> Result<(), String> {
        if std::fs::read_to_string(&file).ok().as_deref() != Some(text.as_str()) {
            std::fs::write(&file, text).map_err(|e| format!("couldn't write {}: {e}", file.display()))?;
        }
        Ok(())
    };
    write(dir.join("Cargo.toml"), toml)?;
    write(dir.join("src").join("lib.rs"), rust::runtime_library())?;
    // Views draw with Lust's font, which goes into the runtime.
    let fonts = dir.join("assets").join("fonts");
    std::fs::create_dir_all(&fonts).map_err(|e| format!("couldn't make a build folder: {e}"))?;
    for (name, bytes) in native_ui::FONT_FILES.iter().zip(native_ui::FONTS.iter()) {
        let f = fonts.join(name);
        if std::fs::metadata(&f).map(|m| m.len() as usize != bytes.len()).unwrap_or(true) {
            std::fs::write(&f, bytes).map_err(|e| format!("couldn't write the font {name}: {e}"))?;
        }
    }
    let target = dir.join("target");
    let out = std::process::Command::new("cargo")
        .args(["build", "--release", "--quiet", "--message-format=json", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target)
        .output()
        .map_err(|_| NEEDS_RUST.to_string())?;
    if !out.status.success() {
        let said = String::from_utf8_lossy(&out.stderr);
        if said.contains("link.exe") && (said.contains("not found") || said.contains("could not be found")) {
            return Err(NEEDS_TOOLS.to_string());
        }
        return Err(format!(
            "Lust's libraries for built programs didn't build, or Cargo couldn't get them. Rust said:\n{said}"
        ));
    }
    // Everything a build needs is copied into the runtime's own folder: every compiled library
    // into lib/, and the folders build scripts link from (SQLite's, and Windows' own on Windows)
    // into native/. Then Cargo's working folder goes; it's ten times the size.
    let made = cargo_made(&String::from_utf8_lossy(&out.stdout));
    let lib = dir.join("lib");
    let _ = std::fs::remove_dir_all(&lib);
    let _ = std::fs::remove_dir_all(dir.join("native"));
    std::fs::create_dir_all(&lib).map_err(|e| format!("couldn't make {}: {e}", lib.display()))?;
    let copy_in = |file: &str| -> Result<String, String> {
        let from = Path::new(file);
        let name = from
            .file_name()
            .ok_or_else(|| format!("Cargo named a library without a file: {file}"))?;
        std::fs::copy(from, lib.join(name)).map_err(|e| format!("couldn't copy {file}: {e}"))?;
        Ok(format!("lib/{}", name.to_string_lossy()))
    };
    let mut ready = String::new();
    let runtime = made
        .rlibs
        .iter()
        .find(|(id, f)| {
            id.contains("lust_rt")
                && Path::new(f)
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("liblust_rt"))
        })
        .ok_or("Lust's runtime built, but Cargo didn't say where its library went.")?;
    let mut copied = std::collections::HashMap::new();
    for (_, file) in &made.rlibs {
        if !copied.contains_key(file) {
            copied.insert(file.clone(), copy_in(file)?);
        }
    }
    ready.push_str(&copied[&runtime.1]);
    ready.push('\n');
    for (name, file) in rlibs_wanted(&made, &direct_deps()) {
        ready.push_str(&format!("extern {name}={}\n", copied[&file]));
    }
    let mut folders: Vec<&String> = Vec::new();
    for at in &made.link_paths {
        if !folders.contains(&at) {
            folders.push(at);
        }
    }
    for (n, at) in folders.into_iter().enumerate() {
        // kind=folder, or just a folder.
        let (kind, folder) = at
            .split_once('=')
            .filter(|(k, _)| !k.contains(['/', '\\', ':']))
            .unwrap_or(("all", at));
        let to = dir.join("native").join(n.to_string());
        copy_folder(Path::new(folder), &to)?;
        ready.push_str(&format!("{kind}=native/{n}\n"));
    }
    let _ = std::fs::remove_dir_all(&target);
    std::fs::write(dir.join("ready.txt"), ready).map_err(|e| format!("couldn't write {}: {e}", dir.join("ready.txt").display()))
}

/// Copies a folder and everything in it.
fn copy_folder(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("couldn't make {}: {e}", to.display()))?;
    for entry in std::fs::read_dir(from)
        .map_err(|e| format!("couldn't look in {}: {e}", from.display()))?
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            copy_folder(&path, &to.join(entry.file_name()))?;
        } else {
            std::fs::copy(&path, to.join(entry.file_name())).map_err(|e| format!("couldn't copy {}: {e}", path.display()))?;
        }
    }
    Ok(())
}

/// Builds a program that uses outside libraries (databases, views, the web) against the shared
/// runtime, so only the program's own code is compiled. A release build compiles the program
/// with its own copy of the runtime instead, as one, so Rust can make the two as fast as they
/// can be together; the outside libraries are still the ones compiled already.
fn build_with_runtime(path: &Path, rust: &str, exe: &Path, release: bool) -> Result<(), String> {
    let rt = ensure_runtime()?;
    let program = if release {
        rust.to_string()
    } else {
        rust::with_prebuilt_runtime(rust).ok_or("this Rust has no runtime to share, so it can't be built as a program.")?
    };
    let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    // In the runtime's folder, where a program's own copy of the runtime finds Lust's fonts.
    let dir = rt.dir.join("programs");
    std::fs::create_dir_all(&dir).map_err(|e| format!("couldn't make a build folder: {e}"))?;
    let src = dir.join(format!("{stem}-{}.rs", std::process::id()));
    std::fs::write(&src, &program).map_err(|e| format!("couldn't write {}: {e}", src.display()))?;
    let mut externs = Vec::new();
    if release {
        for (name, file) in &rt.externs {
            let mut arg = std::ffi::OsString::from(format!("{name}="));
            arg.push(file);
            externs.push(arg);
        }
    } else {
        let mut arg = std::ffi::OsString::from("lust_rt=");
        arg.push(&rt.rlib);
        externs.push(arg);
    }
    let mut deps_arg = std::ffi::OsString::from("dependency=");
    deps_arg.push(&rt.deps);
    let mut cmd = std::process::Command::new("rustc");
    // Without symbols: they only name Rust functions, and make the program several times bigger.
    cmd.args([
        "--edition",
        "2021",
        "-O",
        "-C",
        "overflow-checks=on",
        "-C",
        "strip=symbols",
        "--crate-name",
        "lust_program",
        "-L",
    ])
    .arg(deps_arg);
    for e in externs {
        cmd.arg("--extern").arg(e);
    }
    for at in &rt.search {
        cmd.arg("-L").arg(at);
    }
    let result = cmd.arg("-o").arg(exe).arg(&src).output();
    let _ = std::fs::remove_file(&src);
    match result {
        Err(_) => Err(NEEDS_RUST.into()),
        Ok(o) if !o.status.success() && String::from_utf8_lossy(&o.stderr).contains("link.exe") => Err(NEEDS_TOOLS.into()),
        Ok(o) if !o.status.success() => {
            let keep = dir.join(format!("{stem}.rs"));
            let _ = std::fs::write(&keep, &program);
            Err(format!(
                "Lust made Rust that doesn't build. That's a bug in Lust, not your program.\nThe Rust is in {}. Rust said:\n{}",
                keep.display(),
                String::from_utf8_lossy(&o.stderr)
            ))
        }
        Ok(_) => Ok(()),
    }
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
    // Built programs share compiled libraries (SQLite and the rest) that take a minute or two to
    // make the first time on a computer. That happens in the background, so lust new itself is
    // instant; once it's done, the first real build is quick.
    if !warm_ready() {
        if which("cargo") {
            if start_warm_up() {
                println!("Built programs are getting ready in the background (once on this computer).");
            }
        } else {
            println!("lust run works now. lust build needs Rust: get it from https://rustup.rs.");
        }
    }
    println!();
    println!("  cd {name}");
    println!("  lust run main.lst");
    println!("  lust build main.lst");
    ExitCode::SUCCESS
}

/// Whether the shared libraries built programs use are compiled on this computer already.
fn warm_ready() -> bool {
    runtime_dir().map(|d| read_runtime(&d).is_some()).unwrap_or(false)
}

fn which(tool: &str) -> bool {
    std::process::Command::new(tool)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Starts `lust warm-up` on its own, detached: no window, no output, and it keeps going after
/// lust new (or the editor that ran it) is done.
fn start_warm_up() -> bool {
    let Ok(me) = std::env::current_exe() else { return false };
    let spawn = |flags: u32| {
        let mut c = std::process::Command::new(&me);
        c.arg("warm-up")
            .current_dir(std::env::temp_dir())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(flags);
        }
        #[cfg(not(windows))]
        let _ = flags;
        c.spawn().is_ok()
    };
    // Windows gives a new process a copy of every handle that may be passed on, and this one's
    // output may be an editor's pipe. The warm-up would then hold that pipe open for minutes, and
    // the editor would wait for it to close before it calls lust new done.
    #[cfg(windows)]
    keep_output_to_myself();
    // CREATE_NO_WINDOW | DETACHED_PROCESS, and out of the starter's job object where that's allowed
    // (CREATE_BREAKAWAY_FROM_JOB), so closing the editor doesn't stop it half-way.
    spawn(0x0800_0000 | 0x0000_0008 | 0x0100_0000) || spawn(0x0800_0000 | 0x0000_0008)
}

/// Stops this process's input, output and error being passed on to processes it starts.
#[cfg(windows)]
fn keep_output_to_myself() {
    type Handle = *mut core::ffi::c_void;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(which: u32) -> Handle;
        fn SetHandleInformation(handle: Handle, mask: u32, flags: u32) -> i32;
    }
    const HANDLE_FLAG_INHERIT: u32 = 1;
    // STD_INPUT_HANDLE, STD_OUTPUT_HANDLE and STD_ERROR_HANDLE: -10, -11 and -12.
    for which in [-10i32, -11, -12] {
        // SAFETY: GetStdHandle takes any value and gives back a handle, null or invalid (-1);
        // SetHandleInformation only changes a flag on a valid one, and fails harmlessly otherwise.
        unsafe {
            let h = GetStdHandle(which as u32);
            if !h.is_null() && h as isize != -1 {
                SetHandleInformation(h, HANDLE_FLAG_INHERIT, 0);
            }
        }
    }
}

/// `lust warm-up`: builds a small program with a database once, in a scratch folder, so the
/// libraries built programs share are compiled. One at a time; quietly does nothing when they're
/// ready already.
fn warm_up() -> ExitCode {
    match ensure_runtime() {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(1),
    }
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
    let mut target_deps = String::new();
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
        deps.push_str("winit = \"0.30\"\naccesskit = \"0.25\"\naccesskit_winit = \"0.34\"\nsoftbuffer = \"0.4\"\nab_glyph = \"0.2\"\narboard = { version = \"3\", default-features = false }\npng = \"0.17\"\njpeg-decoder = { version = \"0.3\", default-features = false }\nresvg = { version = \"0.49\", default-features = false }\n");
        target_deps.push_str(UI_TARGET_DEPS);
        let fonts = dir.join("assets").join("fonts");
        std::fs::create_dir_all(&fonts).map_err(|e| format!("couldn't make {}: {e}", fonts.display()))?;
        for (file, bytes) in native_ui::FONT_FILES.iter().zip(native_ui::FONTS.iter()) {
            std::fs::write(fonts.join(file), bytes).map_err(|e| format!("couldn't write the font {file}: {e}"))?;
        }
    }
    let toml = format!(
        "# Made by Lust from {}. lust build --library writes it again, so don't edit it.\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n\n[dependencies]\n{deps}{target_deps}",
        path.file_name().unwrap_or_default().to_string_lossy()
    );
    std::fs::write(dir.join("Cargo.toml"), toml).map_err(|e| format!("couldn't write Cargo.toml: {e}"))?;
    std::fs::write(dir.join("src").join("lib.rs"), rust).map_err(|e| format!("couldn't write lib.rs: {e}"))?;
    let out = std::process::Command::new("cargo")
        .args(["check", "--quiet", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", cache_dir().join("target"))
        .output()
        .map_err(|_| NEEDS_RUST.to_string())?;
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
        .map_err(|_| NEEDS_RUST.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "the Rust program using it didn't build:\n{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(target.join("release").join(format!("{bin}{}", std::env::consts::EXE_SUFFIX)))
}

pub fn build_native(path: &Path, rust: &str) -> Result<std::path::PathBuf, String> {
    build_native_with(path, rust, false)
}

/// lust build: a program next to the .lst file. `release` takes longer, so that Rust can make
/// the program's code and the shared runtime's into one, as fast as it can be.
pub fn build_native_with(path: &Path, rust: &str, release: bool) -> Result<std::path::PathBuf, String> {
    if ["db", "web", "net", "pat", "ui", "dlg"]
        .iter()
        .any(|m| rust.contains(&format!("\npub mod {m} {{\n")))
    {
        let exe = path.with_extension(std::env::consts::EXE_EXTENSION);
        build_with_runtime(path, rust, &exe, release)?;
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
        Err(_) => Err(NEEDS_RUST.into()),
        Ok(o) if !o.status.success() && String::from_utf8_lossy(&o.stderr).contains("link.exe") => Err(NEEDS_TOOLS.into()),
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

#[cfg(test)]
mod shared_runtime {
    use super::*;

    #[test]
    fn the_libraries_cargo_toml_names() {
        let deps = direct_deps();
        assert!(deps.contains(&("rusqlite".into(), "0.40".into())));
        assert!(deps.contains(&("jpeg-decoder".into(), "0.3".into())));
        assert!(deps.contains(&("tray-icon".into(), "0.25".into())));
        assert!(deps.iter().all(|(n, _)| !n.starts_with('[')));
    }

    #[test]
    fn reads_what_cargo_made() {
        let messages = concat!(
            r#"{"reason":"compiler-artifact","package_id":"registry+https://github.com/rust-lang/crates.io-index#png@0.18.0","target":{"name":"png"},"filenames":["/c/deps/libpng-new.rlib","/c/deps/libpng-new.rmeta"]}"#,
            "\n",
            r#"{"reason":"compiler-artifact","package_id":"registry+https://github.com/rust-lang/crates.io-index#png@0.17.16","target":{"name":"png"},"filenames":["/c/deps/libpng-old.rlib","/c/deps/libpng-old.rmeta"]}"#,
            "\n",
            r#"{"reason":"build-script-executed","package_id":"x#libsqlite3-sys@0.3","linked_libs":["static=sqlite3"],"linked_paths":["native=/c/build/sqlite/abc/out"],"cfgs":[]}"#,
            "\n",
            r#"{"reason":"compiler-artifact","package_id":"jpeg-decoder 0.3.1 (registry+https://github.com/rust-lang/crates.io-index)","target":{"name":"jpeg_decoder"},"filenames":["C:\\Users\\a\\deps\\libjpeg_decoder-1.rlib"]}"#,
            "\n",
            r#"{"reason":"compiler-artifact","package_id":"x#serde_derive@1.0.0","target":{"kind":["proc-macro"]},"filenames":["/c/deps/libserde_derive-1.so"]}"#,
            "\n",
            r#"{"reason":"build-finished","success":true}"#,
        );
        let made = cargo_made(messages);
        assert_eq!(made.rlibs.len(), 4);
        assert_eq!(made.rlibs[3].1, "/c/deps/libserde_derive-1.so");
        assert_eq!(made.link_paths, vec!["native=/c/build/sqlite/abc/out".to_string()]);
        assert_eq!(made.rlibs[2].1, "C:\\Users\\a\\deps\\libjpeg_decoder-1.rlib");
        let wanted = vec![
            ("png".to_string(), "0.17".to_string()),
            ("jpeg-decoder".to_string(), "0.3".to_string()),
            ("zbus".to_string(), "5".to_string()),
        ];
        assert_eq!(
            rlibs_wanted(&made, &wanted),
            vec![
                ("png".to_string(), "/c/deps/libpng-old.rlib".to_string()),
                ("jpeg_decoder".to_string(), "C:\\Users\\a\\deps\\libjpeg_decoder-1.rlib".to_string())
            ]
        );
        assert_eq!(cargo_made("not json\n"), CargoMade::default());
    }

    #[test]
    fn reads_what_a_runtime_says_about_itself() {
        let dir = std::env::temp_dir().join(format!("lust-ready-{}", std::process::id()));
        let deps = dir.join("target").join("release").join("deps");
        std::fs::create_dir_all(&deps).unwrap();
        std::fs::write(deps.join("liblust_rt-1.rlib"), "").unwrap();
        std::fs::write(
            dir.join("ready.txt"),
            "target/release/deps/liblust_rt-1.rlib\nextern rusqlite=target/release/deps/librusqlite-2.rlib\nnative=target/release/build/sqlite/out\n",
        )
        .unwrap();
        let rt = read_runtime(&dir).unwrap();
        assert_eq!(rt.rlib, deps.join("liblust_rt-1.rlib"));
        assert_eq!(
            rt.externs,
            vec![("rusqlite".to_string(), dir.join("target/release/deps/librusqlite-2.rlib"))]
        );
        assert_eq!(
            rt.search,
            vec![format!("native={}", dir.join("target/release/build/sqlite/out").display())]
        );
        // Not ready without its library.
        std::fs::remove_file(deps.join("liblust_rt-1.rlib")).unwrap();
        assert!(read_runtime(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
