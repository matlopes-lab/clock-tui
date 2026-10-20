use std::path::PathBuf;
use std::process::Command;

#[test]
fn resources_widget_scenarios() {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/resources-widget.sh");
    let status = Command::new("bash")
        .arg(script)
        .status()
        .expect("run resources widget regression tests");

    assert!(status.success(), "resources widget scenarios failed");
}
