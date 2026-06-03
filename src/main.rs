use anyhow::{Context, Result};
use rysh::{RunOptions, Shell};
use std::env;
use std::io::{self, Write};

fn main() {
    match run() {
        Ok(status) => std::process::exit(status),
        Err(err) => {
            eprintln!("rysh: {err:#}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32> {
    let mut args = env::args().skip(1);
    let mut shell = Shell::new();

    match args.next().as_deref() {
        Some("-c") => {
            let source = args.next().context("missing command after -c")?;
            let status = shell.run_script(&source, RunOptions::default())?;
            Ok(status)
        }
        Some(path) => {
            let source = std::fs::read_to_string(rysh::path_for_cli(path))
                .with_context(|| format!("failed to read script {path}"))?;
            let status = shell.run_script(&source, RunOptions::default())?;
            Ok(status)
        }
        None => repl(shell),
    }
}

fn repl(mut shell: Shell) -> Result<i32> {
    let stdin = io::stdin();
    let mut line = String::new();

    loop {
        print!("{}> ", rysh::display_path_for_cli(&env::current_dir()?));
        io::stdout().flush()?;

        line.clear();
        if stdin.read_line(&mut line)? == 0 {
            println!();
            return Ok(0);
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            continue;
        }
        shell.run_script(trimmed, RunOptions::default())?;
        if let Some(status) = shell.take_exit_status() {
            return Ok(status);
        }
    }
}
