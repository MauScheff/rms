use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use serde_json::{Map, Number, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const GENERATOR_SPEC: &str = "rms/schema-generator/v0.1";
pub(crate) const DEFAULT_CASES_PER_INPUT: usize = 64;
const DEFAULT_MAX_DEPTH: usize = 4;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct GeneratedCases {
    pub(crate) spec: &'static str,
    pub(crate) seed: u64,
    pub(crate) cases: Vec<Value>,
}

#[derive(Clone, Copy)]
struct DeterministicRng(u64);

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self(seed ^ 0x9e37_79b9_7f4a_7c15)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn choose(&mut self, length: usize) -> usize {
        if length == 0 {
            0
        } else {
            (self.next() as usize) % length
        }
    }
}

pub(crate) fn generate_cases(schema: &Value, seed: u64, count: usize) -> Result<GeneratedCases> {
    if count == 0 {
        bail!("generated case count must be greater than zero");
    }
    reject_unsupported_keywords(schema, "$")?;
    let validator =
        jsonschema::validator_for(schema).map_err(|error| anyhow!(error.to_string()))?;
    let mut rng = DeterministicRng::new(seed);
    let mut cases = Vec::new();
    let mut seen = BTreeSet::new();
    for index in 0..count {
        let value = generate_value(schema, &mut rng, 0, index)?;
        if !validator.is_valid(&value) {
            bail!("derived generator produced a value outside its source schema");
        }
        let key = serde_json::to_string(&value)?;
        if seen.insert(key) {
            cases.push(value);
        }
    }
    if cases.is_empty() {
        bail!("schema generator produced no values");
    }
    Ok(GeneratedCases {
        spec: GENERATOR_SPEC,
        seed,
        cases,
    })
}

#[allow(dead_code)]
pub(crate) fn shrink_valid(schema: &Value, value: &Value) -> Result<Vec<Value>> {
    reject_unsupported_keywords(schema, "$")?;
    let validator =
        jsonschema::validator_for(schema).map_err(|error| anyhow!(error.to_string()))?;
    let mut candidates = structural_shrinks(value);
    candidates.retain(|candidate| validator.is_valid(candidate));
    candidates.sort_by_key(|candidate| serde_json::to_string(candidate).unwrap_or_default());
    candidates.dedup();
    Ok(candidates)
}

#[allow(dead_code)]
pub(crate) fn shrink_same_failure(
    schema: &Value,
    value: &Value,
    failure_identity: &str,
    observed_failures: &BTreeMap<String, String>,
) -> Result<Value> {
    let initial_key = serde_json::to_string(value)?;
    if observed_failures.get(&initial_key).map(String::as_str) != Some(failure_identity) {
        bail!("initial value does not reproduce failure identity `{failure_identity}`");
    }
    let mut current = value.clone();
    loop {
        let mut accepted = None;
        for candidate in shrink_valid(schema, &current)? {
            let key = serde_json::to_string(&candidate)?;
            if observed_failures.get(&key).map(String::as_str) == Some(failure_identity) {
                accepted = Some(candidate);
                break;
            }
        }
        let Some(next) = accepted else {
            break;
        };
        if next == current {
            break;
        }
        current = next;
    }
    Ok(current)
}

fn reject_unsupported_keywords(schema: &Value, path: &str) -> Result<()> {
    let Some(object) = schema.as_object() else {
        return Ok(());
    };
    const SUPPORTED: &[&str] = &[
        "$schema",
        "$id",
        "title",
        "description",
        "type",
        "const",
        "enum",
        "minimum",
        "maximum",
        "minLength",
        "maxLength",
        "pattern",
        "properties",
        "required",
        "additionalProperties",
        "items",
        "minItems",
        "maxItems",
        "oneOf",
        "anyOf",
        "default",
    ];
    for key in object.keys() {
        if !SUPPORTED.contains(&key.as_str()) {
            bail!("unsupported JSON Schema keyword `{key}` at `{path}`");
        }
    }
    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        for (name, nested) in properties {
            reject_unsupported_keywords(nested, &format!("{path}/properties/{name}"))?;
        }
    }
    if let Some(items) = object.get("items") {
        reject_unsupported_keywords(items, &format!("{path}/items"))?;
    }
    for keyword in ["oneOf", "anyOf"] {
        if let Some(branches) = object.get(keyword).and_then(Value::as_array) {
            for (index, nested) in branches.iter().enumerate() {
                reject_unsupported_keywords(nested, &format!("{path}/{keyword}/{index}"))?;
            }
        }
    }
    Ok(())
}

fn generate_value(
    schema: &Value,
    rng: &mut DeterministicRng,
    depth: usize,
    index: usize,
) -> Result<Value> {
    if depth > DEFAULT_MAX_DEPTH {
        return minimal_value(schema);
    }
    let object = schema
        .as_object()
        .ok_or_else(|| anyhow!("JSON Schema nodes must be objects"))?;
    if let Some(value) = object.get("const") {
        return Ok(value.clone());
    }
    if let Some(values) = object.get("enum").and_then(Value::as_array) {
        if values.is_empty() {
            bail!("enum must contain at least one value");
        }
        return Ok(values[(index + rng.choose(values.len())) % values.len()].clone());
    }
    for keyword in ["oneOf", "anyOf"] {
        if let Some(branches) = object.get(keyword).and_then(Value::as_array) {
            if branches.is_empty() {
                bail!("{keyword} must contain at least one branch");
            }
            let selected = (index + rng.choose(branches.len())) % branches.len();
            return generate_value(&branches[selected], rng, depth + 1, index);
        }
    }
    let kind = object
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_else(|| {
            if object.contains_key("properties") {
                "object"
            } else {
                "null"
            }
        });
    match kind {
        "null" => Ok(Value::Null),
        "boolean" => Ok(Value::Bool((rng.next() + index as u64).is_multiple_of(2))),
        "integer" => generate_integer(object, rng, index).map(Value::Number),
        "number" => generate_number(object, rng, index),
        "string" => generate_string(object, rng, index).map(Value::String),
        "array" => generate_array(object, rng, depth, index),
        "object" => generate_object(object, rng, depth, index),
        other => bail!("unsupported JSON Schema type `{other}`"),
    }
}

fn minimal_value(schema: &Value) -> Result<Value> {
    let mut rng = DeterministicRng::new(0);
    generate_value_without_recursion(schema, &mut rng)
}

fn generate_value_without_recursion(schema: &Value, rng: &mut DeterministicRng) -> Result<Value> {
    let object = schema
        .as_object()
        .ok_or_else(|| anyhow!("JSON Schema nodes must be objects"))?;
    if let Some(value) = object.get("const") {
        return Ok(value.clone());
    }
    if let Some(value) = object
        .get("enum")
        .and_then(Value::as_array)
        .and_then(|values| values.first())
    {
        return Ok(value.clone());
    }
    match object.get("type").and_then(Value::as_str).unwrap_or("null") {
        "boolean" => Ok(Value::Bool(false)),
        "integer" => generate_integer(object, rng, 0).map(Value::Number),
        "number" => generate_number(object, rng, 0),
        "string" => generate_string(object, rng, 0).map(Value::String),
        "array" => Ok(Value::Array(Vec::new())),
        "object" => Ok(Value::Object(Map::new())),
        _ => Ok(Value::Null),
    }
}

fn generate_integer(
    object: &Map<String, Value>,
    rng: &mut DeterministicRng,
    index: usize,
) -> Result<Number> {
    let min = object
        .get("minimum")
        .and_then(Value::as_i64)
        .unwrap_or(-1024);
    let max = object
        .get("maximum")
        .and_then(Value::as_i64)
        .unwrap_or(1024);
    if min > max {
        bail!("integer minimum exceeds maximum");
    }
    let choices = [
        min,
        max,
        0_i64.clamp(min, max),
        min.saturating_add(1).min(max),
    ];
    Ok(Number::from(
        choices[(index + rng.choose(choices.len())) % choices.len()],
    ))
}

fn generate_number(
    object: &Map<String, Value>,
    rng: &mut DeterministicRng,
    index: usize,
) -> Result<Value> {
    let min = object
        .get("minimum")
        .and_then(Value::as_f64)
        .unwrap_or(-1024.0);
    let max = object
        .get("maximum")
        .and_then(Value::as_f64)
        .unwrap_or(1024.0);
    if !min.is_finite() || !max.is_finite() || min > max {
        bail!("number bounds must be finite and ordered");
    }
    let choices = [min, max, 0.0_f64.clamp(min, max), (min + max) / 2.0];
    Number::from_f64(choices[(index + rng.choose(choices.len())) % choices.len()])
        .map(Value::Number)
        .ok_or_else(|| anyhow!("generated number is not representable as JSON"))
}

fn generate_string(
    object: &Map<String, Value>,
    rng: &mut DeterministicRng,
    index: usize,
) -> Result<String> {
    let min = object.get("minLength").and_then(Value::as_u64).unwrap_or(0) as usize;
    let max = object
        .get("maxLength")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(min.saturating_add(32));
    if min > max {
        bail!("string minLength exceeds maxLength");
    }
    let target = match index % 4 {
        0 => min,
        1 => max.min(min.saturating_add(1)),
        2 => max,
        _ => min.saturating_add(rng.choose(max.saturating_sub(min).saturating_add(1))),
    };
    let pattern = object.get("pattern").and_then(Value::as_str);
    let alphabet = match pattern {
        None => "a0_-",
        Some("^[A-Za-z]+$") => "aZ",
        Some("^[A-Za-z0-9_-]+$") => "aZ0_-",
        Some("^[a-z]+$") => "az",
        Some("^[0-9]+$") => "09",
        Some(other) => bail!("unsupported JSON Schema string pattern `{other}`"),
    };
    if target == 0 && pattern.is_some() {
        bail!("a non-empty pattern requires minLength of at least one");
    }
    let symbols = alphabet.chars().collect::<Vec<_>>();
    Ok((0..target)
        .map(|offset| symbols[(rng.choose(symbols.len()) + offset) % symbols.len()])
        .collect())
}

fn generate_array(
    object: &Map<String, Value>,
    rng: &mut DeterministicRng,
    depth: usize,
    index: usize,
) -> Result<Value> {
    let min = object.get("minItems").and_then(Value::as_u64).unwrap_or(0) as usize;
    let max = object
        .get("maxItems")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(min.saturating_add(4));
    if min > max {
        bail!("array minItems exceeds maxItems");
    }
    let length = match index % 3 {
        0 => min,
        1 => max.min(min.saturating_add(1)),
        _ => max,
    };
    let unconstrained = Value::Object(Map::new());
    let item_schema = object.get("items").unwrap_or(&unconstrained);
    let values = (0..length)
        .map(|offset| generate_value(item_schema, rng, depth + 1, index + offset))
        .collect::<Result<Vec<_>>>()?;
    Ok(Value::Array(values))
}

fn generate_object(
    object: &Map<String, Value>,
    rng: &mut DeterministicRng,
    depth: usize,
    index: usize,
) -> Result<Value> {
    let properties = object
        .get("properties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let required = object
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let mut result = Map::new();
    for (offset, (name, nested)) in properties.iter().enumerate() {
        if required.contains(name.as_str()) || (rng.next() + index as u64).is_multiple_of(2) {
            result.insert(
                name.clone(),
                generate_value(nested, rng, depth + 1, index + offset)?,
            );
        }
    }
    Ok(Value::Object(result))
}

pub(crate) fn structural_shrinks(value: &Value) -> Vec<Value> {
    let mut candidates = Vec::new();
    match value {
        Value::Object(object) => {
            for key in object.keys() {
                let mut candidate = object.clone();
                candidate.remove(key);
                candidates.push(Value::Object(candidate));
            }
            for (key, nested) in object {
                for smaller in structural_shrinks(nested) {
                    let mut candidate = object.clone();
                    candidate.insert(key.clone(), smaller);
                    candidates.push(Value::Object(candidate));
                }
            }
        }
        Value::Array(items) if !items.is_empty() => {
            candidates.push(Value::Array(Vec::new()));
            candidates.push(Value::Array(items[..items.len() / 2].to_vec()));
        }
        Value::String(text) if !text.is_empty() => {
            candidates.push(Value::String(String::new()));
            candidates.push(Value::String(
                text.chars().take(text.chars().count() / 2).collect(),
            ));
        }
        Value::Number(number) if number.as_i64() != Some(0) => candidates.push(Value::from(0)),
        Value::Bool(true) => candidates.push(Value::Bool(false)),
        _ => {}
    }
    candidates.sort_by_key(|candidate| serde_json::to_string(candidate).unwrap_or_default());
    candidates.dedup();
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn generation_is_seeded_valid_and_stable() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "minLength": 1, "maxLength": 4},
                "count": {"type": "integer", "minimum": 1, "maximum": 3},
                "enabled": {"type": "boolean"}
            },
            "required": ["name", "count"],
            "additionalProperties": false
        });
        let left = generate_cases(&schema, 7, 64).unwrap();
        let right = generate_cases(&schema, 7, 64).unwrap();
        assert_eq!(left, right);
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(left.cases.iter().all(|value| validator.is_valid(value)));
        assert!(left.cases.len() > 1);
    }

    #[test]
    fn valid_shrinking_preserves_required_fields() {
        let schema = json!({
            "type": "object",
            "properties": {"name": {"type": "string", "minLength": 1}},
            "required": ["name"],
            "additionalProperties": false
        });
        let shrinks = shrink_valid(&schema, &json!({"name": "abcd"})).unwrap();
        assert!(shrinks.iter().all(|value| value.get("name").is_some()));
    }

    #[test]
    fn semantic_shrinking_preserves_failure_identity() {
        let schema = json!({
            "type": "object",
            "properties": {
                "value": {"type": "integer", "minimum": 0, "maximum": 100},
                "noise": {"type": "string", "maxLength": 20}
            },
            "required": ["value"],
            "additionalProperties": false
        });
        let minimized = shrink_same_failure(
            &schema,
            &json!({"value": 100, "noise": "irrelevant"}),
            "overflow",
            &BTreeMap::from([
                (
                    serde_json::to_string(&json!({"value": 100, "noise": "irrelevant"})).unwrap(),
                    "overflow".to_string(),
                ),
                (
                    serde_json::to_string(&json!({"value": 100})).unwrap(),
                    "overflow".to_string(),
                ),
            ]),
        )
        .unwrap();
        assert_eq!(minimized.get("noise"), None);
        assert!(minimized["value"].as_i64().unwrap() >= 50);
    }

    #[test]
    fn unsupported_keywords_are_explicit() {
        let error =
            generate_cases(&json!({"type": "string", "format": "email"}), 0, 1).unwrap_err();
        assert!(error
            .to_string()
            .contains("unsupported JSON Schema keyword `format`"));
    }
}
