use std::path::Path;
use std::process::Command;

#[test]
fn profiler_plan_excludes_only_overlapping_non_owner_tests(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let inventory = temp.path().join("inventory.json");
    let candidates = temp.path().join("candidates.json");
    let selected = temp.path().join("selected.json");
    std::fs::write(
        &inventory,
        r#"{"tests":["suite_all::agent::owner","suite_all::child_a::agent::nested"]}"#,
    )?;
    std::fs::write(&candidates, r#"["agent::owner","child_a::agent::nested"]"#)?;
    std::fs::write(&selected, r#"["agent::owner"]"#)?;

    let output = plan(&inventory, &candidates, &selected)?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(value["expected"], serde_json::json!(["suite_all::agent::owner"]));
    assert_eq!(value["exclusions"], serde_json::json!(["child_a::agent::nested"]));
    Ok(())
}

#[test]
fn profiler_plan_rejects_accidental_over_exclusion(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let inventory = temp.path().join("inventory.json");
    let candidates = temp.path().join("candidates.json");
    let selected = temp.path().join("selected.json");
    std::fs::write(&inventory, r#"{"tests":["suite_all::agent::owner"]}"#)?;
    std::fs::write(&candidates, r#"["agent::owner"]"#)?;
    std::fs::write(&selected, "[]")?;

    let output = plan(&inventory, &candidates, &selected)?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("selected"));
    Ok(())
}

#[test]
fn profiler_verify_normalizes_should_panic_and_rejects_extra_names(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let inventory = temp.path().join("inventory.json");
    let profiles = temp.path().join("profiles");
    std::fs::create_dir(&profiles)?;
    std::fs::write(&inventory, r#"{"tests":["suite_all::system::panics"]}"#)?;
    std::fs::write(
        profiles.join("system.json"),
        r#"{"name":"system","target":"suite_all","exitCode":0,"passed":1,"failed":0,"ignored":0,"tests":["suite_all::system::panics - should panic"],"durationSeconds":1,"metrics":{}}"#,
    )?;
    let coverage = temp.path().join("coverage.json");
    let output = verify(&inventory, &profiles, &coverage)?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    std::fs::write(
        profiles.join("extra.json"),
        r#"{"name":"extra","target":"suite_all","exitCode":0,"passed":1,"failed":0,"ignored":0,"tests":["suite_all::system::extra"],"durationSeconds":1,"metrics":{}}"#,
    )?;
    assert!(!verify(&inventory, &profiles, &coverage)?.status.success());
    Ok(())
}

fn plan(
    inventory: &Path,
    candidates: &Path,
    selected: &Path,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-windows-rust"))
        .args(["plan", "--inventory"])
        .arg(inventory)
        .args(["--cluster", "agent", "--candidates"])
        .arg(candidates)
        .args(["--selected"])
        .arg(selected)
        .output()?)
}

fn verify(
    inventory: &Path,
    profiles: &Path,
    coverage: &Path,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-windows-rust"))
        .args(["verify", "--inventory"])
        .arg(inventory)
        .args(["--profiles"])
        .arg(profiles)
        .args(["--coverage"])
        .arg(coverage)
        .output()?)
}
