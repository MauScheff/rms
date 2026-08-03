use rms_rust_example::{
    transition_record, DescribeWidgetCommand, RustExampleEvent, RustExampleRejection,
    RustExampleTransitionRecord, Widget,
};
use serde_json::{json, Value};

fn variant(name: &str, debug: String) -> Value {
    json!({"name": name, "data": {"debug": debug}})
}

fn record_json(record: &RustExampleTransitionRecord, input: &Value, scenario_start: bool) -> Value {
    let events = record
        .output
        .events
        .iter()
        .map(|event| {
            variant(
                match event {
                    RustExampleEvent::WidgetDescribed => "WidgetDescribed",
                    RustExampleEvent::WidgetRejected => "WidgetRejected",
                },
                format!("{event:?}"),
            )
        })
        .collect::<Vec<_>>();
    let reply = record
        .output
        .reply
        .as_ref()
        .map(|reply| variant("Description", format!("{reply:?}")));
    let rejection = record.output.rejection.as_ref().map(|rejection| {
        variant(
            match rejection {
                RustExampleRejection::EmptyWidgetName => "EmptyWidgetName",
                RustExampleRejection::ExpectedEmptyName => "ExpectedEmptyName",
            },
            format!("{rejection:?}"),
        )
    });
    json!({
        "scenario_start": scenario_start,
        "state_before": {"name": "Ready", "data": {}},
        "state_after": {"name": "Ready", "data": {}},
        "input": input,
        "output": {
            "next_state": {"name": "Ready", "data": {}},
            "events": events,
            "commands": [],
            "effects": [],
            "reply": reply,
            "rejection": rejection,
        },
        "source": {
            "file": record.source.file,
            "function": record.source.function,
            "branch": record.source.branch,
        }
    })
}

fn parse_input(value: &Value) -> DescribeWidgetCommand {
    match value["name"].as_str().expect("probe command name") {
        "Describe" => DescribeWidgetCommand::Describe {
            widget: Widget::new(value["data"]["name"].as_str().expect("Describe.data.name"))
                .expect("non-empty widget name"),
        },
        "RejectEmptyName" => DescribeWidgetCommand::RejectEmptyName {
            name: value["data"]["name"].as_str().unwrap_or("").to_string(),
        },
        other => panic!("unsupported probe command {other}"),
    }
}

#[test]
fn probe_machine() {
    let (Ok(request_path), Ok(output_path)) = (
        std::env::var("RMS_PROBE_REQUEST"),
        std::env::var("RMS_PROBE_OUTPUT"),
    ) else {
        return;
    };
    let request: Value =
        serde_json::from_slice(&std::fs::read(request_path).expect("probe request")).unwrap();
    let output = if request["operation"] == "describe" {
        json!({
            "spec": "rms/machine-probe-description/v0.1",
            "machine": "RustExampleMachine",
            "initial_state": {"name": "Ready", "data": {}},
            "states": [
                {
                    "name": "Ready",
                    "data_schema": {"type": "object"},
                    "examples": [{"name": "Ready", "data": {}}]
                }
            ],
            "inputs": [
                {
                    "kind": "command",
                    "name": "Describe",
                    "data_schema": {
                        "type": "object",
                        "properties": {"name": {"type": "string", "minLength": 1}},
                        "required": ["name"]
                    },
                    "example": {"kind": "command", "name": "Describe", "data": {"name": "example"}}
                },
                {
                    "kind": "command",
                    "name": "RejectEmptyName",
                    "data_schema": {
                        "type": "object",
                        "properties": {"name": {"type": "string"}},
                        "required": ["name"]
                    },
                    "example": {"kind": "command", "name": "RejectEmptyName", "data": {"name": ""}}
                }
            ]
        })
    } else if request["operation"] == "evaluate" {
        let results = request["cases"]
            .as_array()
            .expect("probe evaluation cases")
            .iter()
            .map(|case| {
                let input = &case["input"];
                json!({
                    "id": case["id"],
                    "record": record_json(&transition_record(parse_input(input)), input, true)
                })
            })
            .collect::<Vec<_>>();
        json!({
            "spec": "rms/machine-probe-evaluation/v0.2",
            "machine": "RustExampleMachine",
            "results": results
        })
    } else {
        let records = request["steps"]
            .as_array()
            .expect("probe steps")
            .iter()
            .enumerate()
            .map(|(index, step)| {
                let input = &step["input"];
                record_json(&transition_record(parse_input(input)), input, index == 0)
            })
            .collect::<Vec<_>>();
        json!({
            "spec": "rms/trace-bundle/v0.1",
            "machine": "RustExampleMachine",
            "records": records
        })
    };
    std::fs::write(output_path, serde_json::to_vec_pretty(&output).unwrap()).unwrap();
}
