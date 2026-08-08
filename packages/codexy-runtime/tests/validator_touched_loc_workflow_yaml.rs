use crate::support;

use support::touched_loc::{fixture, regular_lines, stderr, validate, write};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn touched_loc_credits_only_genuine_workflow_run_scalars() -> TestResult {
    assert_yaml_extraction(
        "jobs:\n  release:\n    steps:\n      - run:scripts/reconcile-release\n",
        false,
    )?;

    for workflow in [
        "jobs:\n  release:\n    steps:\n      - run: scripts/reconcile-release --check\n",
        "jobs:\n  release:\n    steps:\n      - run: \"scripts/reconcile-release --check\"\n",
        "jobs:\n  release:\n    steps:\n      - \"run\": scripts/reconcile-release --check\n",
        "jobs:\n  release:\n    steps:\n      - {run: scripts/reconcile-release --check}\n",
        "jobs:\n  release:\n    steps:\n      - name: Release\n        run: scripts/reconcile-release --check\n",
        "jobs:\n  release:\n    steps:\n      - run: |\n          scripts/reconcile-release --check\n",
        "jobs:\n  release:\n    steps:\n      - run: >\n          scripts/reconcile-release\n          --check\n",
        "script: &script scripts/reconcile-release --check\njobs:\n  release:\n    steps:\n      - run: *script\n",
        "jobs:\n  release:\n    steps:\n      - &release\n        run: scripts/reconcile-release --check\n      - *release\n",
    ] {
        assert_yaml_extraction(workflow, true)?;
    }

    for workflow in [
        "jobs:\n  release:\n    steps:\n      - x-run: scripts/reconcile-release\n",
        "jobs:\n  release:\n    env:\n      run:scripts/reconcile-release\n    steps: []\n",
        "jobs:\n  release:\n    steps:\n      - with:\n          run:scripts/reconcile-release\n",
        "jobs:\n  release:\n    steps:\n      - scripts/reconcile-release\n",
        "jobs:\n  release:\n    steps:\n      - name: \"run:scripts/reconcile-release\"\n",
        "jobs:\n  release:\n    steps:\n      - run: [scripts/reconcile-release]\n",
        "jobs:\n  release:\n    steps:\n      - run: {command: scripts/reconcile-release}\n",
        "jobs:\n  release:\n    steps:\n      - run: null\n",
        "jobs:\n  release:\n    steps:\n      - run: 7\n",
        "jobs:\n  release:\n    steps:\n      - run: |\n          scripts/reconcile-release\n          echo unsafe\n",
        "jobs:\n  release:\n    steps:\n      - run: >\n          scripts/reconcile-release\n          | tee output\n",
        "run:scripts/reconcile-release\njobs: {}\n",
        "jobs:\n  release:\n    steps:\n      - run: scripts/reconcile-release\n    broken: [\n",
    ] {
        assert_yaml_extraction(workflow, false)?;
    }
    Ok(())
}

fn assert_yaml_extraction(workflow: &str, accepted: bool) -> TestResult {
    let baseline = format!(
        "name: fixture\njobs:\n  release:\n    steps:\n      - run: |\n{}",
        regular_lines(247)
    );
    let repo = fixture(".github/workflows/release.yml", baseline)?;
    write(repo.path(), ".github/workflows/release.yml", workflow)?;
    write(repo.path(), "scripts/reconcile-release", &regular_lines(247))?;
    let output = validate(repo.path())?;
    assert_eq!(output.status.success(), accepted, "{workflow}: {}", stderr(&output));
    Ok(())
}
