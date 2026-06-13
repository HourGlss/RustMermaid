use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn trace_json_goes_to_stderr_without_corrupting_stdout() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_selkie"))
        .args(["--trace", "-", "-o", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn selkie");

    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"flowchart TD\n  A[Start] --> B[End]\n")
        .expect("write diagram");

    let output = child.wait_with_output().expect("wait for selkie");
    assert!(
        output.status.success(),
        "selkie failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr is utf8");

    assert!(stdout.contains("<svg"), "stdout should contain SVG");
    assert!(
        stderr
            .lines()
            .all(|line| serde_json::from_str::<serde_json::Value>(line).is_ok()),
        "stderr should contain JSON trace lines:\n{stderr}"
    );
    assert!(
        stderr.contains("\"name\":\"selkie.parse\""),
        "trace should include parse span:\n{stderr}"
    );
    assert!(
        stderr.contains("\"name\":\"selkie.render_with_config\""),
        "trace should include render span:\n{stderr}"
    );
}
