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
pub enum DescribeWidgetReply {
    Description(String),
    Rejected { reason: String },
}

pub struct RustExampleMachine;

impl RustExampleMachine {
    pub fn transition(command: DescribeWidgetCommand) -> DescribeWidgetReply {
        match command {
            DescribeWidgetCommand::Describe { widget } => {
                DescribeWidgetReply::Description(describe_widget(&widget).to_string())
            }
            DescribeWidgetCommand::RejectEmptyName { name } => {
                if Widget::new(name).is_some() {
                    DescribeWidgetReply::Rejected {
                        reason: "expected-empty-name".to_string(),
                    }
                } else {
                    DescribeWidgetReply::Rejected {
                        reason: "empty-widget-name".to_string(),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        describe_widget, DescribeWidgetCommand, DescribeWidgetReply, RustExampleMachine, Widget,
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
            DescribeWidgetReply::Description("example".to_string())
        );
    }
}
