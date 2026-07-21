#[test]
fn workflows_use_node24_action_releases() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let expected_actions = [
        (
            ".github/workflows/python-package.yml",
            [
                "actions/checkout@v5",
                "actions/setup-python@v6",
                "actions/upload-artifact@v6",
                "actions/download-artifact@v7",
            ],
        ),
        (
            ".github/workflows/plugin-runtime-binaries.yml",
            [
                "actions/checkout@v5",
                "actions/setup-python@v6",
                "actions/upload-artifact@v6",
                "actions/download-artifact@v7",
            ],
        ),
        (
            ".github/workflows/plugin-version-bump.yml",
            [
                "actions/checkout@v5",
                "actions/setup-python@v6",
                "actions/upload-artifact@v6",
                "actions/download-artifact@v7",
            ],
        ),
        (
            ".github/workflows/rust-test.yml",
            [
                "actions/checkout@v5",
                "actions/setup-python@v6",
                "actions/upload-artifact@v6",
                "actions/download-artifact@v7",
            ],
        ),
        (
            ".github/workflows/touched-loc-gate.yml",
            [
                "actions/checkout@v5",
                "actions/setup-python@v6",
                "actions/upload-artifact@v6",
                "actions/download-artifact@v7",
            ],
        ),
    ];

    for (path, actions) in expected_actions {
        let workflow = std::fs::read_to_string(root.join(path))?;
        for action in actions {
            if workflow.contains(action) {
                continue;
            }
            assert!(
                !workflow.contains(action.split_once('@').ok_or("action reference")?.0),
                "{path} must use {action} when it uses that action"
            );
        }
    }
    Ok(())
}
