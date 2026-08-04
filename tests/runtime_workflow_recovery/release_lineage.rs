use std::{fs, path::Path};

use serde_yaml::Value;

use crate::support;

#[test]
fn final_release_admits_explicit_lineage_before_publication() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/publish-version-release.yml");
    let publisher: Value = serde_yaml::from_str(&fs::read_to_string(path)?)?;
    let source = publisher["jobs"]["publish-v1-3-0"]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Verify selected protected-main source"))
        .and_then(|step| step["run"].as_str())
        .ok_or("protected main source verification")?;
    support::assert_structured_literals(source, "protected main commit admission", &[
        "for commit in \"$STAGING_SOURCE_COMMIT\" \"$ACTIVATION_COMMIT\"; do",
        "case \"$commit\" in *[!0-9a-f]*|'') exit 1 ;; esac",
        "test \"${#commit}\" -eq 40",
    ]);
    let step = publisher["jobs"]["publish-v1-3-0"]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Create and verify the only public version release"))
        .ok_or("final release step")?;
    for (name, input) in [
        ("STAGING_SOURCE_COMMIT", "staging_source_commit"),
        ("ACTIVATION_COMMIT", "activation_commit"),
        ("STAGING_RUN_ID", "staging_run_id"),
    ] {
        assert_eq!(step["env"][name], format!("${{{{ inputs.{input} }}}}"));
    }
    let release = step["run"].as_str().ok_or("final release run")?;
    assert_eq!(step["env"]["GH_TOKEN"], "${{ github.token }}");
    let create = release.find("gh release create v1.3.0").ok_or("version release")?;
    for required in [
        "test -n \"$STAGING_SOURCE_COMMIT\"",
        "test \"$(jq -r .source.stagingSourceCommit dist/runtime-release-receipt.json)\" = \"$STAGING_SOURCE_COMMIT\"",
        "git ls-remote --refs origin \"$tag_ref\"",
        "gh api --method POST --include \"repos/$GITHUB_REPOSITORY/git/refs\"",
        "-f ref=\"$tag_ref\" -f sha=\"$ACTIVATION_COMMIT\"",
        "tag_create_response=tag-create-response.txt",
        "remote v1.3.0 tag API admission failed",
        "remote v1.3.0 tag is not the exact lightweight activation ref",
        "FETCH_HEAD^{commit}",
        "cannot resolve remote v1.3.0 tag",
        "remote v1.3.0 tag does not match activation commit",
        "remote v1.3.0 tag changed during admission",
    ] {
        assert!(release.find(required).ok_or(required)? < create);
    }
    assert!(!release.lines().any(|line| {
        line.split_ascii_whitespace().collect::<Vec<_>>().windows(2).any(|words| words == ["git", "push"])
    }));
    support::assert_structured_literals(
        release,
        "exact-tag release creation",
        &["gh release create v1.3.0 --verify-tag"],
    );
    support::assert_structured_absent_literals(release, "exact-tag release creation", &["--target"]);
    Ok(())
}
