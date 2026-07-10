pub mod widget;

pub use widget::{
    describe_widget, transition, transition_record, DescribeWidgetCommand, DescribeWidgetReply,
    RustExampleEvent, RustExampleMachine, RustExampleRejection, RustExampleState,
    RustExampleTransition, RustExampleTransitionRecord, Widget,
};
