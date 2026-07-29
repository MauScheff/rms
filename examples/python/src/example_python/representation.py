from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Literal


class ExampleLabel:
    __slots__ = ("_value",)

    def __init__(self, value: str) -> None:
        self._value = value

    @property
    def value(self) -> str:
        return self._value


def make_label(value: object) -> ExampleLabel | None:
    normalized = value.strip() if isinstance(value, str) else ""
    return ExampleLabel(normalized) if normalized else None


class ExampleState(str, Enum):
    READY = "Ready"


def initial_state() -> ExampleState:
    return ExampleState.READY


@dataclass(frozen=True, slots=True)
class ExampleCommand:
    kind: Literal["Accept", "Reject"]
    value: ExampleLabel


def accept_command(label: ExampleLabel) -> ExampleCommand:
    return ExampleCommand("Accept", label)


def reject_command(reason: ExampleLabel) -> ExampleCommand:
    return ExampleCommand("Reject", reason)


@dataclass(frozen=True, slots=True)
class ExampleCommandEnvelope:
    command_id: str
    target_machine: str
    correlation_id: str
    causation_id: str | None
    idempotency_key: str | None
    command: ExampleCommand


@dataclass(frozen=True, slots=True)
class ExampleEvent:
    kind: str
    rejection: object | None = None


@dataclass(frozen=True, slots=True)
class ExampleEventEnvelope:
    event_id: str
    source_machine: str
    correlation_id: str
    causation_id: str
    sequence: int
    schema_version: int
    occurred_at: str
    event: ExampleEvent





@dataclass(frozen=True, slots=True)
class ExampleReply:
    kind: Literal["Accepted"]


@dataclass(frozen=True, slots=True)
class ExampleRejection:
    kind: Literal["InvalidCommand", "IllegalTransition", "MalformedInput"]
    reason: str | None = None
