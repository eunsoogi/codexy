from __future__ import annotations

import unittest

from codexy_runtime_tools.component_inspection import doctor
from codexy_runtime_tools.component_watcher_materialization import (
    materialize_watcher,
    watcher_entrypoint,
)
from packages.getcodexy.tests.component_lifecycle_support import fixture


class WatcherRegistrationTests(unittest.TestCase):
    def test_doctor_rejects_a_missing_registered_watcher_target(self) -> None:
        with fixture({"core"}) as state:
            materialize_watcher(state.marketplace / "plugins/codexy")
            watcher_entrypoint(state.marketplace / "plugins/codexy").unlink()
            result = doctor(state.home, codex=state.codex, runner=state.run)

        health = result["component_health"][0]
        self.assertFalse(health["configured"])
        self.assertEqual(health["first_failure_stage"], "configured")
        self.assertEqual(health["reason_code"], "component-not-configured")


if __name__ == "__main__":
    unittest.main()
