from __future__ import annotations

import io
import json
import unittest
from contextlib import redirect_stderr, redirect_stdout
from unittest.mock import patch

from codexy_runtime_tools.component_cli import main
from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from codexy_runtime_tools.component_transaction_state import (
    InventorySnapshot,
    Journal,
    read_journal,
    write_inventory,
    write_journal,
)
from packages.getcodexy.tests.component_lifecycle_support import (
    VERSION,
    _git,
    fixture,
)


class UpdateRecoveryTests(unittest.TestCase):
    def test_started_update_is_resumed_not_inferred_from_unchanged_selection(
        self,
    ) -> None:
        with fixture({"core", "github"}) as state:
            target = inventory_path(state.home)
            target.parent.mkdir(parents=True)
            target.write_text(
                json.dumps(
                    {
                        "schema": "getcodexy.installed-component-inventory.v1",
                        "components": ["core", "github"],
                    }
                ),
                encoding="utf-8",
            )
            write_journal(
                state.home,
                Journal(
                    "op-interrupted-update",
                    "update",
                    ("github",),
                    ("core", "github"),
                    ("core", "github"),
                    ("core", "github"),
                    InventorySnapshot.capture(state.home),
                    "started",
                ),
            )

            receipt = run_operation(
                "install",
                ("devtools",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-after-update",
            )

            upgrade = ("plugin", "marketplace", "upgrade", "codexy", "--json")
            self.assertIn(upgrade, state.calls)
            prior = target.parent / "receipts" / "op-interrupted-update.json"
            self.assertEqual(
                json.loads(prior.read_text(encoding="utf-8"))["outcome"], "completed"
            )
            self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])

    def test_cli_update_and_bootstrap_quarantine_independent_marketplace_drift(
        self,
    ) -> None:
        drifts = (
            "config-ref",
            "metadata-ref-name",
            "metadata-revision",
            "checkout-head",
        )
        upgrade = ("plugin", "marketplace", "upgrade", "codexy", "--json")
        for command in ("update", "bootstrap"):
            for drift in drifts:
                with (
                    self.subTest(command=command, drift=drift),
                    fixture({"core"}) as state,
                ):
                    if command == "update":
                        write_inventory(state.home, ("core",))

                    def runner(args: list[str]) -> object:
                        result = state.run(args)
                        if tuple(args[1:]) == upgrade:
                            _apply_drift(state, drift)
                        return result

                    output, errors = io.StringIO(), io.StringIO()
                    with (
                        patch(
                            "codexy_runtime_tools.component_lifecycle_operation._run",
                            side_effect=lambda args, _home: runner(args),
                        ),
                        redirect_stdout(output),
                        redirect_stderr(errors),
                    ):
                        code = main(
                            [
                                "--codex",
                                str(state.codex),
                                "--codex-home",
                                str(state.home),
                                command,
                                "--json",
                            ]
                        )

                    receipt = json.loads(output.getvalue())
                    self.assertEqual(code, 2)
                    self.assertEqual(errors.getvalue(), "")
                    self.assertEqual(receipt["command"], command)
                    self.assertEqual(receipt["outcome"], "rolled-back")
                    self.assertEqual(receipt["selection_after"], ["core"])
                    self.assertIn(upgrade, state.mutations)
                    self.assertIn(
                        ("plugin", "marketplace", "remove", "codexy", "--json"),
                        state.mutations,
                    )
                    self.assertFalse(
                        any(
                            mutation[:2] == ("plugin", "add")
                            for mutation in state.mutations
                        )
                    )
                    self.assertNotIn(
                        "[marketplaces.codexy]",
                        (state.home / "config.toml").read_text(encoding="utf-8"),
                    )
                    recovery = json.loads(
                        (
                            state.home / "getcodexy" / "marketplace-recovery.json"
                        ).read_text(encoding="utf-8")
                    )
                    self.assertEqual(
                        recovery["reason"], "post-upgrade-marketplace-drift"
                    )
                    self.assertIsNone(read_journal(state.home))

    def test_interrupted_older_update_recovers_its_canonical_mixed_version_state(
        self,
    ) -> None:
        with fixture(
            {"core", "github"}, versions={"core": "1.2.0", "github": "1.2.0"}
        ) as state:
            target = inventory_path(state.home)
            target.parent.mkdir(parents=True)
            target.write_text(
                json.dumps(
                    {
                        "schema": "getcodexy.installed-component-inventory.v1",
                        "components": ["core", "github"],
                    }
                ),
                encoding="utf-8",
            )
            write_journal(
                state.home,
                Journal(
                    "op-interrupted-older-update",
                    "update",
                    ("github",),
                    ("core", "github"),
                    ("core", "github"),
                    ("core", "github"),
                    InventorySnapshot.capture(state.home),
                    "started",
                ),
            )
            state.versions["core"] = VERSION

            receipt = run_operation(
                "install",
                ("devtools",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-after-older-update",
            )

            self.assertIn(
                ("plugin", "marketplace", "upgrade", "codexy", "--json"), state.calls
            )
            self.assertIsNone(read_journal(state.home))
            self.assertEqual(
                json.loads(
                    (
                        target.parent / "receipts" / "op-interrupted-older-update.json"
                    ).read_text(encoding="utf-8")
                )["outcome"],
                "completed",
            )
            self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])


def _apply_drift(state: fixture, drift: str) -> None:
    tag = f"v{VERSION}"
    metadata_path = state.marketplace / ".codex-marketplace-install.json"
    if drift == "config-ref":
        (state.home / "config.toml").write_text(
            '[marketplaces.codexy]\nref = "main"\n', encoding="utf-8"
        )
    elif drift == "metadata-ref-name":
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        metadata["ref_name"] = "main"
        metadata_path.write_text(json.dumps(metadata), encoding="utf-8")
    elif drift == "metadata-revision":
        if state.main_revision is None:
            raise AssertionError("fixture main revision was not created")
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        metadata["revision"] = state.main_revision
        metadata_path.write_text(json.dumps(metadata), encoding="utf-8")
    elif drift == "checkout-head":
        _git(state.marketplace, "checkout", "-q", "--detach", "main")
    else:
        raise AssertionError(f"unknown drift: {drift}")
    if drift != "config-ref":
        config = state.home / "config.toml"
        config.write_text(f'[marketplaces.codexy]\nref = "{tag}"\n', encoding="utf-8")


if __name__ == "__main__":
    unittest.main()
