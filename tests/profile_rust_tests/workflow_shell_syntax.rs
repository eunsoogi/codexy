use super::super::GateFixture;

#[test]
fn gate_ignores_shell_data_that_mentions_the_full_workload(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    for command in [
        "echo cargo test --locked --all-targets",
        "echo ok;# $(cargo test --locked --all-targets)",
        r#"|
          cat <<'EOF'
          cargo test --locked --all-targets
          EOF"#,
        r#"|
          echo "$(
          printf harmless
          )""#,
    ] {
        std::fs::write(
            &fixture.workflow,
            format!("jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n      - run: {command}\n"),
        )?;
        assert!(fixture.run(&[])?.status.success(), "{command}");
    }
    Ok(())
}

#[test]
fn gate_ignores_numeric_heredoc_data_that_mentions_the_full_workload(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with(
        r#"|
          cat <<123
          cargo test --locked --all-targets
          123"#,
    )?;

    assert!(fixture.run(&[])?.status.success());
    Ok(())
}

#[test]
fn gate_ignores_multiline_quoted_data_that_mentions_the_full_workload(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with(
        r#"|
          echo "data
          cargo test --locked --all-targets
          still data""#,
    )?;

    assert!(fixture.run(&[])?.status.success());
    Ok(())
}

#[test]
fn gate_rejects_a_full_workload_after_quoted_heredoc_text(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with(
        r#"|
          echo "<<EOF"
          cargo test --locked --all-targets"#,
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[test]
fn gate_rejects_a_backslash_continued_full_workload(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with(
        r#"|
          cargo \
          test --locked --all-targets"#,
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[test]
fn gate_rejects_a_full_workload_after_an_arithmetic_shift(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with(
        r#"|
          x=$((1 << 2))
          cargo test --locked --all-targets"#,
    )?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[test]
fn gate_rejects_a_full_workload_after_an_escaped_heredoc_delimiter(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = trailing_workload_after_heredoc(r#"\EOF"#)?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[test]
fn gate_rejects_a_full_workload_after_an_adjacent_quoted_heredoc_delimiter(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = trailing_workload_after_heredoc(r#"E"OF""#)?;

    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[test]
fn gate_accepts_an_assignment_only_shell_step() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture_with("FLAG=value")?;

    assert!(fixture.run(&[])?.status.success());
    Ok(())
}

#[test]
fn gate_rejects_shell_wrapped_or_reordered_full_workloads(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    for command in [
        "timeout 180 cargo test --locked --all-targets",
        "time cargo test --locked --all-targets",
        "(cargo test --locked --all-targets)",
        "{ cargo test --locked --all-targets; }",
        "cargo --locked test --all-targets",
        "exec -a cargo0 cargo test --locked --all-targets",
        "/usr/bin/timeout 180 cargo test --locked --all-targets",
        "nice -n 1 cargo test --locked --all-targets",
        "if true; then cargo test --locked --all-targets; fi",
        "sh -c 'cargo test --locked --all-targets'",
        r#""cargo test --locked --all-targets""#,
        "/usr/bin/cargo test --locked --all-targets",
        r#"echo "$(cargo test --locked --all-targets)""#,
        "echo `cargo test --locked --all-targets`",
        r##"|
          echo "# $(cargo test --locked --all-targets)""##,
        r#"|
          echo prefix#$(cargo test --locked --all-targets)"#,
        r#"|
          echo "text # $(cargo test --locked --all-targets)""#,
        r#"|
          echo "it's $(cargo test --locked --all-targets)""#,
        r#"|
          echo "$(
          cargo test --locked --all-targets
          )""#,
    ] {
        std::fs::write(
            &fixture.workflow,
            format!(
                "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n      - run: {command}\n"
            ),
        )?;
        assert!(!fixture.run(&[])?.status.success(), "{command}");
    }
    Ok(())
}

#[test]
fn gate_distinguishes_sudo_workloads_from_cargo_run_arguments(
) -> Result<(), Box<dyn std::error::Error>> {
    for command in [
        "sudo cargo test --locked --all-targets",
        "sudo -u root cargo test --locked --all-targets",
    ] {
        let fixture = fixture_with(command)?;
        assert!(
            !fixture.run(&[])?.status.success(),
            "sudo must not hide a second workload: {command}"
        );
    }

    let fixture = fixture_with("cargo run --bin helper -- test --locked --all-targets")?;
    assert!(
        fixture.run(&[])?.status.success(),
        "arguments passed to cargo run are not a cargo test workload"
    );
    Ok(())
}

fn fixture_with(command: &str) -> Result<GateFixture, Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        format!("jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n      - run: {command}\n"),
    )?;
    Ok(fixture)
}

fn trailing_workload_after_heredoc(
    delimiter: &str,
) -> Result<GateFixture, Box<dyn std::error::Error>> {
    fixture_with(&format!(
        "|\n          cat <<{delimiter}\n          harmless\n          EOF\n          cargo test --locked --all-targets"
    ))
}
