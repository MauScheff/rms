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


def probe_machine() -> None:
    request = json.loads(pathlib.Path(os.environ["RMS_PROBE_REQUEST"]).read_text(encoding="utf-8"))
    if request.get("operation") == "describe":
        output = {
            "spec": "rms/machine-probe-description/v0.1",
            "machine": "ExampleMachine",
            "initial_state": {"name": "Ready", "data": {}},
            "states": [{"name": "Ready", "data_schema": {"type": "object"}}],
            "inputs": [
                {"kind": "command", "name": "Accept", "data_schema": {"type": "object", "properties": {"label": {"type": "string"}}}, "example": {"kind": "command", "name": "Accept", "data": {"label": "example"}}},
                {"kind": "command", "name": "Reject", "data_schema": {"type": "object", "properties": {"reason": {"type": "string"}}}, "example": {"kind": "command", "name": "Reject", "data": {"reason": "rejected"}}},
            ],
        }
    elif request.get("operation") == "evaluate":
        results = []
        for case in request.get("cases", []):
            normalized = case["input"]
            label = make_label(normalized.get("data", {}).get("value", "probe"))
            if label is None:
                raise ValueError("probe label must be non-empty")
            command = reject_command(label) if normalized.get("name") == "Reject" else accept_command(label)
            item = transition_record(command)
            results.append({
                "id": case["id"],
                "record": {
                    "scenario_start": True,
                    "state_before": variant(item.state_before),
                    "state_after": variant(item.state_after),
                    "input": normalized,
                    "output": {
                        "next_state": variant(item.output.next_state),
                        "events": [variant(value) for value in item.output.events],
                        "commands": [],
                        "effects": [variant(value) for value in item.output.effects],
                        "reply": variant(item.output.reply) if item.output.reply else None,
                        "rejection": variant(item.output.rejection) if item.output.rejection else None,
                    },
                    "source": {
                        "file": item.source.file,
                        "function": item.source.function,
                        "branch": item.source.branch,
                    },
                },
            })
        output = {
            "spec": "rms/machine-probe-evaluation/v0.2",
            "machine": "ExampleMachine",
            "results": results,
        }
    else:
        records = []
        for index, step in enumerate(request.get("steps", [])):
            normalized = step.get("input", {})
            label = make_label(normalized.get("data", {}).get("value", "probe"))
            if label is None:
                raise ValueError("probe label must be non-empty")
            command = reject_command(label) if normalized.get("name") == "Reject" else accept_command(label)
            record = transition_record(command)
            records.append({
                "scenario_start": index == 0,
                "state_before": variant(record.state_before),
                "state_after": variant(record.state_after),
                "input": normalized,
                "output": {
                    "next_state": variant(record.output.next_state),
                    "events": [variant(item) for item in record.output.events],
                    "commands": [],
                    "effects": [variant(item) for item in record.output.effects],
                    "reply": variant(record.output.reply) if record.output.reply else None,
                    "rejection": variant(record.output.rejection) if record.output.rejection else None,
                },
                "source": {
                    "file": record.source.file,
                    "function": record.source.function,
                    "branch": record.source.branch,
                },
            })
        output = {"spec": "rms/trace-bundle/v0.1", "machine": "ExampleMachine", "records": records}
    pathlib.Path(os.environ["RMS_PROBE_OUTPUT"]).write_text(json.dumps(output), encoding="utf-8")


if __name__ == "__main__":
    probe_machine()
