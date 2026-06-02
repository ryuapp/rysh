use anyhow::{Context, Result};
use rysh::{RunOptions, Shell};
use std::env;
use std::io::{self, Write};

fn main() {
    if let Err(err) = run() {
        eprintln!("rysh: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let mut shell = Shell::new();

    match args.next().as_deref() {
        Some("-c") => {
            let source = args.next().context("missing command after -c")?;
            let status = shell.run_script(&source, RunOptions::default())?;
            std::process::exit(status);
        }
        Some(path) => {
            let source = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read script {path}"))?;
            let status = shell.run_script(&source, RunOptions::default())?;
            std::process::exit(status);
        }
        None => repl(shell),
    }
}

fn repl(mut shell: Shell) -> Result<()> {
    let stdin = io::stdin();
    let mut line = String::new();

    loop {
        print!("{}> ", env::current_dir()?.display());
        io::stdout().flush()?;

        line.clear();
        if stdin.read_line(&mut line)? == 0 {
            println!();
            return Ok(());
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            continue;
        }
        let status = shell.run_script(trimmed, RunOptions { interactive: true })?;
        if status != 0 {
            eprintln!("status {status}");
        }
    }
}
