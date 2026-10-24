//! The `lust` command. It lives in cli.rs, next to the compiler it uses.

fn main() -> std::process::ExitCode {
    lust::cli_main()
}
