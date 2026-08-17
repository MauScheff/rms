use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const PROOF_CERTIFICATE_SPEC: &str = "rms/proof-certificate/v0.1";

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct ProofCertificate {
    pub(crate) spec: String,
    pub(crate) subject: ProofSubject,
    pub(crate) digests: ProofDigests,
    pub(crate) strategy: ProofStrategy,
    pub(crate) assumptions: BTreeMap<String, String>,
    pub(crate) result: ProofResult,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct ProofSubject {
    pub(crate) module: String,
    pub(crate) contract: Option<String>,
    pub(crate) property: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct ProofDigests {
    pub(crate) contract: Option<String>,
    pub(crate) implementation: String,
    pub(crate) source: String,
    pub(crate) tool: String,
    pub(crate) evidence: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum ProofStrategy {
    DeterministicExhaustive {
        exhausted: bool,
    },
    Solver {
        solver: String,
        solver_digest: String,
        result: String,
    },
    Generated {
        seed: u64,
        cases: usize,
    },
    Fuzz {
        seed: u64,
        budget_seconds: u64,
    },
    SampledTrace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ProofResult {
    Satisfied,
    Violated,
    Inconclusive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProofRequirement {
    pub(crate) subject: ProofSubject,
    pub(crate) digests: ProofDigests,
    pub(crate) assumptions: BTreeMap<String, String>,
    pub(crate) universal: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ReuseDecision {
    pub(crate) reusable: bool,
    pub(crate) reasons: Vec<String>,
}

pub(crate) fn decide_reuse(
    certificate: &ProofCertificate,
    requirement: &ProofRequirement,
) -> ReuseDecision {
    let mut reasons = Vec::new();
    if certificate.spec != PROOF_CERTIFICATE_SPEC {
        reasons.push("certificate spec is not supported".to_string());
    }
    if certificate.subject != requirement.subject {
        reasons.push("proof subject does not match".to_string());
    }
    compare_digest(
        "contract",
        certificate.digests.contract.as_deref(),
        requirement.digests.contract.as_deref(),
        &mut reasons,
    );
    compare_digest(
        "implementation",
        Some(&certificate.digests.implementation),
        Some(&requirement.digests.implementation),
        &mut reasons,
    );
    compare_digest(
        "source",
        Some(&certificate.digests.source),
        Some(&requirement.digests.source),
        &mut reasons,
    );
    compare_digest(
        "tool",
        Some(&certificate.digests.tool),
        Some(&requirement.digests.tool),
        &mut reasons,
    );
    compare_digest(
        "evidence",
        Some(&certificate.digests.evidence),
        Some(&requirement.digests.evidence),
        &mut reasons,
    );
    if certificate.assumptions != requirement.assumptions {
        reasons.push("proof assumptions do not match".to_string());
    }
    if certificate.result != ProofResult::Satisfied {
        reasons.push("certificate result is not satisfied".to_string());
    }
    if requirement.universal && !strategy_discharges_universal(&certificate.strategy) {
        reasons.push(
            "bounded or sampled evidence cannot discharge a universal obligation".to_string(),
        );
    }
    ReuseDecision {
        reusable: reasons.is_empty(),
        reasons,
    }
}

pub(crate) fn strategy_discharges_universal(strategy: &ProofStrategy) -> bool {
    match strategy {
        ProofStrategy::DeterministicExhaustive { exhausted } => *exhausted,
        ProofStrategy::Solver { result, .. } => matches!(result.as_str(), "unsat" | "proved"),
        ProofStrategy::Generated { .. }
        | ProofStrategy::Fuzz { .. }
        | ProofStrategy::SampledTrace => false,
    }
}

fn compare_digest(
    label: &str,
    actual: Option<&str>,
    expected: Option<&str>,
    reasons: &mut Vec<String>,
) {
    if actual != expected || actual.is_some_and(|value| !value.starts_with("sha256:")) {
        reasons.push(format!("{label} digest does not match exactly"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn certificate(strategy: ProofStrategy) -> ProofCertificate {
        ProofCertificate {
            spec: PROOF_CERTIFICATE_SPEC.to_string(),
            subject: ProofSubject {
                module: "rules".to_string(),
                contract: Some("apply-move".to_string()),
                property: "preserves-laws".to_string(),
            },
            digests: ProofDigests {
                contract: Some("sha256:contract".to_string()),
                implementation: "sha256:implementation".to_string(),
                source: "sha256:source".to_string(),
                tool: "sha256:tool".to_string(),
                evidence: "sha256:evidence".to_string(),
            },
            strategy,
            assumptions: BTreeMap::from([("valid-input".to_string(), "true".to_string())]),
            result: ProofResult::Satisfied,
        }
    }

    fn requirement(certificate: &ProofCertificate) -> ProofRequirement {
        ProofRequirement {
            subject: certificate.subject.clone(),
            digests: certificate.digests.clone(),
            assumptions: certificate.assumptions.clone(),
            universal: true,
        }
    }

    #[test]
    fn exact_exhaustive_certificate_is_reusable() {
        let certificate = certificate(ProofStrategy::DeterministicExhaustive { exhausted: true });
        assert!(decide_reuse(&certificate, &requirement(&certificate)).reusable);
    }

    #[test]
    fn any_digest_drift_rejects_reuse() {
        let certificate = certificate(ProofStrategy::DeterministicExhaustive { exhausted: true });
        let mut requirement = requirement(&certificate);
        requirement.digests.source = "sha256:changed".to_string();
        let decision = decide_reuse(&certificate, &requirement);
        assert!(!decision.reusable);
        assert!(decision
            .reasons
            .iter()
            .any(|reason| reason.contains("source")));
    }

    #[test]
    fn bounded_generation_never_discharges_universal_claim() {
        let certificate = certificate(ProofStrategy::Generated { seed: 7, cases: 64 });
        let decision = decide_reuse(&certificate, &requirement(&certificate));
        assert!(!decision.reusable);
        assert!(decision
            .reasons
            .iter()
            .any(|reason| reason.contains("universal")));
    }
}
