"""CLI-focused capability cases shared by the component CLI tests."""

from unittest.mock import patch


class CapabilityCliCases:
    def test_doctor_capability_failure_returns_nonzero_exit(self) -> None:
        from codexy_runtime_tools.component_cli import main

        receipt = {
            "schema": "getcodexy.doctor.v1",
            "command": "doctor",
            "outcome": "completed",
            "errors": [],
            "component_health": [
                dict(
                    component="core",
                    healthy=False,
                    started=True,
                    callable=False,
                    first_failure_stage="callable",
                    reason_code="capability-call-failed",
                )
            ],
        }
        with patch("codexy_runtime_tools.component_cli.doctor", return_value=receipt):
            self.assertEqual(main(["doctor", "--json"]), 2)
