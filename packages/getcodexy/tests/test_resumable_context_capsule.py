import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[3]
PLUGIN_FILES = tuple(
    "skills/dreaming/scripts/resumable-context-capsule.sh skills/dreaming/scripts/resumable-context-capsule.cmd skills/dreaming/scripts/resumable_context_capsule.py".split()
)
KINDS = {"darwin-arm64": "mach-o", "linux-x86_64": "elf", "windows-x86_64": "pe"}
MACHINES = {"arm64": "arm64", "aarch64": "arm64", "x86_64": "x86_64", "AMD64": "x86_64"}
EVENT_KINDS = json.loads(
    '{"compaction":"compaction-resume","fresh-child":"fresh-child-continuation","parent-handoff":"parent-handoff"}'
)
AUTHORITY_TEMPLATE = json.loads(
    '{"currentHead":"head","owner":"child-owned","worktree":"worktree","issue":679,'
    '"pr":null,"branch":"branch","base":"base"}'
)
VOLATILE_TEMPLATE = json.loads(
    '{"issue_pr_identity":{"issue":679,"pr":null},'
    '"owner_worktree":{"owner":"child-owned","branch":"branch","worktree":"worktree"},'
    '"base_head_sha":{"base":"base","head":"head"},"dirty_index_state":{"dirty":false,"index":false},'
    '"checks":["focused"],"unresolved_review_threads":[],"selected_reviewer_state":"pending",'
    '"verification":["installed"],"active_obligation":"validate","external_gate":"none",'
    '"next_action":"continue","child_task":null,"parent_task":null,"preserved_artifacts":null,"delivery":"confirmed","task_surface":"codex-task","event":null,'
    '"authoritative_refresh_handles":[],"omissions":{"authoritative_refresh_handles":"not_applicable",'
    '"pr":"not_created","preserved_artifacts":"not_applicable"}}'
)
POLICY = ROOT / "plugins/codexy/skills/orchestration/references/context-tiers.md"
RUNTIME_SCHEMA = ROOT / "packages/codexy-runtime/schemas/handoff-runtime.schema.json"
STABLE = json.loads(
    '{"policy_digest":"","workflow_profile":"strict","task_classification":"implementation",'
    '"selected_references":["workflow_profiles","task_classification","tdd_classification_policy","execution_budget","proof_completion"]}'
)
STABLE["policy_digest"] = f"sha256:{hashlib.sha256(POLICY.read_bytes()).hexdigest()}"
RUN_OPTIONS = {"text": True, "capture_output": True, "check": False}


class ResumableContextCapsuleTests(unittest.TestCase):
    def setUp(self) -> None:
        subprocess.check_call(
            "cargo build --locked --bin codexy-handoff-validate".split(),
            cwd=ROOT / "packages/codexy-runtime",
        )
        self._new_fixture("core")

    def _new_fixture(self, layout: str) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.plugins = Path(self.temporary.name).resolve() / "plugins"
        self.plugin = self.plugins / "codexy"
        shutil.copytree(ROOT / "plugins/codexy", self.plugin)
        if layout == "github":
            shutil.copytree(
                ROOT / "plugins/codexy-github", self.plugins / "codexy-github"
            )
        self.runtime = self.plugin
        if layout == "sibling":
            self.runtime = self.plugins / "codexy-devtools"
            self.runtime.mkdir()

    def test_component_sources_are_installed_and_manifest_declares_them(self) -> None:
        self.assertTrue(
            RUNTIME_SCHEMA.is_file(), "missing runtime-owned handoff schema"
        )
        missing = [item for item in PLUGIN_FILES if not (self.plugin / item).is_file()]
        self.assertEqual(missing, [], f"missing installed capsule sources: {missing}")
        manifest_path = ROOT / (
            "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json"
        )
        manifest = json.loads(manifest_path.read_text())
        core = next(item for item in manifest["components"] if item["id"] == "core")
        required = core["asset"]["requiredPaths"]
        self.assertEqual(set(PLUGIN_FILES) - set(required), set())
        generated = lambda item: item.startswith(("handoff-runtime.json", "runtime/"))
        self.assertFalse(any(map(generated, required)))

    def test_installed_layouts_preserve_three_consumers(self) -> None:
        python_path = str(Path(sys.executable).parent)
        environment = {"PATH": os.pathsep.join((python_path, os.defpath))}
        if os.name == "nt":
            environment["SystemRoot"] = os.environ["SystemRoot"]
        consumers = dict(
            core="compaction", github="fresh-child", sibling="parent-handoff"
        )
        for layout, consumer in consumers.items():
            self._new_fixture(layout)
            self._install_runtime()
            replay = self.plugins.parent / f"{layout}.json"
            capsule = self._capsule(consumer, replay.with_name(f"{consumer}.json"))
            result = self._run(capsule, environment=environment)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)["consumer"], consumer)
            repeated = self._run(capsule, environment=environment)
            self.assertEqual(repeated.returncode, 2, repeated.stderr)

    def test_selected_platform_path_digest_and_kind_fail_independently(self) -> None:
        manifest = self._install_runtime()
        capsule = self._capsule("fresh-child", self.plugins.parent / "replay.json")
        platform_id = f"{platform.system().lower()}-{MACHINES[platform.machine()]}"
        mutations = {
            "path": lambda item: item.update(path="runtime/unauthorized.bin"),
            "digest": lambda item: item.update(sha256="0" * 64),
            "kind": lambda item: item.update(kind="pe" if os.name != "nt" else "elf"),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                document = json.loads(manifest.read_text())
                mutate(document["platforms"][platform_id])
                manifest.write_text(json.dumps(document))
                result = self._run(capsule)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(name, result.stderr.lower())
                self._install_runtime()

    def test_distinct_linked_and_reparse_ancestors_are_rejected(self) -> None:
        for case in ("runtime", "output", "native bridge", "authority"):
            with self.subTest(case=case):
                self._new_fixture("core")
                replay = self.plugins.parent / "replay.json"
                capsule = self._capsule("parent-handoff", replay)
                self._install_runtime()
                arguments: list[str] = []
                if case == "runtime":
                    linked = self.plugins / "linked-devtools"
                    self._link_directory(linked, self.runtime)
                    arguments = ["--runtime-root", str(linked)]
                elif case == "output":
                    real = self.plugins.parent / "real-output"
                    real.mkdir()
                    linked = self.plugins.parent / "linked-output"
                    self._link_directory(linked, real)
                    arguments = ["--output", str(linked / "result.json")]
                elif case == "native bridge":
                    real = self.runtime / "real-runtime"
                    (self.runtime / "runtime").rename(real)
                    self._link_directory(self.runtime / "runtime", real)
                else:
                    authority = capsule.parent / "authority" / capsule.name
                    authority_root = authority.parent
                    real = authority_root.with_name("real-authority")
                    authority_root.rename(real)
                    self._link_directory(authority_root, real)
                result = self._run(capsule, *arguments)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("ancestor", result.stderr.lower())

    def _install_runtime(self) -> Path:
        name = "codexy-handoff-validate" + (".exe" if os.name == "nt" else "")
        bridge = ROOT / "packages/codexy-runtime/target/debug" / name
        self.assertTrue(bridge.is_file(), f"missing native bridge: {bridge}")
        runtime = self.runtime / "runtime"
        if runtime.is_symlink():
            runtime.unlink()
        else:
            shutil.rmtree(runtime, ignore_errors=True)
        runtime.mkdir()
        bridges = {}
        for platform_id, kind in KINDS.items():
            suffix = ".exe" if platform_id == "windows-x86_64" else ".bin"
            target = runtime / f"codexy-handoff-validate-{platform_id}{suffix}"
            shutil.copy2(bridge, target)
            target.chmod(target.stat().st_mode | 0o100)
            bridges[platform_id] = {
                "path": f"runtime/{target.name}",
                "sha256": hashlib.sha256(target.read_bytes()).hexdigest(),
                "kind": kind,
            }
        document = dict(
            schema="codexy.handoff-runtime.v1",
            version=1,
            source={"commit": "a" * 40, "tree": "b" * 40},
            platforms=bridges,
        )
        manifest = self.runtime / "handoff-runtime.json"
        manifest.write_text(json.dumps(document))
        return manifest

    def _capsule(self, consumer: str, replay: Path) -> Path:
        parent, child = "parent-679", "child-679"
        parent_bound = consumer == "parent-handoff"
        subject = parent if parent_bound else child
        source, target = (child, parent) if parent_bound else (parent, child)
        capsule = {
            "schema": "codexy.resumable-context-capsule.v1",
            "consumer": consumer,
            "subject": subject,
            "sourceTask": source,
            "targetTask": target,
            "replayPath": str(replay),
            "envelope": self._canonical_envelope(
                EVENT_KINDS[consumer], consumer, subject, parent, child
            ),
        }
        path = replay.with_suffix(".capsule.json")
        path.write_text(json.dumps(capsule))
        authority = path.parent / "authority" / path.name
        authority.parent.mkdir(exist_ok=True)
        document = dict(AUTHORITY_TEMPLATE)
        document.update(schema="codexy.handoff-authority.v1", stable=STABLE)
        authority.write_text(json.dumps(document))
        return path

    def _run(self, capsule: Path, *arguments: str, environment=None):
        suffix = "cmd" if os.name == "nt" else "sh"
        name = f"resumable-context-capsule.{suffix}"
        launcher = self.plugin / f"skills/dreaming/scripts/{name}"
        command = [str(launcher)]
        if os.name == "nt":
            command = [os.environ.get("COMSPEC", "cmd.exe"), "/d", "/c", str(launcher)]
        authority = capsule.parent / "authority" / capsule.name
        command.extend(
            ("--capsule", str(capsule), "--authority", str(authority), *arguments)
        )
        return subprocess.run(command, env=environment, **RUN_OPTIONS)

    def _link_directory(self, link: Path, target: Path) -> None:
        if os.name == "nt":
            subprocess.run(
                ["cmd", "/d", "/c", "mklink", "/J", link, target], check=True
            )
        else:
            link.symlink_to(target, target_is_directory=True)

    def _canonical_envelope(
        self, kind: str, lane: str, subject: str, parent: str, child: str
    ) -> str:
        volatile = dict(VOLATILE_TEMPLATE)
        volatile.update(child_task=child, parent_task=parent)
        volatile["event"] = dict(
            id=f"{kind}|{lane}|{subject}",
            kind=kind,
            lane=lane,
            subject=subject,
            delta="capsule",
        )
        encode = lambda value: json.dumps(value, separators=(",", ":")).encode()
        compact = {
            "schema": "codexy.handoff-envelope.v1",
            "stable_identity": f"codexy.handoff.stable.v1:{hashlib.sha256(encode(STABLE)).hexdigest()}",
            "volatile": volatile,
            "volatile_identity": f"codexy.handoff.volatile.v1:{hashlib.sha256(encode(volatile)).hexdigest()}",
        }
        return json.dumps(compact, separators=(",", ":"))
