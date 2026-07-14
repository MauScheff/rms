use rms_rust_example::{
    transition_record, DescribeWidgetCommand, RustExampleTransitionRecord, Widget,
};
use serde_json::json;

fn case_name(value: &impl std::fmt::Debug) -> String {
    format!("{value:?}")
}

fn record_json(record: &RustExampleTransitionRecord, scenario_start: bool) -> serde_json::Value {
    json!({
        "scenario_start": scenario_start,
        "state_before": case_name(&record.state_before),
        "state_after": case_name(&record.state_after),
        "input": case_name(&record.input),
        "output": {
            "next_state": case_name(&record.output.next_state),
            "events": record.output.events.iter().map(case_name).collect::<Vec<_>>(),
            "commands": record.output.commands.iter().map(case_name).collect::<Vec<_>>(),
            "effects": record.output.effects.iter().map(case_name).collect::<Vec<_>>(),
            "reply": record.output.reply.as_ref().map(case_name),
            "rejection": record.output.rejection.as_ref().map(case_name),
        },
        "source": {
            "file": record.source.file,
            "function": record.source.function,
            "branch": record.source.branch,
        },
    })
}

#[test]
fn produce_transition_trace() {
    let Ok(output) = std::env::var("RMS_TRACE_OUTPUT") else {
        return;
    };
    let records = vec![
        transition_record(DescribeWidgetCommand::Describe {
            widget: Widget::new("example").unwrap(),
        }),
        transition_record(DescribeWidgetCommand::RejectEmptyName {
            name: String::new(),
        }),
    ];
    let document = json!({
        "spec": "rms/trace-bundle/v0.1",
        "machine": "RustExampleMachine",
        "records": records
            .iter()
            .map(|record| record_json(record, true))
            .collect::<Vec<_>>(),
    });

    std::fs::write(output, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    assert_eq!(records.len(), 2);
}
