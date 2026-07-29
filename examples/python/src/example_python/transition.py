from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable

from .representation import (
    ExampleState, ExampleCommand, ExampleEvent, ExampleReply, ExampleRejection
)


@dataclass(frozen=True, slots=True)
class ExampleSourceProvenance:
    file: str
    function: str
    branch: str


@dataclass(frozen=True, slots=True)
class ExampleTransition:
    next_state: ExampleState
    events: tuple[ExampleEvent, ...]
    commands: tuple[ExampleCommand, ...]
    effects: tuple[object, ...]
    reply: ExampleReply | None
    rejection: ExampleRejection | None


@dataclass(frozen=True, slots=True)
class ExampleTransitionRecord:
    state_before: ExampleState
    state_after: ExampleState
    input: object
    output: ExampleTransition
    source: ExampleSourceProvenance


def transition(command: ExampleCommand) -> ExampleTransition:
    return transition_record(command).output


def transition_record(command: ExampleCommand) -> ExampleTransitionRecord:
    state = ExampleState.READY
    if command.kind == "Accept":
        event = ExampleEvent("Accepted")
        reply = ExampleReply("Accepted")
        rejection = None
        branch = "Accept"
    else:
        rejection = ExampleRejection("InvalidCommand", command.value.value)
        event = ExampleEvent("Rejected", rejection)
        reply = None
        branch = "Reject"
    output = ExampleTransition(state, (event,), (), (), reply, rejection)
    return ExampleTransitionRecord(state, state, command, output, ExampleSourceProvenance(
        "src/example_python/transition.py", "transition_record", branch
    ))


def replay_trace(commands: Iterable[ExampleCommand]) -> tuple[ExampleTransitionRecord, ...]:
    return tuple(transition_record(command) for command in commands)


class ExampleMachine:
    transition = staticmethod(transition)


def generate_property_cases() -> tuple[str, ...]:
    return tuple(f"generated-case-{index}" for index in range(64))


def generate_malformed_input_cases() -> tuple[str, ...]:
    return tuple(" " * width for width in range(64))
