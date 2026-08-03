use super::{property, sha256_bytes};
use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use serde_json::{json, Map, Value};
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::process::{Command, Stdio};

pub(super) const CONTRACT_SPEC: &str = "rms/contract/v0.3";
pub(super) const CONTRACT_SPEC_V2: &str = "rms/contract/v0.2";
pub(super) const LEGACY_CONTRACT_SPEC: &str = "rms/contract/v0.1";
pub(super) const INVOCATION_SPEC: &str = "rms/invocation-record/v0.1";
pub(super) const COMPATIBILITY_SPEC: &str = "rms/compatibility-analysis/v0.1";

#[derive(Clone, Debug, Serialize)]
pub(super) struct ContractIssue {
    pub(super) check: String,
    pub(super) message: String,
    pub(super) blocking: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct SolverEvidence {
    pub(super) adapter: String,
    pub(super) executable: String,
    pub(super) version: Option<String>,
    pub(super) input_digest: String,
    pub(super) timeout_ms: u64,
    pub(super) result: String,
    pub(super) output: String,
    pub(super) model: Option<String>,
    pub(super) unsat_core: Option<String>,
    pub(super) model_revalidated: Option<bool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ExpectedSolverResult {
    Sat,
    Unsat,
    Informational,
}

#[derive(Clone, Debug)]
pub(super) struct SmtObligation {
    pub(super) id: String,
    pub(super) expectation: ExpectedSolverResult,
    pub(super) script: String,
    expression: Value,
    observations: Vec<Value>,
    unsupported_reason: Option<String>,
}

pub(super) fn is_contract_spec(spec: Option<&str>) -> bool {
    matches!(spec, Some(CONTRACT_SPEC | CONTRACT_SPEC_V2))
}

pub(super) fn validate(value: &Value, strict: bool) -> Vec<ContractIssue> {
    let mut issues = Vec::new();
    if !is_contract_spec(value.get("spec").and_then(Value::as_str)) {
        return issues;
    }
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("<unnamed-contract>");
    let kind = value.get("kind").and_then(Value::as_str).unwrap_or("");
    let semantics = value.get("semantics").and_then(Value::as_object);
    let is_v3 = value.get("spec").and_then(Value::as_str) == Some(CONTRACT_SPEC);
    let mut ids = BTreeSet::new();
    let mut unresolved = Vec::new();

    match kind {
        "command" | "query" | "capability" => {
            let behavior = semantics.and_then(|value| value.get("behavior"));
            let mut observation_ids = BTreeSet::new();
            for observation in behavior
                .and_then(|value| value.get("observations"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(id) = observation.get("id").and_then(Value::as_str) {
                    if !observation_ids.insert(id.to_string()) {
                        issues.push(issue(
                            "contract.observation-id-duplicate",
                            format!("contract `{name}` repeats observation id `{id}`"),
                            true,
                        ));
                    }
                }
            }
            if is_v3 {
                inspect_clauses(
                    behavior
                        .and_then(|value| value.get("assumptions"))
                        .and_then(Value::as_array),
                    "assumptions",
                    name,
                    &mut ids,
                    &mut unresolved,
                    &mut issues,
                );
            }
            inspect_clauses(
                behavior
                    .and_then(|value| value.get("requires"))
                    .and_then(Value::as_array),
                "requires",
                name,
                &mut ids,
                &mut unresolved,
                &mut issues,
            );
            inspect_clauses(
                behavior
                    .and_then(|value| value.get("guarantees"))
                    .and_then(Value::as_array),
                "guarantees",
                name,
                &mut ids,
                &mut unresolved,
                &mut issues,
            );
            inspect_clauses(
                behavior
                    .and_then(|value| value.get("failures"))
                    .and_then(Value::as_array),
                "failures",
                name,
                &mut ids,
                &mut unresolved,
                &mut issues,
            );
            inspect_clauses(
                behavior
                    .and_then(|value| value.get("invariants"))
                    .and_then(Value::as_array),
                "invariants",
                name,
                &mut ids,
                &mut unresolved,
                &mut issues,
            );
            if let Some(cases) = behavior
                .and_then(|value| value.get("cases"))
                .and_then(Value::as_array)
            {
                for case in cases {
                    let case_id = case
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("<missing>");
                    if !ids.insert(case_id.to_string()) {
                        issues.push(issue(
                            "contract.clause-id-duplicate",
                            format!("contract `{name}` repeats clause or case id `{case_id}`"),
                            true,
                        ));
                    }
                    match case.pointer("/outcome/kind").and_then(Value::as_str) {
                        Some("rejected")
                            if case
                                .pointer("/outcome/category")
                                .and_then(Value::as_str)
                                .is_none() =>
                        {
                            issues.push(issue(
                                "contract.rejection-category-missing",
                                format!(
                                    "contract `{name}` rejected case `{case_id}` requires a stable category"
                                ),
                                true,
                            ));
                        }
                        Some("accepted") if case.pointer("/outcome/category").is_some() => {
                            issues.push(issue(
                                "contract.accepted-category",
                                format!(
                                    "contract `{name}` accepted case `{case_id}` cannot declare a rejection category"
                                ),
                                true,
                            ));
                        }
                        _ => {}
                    }
                    inspect_clauses(
                        case.get("ensures").and_then(Value::as_array),
                        &format!("cases.{case_id}.ensures"),
                        name,
                        &mut ids,
                        &mut unresolved,
                        &mut issues,
                    );
                    if kind == "query" {
                        for field in ["state_changes", "events", "effects"] {
                            if case
                                .pointer(&format!("/permits/{field}"))
                                .and_then(Value::as_array)
                                .is_some_and(|items| !items.is_empty())
                            {
                                issues.push(issue(
                                    "contract.query-frame",
                                    format!(
                                        "query contract `{name}` case `{case_id}` permits `{field}`"
                                    ),
                                    true,
                                ));
                            }
                        }
                    }
                }
            }
            if strict
                && behavior
                    .and_then(|value| value.get("cases"))
                    .and_then(Value::as_array)
                    .is_none_or(Vec::is_empty)
                && !value
                    .get("semantics")
                    .is_some_and(contains_external_realization)
            {
                issues.push(issue(
                    "contract.case-coverage-unresolved",
                    format!(
                        "contract `{name}` has no exhaustive core cases and no external realization"
                    ),
                    true,
                ));
            }
        }
        "event" => {
            inspect_clauses(
                semantics
                    .and_then(|value| value.get("event"))
                    .and_then(|value| value.get("guarantees"))
                    .and_then(Value::as_array),
                "event.guarantees",
                name,
                &mut ids,
                &mut unresolved,
                &mut issues,
            );
        }
        "api" => {
            let operations = semantics
                .and_then(|value| value.get("api"))
                .and_then(|value| value.get("operations"))
                .and_then(Value::as_array);
            if strict && operations.is_none_or(Vec::is_empty) {
                issues.push(issue(
                    "contract.api-operations-unresolved",
                    format!("API contract `{name}` has no referenced operations"),
                    true,
                ));
            }
        }
        _ => {}
    }

    let migration_draft = value
        .pointer("/x-rms/migration_draft")
        .and_then(Value::as_bool)
        == Some(true);
    if strict || !migration_draft {
        for location in unresolved {
            issues.push(issue(
                "contract.clause-unresolved",
                format!(
                    "contract `{name}` clause `{location}` has no executable realization{}",
                    if migration_draft {
                        " and migration drafts are rejected by strict checks"
                    } else {
                        "; unresolved clauses are permitted only in generated migration drafts"
                    }
                ),
                true,
            ));
        }
    }

    match property_definitions(value) {
        Ok(definitions) => {
            for definition in definitions {
                if let Err(errors) = property::compile_property(&definition) {
                    for error in errors {
                        issues.push(issue(
                            "contract.expression-invalid",
                            format!("contract `{name}`: {error}"),
                            true,
                        ));
                    }
                }
            }
        }
        Err(errors) => {
            for error in errors {
                issues.push(issue("contract.compile", error, true));
            }
        }
    }
    issues
}

fn contains_external_realization(value: &Value) -> bool {
    match value {
        Value::Object(object) => {
            object.get("kind").and_then(Value::as_str) == Some("external")
                || object.values().any(contains_external_realization)
        }
        Value::Array(items) => items.iter().any(contains_external_realization),
        _ => false,
    }
}

pub(super) fn property_definitions(
    contract: &Value,
) -> std::result::Result<Vec<Value>, Vec<String>> {
    if !is_contract_spec(contract.get("spec").and_then(Value::as_str)) {
        return Ok(Vec::new());
    }
    let name = contract
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("unnamed-contract");
    let kind = contract.get("kind").and_then(Value::as_str).unwrap_or("");
    let is_v3 = contract.get("spec").and_then(Value::as_str) == Some(CONTRACT_SPEC);
    let semantics = contract.get("semantics").and_then(Value::as_object);
    let profile = match kind {
        "command" | "query" | "capability" => semantics.and_then(|value| value.get("behavior")),
        "event" => semantics.and_then(|value| value.get("event")),
        "api" => return Ok(Vec::new()),
        other => {
            return Err(vec![format!(
                "contract `{name}` has unsupported kind `{other}`"
            )])
        }
    };
    let Some(profile) = profile else {
        let required_path = match kind {
            "command" | "query" | "capability" => "semantics.behavior",
            "event" => "semantics.event",
            _ => "semantics",
        };
        return Err(vec![format!(
            "contract `{name}` kind `{kind}` requires the `{required_path}` object; there is no `semantic_profile` field"
        )]);
    };
    let observations = profile
        .get("observations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut definitions = Vec::new();
    if kind == "event" {
        add_core_clauses(
            &mut definitions,
            profile,
            "guarantees",
            "guarantee",
            name,
            &observations,
            None,
        );
        if let Some(observability) = profile.get("observability") {
            for definition in &mut definitions {
                definition["observability"] = observability.clone();
            }
        }
        return Ok(definitions);
    }
    let assumption_expressions = if is_v3 {
        core_expressions(profile, "assumptions")
    } else {
        Vec::new()
    };
    let applicability = predicate_all(assumption_expressions.clone());
    let requirements = core_expressions(profile, "requires");
    let requirement = predicate_all(requirements.clone());
    if !is_v3 {
        add_core_clauses(
            &mut definitions,
            profile,
            "requires",
            "requirement",
            name,
            &observations,
            None,
        );
    }
    let valid_domain = predicate_all(vec![applicability.clone(), requirement.clone()]);
    add_core_clauses(
        &mut definitions,
        profile,
        "guarantees",
        "guarantee",
        name,
        &observations,
        Some(&valid_domain),
    );
    add_core_clauses(
        &mut definitions,
        profile,
        "invariants",
        "invariant",
        name,
        &observations,
        None,
    );

    let cases = profile
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut guards = Vec::new();
    for case in &cases {
        let Some(id) = case.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(when) = case.get("when") else {
            continue;
        };
        guards.push((id.to_string(), when.clone()));
        let mut consequences = Vec::new();
        if let Some(outcome) = case.pointer("/outcome/expression") {
            consequences.push(outcome.clone());
        }
        for clause in case
            .get("ensures")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if clause.pointer("/evaluation/kind").and_then(Value::as_str) == Some("core") {
                if let Some(expression) = clause.pointer("/evaluation/expression") {
                    consequences.push(expression.clone());
                }
            }
        }
        if consequences.is_empty() {
            consequences.push(json!({"constant": true}));
        }
        let consequence = predicate_all(consequences);
        let domain = if is_v3 {
            applicability.clone()
        } else {
            requirement.clone()
        };
        let predicate = implies(domain.clone(), implies(when.clone(), consequence));
        let activation = predicate_all(vec![domain, when.clone()]);
        definitions.push(step_definition(
            name,
            id,
            "case",
            predicate,
            observations.clone(),
            Some(activation),
            case.get("permits").cloned(),
        ));
    }
    for failure in profile
        .get("failures")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|clause| clause.pointer("/evaluation/kind").and_then(Value::as_str) == Some("core"))
    {
        let Some(id) = failure.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(expression) = failure.pointer("/evaluation/expression") else {
            continue;
        };
        let rejection_guards = cases
            .iter()
            .filter(|case| case.pointer("/outcome/category").and_then(Value::as_str) == Some(id))
            .filter_map(|case| case.get("when").cloned())
            .collect::<Vec<_>>();
        if rejection_guards.is_empty() {
            continue;
        }
        let activation = predicate_all(vec![valid_domain.clone(), predicate_all(rejection_guards)]);
        definitions.push(step_definition(
            name,
            id,
            "guarantee",
            implies(activation, expression.clone()),
            observations.clone(),
            None,
            None,
        ));
    }
    if kind == "query" {
        definitions.push(step_definition(
            name,
            "query-frame",
            "case",
            json!({"constant": true}),
            observations.clone(),
            Some(json!({"constant": true})),
            Some(json!({
                "state_changes": [],
                "events": [],
                "effects": []
            })),
        ));
    }
    if !guards.is_empty() {
        let coverage_domain = if is_v3 {
            applicability.clone()
        } else {
            requirement.clone()
        };
        definitions.push(step_definition(
            name,
            "case-coverage",
            "coverage",
            implies(
                coverage_domain,
                json!({"any": guards.iter().map(|(_, guard)| guard.clone()).collect::<Vec<_>>() }),
            ),
            observations.clone(),
            None,
            None,
        ));
    }
    if profile
        .pointer("/case_policy/overlap")
        .and_then(Value::as_str)
        == Some("forbidden")
    {
        for left in 0..guards.len() {
            for right in (left + 1)..guards.len() {
                definitions.push(step_definition(
                    name,
                    &format!("case-disjoint-{}-{}", guards[left].0, guards[right].0),
                    "disjointness",
                    implies(
                        if is_v3 {
                            applicability.clone()
                        } else {
                            requirement.clone()
                        },
                        json!({"not": {"all": [guards[left].1.clone(), guards[right].1.clone()]}}),
                    ),
                    observations.clone(),
                    None,
                    None,
                ));
            }
        }
    }
    if is_v3 {
        let rejected = cases
            .iter()
            .filter(|case| {
                case.pointer("/outcome/kind").and_then(Value::as_str) == Some("rejected")
            })
            .filter_map(|case| case.get("when").cloned())
            .collect::<Vec<_>>();
        let accepted = cases
            .iter()
            .filter(|case| {
                case.pointer("/outcome/kind").and_then(Value::as_str) == Some("accepted")
            })
            .filter_map(|case| case.get("when").cloned())
            .collect::<Vec<_>>();
        let invalid = predicate_all(vec![
            applicability.clone(),
            json!({"not": requirement.clone()}),
        ]);
        definitions.push(step_definition(
            name,
            "invalid-domain-rejection-coverage",
            "case",
            implies(invalid.clone(), predicate_any(rejected)),
            observations.clone(),
            None,
            None,
        ));
        definitions.push(step_definition(
            name,
            "invalid-domain-accepted-exclusion",
            "case",
            implies(invalid.clone(), json!({"not": predicate_any(accepted)})),
            observations.clone(),
            None,
            None,
        ));
        for case in &cases {
            if case.pointer("/outcome/kind").and_then(Value::as_str) != Some("rejected") {
                continue;
            }
            let Some(id) = case.get("id").and_then(Value::as_str) else {
                continue;
            };
            let Some(guard) = case.get("when") else {
                continue;
            };
            definitions.push(step_definition(
                name,
                &format!("invalid-frame-{id}"),
                "case",
                json!({"constant": true}),
                observations.clone(),
                Some(predicate_all(vec![invalid.clone(), guard.clone()])),
                Some(json!({"state_changes": [], "events": [], "effects": []})),
            ));
        }
        let assumptions = profile
            .get("assumptions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|clause| {
                let expression = match clause.pointer("/evaluation/kind").and_then(Value::as_str) {
                    Some("core") => clause.pointer("/evaluation/expression")?.clone(),
                    Some("external" | "unresolved") => json!({"constant": false}),
                    _ => return None,
                };
                Some(json!({
                    "id": clause.get("id")?.as_str()?,
                    "kind": "environment",
                    "expression": {"always": expression}
                }))
            })
            .collect::<Vec<_>>();
        for definition in &mut definitions {
            definition["assumptions"] = json!(assumptions);
        }
    }
    if let Some(observability) = profile.get("observability") {
        for definition in &mut definitions {
            definition["observability"] = observability.clone();
        }
    }
    Ok(definitions)
}

fn core_expressions(profile: &Value, field: &str) -> Vec<Value> {
    profile
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|clause| clause.pointer("/evaluation/kind").and_then(Value::as_str) == Some("core"))
        .filter_map(|clause| clause.pointer("/evaluation/expression").cloned())
        .collect()
}

fn add_core_clauses(
    definitions: &mut Vec<Value>,
    profile: &Value,
    field: &str,
    role: &str,
    contract: &str,
    observations: &[Value],
    antecedent: Option<&Value>,
) {
    for clause in profile
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if clause.pointer("/evaluation/kind").and_then(Value::as_str) != Some("core") {
            continue;
        }
        if let (Some(id), Some(expression)) = (
            clause.get("id").and_then(Value::as_str),
            clause.pointer("/evaluation/expression"),
        ) {
            let expression = antecedent
                .map(|antecedent| implies(antecedent.clone(), expression.clone()))
                .unwrap_or_else(|| expression.clone());
            definitions.push(step_definition(
                contract,
                id,
                role,
                expression,
                observations.to_vec(),
                None,
                None,
            ));
        }
    }
}

fn predicate_all(mut predicates: Vec<Value>) -> Value {
    match predicates.len() {
        0 => json!({"constant": true}),
        1 => predicates.remove(0),
        _ => json!({"all": predicates}),
    }
}

fn predicate_any(mut predicates: Vec<Value>) -> Value {
    match predicates.len() {
        0 => json!({"constant": false}),
        1 => predicates.remove(0),
        _ => json!({"any": predicates}),
    }
}

fn implies(antecedent: Value, consequent: Value) -> Value {
    json!({"any": [{"not": antecedent}, consequent]})
}

fn inspect_clauses(
    clauses: Option<&Vec<Value>>,
    location: &str,
    contract: &str,
    ids: &mut BTreeSet<String>,
    unresolved: &mut Vec<String>,
    issues: &mut Vec<ContractIssue>,
) {
    for clause in clauses.into_iter().flatten() {
        let id = clause
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>");
        if !ids.insert(id.to_string()) {
            issues.push(issue(
                "contract.clause-id-duplicate",
                format!("contract `{contract}` repeats clause id `{id}`"),
                true,
            ));
        }
        match clause.pointer("/evaluation/kind").and_then(Value::as_str) {
            Some("unresolved") => unresolved.push(format!("{location}.{id}")),
            Some("external") => {
                if clause
                    .pointer("/evaluation/property")
                    .and_then(Value::as_str)
                    .is_none_or(str::is_empty)
                {
                    issues.push(issue(
                        "contract.external-property-missing",
                        format!("clause `{id}` requires an exact external property id"),
                        true,
                    ));
                }
            }
            _ => {}
        }
    }
}

fn step_definition(
    contract: &str,
    clause: &str,
    role: &str,
    expression: Value,
    observations: Vec<Value>,
    activation: Option<Value>,
    permits: Option<Value>,
) -> Value {
    let mut definition = json!({
        "id": format!("contract:{contract}#{clause}"),
        "kind": "behavioral-contract",
        "proves": clause,
        "input_space": format!("observed invocations of `{contract}`"),
        "oracle": [format!("behavioral contract clause `{clause}` holds")],
        "observations": observations,
        "step": {
            "role": role,
            "contract": contract,
            "clause": clause,
            "expression": expression
        }
    });
    if let Some(activation) = activation {
        definition["step"]["activation"] = activation;
    }
    if let Some(permits) = permits {
        definition["step"]["permits"] = permits;
    }
    definition
}

pub(super) fn migrate_v01(value: &YamlValue) -> Result<YamlValue> {
    let json_value = serde_json::to_value(value)?;
    if json_value.get("spec").and_then(Value::as_str) != Some(LEGACY_CONTRACT_SPEC) {
        bail!("contract migration requires `{LEGACY_CONTRACT_SPEC}` input");
    }
    let name = required(&json_value, "name")?;
    let kind = required(&json_value, "kind")?;
    let version = json_value
        .get("version")
        .cloned()
        .unwrap_or_else(|| json!(1));
    let meaning = required(&json_value, "meaning")?;
    let unresolved = |items: Option<&Vec<Value>>| {
        items
            .into_iter()
            .flatten()
            .map(|item| {
                json!({
                    "id": item.get("id").and_then(Value::as_str).unwrap_or("unresolved-clause"),
                    "statement": item.get("statement").and_then(Value::as_str).unwrap_or("Unresolved legacy clause."),
                    "evaluation": {"kind": "unresolved"}
                })
            })
            .collect::<Vec<_>>()
    };
    let protocol = json_value.pointer("/semantics/protocol").cloned();
    let mut semantics = Map::new();
    match kind {
        "command" | "query" | "capability" => {
            semantics.insert(
                "behavior".to_string(),
                json!({
                    "observations": [],
                    "requires": unresolved(json_value.get("preconditions").and_then(Value::as_array)),
                    "guarantees": unresolved(json_value.get("postconditions").and_then(Value::as_array)),
                    "failures": unresolved(json_value.get("failure_categories").and_then(Value::as_array)),
                    "cases": [],
                    "invariants": [],
                    "case_policy": {"coverage": "exhaustive", "overlap": "forbidden"}
                }),
            );
        }
        "event" => {
            let mut guarantees =
                unresolved(json_value.get("preconditions").and_then(Value::as_array));
            guarantees.extend(unresolved(
                json_value.get("postconditions").and_then(Value::as_array),
            ));
            guarantees.extend(unresolved(
                json_value
                    .get("failure_categories")
                    .and_then(Value::as_array),
            ));
            for (index, statement) in json_value
                .pointer("/semantics/guarantees")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .enumerate()
            {
                guarantees.push(json!({
                    "id": format!("legacy-guarantee-{}", index + 1),
                    "statement": statement,
                    "evaluation": {"kind": "unresolved"}
                }));
            }
            semantics.insert(
                "event".to_string(),
                json!({"observations": [], "guarantees": guarantees}),
            );
        }
        "api" => {
            semantics.insert("api".to_string(), json!({"operations": []}));
        }
        other => bail!("legacy contract `{name}` has unsupported kind `{other}`"),
    }
    if let Some(protocol) = protocol {
        semantics.insert("protocol".to_string(), protocol);
    }
    let mut migrated = Map::new();
    migrated.insert("spec".to_string(), json!(CONTRACT_SPEC_V2));
    migrated.insert("name".to_string(), json!(name));
    migrated.insert("version".to_string(), version);
    migrated.insert("kind".to_string(), json!(kind));
    migrated.insert("meaning".to_string(), json!(meaning));
    migrated.insert("semantics".to_string(), Value::Object(semantics));
    for field in ["schema", "examples", "compatibility"] {
        if let Some(value) = json_value.get(field) {
            migrated.insert(field.to_string(), value.clone());
        }
    }
    if let Some(extensions) = json_value.as_object() {
        for (key, value) in extensions.iter().filter(|(key, _)| key.starts_with("x-")) {
            migrated.insert(key.clone(), value.clone());
        }
    }
    let rms_extension = migrated
        .entry("x-rms".to_string())
        .or_insert_with(|| json!({}));
    if !rms_extension.is_object() {
        *rms_extension = json!({});
    }
    rms_extension["migration_draft"] = json!(true);
    serde_yaml::to_value(Value::Object(migrated)).context("failed to render migrated contract")
}

pub(super) fn migrate(value: &YamlValue) -> Result<YamlValue> {
    let json_value = serde_json::to_value(value)?;
    match json_value.get("spec").and_then(Value::as_str) {
        Some(LEGACY_CONTRACT_SPEC) => migrate_v01(value),
        Some(CONTRACT_SPEC_V2) => migrate_v02(value),
        Some(CONTRACT_SPEC) => bail!("contract is already `{CONTRACT_SPEC}` and cannot downgrade"),
        Some(other) => bail!("unsupported contract migration source `{other}`"),
        None => bail!("contract migration input has no `spec`"),
    }
}

fn migrate_v02(value: &YamlValue) -> Result<YamlValue> {
    let mut migrated = serde_json::to_value(value)?;
    migrated["spec"] = json!(CONTRACT_SPEC);
    let kind = migrated.get("kind").and_then(Value::as_str).unwrap_or("");
    let mut draft = false;
    if matches!(kind, "command" | "query" | "capability") {
        let behavior = migrated
            .pointer_mut("/semantics/behavior")
            .ok_or_else(|| anyhow!("v0.2 contract requires `semantics.behavior`"))?;
        let requires = behavior
            .get("requires")
            .cloned()
            .unwrap_or_else(|| json!([]));
        behavior["observability"] = json!("none");
        behavior["assumptions"] = requires;
        behavior["requires"] = json!([]);
        let cases = behavior.get("cases").and_then(Value::as_array);
        draft = cases.is_none_or(|cases| {
            cases.is_empty()
                || cases.iter().any(|case| {
                    case.pointer("/outcome/kind").and_then(Value::as_str) == Some("rejected")
                        && case
                            .pointer("/outcome/category")
                            .and_then(Value::as_str)
                            .is_none()
                })
        });
    } else if kind == "event" {
        migrated["semantics"]["event"]["observability"] = json!("none");
    }
    if draft {
        if !migrated.get("x-rms").is_some_and(Value::is_object) {
            migrated["x-rms"] = json!({});
        }
        migrated["x-rms"]["migration_draft"] = json!(true);
    } else if let Some(extension) = migrated.get_mut("x-rms").and_then(Value::as_object_mut) {
        extension.remove("migration_draft");
    }
    serde_yaml::to_value(migrated).context("failed to render v0.3 migrated contract")
}

pub(super) fn smt_obligations(contract: &Value) -> Result<Vec<SmtObligation>> {
    let name = contract
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("contract requires `name`"))?;
    let Some(behavior) = contract.pointer("/semantics/behavior") else {
        return Ok(Vec::new());
    };
    let observations = observation_sorts(behavior)?;
    let observation_definitions = behavior
        .get("observations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut obligations = Vec::new();
    let is_v3 = contract.get("spec").and_then(Value::as_str) == Some(CONTRACT_SPEC);
    let applicability = if is_v3 {
        core_clause_conjunction(behavior, "assumptions").ok()
    } else {
        Some(json!({"constant": true}))
    };
    let requirements = core_clause_conjunction(behavior, "requires").ok();
    let domain = match (&applicability, &requirements) {
        (Some(applicability), Some(requirements)) => Some(predicate_all(vec![
            applicability.clone(),
            requirements.clone(),
        ])),
        (_, Some(requirements)) => Some(requirements.clone()),
        _ => None,
    };
    if let Some(domain) = &domain {
        obligations.push(obligation(
            format!("{name}.requirements.satisfiable"),
            ExpectedSolverResult::Sat,
            &observations,
            domain,
            &observation_definitions,
        )?);
        if let Ok(relation) = behavior_relation(behavior) {
            obligations.push(obligation(
                format!("{name}.guarantees.consistent"),
                ExpectedSolverResult::Sat,
                &observations,
                &json!({"all": [domain.clone(), relation]}),
                &observation_definitions,
            )?);
        }
    }
    let guards = behavior
        .get("cases")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|case| {
            Some((
                case.get("id")?.as_str()?.to_string(),
                case.get("when")?.clone(),
            ))
        })
        .collect::<Vec<_>>();
    for (id, guard) in &guards {
        let expression = domain
            .as_ref()
            .map(|domain| json!({"all": [domain.clone(), guard.clone()]}))
            .unwrap_or_else(|| guard.clone());
        obligations.push(obligation(
            format!("{name}.case.{id}.satisfiable"),
            ExpectedSolverResult::Sat,
            &observations,
            &expression,
            &observation_definitions,
        )?);
    }
    if !guards.is_empty() {
        let uncovered = json!({"all": [
            domain.clone().unwrap_or_else(|| json!({"constant": true})),
            {"not": {"any": guards.iter().map(|(_, value)| value.clone()).collect::<Vec<_>>()}}
        ]});
        obligations.push(obligation(
            format!("{name}.case-coverage"),
            ExpectedSolverResult::Unsat,
            &observations,
            &uncovered,
            &observation_definitions,
        )?);
    }
    if behavior
        .pointer("/case_policy/overlap")
        .and_then(Value::as_str)
        == Some("forbidden")
    {
        for left in 0..guards.len() {
            for right in (left + 1)..guards.len() {
                obligations.push(obligation(
                    format!(
                        "{name}.case-disjoint.{}.{}",
                        guards[left].0, guards[right].0
                    ),
                    ExpectedSolverResult::Unsat,
                    &observations,
                    &json!({"all": [
                        domain.clone().unwrap_or_else(|| json!({"constant": true})),
                        guards[left].1.clone(),
                        guards[right].1.clone()
                    ]}),
                    &observation_definitions,
                )?);
            }
        }
    }
    if is_v3 {
        let applicability = applicability.unwrap_or_else(|| json!({"constant": true}));
        let requirements = requirements.unwrap_or_else(|| json!({"constant": true}));
        let invalid = predicate_all(vec![
            applicability.clone(),
            json!({"not": requirements.clone()}),
        ]);
        let rejected = behavior
            .get("cases")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|case| {
                case.pointer("/outcome/kind").and_then(Value::as_str) == Some("rejected")
            })
            .filter_map(|case| case.get("when").cloned())
            .collect::<Vec<_>>();
        let accepted = behavior
            .get("cases")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|case| {
                case.pointer("/outcome/kind").and_then(Value::as_str) == Some("accepted")
            })
            .filter_map(|case| case.get("when").cloned())
            .collect::<Vec<_>>();
        obligations.push(obligation(
            format!("{name}.invalid-domain-rejection-coverage"),
            ExpectedSolverResult::Unsat,
            &observations,
            &predicate_all(vec![
                invalid.clone(),
                json!({"not": predicate_any(rejected)}),
            ]),
            &observation_definitions,
        )?);
        obligations.push(obligation(
            format!("{name}.invalid-domain-accepted-exclusion"),
            ExpectedSolverResult::Unsat,
            &observations,
            &predicate_all(vec![invalid, predicate_any(accepted)]),
            &observation_definitions,
        )?);
        obligations.extend(invariant_preservation_obligations(
            name,
            behavior,
            &observations,
            &observation_definitions,
            &applicability,
        )?);
    }
    Ok(obligations)
}

fn invariant_preservation_obligations(
    name: &str,
    behavior: &Value,
    sorts: &BTreeMap<String, String>,
    observation_definitions: &[Value],
    applicability: &Value,
) -> Result<Vec<SmtObligation>> {
    let relation = behavior_relation(behavior);
    let mut before_by_key = BTreeMap::new();
    for observation in observation_definitions {
        if observation.pointer("/source/kind").and_then(Value::as_str) != Some("state")
            || observation.pointer("/source/phase").and_then(Value::as_str) != Some("before")
        {
            continue;
        }
        if let Some(key) = state_observation_pair_key(observation) {
            before_by_key.insert(key, observation["id"].as_str().unwrap_or("").to_string());
        }
    }
    let mut after_to_before = BTreeMap::new();
    for observation in observation_definitions {
        if observation.pointer("/source/kind").and_then(Value::as_str) != Some("state")
            || observation.pointer("/source/phase").and_then(Value::as_str) != Some("after")
        {
            continue;
        }
        if let (Some(id), Some(key)) = (
            observation.get("id").and_then(Value::as_str),
            state_observation_pair_key(observation),
        ) {
            if let Some(before) = before_by_key.get(&key) {
                after_to_before.insert(id.to_string(), before.clone());
            }
        }
    }
    let mut obligations = Vec::new();
    for invariant in behavior
        .get("invariants")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|clause| clause.pointer("/evaluation/kind").and_then(Value::as_str) == Some("core"))
    {
        let id = invariant
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("invariant");
        let after = invariant
            .pointer("/evaluation/expression")
            .cloned()
            .unwrap_or_else(|| json!({"constant": true}));
        let before = rewrite_observation_references(&after, &after_to_before);
        let expression = match before {
            Err(reason) => {
                obligations.push(unsupported_obligation(
                    format!("{name}.invariant.{id}.transition-preservation"),
                    reason.to_string(),
                    observation_definitions,
                ));
                continue;
            }
            Ok(before) => match &relation {
                Ok(relation) => predicate_all(vec![
                    applicability.clone(),
                    before,
                    relation.clone(),
                    json!({"not": after}),
                ]),
                Err(reason) => {
                    obligations.push(unsupported_obligation(
                        format!("{name}.invariant.{id}.transition-preservation"),
                        reason.to_string(),
                        observation_definitions,
                    ));
                    continue;
                }
            },
        };
        obligations.push(obligation(
            format!("{name}.invariant.{id}.transition-preservation"),
            ExpectedSolverResult::Unsat,
            sorts,
            &expression,
            observation_definitions,
        )?);
    }
    Ok(obligations)
}

fn state_observation_pair_key(observation: &Value) -> Option<String> {
    Some(format!(
        "{}\u{0}{}\u{0}{}",
        observation
            .pointer("/source/instance")
            .and_then(Value::as_str)
            .unwrap_or(""),
        observation
            .pointer("/source/pointer")
            .and_then(Value::as_str)?,
        serde_json::to_string(observation.get("value")?).ok()?
    ))
}

fn rewrite_observation_references(
    value: &Value,
    pairs: &BTreeMap<String, String>,
) -> Result<Value> {
    match value {
        Value::Array(items) => Ok(Value::Array(
            items
                .iter()
                .map(|item| rewrite_observation_references(item, pairs))
                .collect::<Result<_>>()?,
        )),
        Value::Object(object) => {
            if let Some(id) = object.get("observation").and_then(Value::as_str) {
                let before = pairs.get(id).ok_or_else(|| {
                    anyhow!("missing before/after state observation pair for `{id}`")
                })?;
                return Ok(json!({"observation": before}));
            }
            Ok(Value::Object(
                object
                    .iter()
                    .map(|(key, item)| {
                        Ok((key.clone(), rewrite_observation_references(item, pairs)?))
                    })
                    .collect::<Result<Map<_, _>>>()?,
            ))
        }
        scalar => Ok(scalar.clone()),
    }
}

pub(super) fn refinement_obligations(old: &Value, new: &Value) -> Result<Vec<SmtObligation>> {
    if old.get("spec").and_then(Value::as_str) != new.get("spec").and_then(Value::as_str) {
        bail!("cross-version contract comparison is unsupported; migrate both contracts to `{CONTRACT_SPEC}` first");
    }
    let old_behavior = old
        .pointer("/semantics/behavior")
        .ok_or_else(|| anyhow!("old contract has no core behavior relation"))?;
    let new_behavior = new
        .pointer("/semantics/behavior")
        .ok_or_else(|| anyhow!("new contract has no core behavior relation"))?;
    let old_sorts = observation_sorts(old_behavior)?;
    let new_sorts = observation_sorts(new_behavior)?;
    if old_sorts != new_sorts {
        bail!("behavioral observation sorts changed; an explicit view transformation is required");
    }
    let old_domain = core_clause_conjunction(old_behavior, "requires")?;
    let new_domain = core_clause_conjunction(new_behavior, "requires")?;
    let old_relation = behavior_relation(old_behavior)?;
    let new_relation = behavior_relation(new_behavior)?;
    let observation_definitions = old_behavior
        .get("observations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(vec![
        obligation(
            "input-domain-not-narrowed".to_string(),
            ExpectedSolverResult::Unsat,
            &old_sorts,
            &json!({"all": [old_domain.clone(), {"not": new_domain.clone()}]}),
            &observation_definitions,
        )?,
        obligation(
            "new-behavior-refines-old".to_string(),
            ExpectedSolverResult::Unsat,
            &old_sorts,
            &json!({
                "all": [old_domain.clone(), new_relation, {"not": old_relation}]
            }),
            &observation_definitions,
        )?,
        obligation(
            "input-domain-expanded".to_string(),
            ExpectedSolverResult::Informational,
            &old_sorts,
            &json!({"all": [new_domain, {"not": old_domain}]}),
            &observation_definitions,
        )?,
    ])
}

pub(super) fn frame_expansion(old: &Value, new: &Value) -> Vec<String> {
    let old = permitted_frame(old);
    let new = permitted_frame(new);
    ["state_changes", "events", "effects"]
        .into_iter()
        .flat_map(|field| {
            let old = old.get(field).cloned().unwrap_or_default();
            new.get(field)
                .cloned()
                .unwrap_or_default()
                .difference(&old)
                .map(|value| format!("{field}:{value}"))
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(super) fn changed_outcome_kinds(old: &Value, new: &Value) -> Vec<String> {
    let kinds = |value: &Value| {
        value
            .pointer("/semantics/behavior/cases")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|case| {
                Some((
                    case.get("id")?.as_str()?.to_string(),
                    case.pointer("/outcome/kind")?.as_str()?.to_string(),
                ))
            })
            .collect::<BTreeMap<_, _>>()
    };
    let old = kinds(old);
    let new = kinds(new);
    old.into_iter()
        .filter_map(|(id, old_kind)| {
            new.get(&id)
                .filter(|new_kind| **new_kind != old_kind)
                .map(|new_kind| format!("{id}:{old_kind}->{new_kind}"))
        })
        .collect()
}

pub(super) fn has_external_behavior(value: &Value) -> bool {
    fn contains_external(value: &Value) -> bool {
        match value {
            Value::Object(object) => {
                object.get("kind").and_then(Value::as_str) == Some("external")
                    || object.values().any(contains_external)
            }
            Value::Array(items) => items.iter().any(contains_external),
            _ => false,
        }
    }
    value.get("semantics").is_some_and(contains_external)
}

pub(super) fn external_property_ids(value: &Value) -> BTreeSet<String> {
    fn collect(value: &Value, ids: &mut BTreeSet<String>) {
        match value {
            Value::Object(object) => {
                if object.get("kind").and_then(Value::as_str) == Some("external") {
                    if let Some(property) = object.get("property").and_then(Value::as_str) {
                        ids.insert(property.to_string());
                    }
                }
                for value in object.values() {
                    collect(value, ids);
                }
            }
            Value::Array(items) => {
                for value in items {
                    collect(value, ids);
                }
            }
            _ => {}
        }
    }
    let mut ids = BTreeSet::new();
    collect(value, &mut ids);
    ids
}

fn core_clause_conjunction(behavior: &Value, field: &str) -> Result<Value> {
    let mut predicates = Vec::new();
    for clause in behavior
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match clause.pointer("/evaluation/kind").and_then(Value::as_str) {
            Some("core") => predicates.push(
                clause
                    .pointer("/evaluation/expression")
                    .cloned()
                    .ok_or_else(|| anyhow!("core clause has no expression"))?,
            ),
            Some("external" | "unresolved") => {
                bail!("{field} contains a non-core clause")
            }
            _ => bail!("{field} contains an invalid clause realization"),
        }
    }
    Ok(match predicates.len() {
        0 => json!({"constant": true}),
        1 => predicates.remove(0),
        _ => json!({"all": predicates}),
    })
}

fn behavior_relation(behavior: &Value) -> Result<Value> {
    let cases = behavior
        .get("cases")
        .and_then(Value::as_array)
        .filter(|cases| !cases.is_empty())
        .ok_or_else(|| anyhow!("behavior relation has no core cases"))?;
    let mut relations = Vec::new();
    for case in cases {
        let mut predicates = vec![
            case.get("when")
                .cloned()
                .ok_or_else(|| anyhow!("behavior case has no guard"))?,
            case.pointer("/outcome/expression")
                .cloned()
                .ok_or_else(|| anyhow!("behavior case has no outcome expression"))?,
        ];
        for clause in case
            .get("ensures")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            match clause.pointer("/evaluation/kind").and_then(Value::as_str) {
                Some("core") => predicates.push(
                    clause
                        .pointer("/evaluation/expression")
                        .cloned()
                        .ok_or_else(|| anyhow!("core ensure has no expression"))?,
                ),
                Some("external" | "unresolved") => {
                    bail!("behavior relation contains a non-core ensure")
                }
                _ => bail!("behavior relation contains an invalid ensure"),
            }
        }
        relations.push(json!({"all": predicates}));
    }
    Ok(if relations.len() == 1 {
        relations.remove(0)
    } else {
        json!({"any": relations})
    })
}

fn permitted_frame(value: &Value) -> BTreeMap<&'static str, BTreeSet<String>> {
    let mut result = BTreeMap::new();
    for field in ["state_changes", "events", "effects"] {
        result.insert(field, BTreeSet::new());
    }
    for case in value
        .pointer("/semantics/behavior/cases")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for field in ["state_changes", "events", "effects"] {
            for item in case
                .pointer(&format!("/permits/{field}"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                result.entry(field).or_default().insert(item.to_string());
            }
        }
    }
    result
}

pub(super) fn solve_cvc5(obligation: &SmtObligation, timeout_ms: u64) -> SolverEvidence {
    let executable = "cvc5".to_string();
    if let Some(reason) = &obligation.unsupported_reason {
        return SolverEvidence {
            adapter: "smt-lib-v2".to_string(),
            executable,
            version: None,
            input_digest: sha256_bytes(obligation.script.as_bytes()),
            timeout_ms,
            result: "unresolved".to_string(),
            output: format!("unsupported obligation: {reason}"),
            model: None,
            unsat_core: None,
            model_revalidated: None,
        };
    }
    let version = Command::new(&executable)
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|output| output.lines().next().map(str::to_string));
    let input_digest = sha256_bytes(obligation.script.as_bytes());
    let run = |extra: Option<&str>| -> Result<String> {
        let mut child = Command::new(&executable)
            .arg("--lang=smt2")
            .arg(format!("--tlimit-per={timeout_ms}"))
            .arg("--produce-models")
            .arg("--produce-unsat-cores")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to start `{executable}`"))?;
        let mut input = obligation.script.clone();
        input.push_str("\n(check-sat)\n");
        if let Some(extra) = extra {
            input.push_str(extra);
            input.push('\n');
        }
        child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("solver stdin unavailable"))?
            .write_all(input.as_bytes())?;
        let output = child.wait_with_output()?;
        let mut rendered = String::from_utf8_lossy(&output.stdout).to_string();
        if !output.stderr.is_empty() {
            rendered.push_str(&String::from_utf8_lossy(&output.stderr));
        }
        Ok(rendered)
    };
    let output = match run(None) {
        Ok(output) => output,
        Err(error) => {
            return SolverEvidence {
                adapter: "smt-lib-v2".to_string(),
                executable,
                version,
                input_digest,
                timeout_ms,
                result: "unresolved".to_string(),
                output: error.to_string(),
                model: None,
                unsat_core: None,
                model_revalidated: None,
            }
        }
    };
    let first = output
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("unknown")
        .trim()
        .to_string();
    let value_query = format!(
        "(get-value ({}))",
        obligation
            .observations
            .iter()
            .filter_map(|observation| observation.get("id").and_then(Value::as_str))
            .map(smt_symbol)
            .collect::<Vec<_>>()
            .join(" ")
    );
    let model = if first == "sat" {
        run(Some(&value_query)).ok()
    } else {
        None
    };
    let unsat_core = if first == "unsat" {
        run(Some("(get-unsat-core)")).ok().and_then(|output| {
            output
                .lines()
                .skip(1)
                .find(|line| !line.trim().is_empty())
                .map(str::to_string)
        })
    } else {
        None
    };
    let model_revalidated = model.as_deref().map(|model| {
        parse_model_assignments(model, &obligation.observations).and_then(|assignments| {
            property::evaluate_core_expression(
                &obligation.observations,
                &obligation.expression,
                &assignments,
            )
        })
    });
    let validated = match &model_revalidated {
        Some(Ok(true)) => Some(true),
        Some(Ok(false)) | Some(Err(_)) => Some(false),
        None => None,
    };
    let mut rendered_output = output;
    if let Some(Err(error)) = model_revalidated {
        rendered_output.push_str(&format!("\nRMS model revalidation failed: {error:#}"));
    } else if validated == Some(false) {
        rendered_output.push_str("\nRMS evaluator rejected the satisfiable solver model");
    }
    SolverEvidence {
        adapter: "smt-lib-v2".to_string(),
        executable,
        version,
        input_digest,
        timeout_ms,
        result: match first.as_str() {
            "sat" if validated == Some(true) => "sat".to_string(),
            "sat" => "unresolved".to_string(),
            "unsat" => "unsat".to_string(),
            _ => "unresolved".to_string(),
        },
        output: rendered_output,
        model,
        unsat_core,
        model_revalidated: validated,
    }
}

#[derive(Clone, Debug)]
enum SExpression {
    Atom(String),
    List(Vec<SExpression>),
}

fn parse_model_assignments(model: &str, observations: &[Value]) -> Result<BTreeMap<String, Value>> {
    let tokens = tokenize_smt(model)?;
    let mut cursor = 0;
    let mut expressions = Vec::new();
    while cursor < tokens.len() {
        expressions.push(parse_s_expression(&tokens, &mut cursor)?);
    }
    let values = expressions
        .iter()
        .rev()
        .find_map(|expression| match expression {
            SExpression::List(values) => Some(values),
            SExpression::Atom(_) => None,
        })
        .ok_or_else(|| anyhow!("cvc5 returned no model values"))?;
    let types = observations
        .iter()
        .filter_map(|observation| {
            Some((
                observation.get("id")?.as_str()?.to_string(),
                observation.get("value")?.clone(),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let mut assignments = BTreeMap::new();
    for pair in values {
        let SExpression::List(pair) = pair else {
            continue;
        };
        if pair.len() != 2 {
            continue;
        }
        let SExpression::Atom(symbol) = &pair[0] else {
            continue;
        };
        let id = symbol.trim_matches('|');
        let value_type = types
            .get(id)
            .ok_or_else(|| anyhow!("solver returned unknown observation `{id}`"))?;
        assignments.insert(id.to_string(), model_value(&pair[1], value_type)?);
    }
    for id in types.keys() {
        if !assignments.contains_key(id) {
            bail!("solver model omitted observation `{id}`");
        }
    }
    Ok(assignments)
}

fn tokenize_smt(input: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut characters = input.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '(' | ')' => tokens.push(character.to_string()),
            ';' => {
                for next in characters.by_ref() {
                    if next == '\n' {
                        break;
                    }
                }
            }
            '"' => {
                let mut value = String::from("\"");
                loop {
                    let next = characters
                        .next()
                        .ok_or_else(|| anyhow!("unterminated SMT string"))?;
                    value.push(next);
                    if next == '"' {
                        if characters.peek() == Some(&'"') {
                            if let Some(escaped_quote) = characters.next() {
                                value.push(escaped_quote);
                            }
                        } else {
                            break;
                        }
                    }
                }
                tokens.push(value);
            }
            '|' => {
                let mut value = String::from("|");
                loop {
                    let next = characters
                        .next()
                        .ok_or_else(|| anyhow!("unterminated SMT quoted symbol"))?;
                    value.push(next);
                    if next == '|' {
                        break;
                    }
                }
                tokens.push(value);
            }
            value if value.is_whitespace() => {}
            value => {
                let mut token = value.to_string();
                while characters
                    .peek()
                    .is_some_and(|next| !next.is_whitespace() && !matches!(next, '(' | ')'))
                {
                    if let Some(next) = characters.next() {
                        token.push(next);
                    }
                }
                tokens.push(token);
            }
        }
    }
    Ok(tokens)
}

fn parse_s_expression(tokens: &[String], cursor: &mut usize) -> Result<SExpression> {
    let token = tokens
        .get(*cursor)
        .ok_or_else(|| anyhow!("unexpected end of SMT model"))?;
    *cursor += 1;
    if token == "(" {
        let mut values = Vec::new();
        while tokens.get(*cursor).is_some_and(|token| token != ")") {
            values.push(parse_s_expression(tokens, cursor)?);
        }
        if tokens.get(*cursor).is_none() {
            bail!("unterminated SMT list");
        }
        *cursor += 1;
        Ok(SExpression::List(values))
    } else if token == ")" {
        bail!("unexpected SMT closing parenthesis")
    } else {
        Ok(SExpression::Atom(token.clone()))
    }
}

fn model_value(expression: &SExpression, value_type: &Value) -> Result<Value> {
    match value_type.as_str() {
        Some("occurrence" | "boolean") => match expression {
            SExpression::Atom(value) if value == "true" => Ok(Value::Bool(true)),
            SExpression::Atom(value) if value == "false" => Ok(Value::Bool(false)),
            _ => bail!("solver returned a non-boolean model value"),
        },
        Some("integer") => Ok(json!(s_expression_integer(expression)?)),
        Some("string") => match expression {
            SExpression::Atom(value) if value.starts_with('"') && value.ends_with('"') => Ok(
                Value::String(value[1..value.len() - 1].replace("\"\"", "\"")),
            ),
            _ => bail!("solver returned a non-string model value"),
        },
        Some(other) => bail!("unknown observation model type `{other}`"),
        None if value_type.get("variant").is_some() => match expression {
            SExpression::Atom(value) if value.starts_with('"') && value.ends_with('"') => Ok(
                Value::String(value[1..value.len() - 1].replace("\"\"", "\"")),
            ),
            _ => bail!("solver returned a non-variant model value"),
        },
        None => {
            let dimension = value_type
                .get("quantity")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("unknown observation model type"))?;
            let (numerator, denominator) = s_expression_rational(expression)?;
            let decimal = finite_decimal(numerator, denominator)?;
            let unit = match dimension {
                "time" => "ns",
                "information" => "bit",
                "ratio" => "ratio",
                "transition" => "transition",
                "message" => "message",
                "attempt" => "attempt",
                "item" => "item",
                other => bail!("unknown quantity dimension `{other}`"),
            };
            Ok(json!({"value": decimal, "unit": unit}))
        }
    }
}

fn s_expression_integer(expression: &SExpression) -> Result<i128> {
    match expression {
        SExpression::Atom(value) => value
            .parse::<i128>()
            .with_context(|| format!("invalid SMT integer `{value}`")),
        SExpression::List(values)
            if values.len() == 2
                && matches!(&values[0], SExpression::Atom(operator) if operator == "-") =>
        {
            s_expression_integer(&values[1])?
                .checked_neg()
                .ok_or_else(|| anyhow!("SMT integer overflow"))
        }
        _ => bail!("solver returned a non-integer model value"),
    }
}

fn s_expression_rational(expression: &SExpression) -> Result<(i128, i128)> {
    match expression {
        SExpression::Atom(value) if value.contains('.') => {
            let negative = value.starts_with('-');
            let unsigned = value.trim_start_matches('-');
            let (whole, fraction) = unsigned
                .split_once('.')
                .ok_or_else(|| anyhow!("invalid SMT decimal `{value}`"))?;
            let denominator = 10_i128
                .checked_pow(fraction.len() as u32)
                .ok_or_else(|| anyhow!("SMT decimal precision overflow"))?;
            let mut numerator = format!("{whole}{fraction}").parse::<i128>()?;
            if negative {
                numerator = -numerator;
            }
            Ok((numerator, denominator))
        }
        SExpression::Atom(_) | SExpression::List(_) => match expression {
            SExpression::List(values)
                if values.len() == 3
                    && matches!(&values[0], SExpression::Atom(operator) if operator == "/") =>
            {
                Ok((
                    s_expression_integer(&values[1])?,
                    s_expression_integer(&values[2])?,
                ))
            }
            _ => Ok((s_expression_integer(expression)?, 1)),
        },
    }
}

fn finite_decimal(numerator: i128, denominator: i128) -> Result<String> {
    if denominator == 0 {
        bail!("SMT rational denominator is zero");
    }
    let negative = (numerator < 0) != (denominator < 0);
    let numerator = numerator.abs();
    let mut denominator = denominator.abs();
    let mut twos = 0_u32;
    let mut fives = 0_u32;
    while denominator % 2 == 0 {
        denominator /= 2;
        twos += 1;
    }
    while denominator % 5 == 0 {
        denominator /= 5;
        fives += 1;
    }
    if denominator != 1 {
        bail!("solver model contains a non-terminating exact decimal");
    }
    let scale = twos.max(fives);
    let factor_two = 2_i128
        .checked_pow(scale - twos)
        .ok_or_else(|| anyhow!("decimal overflow"))?;
    let factor_five = 5_i128
        .checked_pow(scale - fives)
        .ok_or_else(|| anyhow!("decimal overflow"))?;
    let scaled = numerator
        .checked_mul(factor_two)
        .and_then(|value| value.checked_mul(factor_five))
        .ok_or_else(|| anyhow!("decimal overflow"))?;
    let base = 10_i128
        .checked_pow(scale)
        .ok_or_else(|| anyhow!("decimal overflow"))?;
    let whole = scaled / base;
    let remainder = scaled % base;
    let sign = if negative { "-" } else { "" };
    if scale == 0 {
        Ok(format!("{sign}{whole}"))
    } else {
        Ok(format!(
            "{sign}{whole}.{remainder:0width$}",
            width = scale as usize
        ))
    }
}

fn observation_sorts(behavior: &Value) -> Result<BTreeMap<String, String>> {
    let mut observations = BTreeMap::new();
    for observation in behavior
        .get("observations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = observation
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("observation requires `id`"))?;
        let value = observation
            .get("value")
            .ok_or_else(|| anyhow!("observation `{id}` requires `value`"))?;
        let sort = match value.as_str() {
            Some("occurrence" | "boolean") => "Bool",
            Some("integer") => "Int",
            Some("string") => "String",
            None if value.get("variant").is_some() => "String",
            None if value.get("quantity").is_some() => "Real",
            _ => bail!("observation `{id}` has unsupported SMT sort"),
        };
        observations.insert(id.to_string(), sort.to_string());
    }
    Ok(observations)
}

fn obligation(
    id: String,
    expectation: ExpectedSolverResult,
    observations: &BTreeMap<String, String>,
    expression: &Value,
    observation_definitions: &[Value],
) -> Result<SmtObligation> {
    let mut script = String::from("(set-logic ALL)\n");
    for (id, sort) in observations {
        script.push_str(&format!("(declare-const {} {sort})\n", smt_symbol(id)));
    }
    for observation in observation_definitions {
        let Some(id) = observation.get("id").and_then(Value::as_str) else {
            continue;
        };
        if let Some(cases) = observation
            .pointer("/value/variant")
            .and_then(Value::as_array)
        {
            let alternatives = cases
                .iter()
                .filter_map(Value::as_str)
                .map(|case| {
                    Ok(format!(
                        "(= {} {})",
                        smt_symbol(id),
                        smt_literal(&json!(case))?
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            script.push_str(&format!("(assert (or {}))\n", alternatives.join(" ")));
        }
    }
    script.push_str(&format!(
        "(assert (! {} :named obligation))\n",
        smt_predicate(expression)?
    ));
    Ok(SmtObligation {
        id,
        expectation,
        script,
        expression: expression.clone(),
        observations: observation_definitions.to_vec(),
        unsupported_reason: None,
    })
}

fn unsupported_obligation(
    id: String,
    reason: String,
    observation_definitions: &[Value],
) -> SmtObligation {
    SmtObligation {
        id,
        expectation: ExpectedSolverResult::Unsat,
        script: String::new(),
        expression: json!({"constant": true}),
        observations: observation_definitions.to_vec(),
        unsupported_reason: Some(reason),
    }
}

fn smt_predicate(value: &Value) -> Result<String> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("SMT predicate must be an object"))?;
    if object.len() != 1 {
        bail!("SMT predicate must contain one variant");
    }
    let (kind, body) = object
        .iter()
        .next()
        .ok_or_else(|| anyhow!("SMT predicate has no variant"))?;
    Ok(match kind.as_str() {
        "constant" => if body.as_bool() == Some(true) {
            "true"
        } else {
            "false"
        }
        .to_string(),
        "occurred" => smt_symbol(
            body.as_str()
                .ok_or_else(|| anyhow!("occurred requires id"))?,
        ),
        "equals" => format!(
            "(= {} {})",
            smt_term(
                body.get("left")
                    .ok_or_else(|| anyhow!("equals requires left"))?
            )?,
            smt_term(
                body.get("right")
                    .ok_or_else(|| anyhow!("equals requires right"))?
            )?
        ),
        "compare" => {
            let operator = match body.get("operator").and_then(Value::as_str) {
                Some("lt") => "<",
                Some("lte") => "<=",
                Some("eq") => "=",
                Some("gte") => ">=",
                Some("gt") => ">",
                _ => bail!("compare has unsupported operator"),
            };
            format!(
                "({operator} {} {})",
                smt_term(
                    body.get("left")
                        .ok_or_else(|| anyhow!("compare requires left"))?
                )?,
                smt_term(
                    body.get("right")
                        .ok_or_else(|| anyhow!("compare requires right"))?
                )?
            )
        }
        "not" => format!("(not {})", smt_predicate(body)?),
        "all" => format!(
            "(and {})",
            body.as_array()
                .ok_or_else(|| anyhow!("all requires array"))?
                .iter()
                .map(smt_predicate)
                .collect::<Result<Vec<_>>>()?
                .join(" ")
        ),
        "any" => format!(
            "(or {})",
            body.as_array()
                .ok_or_else(|| anyhow!("any requires array"))?
                .iter()
                .map(smt_predicate)
                .collect::<Result<Vec<_>>>()?
                .join(" ")
        ),
        other => bail!("unsupported SMT predicate `{other}`"),
    })
}

fn smt_term(value: &Value) -> Result<String> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("SMT term must be an object"))?;
    if object.len() != 1 {
        bail!("SMT term must contain one variant");
    }
    let (kind, body) = object
        .iter()
        .next()
        .ok_or_else(|| anyhow!("SMT term has no variant"))?;
    Ok(match kind.as_str() {
        "observation" => smt_symbol(
            body.as_str()
                .ok_or_else(|| anyhow!("observation term requires id"))?,
        ),
        "literal" => smt_literal(body)?,
        "add" => format!(
            "(+ {})",
            body.as_array()
                .ok_or_else(|| anyhow!("add requires array"))?
                .iter()
                .map(smt_term)
                .collect::<Result<Vec<_>>>()?
                .join(" ")
        ),
        "subtract" => format!(
            "(- {} {})",
            smt_term(
                body.get("left")
                    .ok_or_else(|| anyhow!("subtract requires left"))?
            )?,
            smt_term(
                body.get("right")
                    .ok_or_else(|| anyhow!("subtract requires right"))?
            )?
        ),
        other => bail!("unsupported SMT term `{other}`"),
    })
}

fn smt_literal(value: &Value) -> Result<String> {
    if let Some(value) = value.as_bool() {
        return Ok(if value { "true" } else { "false" }.to_string());
    }
    if let Some(value) = value.as_i64() {
        return Ok(value.to_string());
    }
    if let Some(value) = value.as_u64() {
        return Ok(value.to_string());
    }
    if let Some(value) = value.as_str() {
        return Ok(format!("\"{}\"", value.replace('"', "\"\"")));
    }
    if value.get("unit").is_some() && value.get("value").is_some() {
        return normalized_quantity_literal(value);
    }
    bail!("unsupported SMT literal")
}

fn normalized_quantity_literal(value: &Value) -> Result<String> {
    let raw = value
        .get("value")
        .map(ToString::to_string)
        .ok_or_else(|| anyhow!("quantity requires value"))?;
    let raw = raw.trim_matches('"');
    let unit = value
        .get("unit")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("quantity requires unit"))?;
    let factor: i128 = match unit {
        "ns" | "bit" | "ratio" | "transition" | "message" | "attempt" | "item" => 1,
        "us" => 1_000,
        "ms" => 1_000_000,
        "s" => 1_000_000_000,
        "min" => 60_000_000_000,
        "h" => 3_600_000_000_000,
        "byte" => 8,
        "KiB" => 8_192,
        "MiB" => 8_388_608,
        "GiB" => 8_589_934_592,
        "percent" => return Ok(format!("(/ {raw} 100)")),
        _ => bail!("unknown quantity unit `{unit}`"),
    };
    Ok(if factor == 1 {
        raw.to_string()
    } else {
        format!("(* {raw} {factor})")
    })
}

fn smt_symbol(value: &str) -> String {
    format!("|{}|", value.replace('|', "_"))
}

fn required<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("legacy contract requires `{field}`"))
}

fn issue(check: impl Into<String>, message: impl Into<String>, blocking: bool) -> ContractIssue {
    ContractIssue {
        check: check.into(),
        message: message.into(),
        blocking,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use walkdir::WalkDir;

    fn behavioral_contract() -> Value {
        json!({
            "spec": CONTRACT_SPEC_V2,
            "name": "choose",
            "version": 1,
            "kind": "command",
            "meaning": "Choose a positive value.",
            "semantics": {"behavior": {
                "observations": [
                    {"id": "amount", "source": {"kind": "input", "pointer": "/amount"}, "value": "integer"},
                    {"id": "accepted", "source": {"kind": "output", "output_kind": "reply", "name": "Accepted"}, "value": "occurrence"}
                ],
                "requires": [{"id": "positive", "statement": "Amount is positive.", "evaluation": {"kind": "core", "expression": {"compare": {"left": {"observation": "amount"}, "operator": "gt", "right": {"literal": 0}}}}}],
                "guarantees": [], "failures": [], "invariants": [],
                "cases": [{"id": "accepted", "statement": "Valid values are accepted.", "when": {"constant": true}, "outcome": {"kind": "accepted", "expression": {"occurred": "accepted"}}, "ensures": [], "permits": {"state_changes": [], "events": [], "effects": []}}],
                "case_policy": {"coverage": "exhaustive", "overlap": "forbidden"}
            }}
        })
    }

    fn total_contract() -> Value {
        json!({
            "spec": CONTRACT_SPEC,
            "name": "choose-total",
            "version": 3,
            "kind": "command",
            "meaning": "Choose or reject every applicable value.",
            "semantics": {"behavior": {
                "observability": "full",
                "observations": [
                    {"id": "trusted", "source": {"kind": "input", "pointer": "/trusted"}, "value": "boolean"},
                    {"id": "amount", "source": {"kind": "input", "pointer": "/amount"}, "value": "integer"},
                    {"id": "result", "source": {"kind": "output", "pointer": "/kind"}, "value": {"variant": ["Accepted", "Rejected"]}}
                ],
                "assumptions": [{"id": "trusted-clock", "statement": "The input source is trusted.", "evaluation": {"kind": "core", "expression": {"equals": {"left": {"observation": "trusted"}, "right": {"literal": true}}}}}],
                "requires": [{"id": "positive", "statement": "Amount is positive.", "evaluation": {"kind": "core", "expression": {"compare": {"left": {"observation": "amount"}, "operator": "gt", "right": {"literal": 0}}}}}],
                "guarantees": [], "failures": [], "invariants": [],
                "cases": [
                    {"id": "accepted", "statement": "Ordinary valid values are accepted.", "when": {"all": [{"compare": {"left": {"observation": "amount"}, "operator": "gt", "right": {"literal": 0}}}, {"not": {"equals": {"left": {"observation": "amount"}, "right": {"literal": 13}}}}]}, "outcome": {"kind": "accepted", "expression": {"equals": {"left": {"observation": "result"}, "right": {"literal": "Accepted"}}}}, "ensures": [], "permits": {"state_changes": [], "events": [], "effects": []}},
                    {"id": "business-rejection", "statement": "Thirteen is unavailable.", "when": {"equals": {"left": {"observation": "amount"}, "right": {"literal": 13}}}, "outcome": {"kind": "rejected", "category": "unavailable", "expression": {"equals": {"left": {"observation": "result"}, "right": {"literal": "Rejected"}}}}, "ensures": [], "permits": {"state_changes": ["/reservation"], "events": [], "effects": []}},
                    {"id": "invalid-rejection", "statement": "Invalid values are rejected.", "when": {"compare": {"left": {"observation": "amount"}, "operator": "lte", "right": {"literal": 0}}}, "outcome": {"kind": "rejected", "category": "invalid-input", "expression": {"equals": {"left": {"observation": "result"}, "right": {"literal": "Rejected"}}}}, "ensures": [], "permits": {"state_changes": [], "events": [], "effects": []}}
                ],
                "case_policy": {"coverage": "exhaustive", "overlap": "forbidden"}
            }}
        })
    }

    fn invocation_record(amount: i64, trusted: bool, result: &str) -> Value {
        json!({
            "spec": INVOCATION_SPEC,
            "contract": "choose-total",
            "binding": "rust",
            "contract_digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "input": {"amount": amount, "trusted": trusted},
            "output": {"kind": result, "events": [], "effects": []}
        })
    }

    #[test]
    fn v03_totality_assigns_provider_blame_and_assumption_gaps_are_inconclusive() {
        let definitions = property_definitions(&total_contract()).unwrap();
        let evaluate = |id: &str, record: Value| {
            let definition = definitions.iter().find(|value| value["id"] == id).unwrap();
            let compiled = property::compile_property(definition).unwrap();
            property::evaluate_trace(&compiled, &record).unwrap()
        };
        assert_eq!(
            evaluate(
                "contract:choose-total#invalid-rejection",
                invocation_record(0, true, "Accepted")
            )
            .verdict(),
            &property::Verdict::Violated
        );
        let violation = serde_json::to_value(evaluate(
            "contract:choose-total#invalid-rejection",
            invocation_record(0, true, "Accepted"),
        ))
        .unwrap();
        assert_eq!(
            violation.pointer("/explanation/blame"),
            Some(&json!("provider"))
        );

        let mut mutated = invocation_record(0, true, "Rejected");
        mutated["state_before"] = json!({"count": 0});
        mutated["state_after"] = json!({"count": 1});
        let frame = evaluate(
            "contract:choose-total#invalid-frame-invalid-rejection",
            mutated,
        );
        assert_eq!(frame.verdict(), &property::Verdict::Violated);

        let assumption = evaluate(
            "contract:choose-total#invalid-domain-rejection-coverage",
            invocation_record(0, false, "Accepted"),
        );
        assert_eq!(assumption.verdict(), &property::Verdict::Inconclusive);
        assert!(serde_json::to_value(assumption)
            .unwrap()
            .pointer("/explanation/blame")
            .is_none());

        for record in [
            invocation_record(1, true, "Accepted"),
            invocation_record(13, true, "Rejected"),
            invocation_record(0, true, "Rejected"),
        ] {
            for definition in &definitions {
                let evaluation = property::evaluate_trace(
                    &property::compile_property(definition).unwrap(),
                    &record,
                )
                .unwrap();
                assert_eq!(evaluation.verdict(), &property::Verdict::Satisfied);
            }
        }
    }

    #[test]
    fn v02_migration_preserves_requires_as_assumptions_and_marks_incomplete_drafts() {
        let migrated = migrate(&serde_yaml::to_value(behavioral_contract()).unwrap()).unwrap();
        let migrated = serde_json::to_value(migrated).unwrap();
        assert_eq!(migrated["spec"], CONTRACT_SPEC);
        assert_eq!(
            migrated.pointer("/semantics/behavior/observability"),
            Some(&json!("none"))
        );
        assert_eq!(
            migrated.pointer("/semantics/behavior/requires"),
            Some(&json!([]))
        );
        assert_eq!(
            migrated.pointer("/semantics/behavior/assumptions/0/id"),
            Some(&json!("positive"))
        );

        let mut incomplete = behavioral_contract();
        incomplete["semantics"]["behavior"]["cases"] = json!([]);
        let draft = migrate(&serde_yaml::to_value(incomplete).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(draft)
                .unwrap()
                .pointer("/x-rms/migration_draft"),
            Some(&json!(true))
        );
        assert!(
            refinement_obligations(&behavioral_contract(), &total_contract())
                .unwrap_err()
                .to_string()
                .contains("migrate both")
        );
    }

    #[test]
    fn invariant_preservation_is_named_and_missing_pairs_are_explicitly_unsupported() {
        let mut contract = total_contract();
        contract["semantics"]["behavior"]["observations"]
            .as_array_mut()
            .unwrap()
            .extend([
                json!({"id": "count_before", "source": {"kind": "state", "phase": "before", "instance": "counter", "pointer": "/count"}, "value": "integer"}),
                json!({"id": "count_after", "source": {"kind": "state", "phase": "after", "instance": "counter", "pointer": "/count"}, "value": "integer"}),
            ]);
        contract["semantics"]["behavior"]["invariants"] = json!([{
            "id": "nonnegative",
            "statement": "Count remains nonnegative.",
            "evaluation": {"kind": "core", "expression": {"compare": {"left": {"observation": "count_after"}, "operator": "gte", "right": {"literal": 0}}}}
        }]);
        let obligations = smt_obligations(&contract).unwrap();
        let preservation = obligations
            .iter()
            .find(|obligation| {
                obligation
                    .id
                    .ends_with("invariant.nonnegative.transition-preservation")
            })
            .unwrap();
        assert!(preservation.unsupported_reason.is_none());
        assert!(preservation.script.contains("count_before"));
        assert!(preservation.script.contains("count_after"));

        contract["semantics"]["behavior"]["observations"]
            .as_array_mut()
            .unwrap()
            .retain(|observation| observation["id"] != "count_before");
        let unsupported = smt_obligations(&contract)
            .unwrap()
            .into_iter()
            .find(|obligation| {
                obligation
                    .id
                    .ends_with("invariant.nonnegative.transition-preservation")
            })
            .unwrap();
        assert!(solve_cvc5(&unsupported, 10)
            .output
            .contains("missing before/after"));
    }

    #[test]
    fn core_contract_compiles_into_step_properties() {
        let definitions = property_definitions(&behavioral_contract()).unwrap();
        assert!(definitions
            .iter()
            .any(|value| value["id"] == "contract:choose#positive"));
        assert!(definitions
            .iter()
            .all(|value| property::compile_property(value).is_ok()));
    }

    #[test]
    fn missing_behavior_profile_names_the_exact_contract_path() {
        let mut contract = behavioral_contract();
        contract["semantics"] = json!({
            "observations": [],
            "requires": [],
            "guarantees": [],
            "failures": [],
            "cases": [],
            "invariants": []
        });

        let errors = property_definitions(&contract).unwrap_err();

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("requires the `semantics.behavior` object"));
        assert!(errors[0].contains("there is no `semantic_profile` field"));
    }

    #[test]
    fn strict_validation_rejects_duplicate_unresolved_and_impure_query_semantics() {
        let mut contract = behavioral_contract();
        contract["kind"] = json!("query");
        contract["semantics"]["behavior"]["requires"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id": "positive",
                "statement": "Duplicate id.",
                "evaluation": {"kind": "unresolved"}
            }));
        contract["semantics"]["behavior"]["cases"][0]["permits"]["effects"] = json!(["network"]);
        let checks = validate(&contract, true)
            .into_iter()
            .map(|issue| issue.check)
            .collect::<BTreeSet<_>>();
        assert!(checks.contains("contract.clause-id-duplicate"));
        assert!(checks.contains("contract.clause-unresolved"));
        assert!(checks.contains("contract.query-frame"));
    }

    #[test]
    fn strict_validation_requires_api_reference_closure() {
        let contract = json!({
            "spec": CONTRACT_SPEC,
            "name": "empty-api",
            "version": 1,
            "kind": "api",
            "meaning": "An API draft.",
            "semantics": {"api": {"operations": []}}
        });
        assert!(validate(&contract, true)
            .iter()
            .any(|issue| issue.check == "contract.api-operations-unresolved"));
    }

    #[test]
    fn migration_is_draft_only_and_preserves_identity() {
        let legacy = serde_yaml::from_str::<YamlValue>(
            "spec: rms/contract/v0.1\nname: choose\nversion: 1\nkind: command\nmeaning: Choose.\npreconditions:\n- id: valid\n  statement: Input is valid.\n",
        ).unwrap();
        let migrated = migrate_v01(&legacy).unwrap();
        let migrated = serde_json::to_value(migrated).unwrap();
        assert_eq!(migrated["spec"], CONTRACT_SPEC_V2);
        assert_eq!(migrated["name"], "choose");
        assert_eq!(
            migrated
                .pointer("/x-rms/migration_draft")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            migrated
                .pointer("/semantics/behavior/requires/0/evaluation/kind")
                .and_then(Value::as_str),
            Some("unresolved")
        );
    }

    #[test]
    fn smt_output_is_deterministic_and_solver_neutral() {
        let first = smt_obligations(&behavioral_contract()).unwrap();
        let second = smt_obligations(&behavioral_contract()).unwrap();
        assert_eq!(first[0].script, second[0].script);
        assert!(first
            .iter()
            .any(|obligation| obligation.id.ends_with("case-coverage")));
    }

    #[test]
    fn refinement_encodes_domain_relation_and_additive_witness_obligations() {
        let old = behavioral_contract();
        let mut new = old.clone();
        new["semantics"]["behavior"]["requires"][0]["evaluation"]["expression"]["compare"]
            ["operator"] = json!("gte");
        let obligations = refinement_obligations(&old, &new).unwrap();
        assert_eq!(obligations.len(), 3);
        assert!(obligations.iter().any(|obligation| {
            obligation.id == "input-domain-not-narrowed"
                && obligation.expectation == ExpectedSolverResult::Unsat
        }));
        assert!(obligations.iter().any(|obligation| {
            obligation.id == "new-behavior-refines-old"
                && obligation.expectation == ExpectedSolverResult::Unsat
        }));
        assert!(obligations.iter().any(|obligation| {
            obligation.id == "input-domain-expanded"
                && obligation.expectation == ExpectedSolverResult::Informational
        }));
    }

    #[test]
    fn compatibility_detects_new_frames_and_changed_outcomes() {
        let old = behavioral_contract();
        let mut new = old.clone();
        new["semantics"]["behavior"]["cases"][0]["permits"]["effects"] = json!(["network"]);
        new["semantics"]["behavior"]["cases"][0]["outcome"]["kind"] = json!("rejected");
        assert_eq!(frame_expansion(&old, &new), vec!["effects:network"]);
        assert_eq!(
            changed_outcome_kinds(&old, &new),
            vec!["accepted:accepted->rejected"]
        );
    }

    #[test]
    fn missing_cvc5_is_unresolved_not_success() {
        if Command::new("cvc5").arg("--version").output().is_ok() {
            return;
        }
        let obligation = smt_obligations(&behavioral_contract()).unwrap().remove(0);
        assert_eq!(solve_cvc5(&obligation, 10).result, "unresolved");
    }

    #[test]
    #[ignore = "requires the external cvc5 reference solver"]
    fn cvc5_reference_solver_conformance() {
        let evidence = smt_obligations(&behavioral_contract())
            .unwrap()
            .iter()
            .map(|obligation| solve_cvc5(obligation, 5_000))
            .collect::<Vec<_>>();
        assert!(evidence
            .iter()
            .all(|result| matches!(result.result.as_str(), "sat" | "unsat")));
        assert!(evidence.iter().all(|result| result.version.is_some()));
        assert!(evidence
            .iter()
            .all(|result| result.input_digest.len() == 64));
        assert!(evidence
            .iter()
            .any(|result| result.result == "sat" && result.model_revalidated == Some(true)));
        assert!(evidence.iter().any(|result| result.result == "unsat"));
    }

    #[test]
    fn satisfiable_models_are_rechecked_by_the_rms_evaluator() {
        let contract = behavioral_contract();
        let obligation = smt_obligations(&contract)
            .unwrap()
            .into_iter()
            .find(|obligation| obligation.id.ends_with("case.accepted.satisfiable"))
            .unwrap();
        let assignments = parse_model_assignments(
            "sat\n((|accepted| true) (|amount| 1))\n",
            &obligation.observations,
        )
        .unwrap();
        assert!(property::evaluate_core_expression(
            &obligation.observations,
            &obligation.expression,
            &assignments,
        )
        .unwrap());
    }

    #[test]
    fn contract_v03_totality_invariants_migration_and_observability_are_closed() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap();
        let mut contract_directory_count = 0;
        let mut all_contract_count = 0;
        for entry in WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                !entry.path().components().any(|component| {
                    matches!(
                        component.as_os_str().to_str(),
                        Some(".git" | ".rms" | "target" | "node_modules" | "dist" | "build")
                    )
                })
            })
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| {
                matches!(
                    entry
                        .path()
                        .extension()
                        .and_then(|extension| extension.to_str()),
                    Some("yaml" | "yml")
                )
            })
        {
            let source = std::fs::read_to_string(entry.path()).unwrap();
            if !source.contains("spec: rms/contract/") {
                continue;
            }
            let yaml: YamlValue = serde_yaml::from_str(&source).unwrap();
            let value = serde_json::to_value(yaml).unwrap();
            assert!(
                is_contract_spec(value.get("spec").and_then(Value::as_str)),
                "{} is not a supported behavioral contract",
                entry.path().display()
            );
            let issues = validate(&value, true)
                .into_iter()
                .filter(|issue| issue.blocking)
                .map(|issue| issue.message)
                .collect::<Vec<_>>();
            assert!(
                issues.is_empty(),
                "{} has incomplete behavior: {}",
                entry.path().display(),
                issues.join("; ")
            );
            all_contract_count += 1;
            if entry
                .path()
                .components()
                .any(|component| component.as_os_str() == "contracts")
            {
                contract_directory_count += 1;
            }
        }
        assert_eq!(contract_directory_count, 103);
        assert_eq!(all_contract_count, 105);
    }
}
