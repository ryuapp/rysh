use crate::commands;
use crate::parser::{Command as AstCommand, ListItem, Pipeline, RedirectKind, Word, parse};
use crate::path::{is_explicit_path, shell_path};
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Default)]
pub struct RunOptions {}

#[derive(Debug)]
pub struct Shell {
    pub(crate) vars: HashMap<String, String>,
    last_status: i32,
    exit_status: Option<i32>,
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Shell {
    pub fn new() -> Self {
        Self {
            vars: env::vars().collect(),
            last_status: 0,
            exit_status: None,
        }
    }

    pub fn run_script(&mut self, source: &str, _options: RunOptions) -> Result<i32> {
        let list = parse(source)?;
        let mut status = 0;

        for (op, pipeline) in list.items {
            let should_run = match op {
                ListItem::Always => true,
                ListItem::AndIf => status == 0,
                ListItem::OrIf => status != 0,
            };
            if should_run {
                status = self.run_pipeline(&pipeline)?;
                self.last_status = status;
                if self.exit_status.is_some() {
                    break;
                }
            }
        }

        Ok(status)
    }

    pub fn run_script_capture(&mut self, source: &str) -> Result<(i32, Vec<u8>)> {
        let list = parse(source)?;
        let mut status = 0;
        let mut stdout = Vec::new();

        for (op, pipeline) in list.items {
            let should_run = match op {
                ListItem::Always => true,
                ListItem::AndIf => status == 0,
                ListItem::OrIf => status != 0,
            };
            if should_run {
                let output = self.run_pipeline_capture(&pipeline)?;
                status = output.status;
                stdout.extend(output.stdout);
                self.last_status = status;
                if self.exit_status.is_some() {
                    break;
                }
            }
        }

        Ok((status, stdout))
    }

    pub(crate) fn resolve_program(&self, name: &str) -> Option<PathBuf> {
        resolve_program(name, &self.vars).ok()
    }

    pub fn take_exit_status(&mut self) -> Option<i32> {
        self.exit_status.take()
    }

    fn run_pipeline(&mut self, pipeline: &Pipeline) -> Result<i32> {
        Ok(self.run_pipeline_inner(pipeline, false)?.status)
    }

    fn run_pipeline_capture(&mut self, pipeline: &Pipeline) -> Result<CommandOutput> {
        self.run_pipeline_inner(pipeline, true)
    }

    fn run_pipeline_inner(
        &mut self,
        pipeline: &Pipeline,
        capture_stdout: bool,
    ) -> Result<CommandOutput> {
        if pipeline.commands.len() == 1 {
            return self.run_command(&pipeline.commands[0], None, capture_stdout);
        }

        let mut input = None;
        let mut status = 0;
        let mut stdout = Vec::new();
        for (idx, command) in pipeline.commands.iter().enumerate() {
            let is_last = idx + 1 == pipeline.commands.len();
            let output = self.run_command(command, input.take(), !is_last || capture_stdout)?;
            status = output.status;
            if output.exit {
                return Ok(output);
            }
            if is_last {
                stdout = output.stdout;
            } else {
                input = Some(output.stdout);
            }
        }
        Ok(CommandOutput {
            status,
            stdout,
            exit: false,
        })
    }

    fn run_command(
        &mut self,
        command: &AstCommand,
        stdin_bytes: Option<Vec<u8>>,
        capture_stdout: bool,
    ) -> Result<CommandOutput> {
        let mut env_overlay = HashMap::new();
        for assignment in &command.assignments {
            env_overlay.insert(
                assignment.name.clone(),
                self.expand_word(&assignment.value)?,
            );
        }

        if command.args.is_empty() {
            self.vars.extend(env_overlay);
            return Ok(CommandOutput::status(0));
        }

        let args = self.expand_words(&command.args, &env_overlay)?;
        let name = args.first().context("missing command name")?;
        let argv = &args[1..];

        if let Some(output) = self.run_builtin(name, argv, &env_overlay, command, capture_stdout)? {
            return Ok(output);
        }

        self.run_external(
            name,
            argv,
            &env_overlay,
            command,
            stdin_bytes,
            capture_stdout,
        )
    }

    fn run_builtin(
        &mut self,
        name: &str,
        argv: &[String],
        env_overlay: &HashMap<String, String>,
        command: &AstCommand,
        capture_stdout: bool,
    ) -> Result<Option<CommandOutput>> {
        let Some(result) = commands::run(self, name, argv, env_overlay)? else {
            return Ok(None);
        };

        write_builtin_streams(command, capture_stdout, &result.stdout, &result.stderr)?;
        if result.exit {
            self.exit_status = Some(result.status);
        }
        Ok(Some(CommandOutput {
            status: result.status,
            stdout: result.stdout,
            exit: result.exit,
        }))
    }

    fn run_external(
        &self,
        name: &str,
        argv: &[String],
        env_overlay: &HashMap<String, String>,
        ast: &AstCommand,
        stdin_bytes: Option<Vec<u8>>,
        capture_stdout: bool,
    ) -> Result<CommandOutput> {
        let program = resolve_program(name, &self.vars)?;
        let mut command = Command::new(program);
        command.args(argv);
        command.envs(&self.vars);
        command.envs(env_overlay);

        if stdin_bytes.is_some() {
            command.stdin(Stdio::piped());
        } else if let Some(path) = redirect_path(ast, RedirectKind::Stdin, self)? {
            command.stdin(File::open(path)?);
        }

        if capture_stdout {
            command.stdout(Stdio::piped());
        } else if let Some((path, append)) = stdout_redirect(ast, self)? {
            command.stdout(open_output(path, append)?);
        }

        if let Some((path, append)) = stderr_redirect(ast, self)? {
            command.stderr(open_output(path, append)?);
        }

        let mut child = command
            .spawn()
            .with_context(|| format!("failed to run {name}"))?;
        if let Some(bytes) = stdin_bytes
            && let Some(mut stdin) = child.stdin.take()
        {
            stdin.write_all(&bytes)?;
        }

        let output = child.wait_with_output()?;
        Ok(CommandOutput {
            status: output.status.code().unwrap_or(1),
            stdout: output.stdout,
            exit: false,
        })
    }

    fn expand_words(
        &self,
        words: &[Word],
        overlay: &HashMap<String, String>,
    ) -> Result<Vec<String>> {
        words
            .iter()
            .map(|word| self.expand_word_with(word, overlay))
            .collect()
    }

    fn expand_word(&self, word: &Word) -> Result<String> {
        self.expand_word_with(word, &HashMap::new())
    }

    fn expand_word_with(&self, word: &Word, overlay: &HashMap<String, String>) -> Result<String> {
        let mut out = String::new();
        let mut chars = word.raw.chars().peekable();
        let mut single = false;
        let mut double = false;

        while let Some(ch) = chars.next() {
            match ch {
                '\'' if !double => single = !single,
                '"' if !single => double = !double,
                '\\' if !single => {
                    if let Some(next) = chars.next() {
                        out.push(next);
                    }
                }
                '$' if !single => {
                    if matches!(chars.peek(), Some('?')) {
                        chars.next();
                        out.push_str(&self.last_status.to_string());
                    } else if matches!(chars.peek(), Some('(')) {
                        chars.next();
                        let source = read_command_substitution(&mut chars)?;
                        let (_, mut stdout) =
                            self.clone_for_substitution().run_script_capture(&source)?;
                        trim_trailing_newlines(&mut stdout);
                        out.push_str(&String::from_utf8_lossy(&stdout));
                    } else if matches!(chars.peek(), Some('{')) {
                        chars.next();
                        let mut name = String::new();
                        for next in chars.by_ref() {
                            if next == '}' {
                                break;
                            }
                            name.push(next);
                        }
                        out.push_str(resolve_var(&name, overlay, &self.vars).unwrap_or_default());
                    } else {
                        let mut name = String::new();
                        while let Some(next) = chars.peek().copied() {
                            if next == '_' || next.is_ascii_alphanumeric() {
                                name.push(next);
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        if name.is_empty() {
                            out.push('$');
                        } else {
                            out.push_str(
                                resolve_var(&name, overlay, &self.vars).unwrap_or_default(),
                            );
                        }
                    }
                }
                _ => out.push(ch),
            }
        }

        Ok(out)
    }

    fn clone_for_substitution(&self) -> Self {
        Self {
            vars: self.vars.clone(),
            last_status: self.last_status,
            exit_status: None,
        }
    }
}

fn read_command_substitution(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<String> {
    let mut depth = 1usize;
    let mut source = String::new();
    let mut single = false;
    let mut double = false;
    let mut escaped = false;

    for ch in chars.by_ref() {
        if escaped {
            escaped = false;
            source.push(ch);
            continue;
        }
        match ch {
            '\\' if !single => {
                escaped = true;
                source.push(ch);
            }
            '\'' if !double => {
                single = !single;
                source.push(ch);
            }
            '"' if !single => {
                double = !double;
                source.push(ch);
            }
            '(' if !single => {
                depth += 1;
                source.push(ch);
            }
            ')' if !single => {
                depth -= 1;
                if depth == 0 {
                    return Ok(source);
                }
                source.push(ch);
            }
            _ => source.push(ch),
        }
    }

    bail!("unterminated command substitution")
}

fn trim_trailing_newlines(bytes: &mut Vec<u8>) {
    while matches!(bytes.last(), Some(b'\n' | b'\r')) {
        bytes.pop();
    }
}

fn resolve_var<'a>(
    name: &str,
    overlay: &'a HashMap<String, String>,
    vars: &'a HashMap<String, String>,
) -> Option<&'a str> {
    overlay
        .get(name)
        .or_else(|| vars.get(name))
        .map(String::as_str)
}

fn write_builtin_streams(
    command: &AstCommand,
    capture_stdout: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<()> {
    if !capture_stdout && !stdout.is_empty() {
        if let Some((path, append)) = stdout_redirect(command, &Shell::new())? {
            let mut file = open_output(path, append)?;
            file.write_all(stdout)?;
        } else {
            io::stdout().write_all(stdout)?;
        }
    }
    if !stderr.is_empty() {
        if let Some((path, append)) = stderr_redirect(command, &Shell::new())? {
            let mut file = open_output(path, append)?;
            file.write_all(stderr)?;
        } else {
            io::stderr().write_all(stderr)?;
        }
    }
    Ok(())
}

fn stdout_redirect(command: &AstCommand, shell: &Shell) -> Result<Option<(PathBuf, bool)>> {
    for redirect in command.redirects.iter().rev() {
        match redirect.kind {
            RedirectKind::StdoutTruncate => {
                return Ok(Some((
                    expand_redirect_target(redirect.target.as_ref(), shell)?,
                    false,
                )));
            }
            RedirectKind::StdoutAppend => {
                return Ok(Some((
                    expand_redirect_target(redirect.target.as_ref(), shell)?,
                    true,
                )));
            }
            _ => {}
        }
    }
    Ok(None)
}

fn stderr_redirect(command: &AstCommand, shell: &Shell) -> Result<Option<(PathBuf, bool)>> {
    for redirect in command.redirects.iter().rev() {
        match redirect.kind {
            RedirectKind::StderrTruncate => {
                return Ok(Some((
                    expand_redirect_target(redirect.target.as_ref(), shell)?,
                    false,
                )));
            }
            RedirectKind::StderrAppend => {
                return Ok(Some((
                    expand_redirect_target(redirect.target.as_ref(), shell)?,
                    true,
                )));
            }
            RedirectKind::StderrToStdout => {
                if let Some(stdout) = stdout_redirect(command, shell)? {
                    return Ok(Some(stdout));
                }
            }
            _ => {}
        }
    }
    Ok(None)
}

fn redirect_path(
    command: &AstCommand,
    kind: RedirectKind,
    shell: &Shell,
) -> Result<Option<PathBuf>> {
    for redirect in command.redirects.iter().rev() {
        if redirect.kind == kind {
            return Ok(Some(expand_redirect_target(
                redirect.target.as_ref(),
                shell,
            )?));
        }
    }
    Ok(None)
}

fn expand_redirect_target(target: Option<&Word>, shell: &Shell) -> Result<PathBuf> {
    let target = target.context("redirect target missing")?;
    Ok(shell_path(&shell.expand_word(target)?))
}

fn open_output(path: PathBuf, append: bool) -> Result<File> {
    Ok(OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(!append)
        .append(append)
        .open(path)?)
}

fn resolve_program(name: &str, vars: &HashMap<String, String>) -> Result<PathBuf> {
    let path = Path::new(name);
    if is_explicit_path(name) || path.components().count() > 1 {
        return Ok(shell_path(name));
    }

    let paths = vars
        .get("PATH")
        .or_else(|| vars.get("Path"))
        .cloned()
        .unwrap_or_default();
    let path_ext = vars
        .get("PATHEXT")
        .cloned()
        .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_string());
    let exts: Vec<_> = path_ext.split(';').collect();
    let has_ext = Path::new(name).extension().is_some();

    for dir in env::split_paths(&paths) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
        if !has_ext {
            for ext in &exts {
                let candidate = dir.join(format!("{name}{ext}"));
                if candidate.is_file() {
                    return Ok(candidate);
                }
            }
        }
    }

    bail!("{name}: command not found")
}

#[derive(Debug)]
struct CommandOutput {
    status: i32,
    stdout: Vec<u8>,
    exit: bool,
}

impl CommandOutput {
    fn status(status: i32) -> Self {
        Self {
            status,
            stdout: Vec::new(),
            exit: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_quotes_and_vars() {
        let mut shell = Shell::new();
        shell.vars.insert("FOO".into(), "bar".into());
        assert_eq!(
            shell
                .expand_word(&Word::new("'$FOO' \"$FOO\" \\$FOO"))
                .unwrap(),
            "$FOO bar $FOO"
        );
    }

    #[test]
    fn handles_and_or_status() {
        let mut shell = Shell::new();
        assert_eq!(
            shell
                .run_script("false && exit 9 || true", RunOptions::default())
                .unwrap(),
            0
        );
    }

    #[test]
    fn exit_stops_following_commands() {
        let mut shell = Shell::new();
        let (status, stdout) = shell.run_script_capture("exit 7; echo no").unwrap();

        assert_eq!(status, 7);
        assert_eq!(shell.take_exit_status(), Some(7));
        assert!(stdout.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn cd_accepts_msys_drive_path() {
        let current_dir = env::current_dir().unwrap();
        let drive = current_dir
            .display()
            .to_string()
            .chars()
            .next()
            .unwrap()
            .to_ascii_lowercase();
        let mut shell = Shell::new();

        assert_eq!(
            shell
                .run_script(&format!("cd /{drive}/"), RunOptions::default())
                .unwrap(),
            0
        );
    }

    #[test]
    fn failed_cd_does_not_execute_argument_as_command() {
        let mut shell = Shell::new();
        let (status, stdout) = shell
            .run_script_capture(&format!("cd echo 2> {}", null_device()))
            .unwrap();

        assert_eq!(status, 1);
        assert!(stdout.is_empty());
    }

    #[cfg(windows)]
    fn null_device() -> &'static str {
        "NUL"
    }

    #[cfg(not(windows))]
    fn null_device() -> &'static str {
        "/dev/null"
    }
}
