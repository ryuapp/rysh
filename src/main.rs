use anyhow::{Context, Result};
use rysh::{RunOptions, Shell};
use std::env;

mod terminal;

use terminal::{LineRead, Terminal};

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
    let mut terminal = Terminal::new()?;

    println!("rysh is coming...");
    #[cfg(not(windows))]
    println!("NOTE: rysh is only tuned for Windows.");

    loop {
        let prompt = format!("{}> ", rysh::display_path_for_cli(&env::current_dir()?));
        let line = match terminal.read_line(&prompt)? {
            LineRead::Line(line) => line,
            LineRead::Interrupted => continue,
            LineRead::Eof => return Ok(0),
        };

        if !line.is_empty()
            && let Err(err) = shell.run_script(&line, RunOptions::default())
        {
            eprintln!("rysh: {err:#}");
            continue;
        }
        if let Some(status) = shell.take_exit_status() {
            return Ok(status);
        }
    }
}
