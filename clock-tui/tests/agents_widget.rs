use std::path::PathBuf;
use std::process::Command;

#[test]
fn agents_widget_scenarios() {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/agents-widget.sh");
    let status = Command::new("bash")
        .arg(script)
        .status()
        .expect("run agents widget regression tests");

    assert!(status.success(), "agents widget scenarios failed");
}
