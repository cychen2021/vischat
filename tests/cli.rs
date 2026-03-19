use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vischat"))
}

#[test]
fn test_help_flag_prints_usage() {
    let output = bin().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vischat"));
}

#[test]
fn test_nonexistent_file_exits_nonzero() {
    let output = bin()
        .arg("/nonexistent/vischat_no_such_file.jsonl")
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_no_displayable_items_exits_ok_with_message() {
    // A system non-init record is ignored by the parser → all_items is empty
    let path = std::env::temp_dir().join("vischat_cli_no_items.jsonl");
    std::fs::write(
        &path,
        r#"{"type":"system","subtype":"other","session_id":"s","uuid":"u"}"#,
    )
    .unwrap();

    let output = bin().arg(path.to_str().unwrap()).output().unwrap();
    let _ = std::fs::remove_file(&path);

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No displayable items"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn test_missing_file_arg_exits_nonzero() {
    let output = bin().output().unwrap();
    assert!(!output.status.success());
}
