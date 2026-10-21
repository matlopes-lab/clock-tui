use std::path::PathBuf;
use std::process::Command;

#[test]
fn clickup_widget_scenarios() {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/clickup-widget.sh");
    let status = Command::new("bash")
        .arg(script)
        .status()
        .expect("run clickup widget regression tests");

    assert!(status.success(), "clickup widget scenarios failed");
}
