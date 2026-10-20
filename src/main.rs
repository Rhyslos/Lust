//! The `lust` command.

mod ast;
mod check;
mod diag;
mod lexer;
mod lua;
mod parser;
mod run;
mod tests;
mod types;

use std::path::Path;
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = "Lust 2 (alpha)

  lust run <file>      check a .lust or .lsc file, then run it
  lust check <file>    only check it, and list any mistakes
  lust lua <file>      show the Lua a .lsc file turns into
  lust build <file>    write the Lua next to a .lsc file, as file.lua
  lust test <folder>   run the test programs in a folder
  lust version         show which Lust this is

lust <file> is the same as lust run <file>.
For now, .lust files run through Lua too. Real programs, through Rust, come next.";

pub struct Compiled {
    pub out: lua::Output,
}

/// Reads, checks and writes one file. Mistakes are printed, and give None.
pub fn compile(path: &Path, quiet: bool) -> Option<Compiled> {
    let display = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    if ext != "lust" && ext != "lsc" {
        eprintln!("{display} isn't a Lust file. Lust files end in .lust, or .lsc for LustScript.");
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
    let toks = lexer::lex(&src, &mut diags);
    let prog = parser::parse(toks, &mut diags, is_script);
    let info = check::check(&prog, &mut diags, is_script);
    if !diags.is_empty() {
        if !quiet {
            let list = diags.sorted();
            for (pos, msg) in &list {
                eprintln!("{display}:{}:{}  {msg}", pos.line, pos.col);
            }
            eprintln!();
            eprintln!("{} mistake{}. Nothing was run.", list.len(), if list.len() == 1 { "" } else { "s" });
        }
        return None;
    }
    Some(Compiled {
        out: lua::write(&prog, &info),
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
        "build" => {
            if path.extension().map(|e| e == "lust").unwrap_or(false) {
                eprintln!("building .lust programs (through Rust) isn't ready yet. For now: lust run {display}");
                return ExitCode::from(1);
            }
            match compile(path, false) {
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
            }
        }
        "run" => match compile(path, false) {
            Some(c) => run::run(&c.out, &display),
            None => ExitCode::from(1),
        },
        other => {
            eprintln!("lust doesn't know the command {other}.\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}
