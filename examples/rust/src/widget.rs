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

#[cfg(test)]
mod tests {
    use super::{describe_widget, Widget};

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
}
