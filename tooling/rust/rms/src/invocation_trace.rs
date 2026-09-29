//! Pure invocation evidence validation. No transition envelopes or invented state.
use serde_json::Value;

pub(crate) const SPEC: &str = "rms/invocation-record/v0.1";

pub(crate) struct Binding {
    pub id: String,
    pub contract: String,
    pub digest: String,
}

pub(crate) fn is_invocation(document: &Value) -> bool {
    document.get("spec").and_then(Value::as_str) == Some(SPEC)
        || document
            .get("records")
            .and_then(Value::as_array)
            .is_some_and(|records| {
                records
                    .iter()
                    .any(|record| record.get("spec").and_then(Value::as_str) == Some(SPEC))
            })
}

pub(crate) fn records(document: &Value) -> Result<Vec<&Value>, String> {
    match document.get("spec").and_then(Value::as_str) {
        Some(SPEC) => Ok(vec![document]),
        Some("rms/trace-bundle/v0.1") => {
            if document
                .get("complete")
                .is_some_and(|value| value != &Value::Bool(true))
            {
                return Err("invocation evidence bundle must be complete".into());
            }
            let records = document
                .get("records")
                .and_then(Value::as_array)
                .filter(|records| !records.is_empty())
                .ok_or("invocation evidence bundle must contain nonempty records")?;
            Ok(records.iter().collect())
        }
        _ => Err("expected a canonical invocation record or invocation trace bundle".into()),
    }
}

pub(crate) fn validate(document: &Value, bindings: Option<&[Binding]>) -> Vec<String> {
    let records = match records(document) {
        Ok(records) => records,
        Err(error) => return vec![error],
    };
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../schemas/invocation-record.schema.json"
    ))
    .expect("embedded invocation schema is JSON");
    let validator =
        jsonschema::validator_for(&schema).expect("embedded invocation schema is valid");
    let mut errors = Vec::new();
    for (index, record) in records.iter().enumerate() {
        for error in validator.iter_errors(record) {
            errors.push(format!(
                "invocation {index}: {error} at {}",
                error.instance_path()
            ));
        }
        if let Some(bindings) = bindings {
            let matches = bindings
                .iter()
                .filter(|binding| {
                    record.get("binding").and_then(Value::as_str) == Some(binding.id.as_str())
                })
                .collect::<Vec<_>>();
            let [binding] = matches.as_slice() else {
                errors.push(format!("invocation {index}: binding must identify one invocation binding for this producer command"));
                continue;
            };
            if record.get("contract").and_then(Value::as_str) != Some(binding.contract.as_str()) {
                errors.push(format!(
                    "invocation {index}: contract does not match binding {}",
                    binding.id
                ));
            }
            if record.get("contract_digest").and_then(Value::as_str)
                != Some(binding.digest.as_str())
            {
                errors.push(format!(
                    "invocation {index}: contract digest does not match current contract bytes"
                ));
            }
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn record() -> Value {
        json!({"spec": SPEC, "contract": "codec", "binding": "codec-public",
            "contract_digest": format!("sha256:{}", "a".repeat(64)), "input": {"value": 1}, "output": {"encoded": "1"}})
    }
    fn bindings() -> Vec<Binding> {
        vec![Binding {
            id: "codec-public".into(),
            contract: "codec".into(),
            digest: format!("sha256:{}", "a".repeat(64)),
        }]
    }
    #[test]
    fn accepts_canonical_records_and_bundles_without_transition_fields() {
        for document in [
            record(),
            json!({"spec": "rms/trace-bundle/v0.1", "records": [record(), record()]}),
        ] {
            assert!(validate(&document, Some(&bindings())).is_empty());
        }
    }
    #[test]
    fn rejects_malformed_mixed_unknown_binding_and_stale_contract_evidence() {
        for field in [
            "spec",
            "contract",
            "binding",
            "contract_digest",
            "input",
            "output",
        ] {
            let mut document = record();
            document.as_object_mut().unwrap().remove(field);
            assert!(
                !validate(&document, Some(&bindings())).is_empty(),
                "missing {field}"
            );
        }
        for (field, value) in [
            ("binding", json!("other")),
            ("contract", json!("other")),
            (
                "contract_digest",
                json!(format!("sha256:{}", "b".repeat(64))),
            ),
            ("contract_digest", json!("malformed")),
            ("source", json!("not-object")),
            ("unknown", json!(true)),
        ] {
            let mut document = record();
            document[field] = value;
            assert!(
                !validate(&document, Some(&bindings())).is_empty(),
                "{field}"
            );
        }
        for document in [
            json!({"spec": "rms/trace-bundle/v0.1", "records": []}),
            json!({"spec": "rms/trace-bundle/v0.1", "records": [record(), {"input": {}, "output": {}}]}),
            json!({"spec": "rms/trace-bundle/v0.1", "complete": false, "records": [record()]}),
        ] {
            assert!(!validate(&document, Some(&bindings())).is_empty());
        }
        assert!(!validate(&record(), Some(&[])).is_empty());
    }
}
