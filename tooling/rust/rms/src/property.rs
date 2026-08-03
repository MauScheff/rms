use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::cmp::Ordering;
use std::collections::BTreeMap;

pub(super) const ANALYSIS_SPEC: &str = "rms/property-analysis/v0.2";
pub(super) const LEGACY_ANALYSIS_SPEC: &str = "rms/property-analysis/v0.1";
pub(super) const OBSERVATION_SPEC: &str = "rms/property-observation/v0.1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Verdict {
    Satisfied,
    Violated,
    Inconclusive,
    Invalid,
    Unsupported,
}

impl Verdict {
    pub(super) fn label(&self) -> &'static str {
        match self {
            Self::Satisfied => "satisfied",
            Self::Violated => "violated",
            Self::Inconclusive => "inconclusive",
            Self::Invalid => "invalid",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct PropertyExplanation {
    summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    decisive_observations: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    decisive_sources: Vec<DecisiveObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trigger_observation: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_observation: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pending_obligation: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    active_assumptions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    normalized_quantities: Vec<NormalizedQuantity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blame: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contract: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    clause: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct DecisiveObservation {
    index: usize,
    source: ObservationSourceMetadata,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Evaluation {
    property: String,
    verdict: Verdict,
    trace_complete: bool,
    observations: usize,
    explanation: PropertyExplanation,
}

impl Evaluation {
    pub(super) fn verdict(&self) -> &Verdict {
        &self.verdict
    }

    pub(super) fn has_trigger_or_response(&self) -> bool {
        self.explanation.trigger_observation.is_some()
            || self.explanation.response_observation.is_some()
    }
}

pub(super) fn invalid_monitor_evaluation(property: &str, summary: impl Into<String>) -> Evaluation {
    Evaluation {
        property: property.to_string(),
        verdict: Verdict::Invalid,
        trace_complete: false,
        observations: 0,
        explanation: PropertyExplanation {
            summary: summary.into(),
            decisive_observations: Vec::new(),
            decisive_sources: Vec::new(),
            trigger_observation: None,
            response_observation: None,
            pending_obligation: Some("well-formed-observation-stream".to_string()),
            active_assumptions: Vec::new(),
            normalized_quantities: Vec::new(),
            blame: Some("binding/evidence".to_string()),
            contract: None,
            clause: None,
        },
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct NormalizedQuantity {
    original_value: String,
    original_unit: String,
    dimension: String,
    normalized_numerator: String,
    normalized_denominator: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ObservationEnvelope {
    spec: String,
    sequence: usize,
    #[serde(default)]
    source: ObservationSourceMetadata,
    facts: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct ObservationSourceMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    instance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    causation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transition_case: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct CompiledProperty {
    id: String,
    observations: BTreeMap<String, ObservationDefinition>,
    assumptions: Vec<Assumption>,
    expression: PropertyExpression,
}

#[derive(Clone, Debug)]
pub(super) struct CompiledCoreExpression {
    observations: BTreeMap<String, ObservationDefinition>,
    predicate: Predicate,
}

impl CompiledCoreExpression {
    pub(super) fn evaluate(&self, assignments: &BTreeMap<String, Value>) -> Result<bool> {
        let facts = self
            .observations
            .iter()
            .map(|(id, definition)| {
                let observed = assignments
                    .get(id)
                    .map(|value| typed_observed_value(value, &definition.value_type))
                    .transpose()?
                    .unwrap_or(ObservedValue::Missing);
                Ok((id.clone(), observed))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let frame = Frame {
            index: 0,
            facts,
            source: ObservationSourceMetadata::default(),
            raw: Value::Null,
        };
        evaluate_predicate(&self.predicate, &frame, &mut Vec::new())
    }
}

impl CompiledProperty {
    pub(super) fn supports_vacuity_analysis(&self) -> bool {
        matches!(
            &self.expression,
            PropertyExpression::Temporal(
                TemporalExpression::Precedence { .. } | TemporalExpression::BoundedResponse { .. }
            )
        )
    }
}

#[derive(Clone, Debug)]
enum PropertyExpression {
    Temporal(TemporalExpression),
    Step(StepExpression),
}

#[derive(Clone, Debug)]
struct StepExpression {
    role: StepRole,
    predicate: Predicate,
    activation: Option<Predicate>,
    permits: Option<FramePermits>,
    source_digest: Option<String>,
    contract: Option<String>,
    clause: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct FramePermits {
    state_changes: Vec<String>,
    events: Vec<String>,
    effects: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
enum StepRole {
    Requirement,
    Guarantee,
    Invariant,
    Case,
    Coverage,
    Disjointness,
}

impl StepRole {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "requirement" => Some(Self::Requirement),
            "guarantee" => Some(Self::Guarantee),
            "invariant" => Some(Self::Invariant),
            "case" => Some(Self::Case),
            "coverage" => Some(Self::Coverage),
            "disjointness" => Some(Self::Disjointness),
            _ => None,
        }
    }

    fn blame(self) -> &'static str {
        match self {
            Self::Requirement => "caller",
            Self::Guarantee | Self::Invariant | Self::Case => "provider",
            Self::Coverage | Self::Disjointness => "contract",
        }
    }
}

#[derive(Clone, Debug)]
struct ObservationDefinition {
    id: String,
    source: ObservationSource,
    value_type: ObservationType,
}

#[derive(Clone, Debug)]
enum ObservationSource {
    Input {
        input_kind: Option<String>,
        name: String,
    },
    InputValue {
        pointer: String,
    },
    Output {
        output_kind: String,
        name: String,
    },
    OutputValue {
        pointer: String,
    },
    Transition {
        case: String,
    },
    State {
        phase: String,
        pointer: String,
        instance: Option<String>,
    },
    ProtocolMessage {
        name: String,
    },
    ProtocolState {
        name: String,
    },
    TraceMetric {
        name: String,
    },
}

#[derive(Clone, Debug)]
enum ObservationType {
    Occurrence,
    Boolean,
    Integer,
    String,
    Variant { cases: Vec<String> },
    Quantity { dimension: Dimension },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Dimension {
    Time,
    Information,
    Ratio,
    Transition,
    Message,
    Attempt,
    Item,
}

impl Dimension {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "time" => Some(Self::Time),
            "information" => Some(Self::Information),
            "ratio" => Some(Self::Ratio),
            "transition" | "transitions" => Some(Self::Transition),
            "message" | "messages" => Some(Self::Message),
            "attempt" | "attempts" => Some(Self::Attempt),
            "item" | "items" => Some(Self::Item),
            _ => None,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Time => "time",
            Self::Information => "information",
            Self::Ratio => "ratio",
            Self::Transition => "transition",
            Self::Message => "message",
            Self::Attempt => "attempt",
            Self::Item => "item",
        }
    }
}

#[derive(Clone, Debug)]
struct Assumption {
    id: String,
    kind: AssumptionKind,
    expression: TemporalExpression,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AssumptionKind {
    Environment,
    SearchPreference,
}

#[derive(Clone, Debug)]
enum TemporalExpression {
    Always(Predicate),
    Eventually(Predicate),
    Precedence {
        before: Predicate,
        after: Predicate,
    },
    Exclusion {
        left: Predicate,
        right: Predicate,
    },
    AtMostOnce(Predicate),
    BoundedResponse {
        trigger: Predicate,
        response: Predicate,
        metric: String,
        bound: Quantity,
    },
}

#[derive(Clone, Debug)]
enum Predicate {
    Constant(bool),
    Occurred(String),
    Equals {
        left: Term,
        right: Term,
    },
    Compare {
        left: Term,
        operator: CompareOperator,
        right: Term,
    },
    Not(Box<Predicate>),
    All(Vec<Predicate>),
    Any(Vec<Predicate>),
}

#[derive(Clone, Debug)]
enum Term {
    Observation(String),
    Literal(Value),
    Quantity(Quantity),
    Add(Vec<Term>),
    Subtract(Box<Term>, Box<Term>),
}

#[derive(Clone, Copy, Debug)]
enum CompareOperator {
    Lt,
    Lte,
    Eq,
    Gte,
    Gt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TermType {
    Boolean,
    Integer,
    String,
    Variant,
    Quantity(Dimension),
}

#[derive(Clone, Debug)]
struct Quantity {
    original_value: String,
    unit: String,
    dimension: Dimension,
    numerator: i128,
    denominator: i128,
}

impl Quantity {
    fn normalized(&self) -> NormalizedQuantity {
        NormalizedQuantity {
            original_value: self.original_value.clone(),
            original_unit: self.unit.clone(),
            dimension: self.dimension.label().to_string(),
            normalized_numerator: self.numerator.to_string(),
            normalized_denominator: self.denominator.to_string(),
        }
    }

    fn compare(&self, other: &Self) -> Result<Ordering> {
        if self.dimension != other.dimension {
            bail!(
                "quantity dimension `{}` is incompatible with `{}`",
                self.dimension.label(),
                other.dimension.label()
            );
        }
        let left = self
            .numerator
            .checked_mul(other.denominator)
            .ok_or_else(|| anyhow!("quantity comparison overflow"))?;
        let right = other
            .numerator
            .checked_mul(self.denominator)
            .ok_or_else(|| anyhow!("quantity comparison overflow"))?;
        Ok(left.cmp(&right))
    }

    fn difference(&self, earlier: &Self) -> Result<Self> {
        if self.dimension != earlier.dimension {
            bail!(
                "quantity dimension `{}` is incompatible with `{}`",
                self.dimension.label(),
                earlier.dimension.label()
            );
        }
        let common = self
            .denominator
            .checked_mul(earlier.denominator)
            .ok_or_else(|| anyhow!("quantity subtraction overflow"))?;
        let left = self
            .numerator
            .checked_mul(earlier.denominator)
            .ok_or_else(|| anyhow!("quantity subtraction overflow"))?;
        let right = earlier
            .numerator
            .checked_mul(self.denominator)
            .ok_or_else(|| anyhow!("quantity subtraction overflow"))?;
        Ok(Self {
            original_value: format!("{}-{}", self.original_value, earlier.original_value),
            unit: self.unit.clone(),
            dimension: self.dimension.clone(),
            numerator: left
                .checked_sub(right)
                .ok_or_else(|| anyhow!("quantity subtraction overflow"))?,
            denominator: common,
        })
    }

    fn sum(&self, other: &Self) -> Result<Self> {
        if self.dimension != other.dimension {
            bail!(
                "quantity dimension `{}` is incompatible with `{}`",
                self.dimension.label(),
                other.dimension.label()
            );
        }
        let common = self
            .denominator
            .checked_mul(other.denominator)
            .ok_or_else(|| anyhow!("quantity addition overflow"))?;
        let left = self
            .numerator
            .checked_mul(other.denominator)
            .ok_or_else(|| anyhow!("quantity addition overflow"))?;
        let right = other
            .numerator
            .checked_mul(self.denominator)
            .ok_or_else(|| anyhow!("quantity addition overflow"))?;
        Ok(Self {
            original_value: format!("{}+{}", self.original_value, other.original_value),
            unit: self.unit.clone(),
            dimension: self.dimension.clone(),
            numerator: left
                .checked_add(right)
                .ok_or_else(|| anyhow!("quantity addition overflow"))?,
            denominator: common,
        })
    }
}

#[derive(Clone, Debug)]
struct Frame {
    index: usize,
    facts: BTreeMap<String, ObservedValue>,
    source: ObservationSourceMetadata,
    raw: Value,
}

#[derive(Clone, Debug)]
enum ObservedValue {
    Missing,
    Scalar(Value),
    Quantity(Quantity),
}

#[derive(Clone, Debug)]
enum EvaluatedTerm {
    Missing,
    Scalar(Value),
    Integer(i128),
    Quantity(Quantity),
}

pub(super) fn compile_property(
    value: &Value,
) -> std::result::Result<CompiledProperty, Vec<String>> {
    let mut issues = Vec::new();
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("<unnamed-property>")
        .to_string();
    let observations = value
        .get("observations")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| match parse_observation(item) {
                    Ok(observation) => Some(observation),
                    Err(error) => {
                        issues.push(format!("{error:#}"));
                        None
                    }
                })
                .map(|observation| (observation.id.clone(), observation))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let assumptions = value
        .get("assumptions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| match parse_assumption(item) {
                    Ok(assumption) => Some(assumption),
                    Err(error) => {
                        issues.push(format!("{error:#}"));
                        None
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let temporal = match value.get("temporal") {
        Some(temporal) => {
            if temporal.get("pattern").is_some()
                || temporal.get("trigger").is_some()
                || temporal.get("condition").is_some()
                || temporal.get("bound").is_some()
            {
                issues.push(format!(
                    "property `{id}` uses removed descriptive temporal fields; declare `temporal.scope` and a closed `temporal.expression`"
                ));
                None
            } else {
                match temporal
                    .get("expression")
                    .ok_or_else(|| anyhow!("property `{id}` temporal block requires `expression`"))
                    .and_then(parse_temporal)
                {
                    Ok(expression) => Some(expression),
                    Err(error) => {
                        issues.push(format!("{error:#}"));
                        None
                    }
                }
            }
        }
        None => None,
    };
    let step = match value.get("step") {
        Some(step) => match parse_step(step) {
            Ok(step) => Some(step),
            Err(error) => {
                issues.push(format!("{error:#}"));
                None
            }
        },
        None => None,
    };
    if temporal.is_some() && step.is_some() {
        issues.push(format!(
            "property `{id}` must declare exactly one of `temporal` or `step`"
        ));
    }
    if temporal.is_none() && step.is_none() {
        issues.push(format!(
            "property `{id}` must declare executable `temporal` or `step` semantics"
        ));
    }
    if temporal.is_some() && observations.is_empty() {
        issues.push(format!(
            "temporal property `{id}` must declare non-empty `observations`"
        ));
    }
    if let Some(temporal) = &temporal {
        validate_temporal_references(temporal, &observations, &mut issues);
    }
    if let Some(step) = &step {
        validate_predicate_references(&step.predicate, &observations, &mut issues);
        if let Some(activation) = &step.activation {
            validate_predicate_references(activation, &observations, &mut issues);
        }
    }
    for assumption in &assumptions {
        validate_temporal_references(&assumption.expression, &observations, &mut issues);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let expression = match (temporal, step) {
        (Some(temporal), None) => PropertyExpression::Temporal(temporal),
        (None, Some(step)) => PropertyExpression::Step(step),
        _ => {
            return Err(vec![format!(
                "property `{id}` has no unique executable expression"
            )])
        }
    };
    Ok(CompiledProperty {
        id,
        observations,
        assumptions,
        expression,
    })
}

fn parse_step(value: &Value) -> Result<StepExpression> {
    let role = required_string(value, "role", "step")?;
    let role =
        StepRole::parse(role).ok_or_else(|| anyhow!("step has unsupported role `{role}`"))?;
    let predicate = parse_predicate(
        value
            .get("expression")
            .ok_or_else(|| anyhow!("step requires `expression`"))?,
    )?;
    let activation = value.get("activation").map(parse_predicate).transpose()?;
    let permits = value
        .get("permits")
        .map(|permits| {
            let strings = |field: &str| -> Result<Vec<String>> {
                permits
                    .get(field)
                    .and_then(Value::as_array)
                    .ok_or_else(|| anyhow!("step permits requires `{field}` array"))?
                    .iter()
                    .map(|item| {
                        item.as_str()
                            .filter(|value| !value.is_empty())
                            .map(str::to_string)
                            .ok_or_else(|| {
                                anyhow!("step permits `{field}` entries must be strings")
                            })
                    })
                    .collect()
            };
            Ok::<_, anyhow::Error>(FramePermits {
                state_changes: strings("state_changes")?,
                events: strings("events")?,
                effects: strings("effects")?,
            })
        })
        .transpose()?;
    Ok(StepExpression {
        role,
        predicate,
        activation,
        permits,
        source_digest: value
            .get("source_digest")
            .and_then(Value::as_str)
            .map(str::to_string),
        contract: value
            .get("contract")
            .and_then(Value::as_str)
            .map(str::to_string),
        clause: value
            .get("clause")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn parse_observation(value: &Value) -> Result<ObservationDefinition> {
    let id = required_string(value, "id", "observation")?.to_string();
    let source = value
        .get("source")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("observation `{id}` requires object `source`"))?;
    let kind = source
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("observation `{id}` source requires `kind`"))?;
    let source = match kind {
        "input" if source.get("pointer").is_some() => ObservationSource::InputValue {
            pointer: required_string_map(source, "pointer", &format!("observation `{id}` source"))?
                .to_string(),
        },
        "input" => ObservationSource::Input {
            input_kind: source
                .get("input_kind")
                .and_then(Value::as_str)
                .map(str::to_string),
            name: required_string_map(source, "name", &format!("observation `{id}` source"))?
                .to_string(),
        },
        "output" if source.get("pointer").is_some() => ObservationSource::OutputValue {
            pointer: required_string_map(source, "pointer", &format!("observation `{id}` source"))?
                .to_string(),
        },
        "output" => ObservationSource::Output {
            output_kind: required_string_map(
                source,
                "output_kind",
                &format!("observation `{id}` source"),
            )?
            .to_string(),
            name: required_string_map(source, "name", &format!("observation `{id}` source"))?
                .to_string(),
        },
        "transition" => ObservationSource::Transition {
            case: required_string_map(source, "case", &format!("observation `{id}` source"))?
                .to_string(),
        },
        "state" => ObservationSource::State {
            phase: required_string_map(source, "phase", &format!("observation `{id}` source"))?
                .to_string(),
            pointer: required_string_map(source, "pointer", &format!("observation `{id}` source"))?
                .to_string(),
            instance: source
                .get("instance")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
        "protocol-message" => ObservationSource::ProtocolMessage {
            name: required_string_map(source, "name", &format!("observation `{id}` source"))?
                .to_string(),
        },
        "protocol-state" => ObservationSource::ProtocolState {
            name: required_string_map(source, "name", &format!("observation `{id}` source"))?
                .to_string(),
        },
        "trace-metric" => ObservationSource::TraceMetric {
            name: required_string_map(source, "name", &format!("observation `{id}` source"))?
                .to_string(),
        },
        other => bail!("observation `{id}` has unsupported source kind `{other}`"),
    };
    let value_type = parse_observation_type(
        value
            .get("value")
            .ok_or_else(|| anyhow!("observation `{id}` requires `value`"))?,
    )
    .with_context(|| format!("observation `{id}`"))?;
    validate_source_type(&id, &source, &value_type)?;
    Ok(ObservationDefinition {
        id,
        source,
        value_type,
    })
}

fn parse_observation_type(value: &Value) -> Result<ObservationType> {
    if let Some(label) = value.as_str() {
        return match label {
            "occurrence" => Ok(ObservationType::Occurrence),
            "boolean" => Ok(ObservationType::Boolean),
            "integer" => Ok(ObservationType::Integer),
            "string" => Ok(ObservationType::String),
            other => bail!("unsupported observation value type `{other}`"),
        };
    }
    if let Some(cases) = value.get("variant").and_then(Value::as_array) {
        let cases = cases
            .iter()
            .map(|case| {
                case.as_str()
                    .filter(|case| !case.is_empty())
                    .map(str::to_string)
                    .ok_or_else(|| anyhow!("variant observation cases must be stable ids"))
            })
            .collect::<Result<Vec<_>>>()?;
        if cases.is_empty() {
            bail!("variant observation requires at least one closed case");
        }
        return Ok(ObservationType::Variant { cases });
    }
    let quantity = value
        .get("quantity")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("quantity observation must declare `value.quantity`"))?;
    let dimension = Dimension::parse(quantity)
        .ok_or_else(|| anyhow!("unknown quantity dimension `{quantity}`"))?;
    Ok(ObservationType::Quantity { dimension })
}

fn validate_source_type(
    id: &str,
    source: &ObservationSource,
    value_type: &ObservationType,
) -> Result<()> {
    let occurrence_source = matches!(
        source,
        ObservationSource::Input { .. }
            | ObservationSource::Output { .. }
            | ObservationSource::Transition { .. }
            | ObservationSource::ProtocolMessage { .. }
            | ObservationSource::ProtocolState { .. }
    );
    if matches!(value_type, ObservationType::Occurrence) && !occurrence_source {
        bail!("observation `{id}` cannot read `occurrence` from a value source");
    }
    if occurrence_source && !matches!(value_type, ObservationType::Occurrence) {
        bail!("observation `{id}` occurrence source requires `value: occurrence`");
    }
    Ok(())
}

fn parse_assumption(value: &Value) -> Result<Assumption> {
    let id = required_string(value, "id", "assumption")?.to_string();
    let kind = match required_string(value, "kind", &format!("assumption `{id}`"))? {
        "environment" => AssumptionKind::Environment,
        "search-preference" => AssumptionKind::SearchPreference,
        other => bail!("assumption `{id}` has unsupported kind `{other}`"),
    };
    let expression = parse_temporal(
        value
            .get("expression")
            .ok_or_else(|| anyhow!("assumption `{id}` requires `expression`"))?,
    )
    .with_context(|| format!("assumption `{id}`"))?;
    Ok(Assumption {
        id,
        kind,
        expression,
    })
}

fn parse_temporal(value: &Value) -> Result<TemporalExpression> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("temporal expression must be an object"))?;
    if object.len() != 1 {
        bail!("temporal expression must contain exactly one closed variant");
    }
    let Some((kind, body)) = object.iter().next() else {
        bail!("temporal expression must contain exactly one closed variant");
    };
    match kind.as_str() {
        "always" => Ok(TemporalExpression::Always(parse_predicate(body)?)),
        "eventually" => Ok(TemporalExpression::Eventually(parse_predicate(body)?)),
        "precedence" => Ok(TemporalExpression::Precedence {
            before: parse_predicate(
                body.get("before")
                    .ok_or_else(|| anyhow!("precedence requires `before`"))?,
            )?,
            after: parse_predicate(
                body.get("after")
                    .ok_or_else(|| anyhow!("precedence requires `after`"))?,
            )?,
        }),
        "exclusion" => Ok(TemporalExpression::Exclusion {
            left: parse_predicate(
                body.get("left")
                    .ok_or_else(|| anyhow!("exclusion requires `left`"))?,
            )?,
            right: parse_predicate(
                body.get("right")
                    .ok_or_else(|| anyhow!("exclusion requires `right`"))?,
            )?,
        }),
        "at_most_once" => Ok(TemporalExpression::AtMostOnce(parse_predicate(body)?)),
        "bounded_response" => {
            let trigger = parse_predicate(
                body.get("trigger")
                    .ok_or_else(|| anyhow!("bounded_response requires `trigger`"))?,
            )?;
            let response = parse_predicate(
                body.get("response")
                    .ok_or_else(|| anyhow!("bounded_response requires `response`"))?,
            )?;
            let within = body
                .get("within")
                .ok_or_else(|| anyhow!("bounded_response requires `within`"))?;
            let metric = required_string(within, "metric", "bounded_response.within")?.to_string();
            let bound = parse_quantity(within)?;
            Ok(TemporalExpression::BoundedResponse {
                trigger,
                response,
                metric,
                bound,
            })
        }
        other => bail!("unsupported temporal expression `{other}`"),
    }
}

fn parse_predicate(value: &Value) -> Result<Predicate> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("predicate must be an object"))?;
    if object.len() != 1 {
        bail!("predicate must contain exactly one closed variant");
    }
    let Some((kind, body)) = object.iter().next() else {
        bail!("predicate must contain exactly one closed variant");
    };
    match kind.as_str() {
        "constant" => Ok(Predicate::Constant(
            body.as_bool()
                .ok_or_else(|| anyhow!("`constant` requires a boolean"))?,
        )),
        "occurred" => Ok(Predicate::Occurred(
            body.as_str()
                .ok_or_else(|| anyhow!("`occurred` requires an observation id"))?
                .to_string(),
        )),
        "equals" if body.get("observation").is_some() => Ok(Predicate::Equals {
            left: Term::Observation(required_string(body, "observation", "equals")?.to_string()),
            right: Term::Literal(
                body.get("value")
                    .cloned()
                    .ok_or_else(|| anyhow!("equals requires `value`"))?,
            ),
        }),
        "equals" => Ok(Predicate::Equals {
            left: parse_term(
                body.get("left")
                    .ok_or_else(|| anyhow!("equals requires `left`"))?,
            )?,
            right: parse_term(
                body.get("right")
                    .ok_or_else(|| anyhow!("equals requires `right`"))?,
            )?,
        }),
        "compare" => {
            let operator = match required_string(body, "operator", "compare")? {
                "lt" => CompareOperator::Lt,
                "lte" => CompareOperator::Lte,
                "eq" => CompareOperator::Eq,
                "gte" => CompareOperator::Gte,
                "gt" => CompareOperator::Gt,
                other => bail!("compare has unsupported operator `{other}`"),
            };
            if body.get("observation").is_some() {
                Ok(Predicate::Compare {
                    left: Term::Observation(
                        required_string(body, "observation", "compare")?.to_string(),
                    ),
                    operator,
                    right: Term::Quantity(parse_quantity(body)?),
                })
            } else {
                Ok(Predicate::Compare {
                    left: parse_term(
                        body.get("left")
                            .ok_or_else(|| anyhow!("compare requires `left`"))?,
                    )?,
                    operator,
                    right: parse_term(
                        body.get("right")
                            .ok_or_else(|| anyhow!("compare requires `right`"))?,
                    )?,
                })
            }
        }
        "not" => Ok(Predicate::Not(Box::new(parse_predicate(body)?))),
        "all" => Ok(Predicate::All(parse_predicate_array(body, "all")?)),
        "any" => Ok(Predicate::Any(parse_predicate_array(body, "any")?)),
        other => bail!("unsupported predicate `{other}`"),
    }
}

fn parse_term(value: &Value) -> Result<Term> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("term must be an object"))?;
    if object.len() != 1 {
        bail!("term must contain exactly one closed variant");
    }
    let Some((kind, body)) = object.iter().next() else {
        bail!("term must contain exactly one closed variant");
    };
    match kind.as_str() {
        "observation" => Ok(Term::Observation(
            body.as_str()
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow!("observation term requires an id"))?
                .to_string(),
        )),
        "literal" if body.get("unit").is_some() && body.get("value").is_some() => {
            Ok(Term::Quantity(parse_quantity(body)?))
        }
        "literal" => Ok(Term::Literal(body.clone())),
        "add" => {
            let terms = body
                .as_array()
                .filter(|items| items.len() >= 2)
                .ok_or_else(|| anyhow!("add requires at least two terms"))?;
            Ok(Term::Add(
                terms.iter().map(parse_term).collect::<Result<Vec<_>>>()?,
            ))
        }
        "subtract" => Ok(Term::Subtract(
            Box::new(parse_term(
                body.get("left")
                    .ok_or_else(|| anyhow!("subtract requires `left`"))?,
            )?),
            Box::new(parse_term(
                body.get("right")
                    .ok_or_else(|| anyhow!("subtract requires `right`"))?,
            )?),
        )),
        other => bail!("unsupported term `{other}`"),
    }
}

fn parse_predicate_array(value: &Value, label: &str) -> Result<Vec<Predicate>> {
    let items = value
        .as_array()
        .filter(|items| !items.is_empty())
        .ok_or_else(|| anyhow!("`{label}` requires a non-empty predicate array"))?;
    items.iter().map(parse_predicate).collect()
}

fn validate_temporal_references(
    expression: &TemporalExpression,
    observations: &BTreeMap<String, ObservationDefinition>,
    issues: &mut Vec<String>,
) {
    for predicate in temporal_predicates(expression) {
        validate_predicate_references(predicate, observations, issues);
    }
    if let TemporalExpression::BoundedResponse { metric, bound, .. } = expression {
        match observations.get(metric) {
            Some(ObservationDefinition {
                value_type: ObservationType::Quantity { dimension },
                ..
            }) if dimension == &bound.dimension => {}
            Some(ObservationDefinition {
                value_type: ObservationType::Quantity { dimension },
                ..
            }) => issues.push(format!(
                "bounded_response metric `{metric}` has dimension `{}` but bound unit `{}` has dimension `{}`",
                dimension.label(),
                bound.unit,
                bound.dimension.label()
            )),
            Some(_) => issues.push(format!(
                "bounded_response metric `{metric}` must be a quantity observation"
            )),
            None => issues.push(format!(
                "bounded_response references unknown metric observation `{metric}`"
            )),
        }
    }
}

fn temporal_predicates(expression: &TemporalExpression) -> Vec<&Predicate> {
    match expression {
        TemporalExpression::Always(predicate)
        | TemporalExpression::Eventually(predicate)
        | TemporalExpression::AtMostOnce(predicate) => vec![predicate],
        TemporalExpression::Precedence { before, after } => vec![before, after],
        TemporalExpression::Exclusion { left, right } => vec![left, right],
        TemporalExpression::BoundedResponse {
            trigger, response, ..
        } => vec![trigger, response],
    }
}

fn validate_predicate_references(
    predicate: &Predicate,
    observations: &BTreeMap<String, ObservationDefinition>,
    issues: &mut Vec<String>,
) {
    match predicate {
        Predicate::Constant(_) => {}
        Predicate::Occurred(id) => match observations.get(id) {
            Some(ObservationDefinition {
                value_type: ObservationType::Occurrence,
                ..
            }) => {}
            Some(_) => issues.push(format!("`occurred` requires occurrence observation `{id}`")),
            None => issues.push(format!("predicate references unknown observation `{id}`")),
        },
        Predicate::Equals { left, right } => {
            validate_term_references(left, observations, issues);
            validate_term_references(right, observations, issues);
            if let (Some(left), Some(right)) = (
                infer_term_type(left, observations, issues),
                infer_term_type(right, observations, issues),
            ) {
                let compatible = left == right
                    || matches!(
                        (&left, &right),
                        (TermType::String, TermType::Variant)
                            | (TermType::Variant, TermType::String)
                    );
                if !compatible {
                    issues.push(format!(
                        "equality compares incompatible term types {left:?} and {right:?}"
                    ));
                }
            }
        }
        Predicate::Compare { left, right, .. } => {
            validate_term_references(left, observations, issues);
            validate_term_references(right, observations, issues);
            if let (Some(left), Some(right)) = (
                infer_term_type(left, observations, issues),
                infer_term_type(right, observations, issues),
            ) {
                if left != right
                    || !matches!(
                        left,
                        TermType::Integer | TermType::String | TermType::Quantity(_)
                    )
                {
                    issues.push(format!(
                        "ordered comparison requires matching integer, string, or quantity terms; found {left:?} and {right:?}"
                    ));
                }
            }
            if let (Some(left), Some(right)) = (
                term_quantity_dimension(left, observations),
                term_quantity_dimension(right, observations),
            ) {
                if left != right {
                    issues.push(format!(
                        "comparison quantity dimensions `{}` and `{}` are incompatible",
                        left.label(),
                        right.label()
                    ));
                }
            }
        }
        Predicate::Not(predicate) => validate_predicate_references(predicate, observations, issues),
        Predicate::All(predicates) | Predicate::Any(predicates) => {
            for predicate in predicates {
                validate_predicate_references(predicate, observations, issues);
            }
        }
    }
}

fn infer_term_type(
    term: &Term,
    observations: &BTreeMap<String, ObservationDefinition>,
    issues: &mut Vec<String>,
) -> Option<TermType> {
    match term {
        Term::Observation(id) => {
            observations
                .get(id)
                .and_then(|observation| match &observation.value_type {
                    ObservationType::Occurrence | ObservationType::Boolean => {
                        Some(TermType::Boolean)
                    }
                    ObservationType::Integer => Some(TermType::Integer),
                    ObservationType::String => Some(TermType::String),
                    ObservationType::Variant { .. } => Some(TermType::Variant),
                    ObservationType::Quantity { dimension } => {
                        Some(TermType::Quantity(dimension.clone()))
                    }
                })
        }
        Term::Literal(value) => {
            if value.is_boolean() {
                Some(TermType::Boolean)
            } else if value.is_i64() || value.is_u64() {
                Some(TermType::Integer)
            } else if value.is_string() {
                Some(TermType::String)
            } else {
                issues.push("literal terms must be booleans, integers, strings, closed variants, or exact quantities".to_string());
                None
            }
        }
        Term::Quantity(quantity) => Some(TermType::Quantity(quantity.dimension.clone())),
        Term::Add(terms) => {
            let types = terms
                .iter()
                .filter_map(|term| infer_term_type(term, observations, issues))
                .collect::<Vec<_>>();
            let first = types.first()?.clone();
            if types.len() != terms.len()
                || !types.iter().all(|value| value == &first)
                || !matches!(first, TermType::Integer | TermType::Quantity(_))
            {
                issues.push(
                    "addition requires matching integer or dimensionally equal quantity terms"
                        .to_string(),
                );
                None
            } else {
                Some(first)
            }
        }
        Term::Subtract(left, right) => {
            let left = infer_term_type(left, observations, issues)?;
            let right = infer_term_type(right, observations, issues)?;
            if left != right || !matches!(left, TermType::Integer | TermType::Quantity(_)) {
                issues.push(
                    "subtraction requires matching integer or dimensionally equal quantity terms"
                        .to_string(),
                );
                None
            } else {
                Some(left)
            }
        }
    }
}

fn validate_term_references(
    term: &Term,
    observations: &BTreeMap<String, ObservationDefinition>,
    issues: &mut Vec<String>,
) {
    match term {
        Term::Observation(id) => {
            if !observations.contains_key(id) {
                issues.push(format!("term references unknown observation `{id}`"));
            }
        }
        Term::Literal(_) | Term::Quantity(_) => {}
        Term::Add(terms) => {
            for term in terms {
                validate_term_references(term, observations, issues);
            }
        }
        Term::Subtract(left, right) => {
            validate_term_references(left, observations, issues);
            validate_term_references(right, observations, issues);
        }
    }
}

fn term_quantity_dimension<'a>(
    term: &'a Term,
    observations: &'a BTreeMap<String, ObservationDefinition>,
) -> Option<&'a Dimension> {
    match term {
        Term::Observation(id) => match &observations.get(id)?.value_type {
            ObservationType::Quantity { dimension } => Some(dimension),
            _ => None,
        },
        Term::Quantity(quantity) => Some(&quantity.dimension),
        Term::Add(terms) => terms
            .iter()
            .find_map(|term| term_quantity_dimension(term, observations)),
        Term::Subtract(left, _) => term_quantity_dimension(left, observations),
        Term::Literal(_) => None,
    }
}

pub(super) fn evaluate_trace(property: &CompiledProperty, trace: &Value) -> Result<Evaluation> {
    if let Some(error) = record_integrity_error(trace) {
        let (contract, clause) = match &property.expression {
            PropertyExpression::Step(step) => (step.contract.clone(), step.clause.clone()),
            PropertyExpression::Temporal(_) => (None, None),
        };
        return Ok(Evaluation {
            property: property.id.clone(),
            verdict: Verdict::Invalid,
            trace_complete: false,
            observations: 0,
            explanation: PropertyExplanation {
                summary: error,
                decisive_observations: Vec::new(),
                decisive_sources: Vec::new(),
                trigger_observation: None,
                response_observation: None,
                pending_obligation: Some("well-formed-record".to_string()),
                active_assumptions: Vec::new(),
                normalized_quantities: Vec::new(),
                blame: Some("binding/evidence".to_string()),
                contract,
                clause,
            },
        });
    }
    let (frames, complete) = project_trace(property, trace)?;
    evaluate_frames(property, &frames, complete)
}

fn record_integrity_error(trace: &Value) -> Option<String> {
    match trace.get("spec").and_then(Value::as_str) {
        Some("rms/invocation-record/v0.1") => {
            for field in ["contract", "binding", "contract_digest", "input", "output"] {
                if trace.get(field).is_none() {
                    return Some(format!("invocation record is missing required `{field}`"));
                }
            }
            if !trace
                .get("contract_digest")
                .and_then(Value::as_str)
                .and_then(|digest| digest.strip_prefix("sha256:"))
                .is_some_and(|digest| {
                    digest.len() == 64
                        && digest
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                })
            {
                return Some("invocation record has an invalid contract digest".to_string());
            }
            for field in ["contract", "binding"] {
                if !trace
                    .get(field)
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.is_empty())
                {
                    return Some(format!(
                        "invocation record `{field}` must be a non-empty string"
                    ));
                }
            }
            if trace
                .get("source")
                .is_some_and(|source| !source.is_object())
            {
                return Some("invocation record `source` must be an object".to_string());
            }
            let allowed = [
                "spec",
                "contract",
                "binding",
                "contract_digest",
                "correlation_id",
                "causation_id",
                "scenario_start",
                "input",
                "output",
                "state_before",
                "state_after",
                "outputs",
                "source",
            ];
            if let Some(field) = trace
                .as_object()
                .into_iter()
                .flat_map(|object| object.keys())
                .find(|field| !allowed.contains(&field.as_str()))
            {
                return Some(format!(
                    "invocation record contains unknown field `{field}`"
                ));
            }
            None
        }
        Some("rms/trace-bundle/v0.1") => trace
            .get("records")
            .or_else(|| trace.get("transitions"))
            .or_else(|| trace.get("journal"))
            .and_then(Value::as_array)
            .and_then(|records| {
                records.iter().enumerate().find_map(|(index, record)| {
                    if record.get("spec").and_then(Value::as_str)
                        == Some("rms/invocation-record/v0.1")
                    {
                        return record_integrity_error(record)
                            .map(|error| format!("invocation record {index}: {error}"));
                    }
                    ["input", "output"]
                        .into_iter()
                        .find(|field| record.get(*field).is_none())
                        .map(|field| {
                            format!("transition record {index} is missing required `{field}`")
                        })
                })
            }),
        Some("rms/probe-system-trace/v0.1") => None,
        Some(other) => Some(format!("unsupported record protocol `{other}`")),
        None => Some("record has no `spec` discriminator".to_string()),
    }
}

pub(super) fn evaluate_observation_envelopes(
    property: &CompiledProperty,
    envelopes: &[ObservationEnvelope],
    complete: bool,
) -> Result<Evaluation> {
    for envelope in envelopes {
        if envelope.spec != OBSERVATION_SPEC {
            bail!(
                "observation envelope declares `{}`, expected `{OBSERVATION_SPEC}`",
                envelope.spec
            );
        }
    }
    let frames = envelopes
        .iter()
        .map(|envelope| Frame {
            index: envelope.sequence,
            facts: envelope
                .facts
                .iter()
                .map(|(id, value)| {
                    let observed = match property.observations.get(id) {
                        Some(definition) => typed_observed_value(value, &definition.value_type)
                            .unwrap_or(ObservedValue::Missing),
                        None => ObservedValue::Scalar(value.clone()),
                    };
                    (id.clone(), observed)
                })
                .collect(),
            source: envelope.source.clone(),
            raw: serde_json::json!({
                "spec": OBSERVATION_SPEC,
                "sequence": envelope.sequence,
                "facts": envelope.facts,
            }),
        })
        .collect::<Vec<_>>();
    evaluate_frames(property, &frames, complete)
}

pub(super) fn evaluate_core_expression(
    observations: &[Value],
    expression: &Value,
    assignments: &BTreeMap<String, Value>,
) -> Result<bool> {
    compile_core_expression(observations, expression)?.evaluate(assignments)
}

pub(super) fn compile_core_expression(
    observations: &[Value],
    expression: &Value,
) -> Result<CompiledCoreExpression> {
    let mut definitions = BTreeMap::new();
    for observation in observations {
        let definition = parse_observation(observation)?;
        if definitions
            .insert(definition.id.clone(), definition.clone())
            .is_some()
        {
            bail!("duplicate observation id `{}`", definition.id);
        }
    }
    let predicate = parse_predicate(expression)?;
    let mut issues = Vec::new();
    validate_predicate_references(&predicate, &definitions, &mut issues);
    if !issues.is_empty() {
        bail!(issues.join("; "));
    }
    Ok(CompiledCoreExpression {
        observations: definitions,
        predicate,
    })
}

pub(super) fn normalize_trace(
    property: &CompiledProperty,
    trace: &Value,
) -> Result<Vec<ObservationEnvelope>> {
    if record_integrity_error(trace).is_some() {
        return Ok(Vec::new());
    }
    let (frames, _) = project_trace(property, trace)?;
    Ok(frames
        .into_iter()
        .map(|frame| ObservationEnvelope {
            spec: OBSERVATION_SPEC.to_string(),
            sequence: frame.index,
            source: frame.source,
            facts: frame
                .facts
                .into_iter()
                .filter_map(|(id, value)| match value {
                    ObservedValue::Missing => None,
                    ObservedValue::Scalar(value) => Some((id, value)),
                    ObservedValue::Quantity(quantity) => Some((
                        id,
                        serde_json::json!({
                            "value": quantity.original_value,
                            "unit": quantity.unit
                        }),
                    )),
                })
                .collect(),
        })
        .collect())
}

fn project_trace(property: &CompiledProperty, trace: &Value) -> Result<(Vec<Frame>, bool)> {
    let spec = trace
        .get("spec")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let (records, complete): (Vec<&Value>, bool) = match spec {
        "rms/probe-system-trace/v0.1" => (
            trace
                .get("timeline")
                .and_then(Value::as_array)
                .ok_or_else(|| anyhow!("probe system trace has no `timeline`"))?
                .iter()
                .collect(),
            trace
                .get("exhausted")
                .and_then(Value::as_bool)
                .unwrap_or_else(|| trace.get("result").and_then(Value::as_str) == Some("pass")),
        ),
        "rms/trace-bundle/v0.1" => (
            trace
                .get("records")
                .or_else(|| trace.get("transitions"))
                .or_else(|| trace.get("journal"))
                .and_then(Value::as_array)
                .ok_or_else(|| anyhow!("trace bundle has no records"))?
                .iter()
                .collect(),
            trace
                .get("complete")
                .and_then(Value::as_bool)
                .unwrap_or(true),
        ),
        "rms/invocation-record/v0.1" => (vec![trace], true),
        other => bail!("unsupported trace spec `{other}`"),
    };
    let mut frames = Vec::with_capacity(records.len());
    for (index, raw) in records.iter().enumerate() {
        let source = source_metadata(raw);
        let mut facts = BTreeMap::new();
        for definition in property.observations.values() {
            let observed = project_observation(definition, raw, index)?;
            facts.insert(definition.id.clone(), observed);
        }
        frames.push(Frame {
            index,
            facts,
            source,
            raw: (*raw).clone(),
        });
    }
    Ok((frames, complete))
}

fn source_metadata(raw: &Value) -> ObservationSourceMetadata {
    ObservationSourceMetadata {
        instance: raw
            .get("target")
            .or_else(|| raw.get("machine"))
            .and_then(Value::as_str)
            .map(str::to_string),
        route: raw.get("route").and_then(Value::as_str).map(str::to_string),
        correlation_id: raw
            .get("correlation_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        causation_id: raw
            .get("causation_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        transition_case: raw
            .get("transition_case")
            .and_then(Value::as_str)
            .or_else(|| raw.pointer("/source/branch").and_then(Value::as_str))
            .map(str::to_string),
    }
}

fn project_observation(
    definition: &ObservationDefinition,
    raw: &Value,
    index: usize,
) -> Result<ObservedValue> {
    let value = match &definition.source {
        ObservationSource::Input { input_kind, name } => {
            let input = raw.get("input");
            let observed_name = input.and_then(variant_name);
            let observed_kind = input.and_then(|input| input.get("kind")).and_then(Value::as_str);
            Value::Bool(
                observed_name.is_some_and(|observed| variant_matches(observed, name))
                    && input_kind
                        .as_deref()
                        .is_none_or(|expected| observed_kind.is_none_or(|kind| kind == expected)),
            )
        }
        ObservationSource::InputValue { pointer } => match raw
            .get("input")
            .and_then(|input| input.pointer(pointer))
        {
            Some(value) => value.clone(),
            None => return Ok(ObservedValue::Missing),
        },
        ObservationSource::Output { output_kind, name } => {
            Value::Bool(output_occurs(raw, output_kind, name))
        }
        ObservationSource::OutputValue { pointer } => match raw
            .get("output")
            .and_then(|output| output.pointer(pointer))
            .or_else(|| raw.pointer(pointer))
        {
            Some(value) => value.clone(),
            None => return Ok(ObservedValue::Missing),
        },
        ObservationSource::Transition { case } => Value::Bool(
            raw.get("transition_case")
                .and_then(Value::as_str)
                .or_else(|| raw.pointer("/source/branch").and_then(Value::as_str))
                == Some(case.as_str()),
        ),
        ObservationSource::State {
            phase,
            pointer,
            instance,
        } => {
            if instance.as_deref().is_some_and(|expected| {
                raw.get("target").and_then(Value::as_str) != Some(expected)
            }) {
                return Ok(ObservedValue::Missing);
            }
            let state = match phase.as_str() {
                "before" => raw.get("state_before"),
                "after" => raw
                    .get("state_after")
                    .or_else(|| raw.pointer("/output/next_state")),
                other => bail!("unsupported state phase `{other}`"),
            };
            match state.and_then(|state| state.pointer(pointer)) {
                Some(value) => value.clone(),
                None => return Ok(ObservedValue::Missing),
            }
        }
        ObservationSource::ProtocolMessage { name } => Value::Bool(
            raw.get("route").and_then(Value::as_str) == Some(name)
                || raw.get("protocol_message").and_then(Value::as_str) == Some(name),
        ),
        ObservationSource::ProtocolState { name } => Value::Bool(
            raw.get("protocol_state").and_then(Value::as_str) == Some(name),
        ),
        ObservationSource::TraceMetric { name } => match name.as_str() {
            "elapsed" => raw
                .get("time")
                .filter(|value| value.get("value").is_some() && value.get("unit").is_some())
                .cloned()
                .unwrap_or_else(|| {
                    serde_json::json!({
                        "value": raw.get("time").and_then(number_text).unwrap_or_else(|| index.to_string()),
                        "unit": "ms"
                    })
                }),
            "transition-count" => {
                serde_json::json!({"value": index.to_string(), "unit": "transition"})
            }
            "attempt-count" => serde_json::json!({
                "value": raw.get("attempt").and_then(number_text).unwrap_or_else(|| "0".to_string()),
                "unit": "attempt"
            }),
            "message-count" => serde_json::json!({
                "value": raw.get("outputs").and_then(Value::as_array).map(Vec::len).unwrap_or(0).to_string(),
                "unit": "message"
            }),
            other => bail!("unsupported trace metric `{other}`"),
        },
    };
    typed_observed_value(&value, &definition.value_type)
}

fn typed_observed_value(value: &Value, value_type: &ObservationType) -> Result<ObservedValue> {
    match value_type {
        ObservationType::Occurrence | ObservationType::Boolean => {
            if value.as_bool().is_none() {
                bail!("expected boolean observation value");
            }
            Ok(ObservedValue::Scalar(value.clone()))
        }
        ObservationType::Integer => {
            if value.as_i64().is_none() && value.as_u64().is_none() {
                bail!("expected integer observation value");
            }
            Ok(ObservedValue::Scalar(value.clone()))
        }
        ObservationType::String => {
            if value.as_str().is_none() {
                bail!("expected string observation value");
            }
            Ok(ObservedValue::Scalar(value.clone()))
        }
        ObservationType::Variant { cases } => variant_name(value)
            .filter(|name| cases.iter().any(|case| variant_matches(name, case)))
            .map(|name| ObservedValue::Scalar(Value::String(name.to_string())))
            .ok_or_else(|| anyhow!("expected one of the closed variant observation cases")),
        ObservationType::Quantity { dimension } => {
            let quantity = parse_quantity(value)?;
            if &quantity.dimension != dimension {
                bail!(
                    "quantity has dimension `{}`, expected `{}`",
                    quantity.dimension.label(),
                    dimension.label()
                );
            }
            Ok(ObservedValue::Quantity(quantity))
        }
    }
}

fn output_occurs(raw: &Value, output_kind: &str, name: &str) -> bool {
    if let Some(outputs) = raw.get("outputs").and_then(Value::as_array) {
        if outputs.iter().any(|output| {
            output.get("kind").and_then(Value::as_str) == Some(output_kind)
                && output
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|observed| variant_matches(observed, name))
        }) {
            return true;
        }
    }
    let output = raw.get("output");
    match output_kind {
        "event" | "command" | "effect" => {
            let field = format!("{output_kind}s");
            output
                .and_then(|output| output.get(&field))
                .and_then(Value::as_array)
                .is_some_and(|items| {
                    items.iter().any(|item| {
                        variant_name(item).is_some_and(|observed| variant_matches(observed, name))
                    })
                })
        }
        "reply" | "rejection" => output
            .and_then(|output| output.get(output_kind))
            .and_then(variant_name)
            .is_some_and(|observed| variant_matches(observed, name)),
        _ => false,
    }
}

fn evaluate_frames(
    property: &CompiledProperty,
    frames: &[Frame],
    complete: bool,
) -> Result<Evaluation> {
    let active_assumptions = property
        .assumptions
        .iter()
        .map(|assumption| assumption.id.clone())
        .collect::<Vec<_>>();
    for assumption in property
        .assumptions
        .iter()
        .filter(|assumption| assumption.kind == AssumptionKind::Environment)
    {
        let result = evaluate_temporal(&assumption.expression, frames, complete)?;
        if result.verdict != Verdict::Satisfied {
            let decisive_sources = decisive_sources(frames, &result.decisive);
            return Ok(Evaluation {
                property: property.id.clone(),
                verdict: Verdict::Inconclusive,
                trace_complete: complete,
                observations: frames.len(),
                explanation: PropertyExplanation {
                    summary: format!(
                        "environment assumption `{}` was not satisfied",
                        assumption.id
                    ),
                    decisive_observations: result.decisive,
                    decisive_sources,
                    trigger_observation: result.trigger,
                    response_observation: result.response,
                    pending_obligation: result.pending,
                    active_assumptions,
                    normalized_quantities: result.quantities,
                    blame: None,
                    contract: None,
                    clause: None,
                },
            });
        }
    }
    if let PropertyExpression::Step(step) = &property.expression {
        return evaluate_step(property, step, frames, complete, active_assumptions);
    }
    let PropertyExpression::Temporal(temporal) = &property.expression else {
        unreachable!("step expressions return above")
    };
    let result = evaluate_temporal(temporal, frames, complete)?;
    let decisive_sources = decisive_sources(frames, &result.decisive);
    Ok(Evaluation {
        property: property.id.clone(),
        verdict: result.verdict,
        trace_complete: complete,
        observations: frames.len(),
        explanation: PropertyExplanation {
            summary: result.summary,
            decisive_observations: result.decisive,
            decisive_sources,
            trigger_observation: result.trigger,
            response_observation: result.response,
            pending_obligation: result.pending,
            active_assumptions,
            normalized_quantities: result.quantities,
            blame: None,
            contract: None,
            clause: None,
        },
    })
}

fn evaluate_step(
    property: &CompiledProperty,
    step: &StepExpression,
    frames: &[Frame],
    complete: bool,
    active_assumptions: Vec<String>,
) -> Result<Evaluation> {
    if frames.is_empty() {
        return Ok(Evaluation {
            property: property.id.clone(),
            verdict: Verdict::Inconclusive,
            trace_complete: complete,
            observations: 0,
            explanation: PropertyExplanation {
                summary: "step contract had no observations; coverage is missing".to_string(),
                decisive_observations: Vec::new(),
                decisive_sources: Vec::new(),
                trigger_observation: None,
                response_observation: None,
                pending_obligation: Some("observation".to_string()),
                active_assumptions,
                normalized_quantities: Vec::new(),
                blame: Some("evidence".to_string()),
                contract: step.contract.clone(),
                clause: step.clause.clone(),
            },
        });
    }
    for frame in frames {
        let mut quantities = Vec::new();
        if frame.raw.get("spec").and_then(Value::as_str) == Some("rms/invocation-record/v0.1") {
            let contract_mismatch = step.contract.as_deref().is_some_and(|contract| {
                frame.raw.get("contract").and_then(Value::as_str) != Some(contract)
            });
            let digest_mismatch = step.source_digest.as_deref().is_some_and(|digest| {
                frame.raw.get("contract_digest").and_then(Value::as_str) != Some(digest)
            });
            if contract_mismatch || digest_mismatch {
                return Ok(step_evaluation(
                    property,
                    step,
                    Verdict::Invalid,
                    complete,
                    frames,
                    frame,
                    if contract_mismatch {
                        "invocation record names a different behavioral contract".to_string()
                    } else {
                        "invocation record contract digest does not match the evaluated source"
                            .to_string()
                    },
                    Some("binding/evidence"),
                    active_assumptions,
                    quantities,
                ));
            }
        }
        let mut missing = BTreeMap::new();
        collect_missing_observations(&step.predicate, frame, &mut missing);
        if let Some(activation) = &step.activation {
            collect_missing_observations(activation, frame, &mut missing);
        }
        if !missing.is_empty() {
            return Ok(step_evaluation(
                property,
                step,
                Verdict::Inconclusive,
                complete,
                frames,
                frame,
                format!(
                    "record is missing observations required by the contract: {}",
                    missing.keys().cloned().collect::<Vec<_>>().join(", ")
                ),
                Some("evidence"),
                active_assumptions,
                quantities,
            ));
        }
        if !evaluate_predicate(&step.predicate, frame, &mut quantities)? {
            return Ok(step_evaluation(
                property,
                step,
                Verdict::Violated,
                complete,
                frames,
                frame,
                format!(
                    "{} step predicate failed at observation {}",
                    match step.role {
                        StepRole::Requirement => "caller requirement",
                        StepRole::Guarantee => "provider guarantee",
                        StepRole::Invariant => "invariant",
                        StepRole::Case => "behavior case",
                        StepRole::Coverage => "case coverage",
                        StepRole::Disjointness => "case disjointness",
                    },
                    frame.index
                ),
                Some(step.role.blame()),
                active_assumptions,
                quantities,
            ));
        }
        if let (Some(activation), Some(permits)) = (&step.activation, &step.permits) {
            if evaluate_predicate(activation, frame, &mut quantities)? {
                if let Some(reason) = frame_violation(&frame.raw, permits) {
                    return Ok(step_evaluation(
                        property,
                        step,
                        Verdict::Violated,
                        complete,
                        frames,
                        frame,
                        reason,
                        Some("provider"),
                        active_assumptions,
                        quantities,
                    ));
                }
            }
        }
    }
    Ok(Evaluation {
        property: property.id.clone(),
        verdict: Verdict::Satisfied,
        trace_complete: complete,
        observations: frames.len(),
        explanation: PropertyExplanation {
            summary: "step predicate held for every observed invocation".to_string(),
            decisive_observations: Vec::new(),
            decisive_sources: Vec::new(),
            trigger_observation: None,
            response_observation: None,
            pending_obligation: None,
            active_assumptions,
            normalized_quantities: Vec::new(),
            blame: None,
            contract: step.contract.clone(),
            clause: step.clause.clone(),
        },
    })
}

#[allow(clippy::too_many_arguments)]
fn step_evaluation(
    property: &CompiledProperty,
    step: &StepExpression,
    verdict: Verdict,
    complete: bool,
    frames: &[Frame],
    frame: &Frame,
    summary: String,
    blame: Option<&str>,
    active_assumptions: Vec<String>,
    quantities: Vec<NormalizedQuantity>,
) -> Evaluation {
    Evaluation {
        property: property.id.clone(),
        verdict,
        trace_complete: complete,
        observations: frames.len(),
        explanation: PropertyExplanation {
            summary,
            decisive_observations: vec![frame.index],
            decisive_sources: decisive_sources(frames, &[frame.index]),
            trigger_observation: None,
            response_observation: None,
            pending_obligation: None,
            active_assumptions,
            normalized_quantities: quantities,
            blame: blame.map(str::to_string),
            contract: step.contract.clone(),
            clause: step.clause.clone(),
        },
    }
}

fn collect_missing_observations(
    predicate: &Predicate,
    frame: &Frame,
    missing: &mut BTreeMap<String, ()>,
) {
    match predicate {
        Predicate::Constant(_) => {}
        Predicate::Occurred(id) => {
            if matches!(frame.facts.get(id), None | Some(ObservedValue::Missing)) {
                missing.insert(id.clone(), ());
            }
        }
        Predicate::Equals { left, right } | Predicate::Compare { left, right, .. } => {
            collect_missing_terms(left, frame, missing);
            collect_missing_terms(right, frame, missing);
        }
        Predicate::Not(predicate) => collect_missing_observations(predicate, frame, missing),
        Predicate::All(predicates) | Predicate::Any(predicates) => {
            for predicate in predicates {
                collect_missing_observations(predicate, frame, missing);
            }
        }
    }
}

fn collect_missing_terms(term: &Term, frame: &Frame, missing: &mut BTreeMap<String, ()>) {
    match term {
        Term::Observation(id) => {
            if matches!(frame.facts.get(id), None | Some(ObservedValue::Missing)) {
                missing.insert(id.clone(), ());
            }
        }
        Term::Add(terms) => {
            for term in terms {
                collect_missing_terms(term, frame, missing);
            }
        }
        Term::Subtract(left, right) => {
            collect_missing_terms(left, frame, missing);
            collect_missing_terms(right, frame, missing);
        }
        Term::Literal(_) | Term::Quantity(_) => {}
    }
}

fn frame_violation(raw: &Value, permits: &FramePermits) -> Option<String> {
    let state_before = raw.get("state_before");
    let state_after = raw
        .get("state_after")
        .or_else(|| raw.pointer("/output/next_state"));
    if let (Some(before), Some(after)) = (state_before, state_after) {
        let mut changes = Vec::new();
        changed_json_pointers(before, after, "", &mut changes);
        if let Some(change) = changes.iter().find(|change| {
            !permits.state_changes.iter().any(|allowed| {
                allowed == "/"
                    || *change == allowed
                    || change
                        .strip_prefix(allowed)
                        .is_some_and(|tail| tail.starts_with('/'))
            })
        }) {
            return Some(format!(
                "provider changed state outside the permitted frame at `{change}`"
            ));
        }
    }
    for (kind, allowed) in [("event", &permits.events), ("effect", &permits.effects)] {
        for observed in emitted_names(raw, kind) {
            if !allowed.iter().any(|name| variant_matches(&observed, name)) {
                return Some(format!(
                    "provider emitted undeclared {kind} `{observed}` outside the permitted frame"
                ));
            }
        }
    }
    None
}

fn changed_json_pointers(before: &Value, after: &Value, pointer: &str, changes: &mut Vec<String>) {
    if before == after {
        return;
    }
    match (before, after) {
        (Value::Object(before), Value::Object(after)) => {
            let mut keys = before.keys().chain(after.keys()).collect::<Vec<_>>();
            keys.sort();
            keys.dedup();
            for key in keys {
                let escaped = key.replace('~', "~0").replace('/', "~1");
                let child = format!("{pointer}/{escaped}");
                match (before.get(key), after.get(key)) {
                    (Some(before), Some(after)) => {
                        changed_json_pointers(before, after, &child, changes)
                    }
                    _ => changes.push(child),
                }
            }
        }
        _ => changes.push(if pointer.is_empty() {
            "/".to_string()
        } else {
            pointer.to_string()
        }),
    }
}

fn emitted_names(raw: &Value, kind: &str) -> Vec<String> {
    let plural = format!("{kind}s");
    let mut names = raw
        .pointer(&format!("/output/{plural}"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(variant_name)
        .map(str::to_string)
        .collect::<Vec<_>>();
    names.extend(
        raw.get("outputs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|output| output.get("kind").and_then(Value::as_str) == Some(kind))
            .filter_map(variant_name)
            .map(str::to_string),
    );
    names
}

fn decisive_sources(frames: &[Frame], indices: &[usize]) -> Vec<DecisiveObservation> {
    indices
        .iter()
        .filter_map(|index| {
            frames
                .iter()
                .find(|frame| frame.index == *index)
                .map(|frame| DecisiveObservation {
                    index: *index,
                    source: frame.source.clone(),
                })
        })
        .collect()
}

struct TemporalResult {
    verdict: Verdict,
    summary: String,
    decisive: Vec<usize>,
    trigger: Option<usize>,
    response: Option<usize>,
    pending: Option<String>,
    quantities: Vec<NormalizedQuantity>,
}

fn evaluate_temporal(
    expression: &TemporalExpression,
    frames: &[Frame],
    complete: bool,
) -> Result<TemporalResult> {
    match expression {
        TemporalExpression::Always(predicate) => {
            for frame in frames {
                if !evaluate_predicate(predicate, frame, &mut Vec::new())? {
                    return Ok(temporal_result(
                        Verdict::Violated,
                        "always predicate became false",
                        vec![frame.index],
                    ));
                }
            }
            Ok(temporal_result(
                Verdict::Satisfied,
                "always predicate held for every observed step",
                Vec::new(),
            ))
        }
        TemporalExpression::Eventually(predicate) => {
            for frame in frames {
                if evaluate_predicate(predicate, frame, &mut Vec::new())? {
                    return Ok(temporal_result(
                        Verdict::Satisfied,
                        "eventually predicate was observed",
                        vec![frame.index],
                    ));
                }
            }
            Ok(TemporalResult {
                verdict: if complete {
                    Verdict::Violated
                } else {
                    Verdict::Inconclusive
                },
                summary: if complete {
                    "eventually predicate was absent from the complete trace"
                } else {
                    "eventually predicate remains pending on an open trace"
                }
                .to_string(),
                decisive: Vec::new(),
                trigger: None,
                response: None,
                pending: Some("eventually".to_string()),
                quantities: Vec::new(),
            })
        }
        TemporalExpression::Precedence { before, after } => {
            let mut seen_before = false;
            for frame in frames {
                seen_before |= evaluate_predicate(before, frame, &mut Vec::new())?;
                if evaluate_predicate(after, frame, &mut Vec::new())? && !seen_before {
                    return Ok(temporal_result(
                        Verdict::Violated,
                        "the `after` predicate occurred without a preceding `before` predicate",
                        vec![frame.index],
                    ));
                }
            }
            Ok(temporal_result(
                Verdict::Satisfied,
                "every `after` predicate had a preceding `before` predicate",
                Vec::new(),
            ))
        }
        TemporalExpression::Exclusion { left, right } => {
            for frame in frames {
                if evaluate_predicate(left, frame, &mut Vec::new())?
                    && evaluate_predicate(right, frame, &mut Vec::new())?
                {
                    return Ok(temporal_result(
                        Verdict::Violated,
                        "mutually exclusive predicates occurred together",
                        vec![frame.index],
                    ));
                }
            }
            Ok(temporal_result(
                Verdict::Satisfied,
                "mutually exclusive predicates never occurred together",
                Vec::new(),
            ))
        }
        TemporalExpression::AtMostOnce(predicate) => {
            let mut first = None;
            for frame in frames {
                if evaluate_predicate(predicate, frame, &mut Vec::new())? {
                    if let Some(first) = first {
                        return Ok(temporal_result(
                            Verdict::Violated,
                            "at-most-once predicate occurred more than once",
                            vec![first, frame.index],
                        ));
                    }
                    first = Some(frame.index);
                }
            }
            Ok(temporal_result(
                Verdict::Satisfied,
                "at-most-once predicate occurred no more than once",
                first.into_iter().collect(),
            ))
        }
        TemporalExpression::BoundedResponse {
            trigger,
            response,
            metric,
            bound,
        } => {
            let mut pending = Vec::<(usize, Quantity)>::new();
            let mut quantities = vec![bound.normalized()];
            let mut last_match = None;
            for frame in frames {
                let metric_value = frame
                    .facts
                    .get(metric)
                    .ok_or_else(|| anyhow!("frame has no metric observation `{metric}`"))?;
                let ObservedValue::Quantity(current) = metric_value else {
                    bail!("metric observation `{metric}` is not a quantity");
                };
                if evaluate_predicate(trigger, frame, &mut quantities)? {
                    pending.push((frame.index, current.clone()));
                }
                if evaluate_predicate(response, frame, &mut quantities)? {
                    if let Some((trigger_index, started)) = pending.first() {
                        let trigger_index = *trigger_index;
                        let elapsed = current.difference(started)?;
                        quantities.push(elapsed.normalized());
                        if elapsed.compare(bound)? == Ordering::Greater {
                            return Ok(TemporalResult {
                                verdict: Verdict::Violated,
                                summary: "bounded response arrived after its deadline".to_string(),
                                decisive: vec![trigger_index, frame.index],
                                trigger: Some(trigger_index),
                                response: Some(frame.index),
                                pending: Some(format!(
                                    "expired response for trigger at {trigger_index}"
                                )),
                                quantities,
                            });
                        }
                        pending.remove(0);
                        last_match = Some((trigger_index, frame.index));
                    }
                }
                if let Some((trigger_index, started)) = pending.first() {
                    let elapsed = current.difference(started)?;
                    quantities.push(elapsed.normalized());
                    if elapsed.compare(bound)? == Ordering::Greater {
                        return Ok(TemporalResult {
                            verdict: Verdict::Violated,
                            summary: "bounded response deadline expired".to_string(),
                            decisive: vec![*trigger_index, frame.index],
                            trigger: Some(*trigger_index),
                            response: None,
                            pending: Some(format!("response for trigger at {}", trigger_index)),
                            quantities,
                        });
                    }
                }
            }
            if let Some((trigger_index, _)) = pending.first() {
                Ok(TemporalResult {
                    verdict: if complete {
                        Verdict::Violated
                    } else {
                        Verdict::Inconclusive
                    },
                    summary: if complete {
                        "complete trace ended with an unanswered trigger"
                    } else {
                        "bounded response remains pending on an open trace"
                    }
                    .to_string(),
                    decisive: vec![*trigger_index],
                    trigger: Some(*trigger_index),
                    response: None,
                    pending: Some(format!("response for trigger at {}", trigger_index)),
                    quantities,
                })
            } else {
                let (trigger, response) = last_match
                    .map(|(trigger, response)| (Some(trigger), Some(response)))
                    .unwrap_or((None, None));
                Ok(TemporalResult {
                    verdict: Verdict::Satisfied,
                    summary: "no bounded response obligation remains pending".to_string(),
                    decisive: last_match
                        .map(|(trigger, response)| vec![trigger, response])
                        .unwrap_or_default(),
                    trigger,
                    response,
                    pending: None,
                    quantities,
                })
            }
        }
    }
}

fn temporal_result(
    verdict: Verdict,
    summary: impl Into<String>,
    decisive: Vec<usize>,
) -> TemporalResult {
    TemporalResult {
        verdict,
        summary: summary.into(),
        decisive,
        trigger: None,
        response: None,
        pending: None,
        quantities: Vec::new(),
    }
}

fn evaluate_predicate(
    predicate: &Predicate,
    frame: &Frame,
    quantities: &mut Vec<NormalizedQuantity>,
) -> Result<bool> {
    match predicate {
        Predicate::Constant(value) => Ok(*value),
        Predicate::Occurred(id) => Ok(matches!(
            frame.facts.get(id),
            Some(ObservedValue::Scalar(Value::Bool(true)))
        )),
        Predicate::Equals { left, right } => {
            let left = evaluate_term(left, frame)?;
            let right = evaluate_term(right, frame)?;
            term_equals(&left, &right)
        }
        Predicate::Compare {
            left,
            operator,
            right,
        } => {
            let left = evaluate_term(left, frame)?;
            let right = evaluate_term(right, frame)?;
            let ordering = compare_terms(&left, &right, quantities)?;
            let Some(ordering) = ordering else {
                return Ok(false);
            };
            Ok(match operator {
                CompareOperator::Lt => ordering == Ordering::Less,
                CompareOperator::Lte => ordering != Ordering::Greater,
                CompareOperator::Eq => ordering == Ordering::Equal,
                CompareOperator::Gte => ordering != Ordering::Less,
                CompareOperator::Gt => ordering == Ordering::Greater,
            })
        }
        Predicate::Not(predicate) => Ok(!evaluate_predicate(predicate, frame, quantities)?),
        Predicate::All(predicates) => {
            for predicate in predicates {
                if !evaluate_predicate(predicate, frame, quantities)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        Predicate::Any(predicates) => {
            for predicate in predicates {
                if evaluate_predicate(predicate, frame, quantities)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
    }
}

fn evaluate_term(term: &Term, frame: &Frame) -> Result<EvaluatedTerm> {
    match term {
        Term::Observation(id) => Ok(match frame.facts.get(id) {
            Some(ObservedValue::Scalar(value)) => scalar_term(value),
            Some(ObservedValue::Quantity(value)) => EvaluatedTerm::Quantity(value.clone()),
            Some(ObservedValue::Missing) | None => EvaluatedTerm::Missing,
        }),
        Term::Literal(value) => Ok(scalar_term(value)),
        Term::Quantity(value) => Ok(EvaluatedTerm::Quantity(value.clone())),
        Term::Add(terms) => {
            let mut terms = terms.iter();
            let Some(first) = terms.next() else {
                bail!("add requires at least one term");
            };
            let mut value = evaluate_term(first, frame)?;
            for term in terms {
                value = add_terms(value, evaluate_term(term, frame)?)?;
            }
            Ok(value)
        }
        Term::Subtract(left, right) => {
            subtract_terms(evaluate_term(left, frame)?, evaluate_term(right, frame)?)
        }
    }
}

fn scalar_term(value: &Value) -> EvaluatedTerm {
    if let Some(value) = value.as_i64() {
        EvaluatedTerm::Integer(i128::from(value))
    } else if let Some(value) = value.as_u64() {
        EvaluatedTerm::Integer(i128::from(value))
    } else {
        EvaluatedTerm::Scalar(value.clone())
    }
}

fn add_terms(left: EvaluatedTerm, right: EvaluatedTerm) -> Result<EvaluatedTerm> {
    match (left, right) {
        (EvaluatedTerm::Integer(left), EvaluatedTerm::Integer(right)) => {
            Ok(EvaluatedTerm::Integer(
                left.checked_add(right)
                    .ok_or_else(|| anyhow!("integer addition overflow"))?,
            ))
        }
        (EvaluatedTerm::Quantity(left), EvaluatedTerm::Quantity(right)) => {
            Ok(EvaluatedTerm::Quantity(left.sum(&right)?))
        }
        (EvaluatedTerm::Missing, _) | (_, EvaluatedTerm::Missing) => Ok(EvaluatedTerm::Missing),
        _ => bail!("add requires integer or dimensionally equal quantity terms"),
    }
}

fn subtract_terms(left: EvaluatedTerm, right: EvaluatedTerm) -> Result<EvaluatedTerm> {
    match (left, right) {
        (EvaluatedTerm::Integer(left), EvaluatedTerm::Integer(right)) => {
            Ok(EvaluatedTerm::Integer(
                left.checked_sub(right)
                    .ok_or_else(|| anyhow!("integer subtraction overflow"))?,
            ))
        }
        (EvaluatedTerm::Quantity(left), EvaluatedTerm::Quantity(right)) => {
            Ok(EvaluatedTerm::Quantity(left.difference(&right)?))
        }
        (EvaluatedTerm::Missing, _) | (_, EvaluatedTerm::Missing) => Ok(EvaluatedTerm::Missing),
        _ => bail!("subtract requires integer or dimensionally equal quantity terms"),
    }
}

fn term_equals(left: &EvaluatedTerm, right: &EvaluatedTerm) -> Result<bool> {
    Ok(match (left, right) {
        (EvaluatedTerm::Missing, _) | (_, EvaluatedTerm::Missing) => false,
        (EvaluatedTerm::Integer(left), EvaluatedTerm::Integer(right)) => left == right,
        (EvaluatedTerm::Scalar(left), EvaluatedTerm::Scalar(right)) => left == right,
        (EvaluatedTerm::Quantity(left), EvaluatedTerm::Quantity(right)) => {
            left.compare(right)? == Ordering::Equal
        }
        _ => false,
    })
}

fn compare_terms(
    left: &EvaluatedTerm,
    right: &EvaluatedTerm,
    quantities: &mut Vec<NormalizedQuantity>,
) -> Result<Option<Ordering>> {
    Ok(match (left, right) {
        (EvaluatedTerm::Integer(left), EvaluatedTerm::Integer(right)) => Some(left.cmp(right)),
        (EvaluatedTerm::Quantity(left), EvaluatedTerm::Quantity(right)) => {
            quantities.push(left.normalized());
            quantities.push(right.normalized());
            Some(left.compare(right)?)
        }
        (
            EvaluatedTerm::Scalar(Value::String(left)),
            EvaluatedTerm::Scalar(Value::String(right)),
        ) => Some(left.cmp(right)),
        _ => None,
    })
}

fn parse_quantity(value: &Value) -> Result<Quantity> {
    let original_value = value
        .get("value")
        .and_then(number_text)
        .ok_or_else(|| anyhow!("quantity requires finite decimal `value`"))?;
    let unit = required_string(value, "unit", "quantity")?.to_string();
    let (dimension, factor_numerator, factor_denominator) =
        unit_definition(&unit).ok_or_else(|| anyhow!("unknown RMS v1 unit `{unit}`"))?;
    let (numerator, denominator) = parse_decimal(&original_value)?;
    Ok(Quantity {
        original_value,
        unit,
        dimension,
        numerator: numerator
            .checked_mul(factor_numerator)
            .ok_or_else(|| anyhow!("quantity normalization overflow"))?,
        denominator: denominator
            .checked_mul(factor_denominator)
            .ok_or_else(|| anyhow!("quantity normalization overflow"))?,
    })
}

fn parse_decimal(value: &str) -> Result<(i128, i128)> {
    let value = value.trim();
    if value.is_empty() || value.contains(['e', 'E']) {
        bail!("quantity value `{value}` must be a finite base-10 decimal");
    }
    let negative = value.starts_with('-');
    let unsigned = value.strip_prefix(['-', '+']).unwrap_or(value);
    let mut parts = unsigned.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.chars().all(|character| character.is_ascii_digit())
        || !fraction.chars().all(|character| character.is_ascii_digit())
    {
        bail!("quantity value `{value}` must be a finite base-10 decimal");
    }
    let denominator = 10_i128
        .checked_pow(fraction.len() as u32)
        .ok_or_else(|| anyhow!("quantity decimal precision overflow"))?;
    let digits = format!("{whole}{fraction}");
    let mut numerator = digits
        .parse::<i128>()
        .with_context(|| format!("quantity value `{value}` is out of range"))?;
    if negative {
        numerator = numerator
            .checked_neg()
            .ok_or_else(|| anyhow!("quantity value `{value}` is out of range"))?;
    }
    Ok((numerator, denominator))
}

fn unit_definition(unit: &str) -> Option<(Dimension, i128, i128)> {
    Some(match unit {
        "ns" => (Dimension::Time, 1, 1),
        "us" => (Dimension::Time, 1_000, 1),
        "ms" => (Dimension::Time, 1_000_000, 1),
        "s" => (Dimension::Time, 1_000_000_000, 1),
        "min" => (Dimension::Time, 60_000_000_000, 1),
        "h" => (Dimension::Time, 3_600_000_000_000, 1),
        "bit" => (Dimension::Information, 1, 1),
        "byte" => (Dimension::Information, 8, 1),
        "KiB" => (Dimension::Information, 8_192, 1),
        "MiB" => (Dimension::Information, 8_388_608, 1),
        "GiB" => (Dimension::Information, 8_589_934_592, 1),
        "ratio" => (Dimension::Ratio, 1, 1),
        "percent" => (Dimension::Ratio, 1, 100),
        "transition" => (Dimension::Transition, 1, 1),
        "message" => (Dimension::Message, 1, 1),
        "attempt" => (Dimension::Attempt, 1, 1),
        "item" => (Dimension::Item, 1, 1),
        _ => return None,
    })
}

fn variant_name(value: &Value) -> Option<&str> {
    value
        .get("name")
        .or_else(|| value.get("tag"))
        .or_else(|| value.get("kind"))
        .and_then(Value::as_str)
        .or_else(|| value.as_str())
}

fn variant_matches(observed: &str, expected: &str) -> bool {
    observed == expected
        || observed
            .rsplit(['.', ':', '#'])
            .next()
            .is_some_and(|suffix| suffix == expected)
}

fn number_text(value: &Value) -> Option<String> {
    match value {
        Value::Number(number) => Some(number.to_string()),
        Value::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn required_string<'a>(value: &'a Value, key: &str, label: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("{label} requires non-empty `{key}`"))
}

fn required_string_map<'a>(
    value: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("{label} requires non-empty `{key}`"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn property(expression: Value) -> CompiledProperty {
        compile_property(&json!({
            "id": "delivery",
            "observations": [
                {
                    "id": "submitted",
                    "source": {"kind": "input", "input_kind": "command", "name": "Submit"},
                    "value": "occurrence"
                },
                {
                    "id": "accepted",
                    "source": {"kind": "output", "output_kind": "event", "name": "Accepted"},
                    "value": "occurrence"
                },
                {
                    "id": "elapsed",
                    "source": {"kind": "trace-metric", "name": "elapsed"},
                    "value": {"quantity": "time"}
                }
            ],
            "temporal": {"scope": "composition", "expression": expression}
        }))
        .unwrap()
    }

    fn trace(outputs: Vec<Value>, exhausted: bool) -> Value {
        json!({
            "spec": "rms/probe-system-trace/v0.1",
            "result": if exhausted {"pass"} else {"inconclusive"},
            "exhausted": exhausted,
            "timeline": outputs
        })
    }

    fn step_property(role: &str, expression: Value, observations: Value) -> CompiledProperty {
        compile_property(&json!({
            "id": "contract:calculate#clause",
            "observations": observations,
            "step": {
                "role": role,
                "contract": "calculate",
                "clause": "clause",
                "expression": expression
            }
        }))
        .unwrap()
    }

    fn invocation(binding: &str, input: Value, output: Value) -> Value {
        json!({
            "spec": "rms/invocation-record/v0.1",
            "contract": "calculate",
            "binding": binding,
            "contract_digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "input": input,
            "output": output
        })
    }

    #[test]
    fn quantities_compare_exactly_across_units() {
        let milliseconds = parse_quantity(&json!({"value": 1000, "unit": "ms"})).unwrap();
        let seconds = parse_quantity(&json!({"value": 1, "unit": "s"})).unwrap();
        assert_eq!(milliseconds.compare(&seconds).unwrap(), Ordering::Equal);
        assert!(parse_quantity(&json!({"value": 1, "unit": "message"}))
            .unwrap()
            .compare(&parse_quantity(&json!({"value": 1, "unit": "transition"})).unwrap())
            .is_err());
    }

    #[test]
    fn behavioral_steps_assign_caller_provider_and_evidence_blame() {
        let observations = json!([
            {"id": "amount", "source": {"kind": "input", "pointer": "/amount"}, "value": "integer"},
            {"id": "result", "source": {"kind": "output", "pointer": "/kind"}, "value": {"variant": ["Accepted", "Rejected"]}}
        ]);
        let requirement = step_property(
            "requirement",
            json!({"compare": {"left": {"observation": "amount"}, "operator": "gt", "right": {"literal": 0}}}),
            observations.clone(),
        );
        let caller = evaluate_trace(
            &requirement,
            &invocation("rust", json!({"amount": 0}), json!({"kind": "Rejected"})),
        )
        .unwrap();
        assert_eq!(caller.verdict, Verdict::Violated);
        assert_eq!(caller.explanation.blame.as_deref(), Some("caller"));

        let guarantee = step_property(
            "guarantee",
            json!({"equals": {"left": {"observation": "result"}, "right": {"literal": "Accepted"}}}),
            observations,
        );
        let provider = evaluate_trace(
            &guarantee,
            &invocation("swift", json!({"amount": 1}), json!({"kind": "Rejected"})),
        )
        .unwrap();
        assert_eq!(provider.verdict, Verdict::Violated);
        assert_eq!(provider.explanation.blame.as_deref(), Some("provider"));

        let malformed = json!({
            "spec": "rms/invocation-record/v0.1",
            "contract": "calculate",
            "contract_digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "input": {"amount": 1},
            "output": {"kind": "Accepted"}
        });
        let invalid = evaluate_trace(&guarantee, &malformed).unwrap();
        assert_eq!(invalid.verdict, Verdict::Invalid);
        assert_eq!(
            invalid.explanation.blame.as_deref(),
            Some("binding/evidence")
        );
    }

    #[test]
    fn rejected_outcomes_are_contract_behavior_and_missing_observations_are_gaps() {
        let property = step_property(
            "case",
            json!({"equals": {"left": {"observation": "result"}, "right": {"literal": "Rejected"}}}),
            json!([
                {"id": "result", "source": {"kind": "output", "pointer": "/kind"}, "value": {"variant": ["Accepted", "Rejected"]}}
            ]),
        );
        let rejected = evaluate_trace(
            &property,
            &invocation("javascript", json!({}), json!({"kind": "Rejected"})),
        )
        .unwrap();
        assert_eq!(rejected.verdict, Verdict::Satisfied);

        let gap = evaluate_trace(
            &property,
            &invocation("python", json!({}), json!({"other": "missing"})),
        )
        .unwrap();
        assert_eq!(gap.verdict, Verdict::Inconclusive);
        assert_eq!(gap.explanation.blame.as_deref(), Some("evidence"));
    }

    #[test]
    fn frames_reject_undeclared_state_events_and_effects() {
        let compiled = compile_property(&json!({
            "id": "contract:calculate#accepted",
            "observations": [],
            "step": {
                "role": "case",
                "contract": "calculate",
                "clause": "accepted",
                "expression": {"constant": true},
                "activation": {"constant": true},
                "permits": {"state_changes": [], "events": [], "effects": []}
            }
        }))
        .unwrap();
        let mut record = invocation(
            "rust",
            json!({}),
            json!({
                "kind": "Accepted",
                "events": [{"kind": "Changed"}],
                "effects": []
            }),
        );
        record["state_before"] = json!({"count": 0});
        record["state_after"] = json!({"count": 1});
        let evaluation = evaluate_trace(&compiled, &record).unwrap();
        assert_eq!(evaluation.verdict, Verdict::Violated);
        assert_eq!(evaluation.explanation.blame.as_deref(), Some("provider"));
        assert!(evaluation.explanation.summary.contains("permitted frame"));
    }

    #[test]
    fn frame_regression_reports_provider_blame_at_the_exact_state_path() {
        let compiled = compile_property(&json!({
            "id": "contract:increment#accepted",
            "observations": [],
            "step": {
                "role": "case",
                "contract": "increment",
                "clause": "accepted",
                "expression": {"constant": true},
                "activation": {"constant": true},
                "permits": {
                    "state_changes": ["/data/count"],
                    "events": [],
                    "effects": []
                }
            }
        }))
        .unwrap();
        let mut bad_transition = invocation(
            "python",
            json!({"name": "BadIncrement", "data": {}}),
            json!({"kind": "Accepted", "events": [], "effects": []}),
        );
        bad_transition["contract"] = json!("increment");
        bad_transition["state_before"] =
            json!({"name": "Ready", "data": {"count": 0, "owner": "alice"}});
        bad_transition["state_after"] =
            json!({"name": "Ready", "data": {"count": 1, "owner": "mallory"}});

        let evaluation = evaluate_trace(&compiled, &bad_transition).unwrap();
        assert_eq!(evaluation.verdict, Verdict::Violated);
        assert_eq!(evaluation.explanation.blame.as_deref(), Some("provider"));
        assert!(evaluation.explanation.summary.contains("`/data/owner`"));
    }

    #[test]
    fn all_bindings_receive_the_same_reference_evaluator_verdict() {
        let property = step_property(
            "guarantee",
            json!({"equals": {"left": {"observation": "result"}, "right": {"literal": "Accepted"}}}),
            json!([
                {"id": "result", "source": {"kind": "output", "pointer": "/kind"}, "value": {"variant": ["Accepted", "Rejected"]}}
            ]),
        );
        let verdicts = ["rust", "swift", "javascript", "python", "executable"]
            .into_iter()
            .map(|binding| {
                evaluate_trace(
                    &property,
                    &invocation(binding, json!({}), json!({"kind": "Accepted"})),
                )
                .unwrap()
                .verdict
            })
            .collect::<Vec<_>>();
        assert!(verdicts
            .iter()
            .all(|verdict| verdict == &Verdict::Satisfied));
    }

    #[test]
    fn step_arithmetic_is_exact_and_type_checked() {
        let property = step_property(
            "guarantee",
            json!({"equals": {
                "left": {"subtract": {
                    "left": {"add": [{"observation": "left"}, {"observation": "right"}]},
                    "right": {"literal": 1}
                }},
                "right": {"literal": 4}
            }}),
            json!([
                {"id": "left", "source": {"kind": "input", "pointer": "/left"}, "value": "integer"},
                {"id": "right", "source": {"kind": "input", "pointer": "/right"}, "value": "integer"}
            ]),
        );
        assert_eq!(
            evaluate_trace(
                &property,
                &invocation("rust", json!({"left": 2, "right": 3}), json!({}))
            )
            .unwrap()
            .verdict,
            Verdict::Satisfied
        );
        let invalid = compile_property(&json!({
            "id": "typed",
            "observations": [
                {"id": "text", "source": {"kind": "input", "pointer": "/text"}, "value": "string"}
            ],
            "step": {
                "role": "guarantee",
                "expression": {"compare": {"left": {"observation": "text"}, "operator": "gt", "right": {"literal": 1}}}
            }
        }));
        assert!(invalid
            .unwrap_err()
            .iter()
            .any(|issue| issue.contains("ordered comparison")));
    }

    #[test]
    fn legacy_temporal_shape_is_rejected() {
        let result = compile_property(&json!({
            "id": "legacy",
            "observations": [{"id": "x", "source": {"kind": "input", "name": "X"}, "value": "occurrence"}],
            "temporal": {"pattern": "always", "condition": "x"}
        }));
        assert!(result
            .unwrap_err()
            .iter()
            .any(|issue| issue.contains("removed descriptive temporal fields")));
    }

    #[test]
    fn bounded_response_evaluates_real_trace() {
        let property = property(json!({
            "bounded_response": {
                "trigger": {"occurred": "submitted"},
                "response": {"occurred": "accepted"},
                "within": {"metric": "elapsed", "value": 10, "unit": "ms"}
            }
        }));
        let trace = trace(
            vec![
                json!({"time": 0, "input": {"kind": "command", "name": "Submit"}, "outputs": []}),
                json!({"time": 5, "input": {"kind": "command", "name": "Poll"}, "outputs": [{"kind": "event", "name": "Accepted", "value": {}}]}),
            ],
            true,
        );
        let evaluation = evaluate_trace(&property, &trace).unwrap();
        assert_eq!(evaluation.verdict, Verdict::Satisfied);
        assert_eq!(evaluation.explanation.trigger_observation, Some(0));
        assert_eq!(evaluation.explanation.response_observation, Some(1));
        let envelopes = normalize_trace(&property, &trace).unwrap();
        let streamed = evaluate_observation_envelopes(&property, &envelopes, true).unwrap();
        assert_eq!(streamed.verdict, evaluation.verdict);
    }

    #[test]
    fn bounded_response_rejects_a_late_probe_response() {
        let compiled = property(json!({
            "bounded_response": {
                "trigger": {"occurred": "submitted"},
                "response": {"occurred": "accepted"},
                "within": {"metric": "elapsed", "value": 250, "unit": "ms"}
            }
        }));
        let delayed = trace(
            vec![
                json!({
                    "time": {"value": 0, "unit": "ns"},
                    "input": {"kind": "command", "name": "Submit"},
                    "outputs": []
                }),
                json!({
                    "time": {"value": 0, "unit": "ns"},
                    "action": "delay",
                    "outputs": []
                }),
                json!({
                    "time": {"value": 251000000, "unit": "ns"},
                    "input": {"kind": "effect-result", "name": "Acknowledged"},
                    "outputs": [{"kind": "event", "name": "Accepted", "value": {}}]
                }),
            ],
            true,
        );

        let evaluation = evaluate_trace(&compiled, &delayed).unwrap();

        assert_eq!(evaluation.verdict, Verdict::Violated);
        assert_eq!(
            evaluation.explanation.summary,
            "bounded response arrived after its deadline"
        );
        assert_eq!(evaluation.explanation.trigger_observation, Some(0));
        assert_eq!(evaluation.explanation.response_observation, Some(2));
        assert!(evaluation
            .explanation
            .normalized_quantities
            .iter()
            .any(|quantity| quantity.normalized_numerator == "251000000"));
    }

    #[test]
    fn open_eventually_trace_is_inconclusive() {
        let property = property(json!({"eventually": {"occurred": "accepted"}}));
        let evaluation = evaluate_trace(&property, &trace(Vec::new(), false)).unwrap();
        assert_eq!(evaluation.verdict, Verdict::Inconclusive);
    }

    #[test]
    fn safety_patterns_produce_decisive_violations() {
        let accepted_before_submit = trace(
            vec![json!({
                "time": 0,
                "input": {"kind": "command", "name": "Poll"},
                "outputs": [{"kind": "event", "name": "Accepted", "value": {}}]
            })],
            true,
        );
        let precedence = property(json!({
            "precedence": {
                "before": {"occurred": "submitted"},
                "after": {"occurred": "accepted"}
            }
        }));
        assert_eq!(
            evaluate_trace(&precedence, &accepted_before_submit)
                .unwrap()
                .verdict,
            Verdict::Violated
        );

        let together = trace(
            vec![json!({
                "time": 0,
                "input": {"kind": "command", "name": "Submit"},
                "outputs": [{"kind": "event", "name": "Accepted", "value": {}}]
            })],
            true,
        );
        let exclusion = property(json!({
            "exclusion": {
                "left": {"occurred": "submitted"},
                "right": {"occurred": "accepted"}
            }
        }));
        assert_eq!(
            evaluate_trace(&exclusion, &together).unwrap().verdict,
            Verdict::Violated
        );

        let twice = trace(
            vec![
                json!({"time": 0, "input": {"kind": "command", "name": "Submit"}, "outputs": []}),
                json!({"time": 1, "input": {"kind": "command", "name": "Submit"}, "outputs": []}),
            ],
            true,
        );
        let at_most_once = property(json!({
            "at_most_once": {"occurred": "submitted"}
        }));
        assert_eq!(
            evaluate_trace(&at_most_once, &twice).unwrap().verdict,
            Verdict::Violated
        );

        let always = property(json!({"always": {"occurred": "submitted"}}));
        assert_eq!(
            evaluate_trace(&always, &accepted_before_submit)
                .unwrap()
                .verdict,
            Verdict::Violated
        );
    }

    #[test]
    fn environment_assumptions_cannot_turn_absence_into_success() {
        let compiled = compile_property(&json!({
            "id": "assumed-delivery",
            "observations": [
                {
                    "id": "accepted",
                    "source": {"kind": "output", "output_kind": "event", "name": "Accepted"},
                    "value": "occurrence"
                }
            ],
            "assumptions": [{
                "id": "delivery-remains-possible",
                "kind": "environment",
                "expression": {"eventually": {"occurred": "accepted"}}
            }],
            "temporal": {
                "scope": "composition",
                "expression": {"always": {"not": {"occurred": "accepted"}}}
            }
        }))
        .unwrap();
        let evaluation = evaluate_trace(&compiled, &trace(Vec::new(), true)).unwrap();
        assert_eq!(evaluation.verdict, Verdict::Inconclusive);
        assert!(evaluation.explanation.summary.contains("assumption"));
    }

    #[test]
    fn dimensional_errors_are_compile_time_errors() {
        let definition = json!({
            "id": "bad-bound",
            "observations": [
                {
                    "id": "elapsed",
                    "source": {"kind": "trace-metric", "name": "elapsed"},
                    "value": {"quantity": "time"}
                },
                {
                    "id": "submitted",
                    "source": {"kind": "input", "name": "Submit"},
                    "value": "occurrence"
                }
            ],
            "temporal": {
                "scope": "machine",
                "expression": {
                    "bounded_response": {
                        "trigger": {"occurred": "submitted"},
                        "response": {"occurred": "submitted"},
                        "within": {"metric": "elapsed", "value": 1, "unit": "message"}
                    }
                }
            }
        });
        assert!(compile_property(&definition)
            .unwrap_err()
            .iter()
            .any(|issue| issue.contains("dimension")));
        assert!(parse_quantity(&json!({"value": 1, "unit": "fortnight"})).is_err());
        assert!(parse_quantity(&json!({"value": "1e3", "unit": "ms"})).is_err());
    }

    #[test]
    fn quantity_observation_errors_preserve_the_actionable_inner_cause() {
        let definition = json!({
            "id": "bad-quantity-shape",
            "observations": [
                {
                    "id": "transition_count",
                    "source": {"kind": "trace-metric", "name": "transition-count"},
                    "value": {"quantity": {"dimension": "transition", "unit": "transition"}}
                }
            ],
            "temporal": {
                "scope": "machine",
                "expression": {
                    "always": {"compare": {
                        "observation": "transition_count",
                        "operator": "gte",
                        "value": 0,
                        "unit": "transition"
                    }}
                }
            }
        });
        let issues = compile_property(&definition).unwrap_err();
        assert!(issues.iter().any(|issue| {
            issue.contains("observation `transition_count`")
                && issue.contains("quantity observation must declare `value.quantity`")
        }));
    }

    #[test]
    fn transition_count_is_a_dimensionally_typed_trace_metric() {
        let definition = json!({
            "id": "one-transition-response",
            "observations": [
                {
                    "id": "accepted",
                    "source": {"kind": "output", "output_kind": "reply", "name": "Accepted"},
                    "value": "occurrence"
                },
                {
                    "id": "updated",
                    "source": {"kind": "output", "output_kind": "event", "name": "Updated"},
                    "value": "occurrence"
                },
                {
                    "id": "transition_count",
                    "source": {"kind": "trace-metric", "name": "transition-count"},
                    "value": {"quantity": "transition"}
                }
            ],
            "temporal": {
                "scope": "machine",
                "expression": {
                    "bounded_response": {
                        "trigger": {"occurred": "accepted"},
                        "response": {"occurred": "updated"},
                        "within": {
                            "metric": "transition_count",
                            "value": 1,
                            "unit": "transition"
                        }
                    }
                }
            }
        });
        assert!(compile_property(&definition).is_ok());
    }

    #[test]
    fn raw_qualified_variants_and_normalized_probe_names_project_identically() {
        let compiled = property(json!({
            "bounded_response": {
                "trigger": {"occurred": "submitted"},
                "response": {"occurred": "accepted"},
                "within": {"metric": "elapsed", "value": 10, "unit": "ms"}
            }
        }));
        let raw = json!({
            "spec": "rms/trace-bundle/v0.1",
            "complete": true,
            "machine": "DeliveryMachine",
            "records": [{
                "input": {"kind": "command", "tag": "DeliveryCommand.Submit"},
                "state_before": {"tag": "DeliveryState.Ready"},
                "state_after": {"tag": "DeliveryState.Ready"},
                "output": {
                    "next_state": {"tag": "DeliveryState.Ready"},
                    "events": [{"tag": "DeliveryEvent.Accepted"}],
                    "commands": [],
                    "effects": [],
                    "reply": null,
                    "rejection": null
                },
                "time": 5
            }]
        });
        let evaluation = evaluate_trace(&compiled, &raw).unwrap();
        let envelopes = normalize_trace(&compiled, &raw).unwrap();
        let streamed = evaluate_observation_envelopes(&compiled, &envelopes, true).unwrap();

        assert_eq!(evaluation.verdict, Verdict::Satisfied);
        assert_eq!(evaluation.explanation.trigger_observation, Some(0));
        assert_eq!(evaluation.explanation.response_observation, Some(0));
        assert_eq!(streamed.verdict, evaluation.verdict);
        assert_eq!(
            envelopes[0].facts.get("submitted"),
            Some(&Value::Bool(true))
        );
        assert_eq!(envelopes[0].facts.get("accepted"), Some(&Value::Bool(true)));
    }
}
