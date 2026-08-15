"""Closed stage-aware rejection variants for component lifecycle receipts."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Callable, Literal

from .component_manifest import ComponentManifest
from .component_resolver import ComponentResolutionError, resolve_components


Command = Literal["install", "update", "remove", "bootstrap"]
Planner = Callable[
    [
        ComponentManifest,
        Command,
        tuple[str, ...],
        tuple[str, ...],
        tuple[str, ...] | None,
    ],
    object,
]


class StateFailure(str, Enum):
    INCONSISTENT_INSTALLED_STATE = "inconsistent-installed-state"


class RejectionStage(str, Enum):
    REQUEST = "request"
    HOST = "host"
    PRESTATE = "prestate"
    PLAN = "plan"


class RejectionKind(str, Enum):
    COMPONENTS_NOT_ACCEPTED = "components-not-accepted"
    MISSING_REMOVAL_TARGET = "missing-removal-target"
    UNKNOWN_COMPONENT = "unknown-component"
    CONFLICTING_COMPONENT_REQUEST = "conflicting-component-request"
    INVALID_INSTALLED_INVENTORY = "invalid-installed-inventory"
    CONFLICTING_INSTALLED_STATE = "conflicting-installed-state"
    UNKNOWN_INSTALLED_COMPONENT = "unknown-installed-component"
    MIXED_VERSION_STATE = "mixed-version-state"
    COMPONENT_VERSION_MISMATCH = "component-version-mismatch"
    INCONSISTENT_INSTALLED_STATE = "inconsistent-installed-state"
    NO_RECORDED_SELECTION = "no-recorded-selection"
    INCOMPATIBLE_COMPONENT_SELECTION = "incompatible-component-selection"
    DEPENDENCY_PROTECTED_REMOVAL = "dependency-protected-removal"


@dataclass(frozen=True)
class Rejection:
    stage: RejectionStage
    kind: RejectionKind
    diagnostic: str = ""

    @classmethod
    def from_failure(
        cls, stage: RejectionStage, failure: ComponentResolutionError | StateFailure
    ) -> "Rejection":
        code = (
            failure.code
            if isinstance(failure, ComponentResolutionError)
            else failure.value
        )
        try:
            return cls(stage, RejectionKind(code), str(failure))
        except ValueError as error:
            raise ValueError(
                "component transition has an unreachable rejection"
            ) from error

    def validate(
        self,
        manifest: ComponentManifest,
        command: Command,
        requested: tuple[str, ...],
        before: tuple[str, ...],
        planner: Planner,
    ) -> None:
        if not any(
            (variant.stage, variant.kind) == (self.stage, self.kind)
            for variant in variants(manifest, command, requested, before, planner)
        ):
            raise ValueError("component transition rejection has an invalid stage")


def valid_rejection(
    manifest: ComponentManifest,
    command: Command,
    requested: tuple[str, ...],
    before: tuple[str, ...],
    errors: tuple[str, ...],
    planner: Planner,
) -> bool:
    if len(errors) != 1:
        return False
    try:
        kind = RejectionKind(errors[0])
    except ValueError:
        return False
    return any(
        variant.kind is kind
        for variant in variants(manifest, command, requested, before, planner)
    )


def variants(
    manifest: ComponentManifest,
    command: Command,
    requested: tuple[str, ...],
    before: tuple[str, ...],
    planner: Planner,
) -> frozenset[Rejection]:
    request = _request_rejection(manifest, command, requested)
    if request is not None:
        return (
            frozenset({Rejection(RejectionStage.REQUEST, request)})
            if not before
            else frozenset()
        )
    result = {
        Rejection(RejectionStage.PRESTATE, RejectionKind.INCONSISTENT_INSTALLED_STATE)
    }
    if not before:
        result.update(Rejection(RejectionStage.HOST, kind) for kind in _HOST_KINDS)
    for recorded in (None, before):
        try:
            planner(manifest, command, requested, before, recorded)
        except ComponentResolutionError as error:
            result.add(Rejection(RejectionStage.PLAN, RejectionKind(error.code)))
    return frozenset(result)


_HOST_KINDS = (
    RejectionKind.INVALID_INSTALLED_INVENTORY,
    RejectionKind.CONFLICTING_INSTALLED_STATE,
    RejectionKind.UNKNOWN_INSTALLED_COMPONENT,
    RejectionKind.MIXED_VERSION_STATE,
    RejectionKind.COMPONENT_VERSION_MISMATCH,
    RejectionKind.INCONSISTENT_INSTALLED_STATE,
)


def _request_rejection(
    manifest: ComponentManifest, command: Command, requested: tuple[str, ...]
) -> RejectionKind | None:
    if command == "bootstrap" and requested:
        return RejectionKind.COMPONENTS_NOT_ACCEPTED
    if command == "remove" and not requested:
        return RejectionKind.MISSING_REMOVAL_TARGET
    if command == "install" or requested:
        try:
            resolve_components(manifest, requested)
        except ComponentResolutionError as error:
            return RejectionKind(error.code)
    return None
