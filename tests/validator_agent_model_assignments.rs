use std::collections::BTreeMap;
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn packaged_agents_use_role_appropriate_gpt_5_6_models() -> TestResult {
    let expected = BTreeMap::from([
        ("codexy-architect", "gpt-5.6-sol"),
        ("codexy-auditor", "gpt-5.6-terra"),
        ("codexy-cartographer", "gpt-5.6-luna"),
        ("codexy-forge", "gpt-5.6-terra"),
        ("codexy-pathfinder", "gpt-5.6-sol"),
        ("codexy-scribe", "gpt-5.6-luna"),
        ("codexy-sculptor", "gpt-5.6-terra"),
        ("codexy-sentinel", "gpt-5.6-sol"),
        ("codexy-shipwright", "gpt-5.6-terra"),
        ("codexy-tracer", "gpt-5.6-sol"),
        ("codexy-warden", "gpt-5.6-sol"),
        ("codexy-weaver", "gpt-5.6-terra"),
    ]);
    let agents_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy/agents");
    let packaged = std::fs::read_dir(&agents_root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("codexy-") && name.ends_with(".toml"))
        })
        .count();

    assert_eq!(
        packaged,
        expected.len(),
        "specialist mapping must be complete"
    );
    for (name, expected_model) in expected {
        let path = agents_root.join(format!("{name}.toml"));
        let agent = toml::from_str::<toml::Value>(&std::fs::read_to_string(&path)?)?;
        assert_eq!(
            agent.get("model").and_then(toml::Value::as_str),
            Some(expected_model),
            "{} must reject stale, non-5.6, or role-inappropriate model assignments",
            path.display()
        );
    }

    Ok(())
}

#[test]
fn sentinel_uses_sol_with_xhigh_reasoning_and_not_ultra() -> TestResult {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy/agents/codexy-sentinel.toml");
    let agent = toml::from_str::<toml::Value>(&std::fs::read_to_string(path)?)?;

    assert_eq!(
        agent.get("model").and_then(toml::Value::as_str),
        Some("gpt-5.6-sol")
    );
    assert_eq!(
        agent
            .get("model_reasoning_effort")
            .and_then(toml::Value::as_str),
        Some("xhigh")
    );
    assert_ne!(
        agent
            .get("model_reasoning_effort")
            .and_then(toml::Value::as_str),
        Some("ultra")
    );

    Ok(())
}
