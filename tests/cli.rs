use std::process::Command;

#[test]
fn runs_inline_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_rysh"))
        .args(["-c", "echo hello"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");
}

#[test]
fn propagates_exit_status() {
    let status = Command::new(env!("CARGO_BIN_EXE_rysh"))
        .args(["-c", "false"])
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(1));
}

#[test]
fn supports_command_substitution() {
    let output = Command::new(env!("CARGO_BIN_EXE_rysh"))
        .args(["-c", "echo $(echo inner)"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "inner\n");
}

#[cfg(windows)]
#[test]
fn runs_script_from_msys_style_drive_path() {
    let script = std::env::temp_dir().join("rysh-msys-path-test.sh");
    std::fs::write(&script, "echo script\n").unwrap();
    let script = script.display().to_string().replace('\\', "/");
    let script = format!(
        "/{}/{}",
        script[..1].to_ascii_lowercase(),
        script[3..].trim_start_matches('/')
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rysh"))
        .arg(script)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "script\n");
}

#[test]
fn failed_cd_does_not_run_argument_as_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_rysh"))
        .args(["-c", "cd echo"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn command_v_reports_builtins_and_missing_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_rysh"))
        .args(["-c", "command -v cd definitely_missing_rysh_command"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "cd\n");
}

#[test]
fn type_reports_builtins() {
    let output = Command::new(env!("CARGO_BIN_EXE_rysh"))
        .args(["-c", "type cd"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "cd is a shell builtin\n"
    );
}
