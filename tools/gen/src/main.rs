//! Writes the generated files, or with `--check`, lists the stale ones and
//! fails.

use std::process::ExitCode;

fn main() -> ExitCode {
    let root = lsdocs_gen::workspace_root();
    let check = std::env::args().skip(1).any(|a| a == "--check");

    if check {
        match lsdocs_gen::stale(&root) {
            Ok(stale) if stale.is_empty() => {
                println!("generated files are up to date");
                ExitCode::SUCCESS
            }
            Ok(stale) => {
                eprintln!("generated files are out of date; run `nur gen`:");
                for path in stale {
                    eprintln!("  {path}");
                }
                ExitCode::FAILURE
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        }
    } else {
        match lsdocs_gen::write(&root) {
            Ok(written) => {
                println!("wrote {written} files");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        }
    }
}
