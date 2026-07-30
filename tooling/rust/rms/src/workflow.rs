use crate::shell_arg;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::fmt;

/// The stable phases through which RMS presents prospective work.
///
/// Keep this vocabulary closed: a new phase changes the public workflow and
/// should be reviewed as a state-space addition rather than introduced as an
/// incidental string.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ActionPhase {
    Clarify,
    Inspect,
    Declare,
    Implement,
    Verify,
    Complete,
}

impl ActionPhase {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Clarify => "clarify",
            Self::Inspect => "inspect",
            Self::Declare => "declare",
            Self::Implement => "implement",
            Self::Verify => "verify",
            Self::Complete => "complete",
        }
    }
}

impl fmt::Display for ActionPhase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Authorization {
    None,
    HostRequired,
}

impl Authorization {
    pub(crate) const fn from_host_requirement(required: bool) -> Self {
        if required {
            Self::HostRequired
        } else {
            Self::None
        }
    }

    #[cfg(test)]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::HostRequired => "host-required",
        }
    }

    pub(crate) const fn is_host_required(self) -> bool {
        matches!(self, Self::HostRequired)
    }
}

/// One public RMS follow-up action.
///
/// The representation is deliberately opaque. A command always has a program,
/// arguments, and rendered display and can never carry a manual instruction.
/// A manual action always has an instruction and can never carry command
/// fields. Custom serialization preserves the existing `rms.surface/v2` wire
/// shape while preventing invalid combinations inside the implementation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SurfaceAction(SurfaceActionKind);

#[derive(Clone, Debug, Eq, PartialEq)]
enum SurfaceActionKind {
    Command {
        phase: ActionPhase,
        program: String,
        args: Vec<String>,
        display: String,
    },
    Manual {
        phase: ActionPhase,
        instruction: String,
        authorization: Authorization,
    },
}

impl SurfaceAction {
    pub(crate) fn command(
        phase: ActionPhase,
        program: impl Into<String>,
        args: Vec<String>,
    ) -> Self {
        let program = program.into();
        let display = std::iter::once(program.as_str())
            .chain(args.iter().map(String::as_str))
            .map(shell_arg)
            .collect::<Vec<_>>()
            .join(" ");
        Self(SurfaceActionKind::Command {
            phase,
            program,
            args,
            display,
        })
    }

    pub(crate) fn manual(
        phase: ActionPhase,
        instruction: impl Into<String>,
        authorization: Authorization,
    ) -> Self {
        Self(SurfaceActionKind::Manual {
            phase,
            instruction: instruction.into(),
            authorization,
        })
    }

    pub(crate) const fn kind(&self) -> &'static str {
        match self.0 {
            SurfaceActionKind::Command { .. } => "command",
            SurfaceActionKind::Manual { .. } => "manual",
        }
    }

    #[cfg(test)]
    pub(crate) fn program(&self) -> Option<&str> {
        match &self.0 {
            SurfaceActionKind::Command { program, .. } => Some(program),
            SurfaceActionKind::Manual { .. } => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn args(&self) -> Option<&[String]> {
        match &self.0 {
            SurfaceActionKind::Command { args, .. } => Some(args),
            SurfaceActionKind::Manual { .. } => None,
        }
    }

    pub(crate) fn display(&self) -> Option<&str> {
        match &self.0 {
            SurfaceActionKind::Command { display, .. } => Some(display),
            SurfaceActionKind::Manual { .. } => None,
        }
    }

    pub(crate) fn instruction(&self) -> Option<&str> {
        match &self.0 {
            SurfaceActionKind::Command { .. } => None,
            SurfaceActionKind::Manual { instruction, .. } => Some(instruction),
        }
    }

    pub(crate) const fn authorization(&self) -> Authorization {
        match self.0 {
            SurfaceActionKind::Command { .. } => Authorization::None,
            SurfaceActionKind::Manual { authorization, .. } => authorization,
        }
    }

    pub(crate) fn display_text(&self) -> &str {
        self.display()
            .or_else(|| self.instruction())
            .unwrap_or("No further action.")
    }
}

impl Serialize for SurfaceAction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.0 {
            SurfaceActionKind::Command {
                phase,
                program,
                args,
                display,
            } => {
                let mut state = serializer.serialize_struct("SurfaceAction", 6)?;
                state.serialize_field("kind", self.kind())?;
                state.serialize_field("phase", phase)?;
                state.serialize_field("program", program)?;
                state.serialize_field("args", args)?;
                state.serialize_field("display", display)?;
                state.serialize_field("authorization", &Authorization::None)?;
                state.end()
            }
            SurfaceActionKind::Manual {
                phase,
                instruction,
                authorization,
            } => {
                let mut state = serializer.serialize_struct("SurfaceAction", 4)?;
                state.serialize_field("kind", self.kind())?;
                state.serialize_field("phase", phase)?;
                state.serialize_field("instruction", instruction)?;
                state.serialize_field("authorization", authorization)?;
                state.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn command_action_serializes_without_manual_state() {
        let action = SurfaceAction::command(
            ActionPhase::Verify,
            "rms",
            vec!["check".to_string(), "--changes".to_string()],
        );

        assert_eq!(
            serde_json::to_value(action).unwrap(),
            json!({
                "kind": "command",
                "phase": "verify",
                "program": "rms",
                "args": ["check", "--changes"],
                "display": "rms check --changes",
                "authorization": "none"
            })
        );
    }

    #[test]
    fn manual_action_serializes_without_command_state() {
        let action = SurfaceAction::manual(
            ActionPhase::Complete,
            "Create the candidate commit.",
            Authorization::HostRequired,
        );

        assert_eq!(
            serde_json::to_value(action).unwrap(),
            json!({
                "kind": "manual",
                "phase": "complete",
                "instruction": "Create the candidate commit.",
                "authorization": "host-required"
            })
        );
    }

    #[test]
    fn workflow_vocabularies_are_closed_and_stable() {
        assert_eq!(
            [
                ActionPhase::Clarify,
                ActionPhase::Inspect,
                ActionPhase::Declare,
                ActionPhase::Implement,
                ActionPhase::Verify,
                ActionPhase::Complete,
            ]
            .map(ActionPhase::label),
            [
                "clarify",
                "inspect",
                "declare",
                "implement",
                "verify",
                "complete",
            ]
        );
        assert_eq!(Authorization::None.label(), "none");
        assert_eq!(Authorization::HostRequired.label(), "host-required");
    }
}
