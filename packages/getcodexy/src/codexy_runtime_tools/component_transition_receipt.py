"""Terminal operation receipts for component lifecycle transitions."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from .component_manifest import ComponentManifest
from .component_hook_activation import ACTIVATION_ERRORS
from .component_resolver import ComponentResolutionError
from .component_transition_rejections import Rejection, valid_rejection
from .component_transaction_identity import valid_operation_id

if TYPE_CHECKING:
    from .component_transition_journal import Journal

RECEIPT_SCHEMA = "getcodexy.operation-receipt.v1"
SOURCE = "installed-component-inventory"


@dataclass(frozen=True)
class OperationReceipt:
    identifier: str
    command: str
    outcome: str
    requested: tuple[str, ...]
    resolved: tuple[str, ...]
    before: tuple[str, ...]
    after: tuple[str, ...]
    errors: tuple[str, ...]

    @classmethod
    def rejected(
        cls,
        identifier: str,
        command: str,
        requested: tuple[str, ...],
        before: tuple[str, ...],
        rejection: Rejection,
    ) -> OperationReceipt:
        return cls(
            identifier,
            command,
            "rejected",
            requested,
            (),
            before,
            before,
            (rejection.kind.value,),
        )

    @classmethod
    def from_journal(
        cls,
        journal: Journal,
        outcome: str,
        after: tuple[str, ...],
        errors: tuple[str, ...] = (),
    ) -> OperationReceipt:
        if outcome not in {"completed", "pending-action", "rolled-back"}:
            raise ValueError("a journal cannot produce a rejected receipt")
        if outcome in {"completed", "pending-action"} and after != journal.target:
            raise ValueError("a completion receipt must use the transition target")
        if outcome == "rolled-back" and after != journal.before:
            raise ValueError("a rollback receipt must use the transition pre-state")
        if outcome == "completed" and errors:
            raise ValueError("a completed receipt cannot contain errors")
        if outcome == "pending-action" and (
            not errors or any(error not in ACTIVATION_ERRORS for error in errors)
        ):
            raise ValueError("a pending-action receipt must contain activation errors")
        return cls(
            journal.identifier,
            journal.command,
            outcome,
            journal.requested,
            journal.resolved,
            journal.before,
            after,
            (
                errors
                if outcome == "pending-action"
                else ()
                if outcome == "completed"
                else ("operation-failed",)
            ),
        )

    @classmethod
    def decode(cls, value: object) -> OperationReceipt:
        fields = {
            "schema",
            "operation_id",
            "command",
            "outcome",
            "requested_components",
            "resolved_components",
            "selection_before",
            "selection_after",
            "installed_components",
            "source_of_truth",
            "errors",
        }
        if (
            not isinstance(value, dict)
            or set(value) != fields
            or value.get("schema") != RECEIPT_SCHEMA
            or value.get("source_of_truth") != SOURCE
            or value.get("installed_components") != value.get("selection_after")
        ):
            raise ValueError("operation receipt has an invalid shape")
        identifier, command, outcome = (
            value.get("operation_id"),
            value.get("command"),
            value.get("outcome"),
        )
        if (
            not valid_operation_id(identifier)
            or command not in {"install", "update", "remove", "bootstrap"}
            or outcome not in {"completed", "pending-action", "rejected", "rolled-back"}
        ):
            raise ValueError("operation receipt has an invalid shape")
        parts = tuple(
            _components(value, field)
            for field in (
                "requested_components",
                "resolved_components",
                "selection_before",
                "selection_after",
            )
        )
        errors = value.get("errors")
        if not isinstance(errors, list) or any(
            not isinstance(error, dict)
            or set(error) != {"code"}
            or not isinstance(error.get("code"), str)
            for error in errors
        ):
            raise ValueError("operation receipt has an invalid shape")
        return cls(
            identifier,
            command,
            outcome,
            *parts,
            tuple(error["code"] for error in errors),
        )

    def encode(self) -> dict[str, object]:
        if (
            not valid_operation_id(self.identifier)
            or self.command not in {"install", "update", "remove", "bootstrap"}
            or self.outcome
            not in {"completed", "pending-action", "rejected", "rolled-back"}
        ):
            raise ValueError("operation receipt has invalid terminal state")
        return {
            "schema": RECEIPT_SCHEMA,
            "operation_id": self.identifier,
            "command": self.command,
            "outcome": self.outcome,
            "requested_components": list(self.requested),
            "resolved_components": list(self.resolved),
            "selection_before": list(self.before),
            "selection_after": list(self.after),
            "installed_components": list(self.after),
            "source_of_truth": SOURCE,
            "errors": [{"code": error} for error in self.errors],
        }

    def validate(self, manifest: ComponentManifest) -> None:
        if (
            not valid_operation_id(self.identifier)
            or self.command not in {"install", "update", "remove", "bootstrap"}
            or self.outcome
            not in {"completed", "pending-action", "rejected", "rolled-back"}
        ):
            raise ValueError("operation receipt has invalid terminal state")
        if (
            self.before not in manifest.compatible_combinations
            or self.after not in manifest.compatible_combinations
        ):
            raise ValueError("operation receipt has invalid component selections")
        if self.outcome == "rejected":
            from .component_transition_model import plan_transition

            if (
                self.resolved
                or self.after != self.before
                or not valid_rejection(
                    manifest,
                    self.command,
                    self.requested,
                    self.before,
                    self.errors,
                    plan_transition,
                )
            ):
                raise ValueError("operation receipt has invalid rejection semantics")
            return
        from .component_transition_model import plan_transition

        try:
            plan = plan_transition(
                manifest, self.command, self.requested, self.before, self.before
            )
        except ComponentResolutionError as error:
            raise ValueError(
                "operation receipt has an invalid request contract"
            ) from error
        if self.resolved != plan.resolved:
            raise ValueError("operation receipt has an invalid resolved selection")
        if self.outcome == "completed" and (self.after != plan.target or self.errors):
            raise ValueError("operation receipt has invalid completion semantics")
        if self.outcome == "pending-action" and (
            self.after != plan.target
            or not self.errors
            or any(error not in ACTIVATION_ERRORS for error in self.errors)
        ):
            raise ValueError("operation receipt has invalid pending-action semantics")
        if self.outcome == "rolled-back" and (
            self.after != self.before or self.errors != ("operation-failed",)
        ):
            raise ValueError("operation receipt has invalid rollback semantics")


def _components(value: dict[str, object], field: str) -> tuple[str, ...]:
    components = value.get(field)
    if not isinstance(components, list) or any(
        not isinstance(component, str) for component in components
    ):
        raise ValueError("operation receipt has invalid components")
    return tuple(components)
