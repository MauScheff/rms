import json
import os
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parents[1] / "src"))

from example_python.representation import accept_command, make_label, reject_command
from example_python.transition import transition_record


def variant(value: object) -> dict[str, object]:
    raw = getattr(value, "kind", None)
    if raw is None:
        raw = getattr(value, "value", value)
    return {"name": str(raw), "data": {}}


def produce_transition_trace() -> None:
    accepted = make_label("accepted")
    rejected = make_label("rejected")
    assert accepted is not None and rejected is not None
    records = []
    for index, item in enumerate(
        [accept_command(accepted), reject_command(rejected)]
    ):
        record = transition_record(item)
        records.append(
            {
                "scenario_start": True,
                "state_before": variant(record.state_before),
                "state_after": variant(record.state_after),
                "input": {"name": "Accept" if index == 0 else "Reject", "data": {}},
                "output": {
                    "next_state": variant(record.output.next_state),
                    "events": [variant(value) for value in record.output.events],
                    "commands": [],
                    "effects": [variant(value) for value in record.output.effects],
                    "reply": variant(record.output.reply) if record.output.reply else None,
                    "rejection": variant(record.output.rejection)
                    if record.output.rejection
                    else None,
                },
                "source": {
                    "file": record.source.file,
                    "function": record.source.function,
                    "branch": record.source.branch,
                },
            }
        )
    bundle = {
        "spec": "rms/trace-bundle/v0.1",
        "machine": "ExampleMachine",
        "records": records,
    }
    pathlib.Path(os.environ["RMS_TRACE_OUTPUT"]).write_text(json.dumps(bundle), encoding="utf-8")
