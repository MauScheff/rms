use crate::schema_generator;
use anyhow::{bail, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const COMPOSITION_MODEL_SPEC: &str = "rms/composition-model/v0.1";

#[derive(Clone, Debug)]
pub(crate) struct CompositionInput {
    pub(crate) source_revision: String,
    pub(crate) tool_digest: String,
    pub(crate) seed: u64,
    pub(crate) cases_per_input: usize,
    pub(crate) participants: Vec<ParticipantInput>,
    pub(crate) wiring: Vec<Wiring>,
    pub(crate) protocol_routes: Vec<ProtocolRoute>,
    pub(crate) effects: BTreeSet<String>,
    pub(crate) authorities: BTreeSet<String>,
    pub(crate) obligations: Vec<CompositionObligation>,
    pub(crate) reusable_proofs: Vec<ReusableProof>,
    pub(crate) algebraic_laws: Vec<AlgebraicLaw>,
}

#[derive(Clone, Debug)]
pub(crate) struct ParticipantInput {
    pub(crate) id: String,
    pub(crate) implementation: String,
    pub(crate) implementation_digest: String,
    pub(crate) initial_state: Value,
    pub(crate) inputs: Vec<GeneratedInput>,
}

#[derive(Clone, Debug)]
pub(crate) struct GeneratedInput {
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) schema: Value,
    pub(crate) public: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct Wiring {
    pub(crate) consumer: String,
    pub(crate) binding: String,
    pub(crate) provider: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ProtocolRoute {
    pub(crate) protocol: String,
    pub(crate) message: String,
    pub(crate) sender: String,
    pub(crate) receiver: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct CompositionObligation {
    pub(crate) id: String,
    pub(crate) status: String,
    pub(crate) detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ReusableProof {
    pub(crate) subject: String,
    pub(crate) certificate_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct AlgebraicLaw {
    pub(crate) id: String,
    pub(crate) kind: AlgebraicLawKind,
    pub(crate) subject: String,
    pub(crate) certificate_digest: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AlgebraicLawKind {
    Idempotent,
    Commutative,
    Associative,
    Monotonic,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct CompositionModel {
    pub(crate) spec: &'static str,
    pub(crate) source_revision: String,
    pub(crate) tool_digest: String,
    pub(crate) seed: u64,
    pub(crate) cases_per_input: usize,
    pub(crate) state: SymbolicState,
    pub(crate) participants: Vec<Participant>,
    pub(crate) wiring: Vec<Wiring>,
    pub(crate) protocol_routes: Vec<ProtocolRoute>,
    pub(crate) effects: Vec<String>,
    pub(crate) authorities: Vec<String>,
    pub(crate) obligations: Vec<CompositionObligation>,
    pub(crate) reusable_proofs: Vec<ReusableProof>,
    pub(crate) reductions: Vec<SearchReduction>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct SymbolicState {
    pub(crate) representation: &'static str,
    pub(crate) components: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct Participant {
    pub(crate) id: String,
    pub(crate) implementation: String,
    pub(crate) implementation_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct SearchReduction {
    pub(crate) law: String,
    pub(crate) kind: AlgebraicLawKind,
    pub(crate) action: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PlannedComposition {
    pub(crate) model: CompositionModel,
    pub(crate) assembly: Value,
}

pub(crate) fn plan(input: CompositionInput) -> Result<PlannedComposition> {
    if input.participants.is_empty() {
        bail!("composition requires at least one participant");
    }
    if input.cases_per_input == 0 {
        bail!("cases-per-input must be greater than zero");
    }
    if let Some(obligation) = input
        .obligations
        .iter()
        .find(|obligation| !matches!(obligation.status.as_str(), "satisfied" | "not-applicable"))
    {
        bail!(
            "composition obligation `{}` is `{}`: {}",
            obligation.id,
            obligation.status,
            obligation.detail
        );
    }
    validate_unique_participants(&input.participants)?;
    validate_wiring(&input.participants, &input.wiring)?;
    validate_protocol_routes(&input.participants, &input.protocol_routes)?;

    let mut stimuli = Vec::new();
    for (participant_index, participant) in input.participants.iter().enumerate() {
        for (input_index, declared) in participant.inputs.iter().enumerate() {
            if !declared.public {
                continue;
            }
            let derived_seed =
                input.seed ^ ((participant_index as u64 + 1) << 32) ^ (input_index as u64 + 1);
            let generated = schema_generator::generate_cases(
                &declared.schema,
                derived_seed,
                input.cases_per_input,
            )?;
            for (case_index, data) in generated.cases.into_iter().enumerate() {
                stimuli.push(json!({
                    "id": format!("{}-{}-{}", participant.id, declared.name, case_index),
                    "target": participant.id,
                    "input": {
                        "kind": declared.kind,
                        "name": declared.name,
                        "data": data
                    }
                }));
            }
        }
    }
    let instances = input
        .participants
        .iter()
        .map(|participant| {
            json!({
                "id": participant.id,
                "implementation": participant.implementation,
                "start": participant.initial_state
            })
        })
        .collect::<Vec<_>>();
    let routing = input
        .wiring
        .iter()
        .map(|wire| {
            json!({
                "consumer": wire.consumer,
                "binding": wire.binding,
                "provider": wire.provider
            })
        })
        .collect::<Vec<_>>();
    let assembly = json!({
        "spec": "rms/probe-assembly/v0.3",
        "instances": instances,
        "stimuli": stimuli,
        "routing": routing,
        "substitutes": [],
        "checks": [],
        "exploration": {
            "max_steps": 30,
            "max_schedules": 100,
            "max_states": 10000
        },
        "faults": []
    });

    let mut participants = input
        .participants
        .iter()
        .map(|participant| Participant {
            id: participant.id.clone(),
            implementation: participant.implementation.clone(),
            implementation_digest: participant.implementation_digest.clone(),
        })
        .collect::<Vec<_>>();
    participants.sort_by(|left, right| left.id.cmp(&right.id));
    let reductions = input
        .algebraic_laws
        .iter()
        .map(|law| SearchReduction {
            law: law.id.clone(),
            kind: law.kind,
            action: match law.kind {
                AlgebraicLawKind::Idempotent => "collapse-duplicate-delivery",
                AlgebraicLawKind::Commutative => "partial-order-reduction",
                AlgebraicLawKind::Associative => "regroup-identical-contracts",
                AlgebraicLawKind::Monotonic => "prune-dominated-state",
            },
        })
        .collect();
    let model = CompositionModel {
        spec: COMPOSITION_MODEL_SPEC,
        source_revision: input.source_revision,
        tool_digest: input.tool_digest,
        seed: input.seed,
        cases_per_input: input.cases_per_input,
        state: SymbolicState {
            representation: "ordered-participant-vector",
            components: participants
                .iter()
                .map(|participant| participant.id.clone())
                .collect(),
        },
        participants,
        wiring: input.wiring,
        protocol_routes: input.protocol_routes,
        effects: input.effects.into_iter().collect(),
        authorities: input.authorities.into_iter().collect(),
        obligations: input.obligations,
        reusable_proofs: input.reusable_proofs,
        reductions,
    };
    Ok(PlannedComposition { model, assembly })
}

fn validate_unique_participants(participants: &[ParticipantInput]) -> Result<()> {
    let mut ids = BTreeSet::new();
    for participant in participants {
        if !ids.insert(&participant.id) {
            bail!("duplicate composition participant `{}`", participant.id);
        }
    }
    Ok(())
}

fn validate_wiring(participants: &[ParticipantInput], wiring: &[Wiring]) -> Result<()> {
    let ids = participants
        .iter()
        .map(|item| item.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut bindings = BTreeMap::<(&str, &str), usize>::new();
    for wire in wiring {
        if !ids.contains(wire.consumer.as_str()) || !ids.contains(wire.provider.as_str()) {
            bail!("composition wiring references an unknown participant");
        }
        *bindings.entry((&wire.consumer, &wire.binding)).or_default() += 1;
    }
    if let Some(((consumer, binding), count)) = bindings.into_iter().find(|(_, count)| *count != 1)
    {
        bail!("composition binding `{consumer}:{binding}` has {count} providers; expected one");
    }
    Ok(())
}

fn validate_protocol_routes(
    participants: &[ParticipantInput],
    routes: &[ProtocolRoute],
) -> Result<()> {
    let ids = participants
        .iter()
        .map(|item| item.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut messages = BTreeSet::new();
    for route in routes {
        if !ids.contains(route.sender.as_str()) || !ids.contains(route.receiver.as_str()) {
            bail!("protocol route references an unknown participant");
        }
        if !messages.insert((&route.protocol, &route.message)) {
            bail!(
                "protocol message `{}:{}` has more than one route",
                route.protocol,
                route.message
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn participant(id: &str) -> ParticipantInput {
        ParticipantInput {
            id: id.to_string(),
            implementation: format!("{id}/implementation.yaml"),
            implementation_digest: format!("sha256:{id}"),
            initial_state: json!("initial"),
            inputs: vec![
                GeneratedInput {
                    kind: "command".to_string(),
                    name: "Apply".to_string(),
                    schema: json!({
                        "type": "object",
                        "properties": {"value": {"type": "integer", "minimum": 0, "maximum": 2}},
                        "required": ["value"],
                        "additionalProperties": false
                    }),
                    public: true,
                },
                GeneratedInput {
                    kind: "effect-result".to_string(),
                    name: "AppliedInternally".to_string(),
                    schema: json!({"type": "object"}),
                    public: false,
                },
            ],
        }
    }

    fn input() -> CompositionInput {
        CompositionInput {
            source_revision: "revision".to_string(),
            tool_digest: "sha256:tool".to_string(),
            seed: 7,
            cases_per_input: 8,
            participants: vec![participant("consumer"), participant("provider")],
            wiring: vec![Wiring {
                consumer: "consumer".to_string(),
                binding: "storage".to_string(),
                provider: "provider".to_string(),
            }],
            protocol_routes: Vec::new(),
            effects: BTreeSet::from(["Persist".to_string()]),
            authorities: BTreeSet::from(["filesystem".to_string()]),
            obligations: vec![CompositionObligation {
                id: "provider".to_string(),
                status: "satisfied".to_string(),
                detail: "one provider".to_string(),
            }],
            reusable_proofs: Vec::new(),
            algebraic_laws: vec![AlgebraicLaw {
                id: "apply-idempotent".to_string(),
                kind: AlgebraicLawKind::Idempotent,
                subject: "Apply".to_string(),
                certificate_digest: "sha256:proof".to_string(),
            }],
        }
    }

    #[test]
    fn composed_model_and_assembly_are_stable() {
        let left = plan(input()).unwrap();
        let right = plan(input()).unwrap();
        assert_eq!(left, right);
        assert_eq!(left.model.state.components, vec!["consumer", "provider"]);
        assert_eq!(
            left.model.reductions[0].action,
            "collapse-duplicate-delivery"
        );
        assert!(!left.assembly["stimuli"].as_array().unwrap().is_empty());
        assert!(left.assembly["stimuli"]
            .as_array()
            .unwrap()
            .iter()
            .all(|stimulus| stimulus["input"]["name"] == "Apply"));
    }

    #[test]
    fn unresolved_obligation_prevents_generation() {
        let mut candidate = input();
        candidate.obligations[0].status = "unresolved".to_string();
        assert!(plan(candidate)
            .unwrap_err()
            .to_string()
            .contains("unresolved"));
    }

    #[test]
    fn ambiguous_wiring_prevents_generation() {
        let mut candidate = input();
        candidate.wiring.push(candidate.wiring[0].clone());
        assert!(plan(candidate)
            .unwrap_err()
            .to_string()
            .contains("2 providers"));
    }
}
