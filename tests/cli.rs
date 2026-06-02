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
