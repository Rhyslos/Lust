//! `lust test <folder>`: runs every program that has an expected result next to it.
//!
//!   name.out   what it should print
//!   name.err   the mistakes Lust should report, or the error it should stop with
//!   name.in    what's typed in while it runs, for ask
//!
//! A program with neither .out nor .err is a helper, and is skipped.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

fn find(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            find(&p, out);
        } else if matches!(p.extension().and_then(|e| e.to_str()), Some("lust" | "lsc"))
            && (p.with_extension("out").exists() || p.with_extension("err").exists())
        {
            out.push(p);
        }
    }
}

fn clean(s: &str) -> String {
    s.replace("\r\n", "\n").trim_end().to_string()
}

pub fn run_folder(folder: &Path) -> ExitCode {
    if !folder.is_dir() {
        eprintln!("there's no folder called {}.", folder.display());
        return ExitCode::from(2);
    }
    let mut files = Vec::new();
    find(folder, &mut files);
    let exe = std::env::current_exe().expect("lust's own path");
    let mut failed = 0;
    for f in &files {
        let rel = f.strip_prefix(folder).unwrap_or(f).display().to_string();
        let dir = f.parent().unwrap();
        let name = f.file_name().unwrap();
        let input = f.with_extension("in");
        let mut cmd = Command::new(&exe);
        cmd.arg("run").arg(name).current_dir(dir).env("LUST_SEED", "1");
        cmd.stdin(if input.exists() {
            Stdio::from(std::fs::File::open(&input).unwrap())
        } else {
            Stdio::null()
        });
        let result = cmd.output().expect("running lust");
        let stdout = clean(&String::from_utf8_lossy(&result.stdout));
        let stderr = clean(&String::from_utf8_lossy(&result.stderr));
        let mut problems = Vec::new();
        if let Ok(want) = std::fs::read_to_string(f.with_extension("out")) {
            if clean(&want) != stdout {
                problems.push(format!(
                    "it should print:\n{}\nbut printed:\n{}",
                    indent(&clean(&want)),
                    indent(&stdout)
                ));
            }
        }
        match std::fs::read_to_string(f.with_extension("err")) {
            Ok(want) => {
                if clean(&want) != stderr {
                    problems.push(format!(
                        "the mistakes should be:\n{}\nbut were:\n{}",
                        indent(&clean(&want)),
                        indent(&stderr)
                    ));
                }
            }
            Err(_) => {
                if !result.status.success() {
                    problems.push(format!("it stopped with:\n{}", indent(&stderr)));
                }
            }
        }
        if problems.is_empty() {
            println!("  ok    {rel}");
        } else {
            failed += 1;
            println!("  FAIL  {rel}");
            for p in problems {
                println!("{}", indent(&p));
            }
        }
    }
    println!();
    if failed == 0 {
        println!("All {} tests passed.", files.len());
        ExitCode::SUCCESS
    } else {
        println!("{failed} of {} tests failed.", files.len());
        ExitCode::from(1)
    }
}

fn indent(s: &str) -> String {
    s.lines().map(|l| format!("        | {l}")).collect::<Vec<_>>().join("\n")
}
