use super::GateFixture;
use std::process::Command;

#[test]
fn gate_normalizes_explicit_repository_root_to_the_runtime_package_root(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    let repository = codexy_runtime::paths::repository_root();
    let runtime = codexy_runtime::paths::runtime_package_root();

    for root in [repository, runtime.as_path()] {
        let output = fixture.run_from_root(root, &[])?;
        assert!(output.status.success(), "{root:?}: {output:?}");
    }
    let working_directories = std::fs::read_to_string(&fixture.cwd_marker)?;
    assert!(
        working_directories.lines().all(|cwd| cwd == runtime.to_string_lossy()),
        "{working_directories}"
    );

    let unrelated = fixture.temp.path().join("unrelated-root");
    std::fs::create_dir(&unrelated)?;
    let output = fixture.run_from_root(&unrelated, &[])?;
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--root must name"),
        "{output:?}"
    );
    Ok(())
}

#[test]
fn aggregate_resolves_the_repository_inventory_from_every_valid_root(
) -> Result<(), Box<dyn std::error::Error>> {
    let repository = codexy_runtime::paths::repository_root();
    let runtime = codexy_runtime::paths::runtime_package_root();
    let profiler = repository.join("scripts/profile-rust-tests");
    let index_path = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "--git-path", "index"])
            .current_dir(&repository)
            .output()?
            .stdout,
    )?;
    let index_path = repository.join(index_path.trim());
    let original_index = std::fs::read(&index_path)?;
    let original_status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&repository)
        .output()?
        .stdout;
    let isolated = tempfile::tempdir()?;
    let shared_index = isolated.path().join("shared-index");
    std::fs::copy(&index_path, &shared_index)?;
    std::fs::write(shared_index.with_extension("lock"), b"held")?;
    let probe = r#"
import contextlib, importlib.util, io, os, pathlib, shutil, subprocess, sys, tempfile
from collections import Counter
from importlib.machinery import SourceFileLoader

profiler_path, repository, runtime = map(pathlib.Path, sys.argv[1:])
sys.path.insert(0, str(profiler_path.parent))
spec = importlib.util.spec_from_loader("profile_rust_tests", SourceFileLoader("profile_rust_tests", str(profiler_path)))
profiler = importlib.util.module_from_spec(spec)
spec.loader.exec_module(profiler)
from profile_rust_receipts import SCHEMA, digest, write
from profile_rust_shards import SHARDS, aggregate, owned_targets, platform_counts

head = subprocess.check_output(("git", "rev-parse", "HEAD"), cwd=repository, text=True).strip()
targets = profiler.declared_test_targets(runtime)
counts = platform_counts(repository)

def write_receipts(directory):
    for platform, count in counts.items():
        for index, shard in enumerate(SHARDS):
            size = count // len(SHARDS) + (index < count % len(SHARDS))
            tests = [f"suite_all::{platform}_{shard}_{number}" for number in range(size)]
            observed = Counter(tests)
            write(directory / f"{platform}-{shard}.json", {
                "schema": SCHEMA, "state": "PASS", "platform": platform,
                "shard": shard, "argv": SHARDS[shard], "head": head,
                "index_tree": index_tree, "run_id": 1, "run_attempt": 1,
                "status": 0, "failed": 0, "ignored": 0, "elapsed": 1,
                "tests": tests, "digest": digest(observed),
                "listed_digest": digest(observed),
                "physical_targets": sorted(owned_targets(targets, shard)),
                "started": index, "finished": index + 1,
            })

with tempfile.TemporaryDirectory() as temporary:
    receipts = pathlib.Path(temporary)
    private_index = receipts / "private-index"
    shutil.copyfile(pathlib.Path(os.environ["GIT_INDEX_FILE"]), private_index)
    os.environ["GIT_INDEX_FILE"] = str(private_index)
    index_tree = subprocess.check_output(("git", "write-tree"), cwd=repository, text=True).strip()
    write_receipts(receipts)
    for label, root_arguments in (
        ("default", ()),
        ("repository", ("--root", str(repository))),
        ("runtime", ("--root", str(runtime))),
    ):
        saved_argv = sys.argv
        try:
            sys.argv = [str(profiler_path), "--aggregate-receipts", str(receipts), *root_arguments]
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                status = profiler.main()
        finally:
            sys.argv = saved_argv
        if status != 0 or "aggregate-receipts\t14\tPASS" not in output.getvalue():
            raise SystemExit(f"{label} aggregate failed: {status} {output.getvalue()!r}")
    if (runtime / "scripts/profile_rust_shard_inventory.json").exists():
        raise SystemExit("package-local shard inventory fallback exists")
    output = io.StringIO()
    with contextlib.redirect_stdout(output):
        status = aggregate(receipts, runtime)
    if status != 1 or "invalid Rust shard inventory" not in output.getvalue():
        raise SystemExit(f"package-local fallback was accepted: {status} {output.getvalue()!r}")
    unrelated = receipts / "unrelated"
    unrelated.mkdir()
    try:
        profiler.runtime_package_root(unrelated)
    except ValueError:
        pass
    else:
        raise SystemExit("unrelated root was accepted")
"#;
    let output = Command::new("python3")
        .args(["-c", probe])
        .arg(profiler)
        .arg(repository)
        .arg(runtime)
        .env("GIT_INDEX_FILE", &shared_index)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    assert_eq!(std::fs::read(&index_path)?, original_index);
    assert_eq!(
        Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&repository)
            .output()?
            .stdout,
        original_status
    );
    Ok(())
}
