use super::interleaved_index::LockedProbeIndex;
use std::process::Command;
#[test]
fn target_summary_recovers_one_interleaved_completed_result()
-> Result<(), Box<dyn std::error::Error>> {
    let accounting = codexy_runtime::paths::repository_root().join("scripts/profile_rust_accounting.py");
    let repository = codexy_runtime::paths::repository_root();
    let index = LockedProbeIndex::new(&repository)?;
    let probe = r#"
import copy, importlib.util, json, os, pathlib, sys, tempfile
from collections import Counter

os.environ["GITHUB_RUN_ID"] = "1"
os.environ["GITHUB_RUN_ATTEMPT"] = "2"

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

transport_spliced = "\n".join((
    "     Running tests/suites/system_suite.rs (target/debug/deps/suite_system-a)",
    "test system::mcp_server_names::legacy_contract ... okTo D:\\a\\_temp\\remote.git",
    " * [new branch] main -> main",
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
))
tests, targets, outcomes = module.observed_test_records(transport_spliced)
expected = "suite_system::system::mcp_server_names::legacy_contract"
if tests.get(expected) != 1 or outcomes.get("ok") != 1 or targets != {"suite_system"}:
    raise SystemExit(f"transport splice: tests={tests!r} targets={targets!r} outcomes={outcomes!r}")

tests, _, outcomes = module.observed_test_records("\n".join((
    "     Running tests/suites/system_suite.rs (target/debug/deps/suite_system-b)",
    "test system::mcp_server_names::legacy_contract ... okay",
)))
if tests or outcomes:
    raise SystemExit(f"non-status prefix: tests={tests!r} outcomes={outcomes!r}")

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
profiler = repository / "scripts" / "profile_rust_tests.py"
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
if "workload_receipt" not in profiler_source or "listed_digest" not in profiler_source or "GITHUB_RUN_ID" not in profiler_source or "GITHUB_RUN_ATTEMPT" not in profiler_source:
    raise SystemExit("shard receipts are not derived from lifecycle accounting")
head = __import__("subprocess").check_output(("git", "rev-parse", "HEAD"), cwd=repository, text=True).strip()
index_tree = __import__("subprocess").check_output(("git", "write-tree"), cwd=repository, text=True).strip()
if index_tree != sys.argv[2]:
    raise SystemExit(f"private index tree drift: {index_tree!r} != {sys.argv[2]!r}")
from profile_rust_receipt_finish import finish_receipt
with tempfile.TemporaryDirectory() as directory:
    receipt_path = pathlib.Path(directory) / "windows-system.json"
    success = finish_receipt(
        receipt_path, __import__("profile_rust_shards").WorkloadSpec("system", SHARDS["system"]),
        __import__("types").SimpleNamespace(windows=True), list(SHARDS["system"]),
        head, index_tree, 1, 1, 0, 0, 0, 280.158, Counter(), Counter(), set(),
        {"profiler-started-epoch": 0}, 0, True,
    )
    if not success or __import__("json").loads(receipt_path.read_text())["state"] != "PASS":
        raise SystemExit("green Windows shard at 280.158 seconds emitted a failed receipt")
targets = sorted(module.declared_test_targets(repository))
platforms = ("posix", "windows")
def receipt_set(directory):
    rows = []
    for platform in platforms:
        for index, shard in enumerate(SHARDS):
            tests = [f"suite_all::{platform}_{shard}_baseline"]
            value = {"schema": SCHEMA, "state": "PASS", "status": 0, "platform": platform, "shard": shard, "argv": SHARDS[shard], "head": head, "index_tree": index_tree, "run_id": 1, "run_attempt": 1, "tests": tests, "digest": __import__("profile_rust_receipts").digest(Counter(tests)), "listed_digest": __import__("profile_rust_receipts").digest(Counter(tests)), "physical_targets": sorted(owned_targets(set(targets), shard)), "elapsed": 1, "started": index, "finished": index + 1}
            rows.append(value)
    return rows
def check(label, mutate, expected):
    with tempfile.TemporaryDirectory() as directory:
        root = pathlib.Path(directory); rows = receipt_set(root); mutate(rows)
        attempts = [value.get("run_attempt") for value in rows]
        for index, value in enumerate(rows): write(root / f"{index}.json", value)
        result = aggregate(root, repository)
        if [json.loads((root / f"{index}.json").read_text()).get("run_attempt") for index in range(len(rows))] != attempts:
            raise SystemExit(f"{label}: receipt attempts were rewritten")
        if result != expected: raise SystemExit(label)
def rehash(value):
    observed = Counter(value["tests"])
    value["digest"] = __import__("profile_rust_receipts").digest(observed)
    value["listed_digest"] = __import__("profile_rust_receipts").digest(observed)
def valid_new_unique_test(rows):
    rows[0]["tests"].append("suite_all::posix_new_unique_test")
    rehash(rows[0])
def duplicate_cross_shard_identity(rows):
    rows[1]["tests"].append(rows[0]["tests"][0])
    rehash(rows[1])
check("complete 14 receipt set", lambda rows: None, 0)
check("valid new unique test receipt", valid_new_unique_test, 0)
with tempfile.TemporaryDirectory() as directory:
    root = pathlib.Path(directory); rows = receipt_set(root)[:7]
    for index, value in enumerate(rows): write(root / f"{index}.json", value)
    if aggregate(root, repository) != 1 or aggregate(root, repository, "posix") != 0 or aggregate(root, repository, "windows") != 1:
        raise SystemExit("local platform aggregation weakened the required CI aggregate")
check("window 299.999", lambda rows: rows[6].update(finished=299.999), 0)
check("window 300.000", lambda rows: rows[6].update(finished=300.000), 1)
check("receipt 280.158 within the shared budget", lambda rows: rows[13].update(elapsed=280.158), 0)
def same_attempt_gap(rows):
    for value in rows:
        if value["platform"] == "posix":
            value.update(started=0, finished=1, run_attempt=1)
    last = next(value for value in rows if (value["platform"], value["shard"]) == ("posix", "agent"))
    last.update(started=301, finished=302, run_attempt=1)
def real_retries(rows):
    replaced = {("posix", "support"), ("posix", "governance")}
    for value in rows:
        if (value["platform"], value["shard"]) in replaced:
            value["run_attempt"] = 2
            value["started"] += 3600
            value["finished"] += 3600
    if Counter(value["run_attempt"] for value in rows) != Counter({1: 12, 2: 2}):
        raise SystemExit("cumulative retry cohort was not 12 attempt-1 plus 2 attempt-2 receipts")
check("same GitHub attempt gap does not split provenance", same_attempt_gap, 1)
check("mixed GitHub retry receipt provenance", real_retries, 0)
check("future GitHub attempt", lambda rows: rows[0].update(run_attempt=3), 1)
def stale_cumulative_retry(rows):
    real_retries(rows)
    stale = next(value for value in rows if value["run_attempt"] == 1)
    stale["state"] = "FAIL"
check("stale failed cumulative retry cohort", stale_cumulative_retry, 1)
def check_without_ci_provenance():
    saved = {name: os.environ.pop(name, None) for name in ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT")}
    try:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory); rows = receipt_set(root)
            for index, value in enumerate(rows): write(root / f"{index}.json", value)
            if aggregate(root, repository) != 1: raise SystemExit("missing current CI provenance")
    finally:
        for name, value in saved.items():
            if value is not None: os.environ[name] = value
check_without_ci_provenance()
def check_invalid_ci_provenance():
    for name in ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT"):
        saved = os.environ[name]; os.environ[name] = "not-a-positive-decimal"
        try:
            with tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory); rows = receipt_set(root)
                for index, value in enumerate(rows): write(root / f"{index}.json", value)
                if aggregate(root, repository) != 1: raise SystemExit(f"malformed current CI provenance: {name}")
        finally: os.environ[name] = saved
check_invalid_ci_provenance()
for label, mutate in (
    ("missing receipt topology", lambda rows: rows.pop()), ("extra receipt topology", lambda rows: rows.append(rows[0].copy())),
    ("duplicate", lambda rows: rows.__setitem__(-1, rows[0].copy())),
    ("unknown", lambda rows: rows[0].update(shard="unknown")), ("wrong head", lambda rows: rows[0].update(head="wrong")), ("wrong index", lambda rows: rows[0].update(index_tree="wrong")),
    ("wrong argv", lambda rows: rows[0].update(argv=("wrong",))), ("wrong targets", lambda rows: rows[0]["physical_targets"].pop()),
    ("pending", lambda rows: rows[0].update(state="PENDING")), ("missing process status", lambda rows: rows[0].pop("status")), ("nonzero process status", lambda rows: rows[0].update(status=1)), ("boolean process status", lambda rows: rows[0].update(status=False)), ("duplicate cross-shard identity", duplicate_cross_shard_identity),
    ("wrong digest", lambda rows: rows[0].update(digest="wrong")), ("single platform", lambda rows: rows.__delitem__(slice(7, None))),
    ("deadline", lambda rows: rows[0].update(elapsed=300.001)), ("window", lambda rows: rows[6].update(finished=301)),
    ("negative elapsed", lambda rows: rows[0].update(elapsed=-1)), ("negative window", lambda rows: rows[0].update(started=2, finished=1)),
    ("boolean timing", lambda rows: rows[0].update(elapsed=True)),
    ("missing run ID", lambda rows: rows[0].pop("run_id")), ("mixed run ID", lambda rows: rows[0].update(run_id=2)),
    ("missing run attempt", lambda rows: rows[0].pop("run_attempt")), ("zero run attempt", lambda rows: rows[0].update(run_attempt=0)),
    ("boolean run attempt", lambda rows: rows[0].update(run_attempt=True)), ("string run attempt", lambda rows: rows[0].update(run_attempt="2")),
    ("float run attempt", lambda rows: rows[0].update(run_attempt=1.0)), ("non-finite run attempt", lambda rows: rows[0].update(run_attempt=float("inf"))),
): check(label, mutate, 1)
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(accounting)
        .arg(index.tree())
        .env("GIT_INDEX_FILE", index.path())
        .output()?;
    assert!(output.status.success(), "{output:?}");
    index.assert_unchanged(&repository)?;
    Ok(())
}
