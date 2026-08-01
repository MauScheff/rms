use rms_rust_example::{describe_widget, Widget};
use serde_json::json;

#[test]
fn produce_transition_trace() {
    let Ok(output) = std::env::var("RMS_TRACE_OUTPUT") else {
        return;
    };
    let widget = Widget::new("example").unwrap();
    let record = json!({
        "spec": "rms/invocation-record/v0.1",
        "contract": "describe-widget",
        "binding": "describe-widget-public",
        "contract_digest": "sha256:8b85cd1bfbb67453a34fe846033fc3ad911b0d6eb757326339db62519863f96c",
        "scenario_start": true,
        "input": {"kind": "Describe", "widget": {"name": widget.name()}},
        "output": {"kind": "Description", "value": describe_widget(&widget)},
        "source": {
            "file": file!(),
            "function": "produce_transition_trace",
            "branch": "accepted-query",
        },
    });
    let document = json!({
        "spec": "rms/trace-bundle/v0.1",
        "machine": "RustExampleMachine",
        "records": [record],
    });

    std::fs::write(output, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
}
