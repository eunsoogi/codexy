"""Public contracts for the bounded single-call MCP scenario core."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from types import MappingProxyType
from typing import Any, Mapping

from .support import (
    DEFAULT_PROTOCOL_VERSION,
    DEFAULT_TRANSPORT,
    MAX_OUTPUT_BYTES,
    SUPPORTED_PROTOCOL_VERSIONS,
    SUPPORTED_TRANSPORTS,
    SDK_TRANSPORT_CHOICE,
    TransportSupport,
    supported_versions,
    validate_support,
)


class ResultKind(str, Enum):
    """The observable result categories of one bounded call."""

    SUCCESS = "success"
    JSON_RPC_ERROR = "json-rpc-error"
    TOOL_ERROR = "tool-error"
    MALFORMED_RESULT = "malformed-result"
    TIMEOUT = "timeout"
    CANCELLED = "cancelled"
    OUTPUT_LIMIT = "output-limit"
    UNSUPPORTED_PROTOCOL = "unsupported-protocol"
    LAUNCH_ERROR = "launch-error"
    PROCESS_ERROR = "process-error"


class ScenarioValidationError(ValueError):
    """Raised when a call would leave its declared execution boundary."""


class UnsupportedProtocolError(ScenarioValidationError):
    """Raised for a protocol version that this increment does not support."""


class UnsupportedTransportError(ScenarioValidationError):
    """Raised for a transport that this increment does not support."""


def _json_copy(value: Any) -> Any:
    try:
        return json.loads(json.dumps(value, allow_nan=False))
    except (TypeError, ValueError) as error:
        raise ScenarioValidationError("value must be JSON-serializable") from error


def _mapping_copy(value: Mapping[str, Any], label: str) -> Mapping[str, Any]:
    if not isinstance(value, Mapping):
        raise ScenarioValidationError(f"{label} must be a mapping")
    copied = {}
    for key, item in value.items():
        if not isinstance(key, str) or not key:
            raise ScenarioValidationError(f"{label} keys must be non-empty strings")
        copied[key] = _json_copy(item)
    return MappingProxyType(copied)


def _validate_pointer(path: str, label: str) -> str:
    if not isinstance(path, str) or not path.startswith("/") or path == "/":
        raise ScenarioValidationError(
            f"{label} must be a non-root JSON Pointer beginning with '/'"
        )
    return path


@dataclass(frozen=True)
class ExpectedResult:
    """A result kind plus optional JSON-pointer/value expectations."""

    kind: ResultKind = ResultKind.SUCCESS
    fields: Mapping[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not isinstance(self.kind, ResultKind):
            raise ScenarioValidationError("expected kind must be a ResultKind")
        fields = {}
        if not isinstance(self.fields, Mapping):
            raise ScenarioValidationError("expected fields must be a mapping")
        for path, value in self.fields.items():
            fields[_validate_pointer(path, "expected field")] = _json_copy(value)
        object.__setattr__(self, "fields", MappingProxyType(fields))


@dataclass(frozen=True)
class SingleCall:
    """One explicit MCP stdio invocation and its persistence boundaries."""

    argv: tuple[str, ...]
    cwd: Path
    environment: Mapping[str, str]
    tool: str
    allowed_tools: frozenset[str]
    arguments: Mapping[str, Any] = field(default_factory=dict)
    stored_fields: Mapping[str, str] = field(default_factory=dict)
    expected: ExpectedResult = field(default_factory=ExpectedResult)
    protocol_version: str = DEFAULT_PROTOCOL_VERSION
    transport: str = DEFAULT_TRANSPORT
    timeout_seconds: float = 5.0
    output_limit_bytes: int = 64 * 1024

    def __post_init__(self) -> None:
        if isinstance(self.argv, str) or not self.argv:
            raise ScenarioValidationError("argv must be a non-empty sequence")
        argv = tuple(self.argv)
        if any(not isinstance(item, str) or not item or "\0" in item for item in argv):
            raise ScenarioValidationError("argv entries must be non-empty strings")
        object.__setattr__(self, "argv", argv)

        cwd = Path(self.cwd)
        if not cwd.is_absolute() or not cwd.is_dir():
            raise ScenarioValidationError("cwd must be an existing absolute directory")
        object.__setattr__(self, "cwd", cwd)

        if not isinstance(self.environment, Mapping):
            raise ScenarioValidationError("environment must be an explicit mapping")
        environment = {}
        for key, value in self.environment.items():
            if (
                not isinstance(key, str)
                or not isinstance(value, str)
                or not key
                or "\0" in key
                or "\0" in value
            ):
                raise ScenarioValidationError(
                    "environment keys and values must be non-empty strings without NUL"
                )
            environment[key] = value
        object.__setattr__(self, "environment", MappingProxyType(environment))

        if not isinstance(self.tool, str) or not self.tool:
            raise ScenarioValidationError("tool must be a non-empty string")
        allowed_tools = frozenset(self.allowed_tools)
        if not allowed_tools or self.tool not in allowed_tools:
            raise ScenarioValidationError("tool must be included in allowed_tools")
        if any(not isinstance(item, str) or not item for item in allowed_tools):
            raise ScenarioValidationError(
                "allowed_tools entries must be non-empty strings"
            )
        object.__setattr__(self, "allowed_tools", allowed_tools)

        object.__setattr__(
            self, "arguments", _mapping_copy(self.arguments, "arguments")
        )
        stored = {}
        if not isinstance(self.stored_fields, Mapping):
            raise ScenarioValidationError("stored_fields must be a mapping")
        for name, path in self.stored_fields.items():
            if not isinstance(name, str) or not name:
                raise ScenarioValidationError(
                    "stored field names must be non-empty strings"
                )
            stored[name] = _validate_pointer(path, "stored field")
        object.__setattr__(self, "stored_fields", MappingProxyType(stored))
        if not isinstance(self.expected, ExpectedResult):
            raise ScenarioValidationError("expected must be an ExpectedResult")
        try:
            validate_support(self.protocol_version, self.transport)
        except ValueError as error:
            if self.protocol_version not in SUPPORTED_PROTOCOL_VERSIONS:
                raise UnsupportedProtocolError(str(error)) from error
            raise UnsupportedTransportError(str(error)) from error
        if self.timeout_seconds <= 0:
            raise ScenarioValidationError("timeout_seconds must be positive")
        if not 0 < self.output_limit_bytes <= MAX_OUTPUT_BYTES:
            raise ScenarioValidationError(
                f"output_limit_bytes must be between 1 and {MAX_OUTPUT_BYTES}"
            )


@dataclass(frozen=True)
class ExecutionError:
    """Safe, bounded error metadata; raw server content is never retained."""

    kind: ResultKind
    message: str
    code: int | str | None = None


@dataclass(frozen=True)
class ExecutionResult:
    """A single-call result containing only explicitly selected data."""

    kind: ResultKind
    stored: Mapping[str, Any]
    meets_expectation: bool
    error: ExecutionError | None
    returncode: int | None
    elapsed_seconds: float
    tool: str
    protocol_version: str
    transport: str
    observed_output_bytes: int

    def __post_init__(self) -> None:
        object.__setattr__(self, "stored", MappingProxyType(dict(self.stored)))

    @property
    def status(self) -> ResultKind:
        return self.kind

    @property
    def ok(self) -> bool:
        return self.kind is ResultKind.SUCCESS and self.meets_expectation


class CancellationToken:
    """Thread-safe cancellation signal accepted by ``run_single_call``."""

    def __init__(self) -> None:
        from threading import Event

        self._event = Event()

    def cancel(self) -> None:
        self._event.set()

    def is_cancelled(self) -> bool:
        return self._event.is_set()
