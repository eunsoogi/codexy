use std::process::Command;

#[test]
fn target_summary_recovers_one_interleaved_completed_result()
-> Result<(), Box<dyn std::error::Error>> {
    let accounting = codexy_runtime::paths::repository_root()
        .join("scripts/profile_rust_accounting.py");
    let probe = r#"
import importlib.util, pathlib, sys, tempfile
from collections import Counter

path = pathlib.Path(sys.argv[1])
spec = importlib.util.spec_from_file_location("accounting", path)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
output = "\n".join((
    "     Running tests/suites/all.rs (target/debug/deps/suite_all-a)",
    "test support::interleaved ... okSyntax error from child stderr",
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
))
tests, targets, outcomes = module.observed_test_records(output)
expected = "suite_all::support::interleaved"
if tests.get(expected) != 1 or outcomes.get("ok") != 1 or targets != {"suite_all"}:
    raise SystemExit(f"tests={tests!r} targets={targets!r} outcomes={outcomes!r}")

def assert_no_inference(label, lines):
    tests, _, outcomes = module.observed_test_records("\n".join(lines))
    if tests or outcomes:
        raise SystemExit(f"{label}: tests={tests!r} outcomes={outcomes!r}")

assert_no_inference("failed summary", (
    "     Running tests/suites/all.rs (target/debug/deps/suite_all-b)",
    "test support::failed ... child failure",
    "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
))
assert_no_inference("ignored summary", (
    "     Running tests/suites/all.rs (target/debug/deps/suite_all-c)",
    "test support::ignored ... ignored by harness",
    "test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s",
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
))
assert_no_inference("non-adjacent summary", (
    "     Running tests/suites/all.rs (target/debug/deps/suite_all-d)",
    "test support::non_adjacent ... child output",
    "unrelated diagnostic output",
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
))
hosted_spliced = "\n".join((
    "     Running tests/suites/support_suite.rs (target/debug/deps/suite_support-hosted)",
    "test support_shared_fixture_tests::materialized_text_fixture_keeps_executable_mode_and_canonical_lf ... /tmp/.tmpsl13GD/shell-keyword-13: 3: okSyntax error: \")\" unexpected",
    "",
    "test support_shared_fixture_tests::text_fixture_normalization_preserves_raw_binary_reads ... ok",
    "test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s",
))
tests, _, outcomes = module.observed_test_records(hosted_spliced)
if tests.get("suite_support::support_shared_fixture_tests::materialized_text_fixture_keeps_executable_mode_and_canonical_lf") or outcomes.get("ok") != 1:
    raise SystemExit(f"hosted spliced completion: tests={tests!r} outcomes={outcomes!r}")
assert_no_inference("malformed summary", (
    "     Running tests/suites/all.rs (target/debug/deps/suite_all-e)",
    "test support::malformed ... child output",
    "test result: ok. passed count malformed",
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
))
assert_no_inference("target boundary", (
    "     Running tests/suites/all.rs (target/debug/deps/suite_all-f)",
    "test support::cross_target ... child output",
    "     Running tests/suites/archive.rs (target/debug/deps/suite_archive-g)",
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
))
from profile_rust_receipts import SCHEMA, load, write
from profile_rust_shards import SHARDS, aggregate, canonical_tests, owned_targets
panic_output = "\n".join((
    "     Running tests/suites/system_suite.rs (target/debug/deps/suite_system-a)",
    "test contract::panic - should panic ... ok",
))
if canonical_tests(panic_output) != Counter({"suite_all::contract::panic": 1}):
    raise SystemExit("panic test identity did not match listed inventory")
with tempfile.TemporaryDirectory() as directory:
    receipt = pathlib.Path(directory) / "receipt.json"
    write(receipt, {"schema": SCHEMA, "state": "PENDING"})
    if load(pathlib.Path(directory))[0]["state"] != "PENDING" or list(pathlib.Path(directory).glob("*.tmp")):
        raise SystemExit("receipt atomic transition was not retained")
    receipt.write_text("{}")
    try:
        load(pathlib.Path(directory))
    except ValueError:
        pass
    else:
        raise SystemExit("receipt schema omission was accepted")
repository = path.parent.parent
profiler = repository / "scripts" / "profile-rust-tests"
profiler_source = profiler.read_text(encoding="utf-8")
if "if \"--shard\" in sys.argv or \"--aggregate-receipts\" in sys.argv:" in profiler_source:
    raise SystemExit("registered shards bypass the profiler lifecycle")
if "from profile_rust_lifecycle import run_workload as lifecycle_run_workload" not in profiler_source:
    raise SystemExit("registered shards do not reuse the extracted profiler lifecycle")
registry_source = (repository / "scripts" / "profile_rust_shards.py").read_text(encoding="utf-8")
if "def run_shard" in registry_source or "subprocess.run(" in registry_source:
    raise SystemExit("the shard registry retained a parallel runner")
if '#526' not in registry_source or "monolithic-all-targets" not in registry_source:
    raise SystemExit("the approved topology supersession is not recorded")
if "workload_receipt" not in profiler_source or "listed_digest" not in profiler_source:
    raise SystemExit("shard receipts are not derived from lifecycle accounting")
head = __import__("subprocess").check_output(("git", "rev-parse", "HEAD"), cwd=repository, text=True).strip()
index_tree = __import__("subprocess").check_output(("git", "write-tree"), cwd=repository, text=True).strip()
targets = sorted(module.declared_test_targets(repository))
def receipt_set(directory):
    rows = []
    for platform, count in (("posix", 2060), ("windows", 1943)):
        for index, shard in enumerate(SHARDS):
            size = count // len(SHARDS) + (index < count % len(SHARDS))
            tests = [f"suite_all::{platform}_{shard}_{number}" for number in range(size)]
            value = {"schema": SCHEMA, "state": "PASS", "platform": platform, "shard": shard, "argv": SHARDS[shard], "head": head, "index_tree": index_tree, "tests": tests, "digest": __import__("profile_rust_receipts").digest(Counter(tests)), "listed_digest": __import__("profile_rust_receipts").digest(Counter(tests)), "physical_targets": sorted(owned_targets(set(targets), shard)), "elapsed": 1, "started": index, "finished": index + 1}
            rows.append(value)
    return rows
def check(label, mutate, expected):
    with tempfile.TemporaryDirectory() as directory:
        root = pathlib.Path(directory); rows = receipt_set(root); mutate(rows)
        for index, value in enumerate(rows): write(root / f"{index}.json", value)
        if aggregate(root, repository) != expected: raise SystemExit(label)
check("complete 14 receipt set", lambda rows: None, 0)
with tempfile.TemporaryDirectory() as directory:
    root = pathlib.Path(directory); rows = receipt_set(root)[:7]
    for index, value in enumerate(rows): write(root / f"{index}.json", value)
    if aggregate(root, repository) != 1 or aggregate(root, repository, "posix") != 0 or aggregate(root, repository, "windows") != 1:
        raise SystemExit("local platform aggregation weakened the required CI aggregate")
check("window 299.999", lambda rows: rows[6].update(finished=299.999), 0)
check("window 300.000", lambda rows: rows[6].update(finished=300.000), 1)
for label, mutate in (
    ("missing", lambda rows: rows.pop()), ("duplicate", lambda rows: rows.__setitem__(-1, rows[0].copy())),
    ("unknown", lambda rows: rows[0].update(shard="unknown")), ("wrong head", lambda rows: rows[0].update(head="wrong")), ("wrong index", lambda rows: rows[0].update(index_tree="wrong")),
    ("wrong argv", lambda rows: rows[0].update(argv=("wrong",))), ("wrong targets", lambda rows: rows[0]["physical_targets"].pop()),
    ("pending", lambda rows: rows[0].update(state="PENDING")), ("wrong count", lambda rows: rows[0]["tests"].pop()),
    ("wrong digest", lambda rows: rows[0].update(digest="wrong")), ("single platform", lambda rows: rows.__delitem__(slice(7, None))),
    ("deadline", lambda rows: rows[0].update(elapsed=271)), ("window", lambda rows: rows[6].update(finished=301)),
    ("negative elapsed", lambda rows: rows[0].update(elapsed=-1)), ("negative window", lambda rows: rows[0].update(started=2, finished=1)),
    ("boolean timing", lambda rows: rows[0].update(elapsed=True)),
): check(label, mutate, 1)
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(accounting)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    Ok(())
}
