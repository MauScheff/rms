use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Widget {
    name: String,
}

impl Widget {
    pub fn new(name: impl Into<String>) -> Option<Self> {
        let name = name.into();
        if name.trim().is_empty() {
            None
        } else {
            Some(Self { name })
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub fn describe_widget(widget: &Widget) -> &str {
    widget.name()
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RustExampleState {
    Ready,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DescribeWidgetCommand {
    Describe { widget: Widget },
    RejectEmptyName { name: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RustExampleCommandEnvelope {
    pub command_id: String,
    pub target_machine: String,
    pub correlation_id: String,
    pub causation_id: String,
    pub idempotency_key: Option<String>,
    pub command: DescribeWidgetCommand,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RustExampleEvent {
    WidgetDescribed,
    WidgetRejected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RustExampleEventEnvelope {
    pub event_id: String,
    pub source_machine: String,
    pub correlation_id: String,
    pub causation_id: String,
    pub sequence: u64,
    pub schema_version: u64,
    pub occurred_at: String,
    pub event: RustExampleEvent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RustExampleRejection {
    EmptyWidgetName,
    ExpectedEmptyName,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DescribeWidgetReply {
    Description(String),
    Rejected { reason: RustExampleRejection },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RustExampleTransition {
    pub next_state: RustExampleState,
    pub events: Vec<RustExampleEvent>,
    pub commands: Vec<DescribeWidgetCommand>,
    pub effects: Vec<()>,
    pub reply: DescribeWidgetReply,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RustExampleTransitionRecord {
    pub state_before: RustExampleState,
    pub state_after: RustExampleState,
    pub input: DescribeWidgetCommand,
    pub output: RustExampleTransition,
    pub source: RustExampleSourceProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RustExampleSourceProvenance {
    pub file: String,
    pub function: String,
    pub branch: String,
}

pub struct RustExampleMachine;

impl RustExampleMachine {
    pub fn transition(command: DescribeWidgetCommand) -> RustExampleTransition {
        transition(command)
    }
}

pub fn transition(command: DescribeWidgetCommand) -> RustExampleTransition {
    transition_record(command).output
}

pub fn transition_record(command: DescribeWidgetCommand) -> RustExampleTransitionRecord {
    let state_before = RustExampleState::Ready;
    let (event, reply, branch) = match &command {
        DescribeWidgetCommand::Describe { widget } => (
            RustExampleEvent::WidgetDescribed,
            DescribeWidgetReply::Description(describe_widget(widget).to_string()),
            "DescribeWidget",
        ),
        DescribeWidgetCommand::RejectEmptyName { name } => {
            let reason = if Widget::new(name.clone()).is_some() {
                RustExampleRejection::ExpectedEmptyName
            } else {
                RustExampleRejection::EmptyWidgetName
            };
            (
                RustExampleEvent::WidgetRejected,
                DescribeWidgetReply::Rejected { reason },
                "RejectEmptyWidgetName",
            )
        }
    };
    let next_state = RustExampleState::Ready;
    let output = RustExampleTransition {
        next_state: next_state.clone(),
        events: vec![event],
        commands: Vec::new(),
        effects: Vec::new(),
        reply,
    };
    RustExampleTransitionRecord {
        state_before,
        state_after: next_state,
        input: command,
        output,
        source: RustExampleSourceProvenance {
            file: "src/widget.rs".to_string(),
            function: "transition_record".to_string(),
            branch: branch.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        describe_widget, DescribeWidgetCommand, DescribeWidgetReply, RustExampleEvent,
        RustExampleMachine, RustExampleTransition, Widget,
    };

    #[test]
    fn rejects_empty_name() {
        assert_eq!(Widget::new(""), None);
    }

    #[test]
    fn accepts_non_empty_name() {
        let widget = Widget::new("example").expect("valid widget");

        assert_eq!(widget.name(), "example");
    }

    #[test]
    fn describes_widget_by_name() {
        let widget = Widget::new("example").expect("valid widget");

        assert_eq!(describe_widget(&widget), "example");
    }

    #[test]
    fn machine_describes_widget_by_name() {
        let widget = Widget::new("example").expect("valid widget");

        assert_eq!(
            RustExampleMachine::transition(DescribeWidgetCommand::Describe { widget }),
            RustExampleTransition {
                next_state: super::RustExampleState::Ready,
                events: vec![RustExampleEvent::WidgetDescribed],
                commands: Vec::new(),
                effects: Vec::new(),
                reply: DescribeWidgetReply::Description("example".to_string()),
            }
        );
    }
}
