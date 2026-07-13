pub mod widget;

pub use widget::{
    describe_widget, transition, transition_record, DescribeWidgetCommand, DescribeWidgetReply,
    RustExampleCommandEnvelope, RustExampleEvent, RustExampleEventEnvelope, RustExampleMachine,
    RustExampleRejection, RustExampleSourceProvenance, RustExampleState, RustExampleTransition,
    RustExampleTransitionRecord, Widget,
};
