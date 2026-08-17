use crate::effect_analysis::{EffectAnalysis, FunctionVerdict};
use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use serde_yaml::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const IMPLEMENTATION_V1_SPEC: &str = "rms/implementation/v0.1";
pub(crate) const IMPLEMENTATION_V2_SPEC: &str = "rms/implementation/v0.2";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct MigrationDiagnostic {
    pub(crate) code: String,
    pub(crate) message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MigrationPlan {
    pub(crate) candidate: Value,
    pub(crate) changed: bool,
    pub(crate) diagnostics: Vec<MigrationDiagnostic>,
}

pub(crate) fn plan(
    candidate: &Value,
    analysis: &EffectAnalysis,
    target: &str,
) -> Result<MigrationPlan> {
    if target != "v0.2" && target != IMPLEMENTATION_V2_SPEC {
        bail!("unsupported implementation migration target `{target}`; expected `v0.2`");
    }
    let spec = string_at(candidate, &["spec"]).unwrap_or_default();
    if spec == IMPLEMENTATION_V2_SPEC {
        return Ok(MigrationPlan {
            candidate: candidate.clone(),
            changed: false,
            diagnostics: vec![diagnostic(
                "migration.idempotent",
                "implementation already uses rms/implementation/v0.2",
            )],
        });
    }
    if spec != IMPLEMENTATION_V1_SPEC {
        bail!("input is not an rms/implementation/v0.1 binding");
    }

    let authority_facades = authority_facades(candidate)?;
    let reports = analysis
        .functions
        .iter()
        .map(|function| (function.id.as_str(), function))
        .collect::<BTreeMap<_, _>>();
    let mut output = candidate.clone();
    set_string(&mut output, &["spec"], IMPLEMENTATION_V2_SPEC)?;
    let functions = sequence_mut(&mut output, &["semantic_functions"])
        .ok_or_else(|| anyhow!("implementation has no semantic_functions sequence"))?;
    let mut diagnostics = Vec::new();

    for function in functions {
        let id = string_at(function, &["id"])
            .ok_or_else(|| anyhow!("semantic function has no id"))?
            .to_string();
        let kind = string_at(function, &["kind"])
            .ok_or_else(|| anyhow!("semantic function `{id}` has no kind"))?
            .to_string();
        let original_purity = string_at(function, &["purity"])
            .ok_or_else(|| anyhow!("semantic function `{id}` has no purity"))?
            .to_string();
        let report = reports
            .get(id.as_str())
            .ok_or_else(|| anyhow!("effect analysis did not report semantic function `{id}`"))?;
        if report.verdict == FunctionVerdict::Unsupported {
            bail!("semantic function `{id}` has no supported effect analysis");
        }
        if report
            .reasons
            .iter()
            .any(|reason| reason.contains("resolved to"))
        {
            bail!(
                "semantic function `{id}` cannot be migrated: {}",
                report.reasons.join("; ")
            );
        }
        if !report.unresolved_calls.is_empty() {
            bail!(
                "semantic function `{id}` has unresolved calls [{}]; migration is ambiguous",
                report.unresolved_calls.join(", ")
            );
        }
        if original_purity == "pure" && !report.transitive_authorities.is_empty() {
            bail!(
                "semantic function `{id}` is declared pure but reaches authorities [{}]",
                report.transitive_authorities.join(", ")
            );
        }
        for authority in &report.transitive_authorities {
            if !authority_facades.contains(authority) {
                bail!(
                    "semantic function `{id}` reaches authority `{authority}` without one exact path#symbol safe facade"
                );
            }
        }

        let trust = infer_trust(&kind)?;
        let purity = if original_purity == "boundary" {
            "effectful"
        } else {
            original_purity.as_str()
        };
        if !matches!(purity, "pure" | "effectful") {
            bail!("semantic function `{id}` has unsupported purity `{purity}`");
        }
        let mapping = function
            .as_mapping_mut()
            .ok_or_else(|| anyhow!("semantic function `{id}` is not a mapping"))?;
        mapping.insert(key("purity"), Value::String(purity.to_string()));
        mapping.insert(key("trust"), Value::String(trust.to_string()));
        mapping.insert(
            key("authorities"),
            Value::Sequence(
                report
                    .transitive_authorities
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
        diagnostics.push(diagnostic(
            "migration.function",
            format!(
                "migrated `{id}` as {purity}/{trust} with {} authority binding(s)",
                report.transitive_authorities.len()
            ),
        ));
    }

    Ok(MigrationPlan {
        candidate: output,
        changed: true,
        diagnostics,
    })
}

fn infer_trust(kind: &str) -> Result<&'static str> {
    match kind {
        "parser" | "adapter" | "interpreter" | "effect-executor" => Ok("boundary"),
        "constructor" | "decision" | "transition" | "projector" | "transformation" => {
            Ok("internal")
        }
        _ => bail!("semantic function kind `{kind}` has ambiguous trust"),
    }
}

fn authority_facades(candidate: &Value) -> Result<BTreeSet<String>> {
    let mut counts = BTreeMap::<String, usize>::new();
    let bindings = sequence(candidate, &["architecture", "authority_bindings"])
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for binding in bindings {
        let authority = string_at(binding, &["authority"])
            .ok_or_else(|| anyhow!("authority binding has no authority"))?;
        let facade = string_at(binding, &["safe_facade"])
            .ok_or_else(|| anyhow!("authority binding `{authority}` has no safe_facade"))?;
        if facade.matches('#').count() != 1 || facade.starts_with('#') || facade.ends_with('#') {
            bail!("authority binding `{authority}` does not use an exact path#symbol facade");
        }
        *counts.entry(authority.to_string()).or_default() += 1;
    }
    if let Some((authority, count)) = counts.iter().find(|(_, count)| **count != 1) {
        bail!("authority `{authority}` resolves through {count} facades; expected exactly one");
    }
    Ok(counts.into_keys().collect())
}

fn key(value: &str) -> Value {
    Value::String(value.to_string())
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for segment in path {
        current = current.as_mapping()?.get(key(segment))?;
    }
    current.as_str()
}

fn sequence<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Vec<Value>> {
    let mut current = value;
    for segment in path {
        current = current.as_mapping()?.get(key(segment))?;
    }
    current.as_sequence()
}

fn sequence_mut<'a>(value: &'a mut Value, path: &[&str]) -> Option<&'a mut Vec<Value>> {
    let mut current = value;
    for segment in path {
        current = current.as_mapping_mut()?.get_mut(key(segment))?;
    }
    current.as_sequence_mut()
}

fn set_string(value: &mut Value, path: &[&str], contents: &str) -> Result<()> {
    let (last, parents) = path
        .split_last()
        .ok_or_else(|| anyhow!("cannot set an empty path"))?;
    let mut current = value;
    for segment in parents {
        current = current
            .as_mapping_mut()
            .and_then(|mapping| mapping.get_mut(key(segment)))
            .ok_or_else(|| anyhow!("missing migration path `{}`", path.join(".")))?;
    }
    current
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("migration path `{}` is not a mapping", path.join(".")))?
        .insert(key(last), Value::String(contents.to_string()));
    Ok(())
}

fn diagnostic(code: impl Into<String>, message: impl Into<String>) -> MigrationDiagnostic {
    MigrationDiagnostic {
        code: code.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect_analysis::{AnalysisResult, FunctionAnalysis};

    fn analysis(authorities: &[&str], unresolved: &[&str]) -> EffectAnalysis {
        EffectAnalysis {
            spec: crate::effect_analysis::EFFECT_ANALYSIS_SPEC,
            binding: "rust".to_string(),
            source_digest: "source".to_string(),
            tool_digest: "tool".to_string(),
            result: AnalysisResult::Fail,
            functions: vec![FunctionAnalysis {
                id: "read".to_string(),
                symbol: "src/lib.rs#read".to_string(),
                declared_purity: "effectful".to_string(),
                declared_authorities: Vec::new(),
                direct_calls: Vec::new(),
                resolved_callees: Vec::new(),
                direct_authorities: authorities
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect(),
                transitive_authorities: authorities
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect(),
                unresolved_calls: unresolved
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect(),
                verdict: FunctionVerdict::Fail,
                reasons: vec!["legacy authority row differs".to_string()],
            }],
        }
    }

    fn legacy() -> Value {
        serde_yaml::from_str(
            r#"spec: rms/implementation/v0.1
architecture:
  authority_bindings:
    - authority: filesystem
      roles: [adapter]
      safe_facade: src/io.rs#read
      evidence: [verification/boundaries/io.md]
semantic_functions:
  - id: read
    symbol: src/lib.rs#read
    kind: adapter
    purity: effectful
"#,
        )
        .unwrap()
    }

    #[test]
    fn migration_is_deterministic_and_idempotent() {
        let migrated = plan(&legacy(), &analysis(&["filesystem"], &[]), "v0.2").unwrap();
        assert!(migrated.changed);
        assert_eq!(
            string_at(&migrated.candidate, &["spec"]),
            Some(IMPLEMENTATION_V2_SPEC)
        );
        let repeated = plan(&migrated.candidate, &analysis(&[], &[]), "v0.2").unwrap();
        assert!(!repeated.changed);
        assert_eq!(repeated.candidate, migrated.candidate);
    }

    #[test]
    fn unresolved_calls_refuse_without_candidate() {
        let error = plan(&legacy(), &analysis(&[], &["callback"]), "v0.2").unwrap_err();
        assert!(error.to_string().contains("ambiguous"));
    }

    #[test]
    fn missing_safe_facade_refuses() {
        let mut input = legacy();
        input.as_mapping_mut().unwrap()[&key("architecture")]
            .as_mapping_mut()
            .unwrap()[&key("authority_bindings")] = Value::Sequence(Vec::new());
        let error = plan(&input, &analysis(&["filesystem"], &[]), "v0.2").unwrap_err();
        assert!(error.to_string().contains("safe facade"));
    }
}
