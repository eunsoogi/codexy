use super::workflow;

#[test]
fn release_workflows_use_supported_action_version_tags() -> Result<(), Box<dyn std::error::Error>> {
    for name in [
        "bootstrap-package.yml",
        "runtime-candidate.yml",
        "runtime-activation.yml",
        "plugin-runtime-binaries.yml",
        "plugin-version-bump.yml",
        "publish-version-release.yml",
        "verify-version-release.yml",
        "verify-release-edit.yml",
    ] {
        let document = workflow(name)?;
        for job in document["jobs"].as_mapping().ok_or("workflow jobs")?.values() {
            let Some(steps) = job["steps"].as_sequence() else {
                assert_eq!(job["uses"], "./.github/workflows/verify-version-release.yml");
                continue;
            };
            for step in steps {
                let Some(uses) = step["uses"].as_str() else {
                    continue;
                };
                assert!(
                    uses.rsplit_once('@').is_some_and(|(_, tag)|
                        !tag.is_empty() && !tag.bytes().all(|byte| byte.is_ascii_hexdigit())),
                    "workflow action must use a supported version tag: {uses}"
                );
            }
        }
    }
    Ok(())
}
