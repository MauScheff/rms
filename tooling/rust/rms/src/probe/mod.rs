use super::{
    apply_probe_trace_conformance, execute_proof_command, get_path, get_str, load_manifest,
    load_probe_binding, load_yaml_value, sha256_bytes, trace_has_errors,
    validate_probe_description, validate_probe_trace_shape, ProbeBinding,
};
use anyhow::{anyhow, bail, Context, Result};
use jsonschema::validator_for;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) const ASSEMBLY_SPEC: &str = "rms/probe-assembly/v0.1";
const TRACE_SPEC: &str = "rms/probe-system-trace/v0.1";
const COUNTEREXAMPLE_SPEC: &str = "rms/probe-counterexample/v0.1";
const DEFAULT_MAX_STEPS: usize = 30;
const DEFAULT_MAX_SCHEDULES: usize = 100;
const DEFAULT_MAX_STATES: usize = 10_000;

mod time_quantity {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_json::Value;

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serde_json::json!({"value": value, "unit": "ns"}).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        parse(&value).map_err(serde::de::Error::custom)
    }

    pub fn parse(value: &Value) -> Result<u64, String> {
        if let Some(value) = value.as_u64() {
            return value
                .checked_mul(1_000_000)
                .ok_or_else(|| "legacy millisecond time quantity overflow".to_string());
        }
        let amount = value
            .get("value")
            .and_then(Value::as_u64)
            .ok_or_else(|| "time quantity requires non-negative integer `value`".to_string())?;
        let unit = value
            .get("unit")
            .and_then(Value::as_str)
            .ok_or_else(|| "time quantity requires `unit`".to_string())?;
        let factor = match unit {
            "ns" => 1,
            "us" => 1_000,
            "ms" => 1_000_000,
            "s" => 1_000_000_000,
            "min" => 60_000_000_000,
            "h" => 3_600_000_000_000,
            other => return Err(format!("unsupported probe time unit `{other}`")),
        };
        amount
            .checked_mul(factor)
            .ok_or_else(|| "time quantity overflow".to_string())
    }
}

mod optional_time_quantity {
    use super::time_quantity;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_json::Value;

    pub fn serialize<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serde_json::json!({"value": value, "unit": "ns"}).serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Option::<Value>::deserialize(deserializer)?;
        value
            .as_ref()
            .map(time_quantity::parse)
            .transpose()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug)]
pub(super) struct AssemblyCliOptions {
    file: PathBuf,
    describe: bool,
    explore: bool,
    replay: bool,
    max_steps: Option<usize>,
    max_schedules: Option<usize>,
    max_states: Option<usize>,
    out: Option<PathBuf>,
    json: bool,
    timeout_seconds: u64,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct PropertyExploration {
    traces: Vec<Value>,
    exhausted: bool,
    assembly_digest: String,
    coverage: Value,
    bounds: Value,
}

impl PropertyExploration {
    pub(super) fn traces(&self) -> &[Value] {
        &self.traces
    }

    pub(super) fn exhausted(&self) -> bool {
        self.exhausted
    }

    pub(super) fn assembly_digest(&self) -> &str {
        &self.assembly_digest
    }

    pub(super) fn coverage(&self) -> &Value {
        &self.coverage
    }

    pub(super) fn bounds(&self) -> &Value {
        &self.bounds
    }
}

impl AssemblyCliOptions {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        file: PathBuf,
        describe: bool,
        explore: bool,
        replay: bool,
        max_steps: Option<usize>,
        max_schedules: Option<usize>,
        max_states: Option<usize>,
        out: Option<PathBuf>,
        json: bool,
        timeout_seconds: u64,
    ) -> Self {
        Self {
            file,
            describe,
            explore,
            replay,
            max_steps,
            max_schedules,
            max_states,
            out,
            json,
            timeout_seconds,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProbeAssembly {
    spec: String,
    instances: Vec<InstanceSpec>,
    #[serde(default)]
    stimuli: Vec<Stimulus>,
    #[serde(default)]
    routing: Vec<RoutingSelection>,
    #[serde(default)]
    substitutes: Vec<SubstituteSpec>,
    #[serde(default)]
    checks: Vec<CheckSpec>,
    #[serde(default)]
    exploration: ExplorationSpec,
    #[serde(default)]
    faults: Vec<FaultSpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InstanceSpec {
    id: String,
    implementation: String,
    #[serde(default = "initial_start")]
    start: Value,
}

fn initial_start() -> Value {
    Value::String("initial".to_string())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Stimulus {
    #[serde(default)]
    id: Option<String>,
    target: String,
    #[serde(default, with = "time_quantity")]
    at: u64,
    input: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RoutingSelection {
    #[serde(default)]
    consumer: Option<String>,
    binding: String,
    provider: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SubstituteSpec {
    id: String,
    source: String,
    output: NamedKind,
    outcomes: Vec<SubstituteOutcome>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SubstituteOutcome {
    #[serde(default)]
    id: Option<String>,
    target: String,
    #[serde(default, with = "time_quantity")]
    after: u64,
    input: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NamedKind {
    kind: String,
    name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CheckSpec {
    id: String,
    when: CheckWhen,
    #[serde(default)]
    within_steps: Option<usize>,
    #[serde(default, with = "optional_time_quantity")]
    within_time: Option<u64>,
    assert: CheckAssertion,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum CheckWhen {
    Always,
    Quiescent,
    Within,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum CheckAssertion {
    State {
        instance: String,
        equals: Value,
    },
    QueueEmpty,
    MessageCount {
        name: String,
        #[serde(default)]
        equals: Option<usize>,
        #[serde(default)]
        min: Option<usize>,
        #[serde(default)]
        max: Option<usize>,
    },
    Oracle {
        command: String,
        runner: String,
    },
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExplorationSpec {
    #[serde(default)]
    max_steps: Option<usize>,
    #[serde(default)]
    max_schedules: Option<usize>,
    #[serde(default)]
    max_states: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FaultSpec {
    route: String,
    kind: FaultKind,
    budget: usize,
    #[serde(default, with = "optional_time_quantity")]
    delay: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "kebab-case")]
enum FaultKind {
    Delay,
    Duplicate,
    Drop,
    Timeout,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbeBridge {
    request: BridgeLeg,
    #[serde(default)]
    outcomes: Vec<BridgeLeg>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BridgeLeg {
    from: NamedKind,
    to: NamedKind,
    #[serde(default)]
    mapper: Option<String>,
}

#[derive(Clone, Debug)]
struct RuntimeInstance {
    id: String,
    implementation_path: PathBuf,
    binding: ProbeBinding,
    manifest: Value,
    module: String,
    start: Value,
    digest: String,
}

#[derive(Clone, Debug, Serialize)]
struct ResolvedRoute {
    id: String,
    binding: String,
    source: String,
    target: String,
    from: NamedKind,
    to: NamedKind,
    mapper: Option<String>,
    mapper_instance: String,
}

#[derive(Clone, Debug)]
struct ProtocolRoute {
    id: String,
    contract: String,
    source: String,
    target: String,
    send_case: String,
    receive_input: String,
    message: String,
    mapper: Option<String>,
    mapper_instance: String,
    model: ProtocolModel,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct ProtocolModel {
    messages: Vec<ProtocolSemanticMessage>,
    states: Vec<String>,
    initial_state: String,
    #[serde(default)]
    terminal_states: Vec<String>,
    transitions: Vec<ProtocolStateTransition>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct ProtocolSemanticMessage {
    id: String,
    kind: String,
    from: String,
    to: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct ProtocolStateTransition {
    from: String,
    on: String,
    to: String,
}

#[derive(Clone, Debug, Deserialize)]
struct ManifestDependencyBinding {
    id: String,
    #[serde(default)]
    provider_module: Option<String>,
    #[serde(default)]
    probe_bridge: Option<ProbeBridge>,
}

#[derive(Clone, Debug, Deserialize)]
struct ManifestProtocolBinding {
    name: String,
    contract: String,
    participant: String,
    mappings: Vec<ManifestProtocolMapping>,
}

#[derive(Clone, Debug, Deserialize)]
struct ManifestProtocolMapping {
    machine_case: String,
    message: String,
    direction: String,
    #[serde(default)]
    mapper: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct ManifestProbeMapper {
    id: String,
    command: String,
    runner: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Envelope {
    id: String,
    idempotency_key: String,
    route: String,
    target: String,
    #[serde(with = "time_quantity")]
    at: u64,
    input: Value,
    source: Option<String>,
    correlation_id: String,
    causation_id: Option<String>,
    attempt: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TimelineEntry {
    step: usize,
    #[serde(with = "time_quantity")]
    time: u64,
    action: String,
    envelope: String,
    idempotency_key: String,
    route: String,
    source: Option<String>,
    target: String,
    input: Value,
    correlation_id: String,
    causation_id: Option<String>,
    attempt: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_before: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_after: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transition_case: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_function: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    outputs: Vec<OutputValue>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct OutputValue {
    kind: String,
    name: String,
    value: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Decision {
    envelope: String,
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    choice: Option<String>,
}

#[derive(Clone, Debug)]
struct World {
    states: BTreeMap<String, Value>,
    queue: Vec<Envelope>,
    time: u64,
    step: usize,
    timeline: Vec<TimelineEntry>,
    decisions: Vec<Decision>,
    message_counts: BTreeMap<String, usize>,
    fault_remaining: BTreeMap<String, usize>,
    satisfied_within: BTreeSet<String>,
    protocol_states: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProbeFailure {
    check: String,
    step: usize,
    #[serde(with = "time_quantity")]
    time: u64,
    message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Coverage {
    states: usize,
    schedules: usize,
    transitions: usize,
    transition_cases: BTreeSet<String>,
    routes: BTreeSet<String>,
    faults: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Bounds {
    max_steps: usize,
    max_schedules: usize,
    max_states: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct InstanceResult {
    id: String,
    module: String,
    implementation: String,
    digest: String,
    state: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SystemTrace {
    spec: String,
    result: String,
    mode: String,
    exhausted: bool,
    assembly_digest: String,
    source_revision: Option<String>,
    instances: Vec<InstanceResult>,
    timeline: Vec<TimelineEntry>,
    protocols: BTreeMap<String, String>,
    checks: Vec<String>,
    failure: Option<ProbeFailure>,
    coverage: Coverage,
    bounds: Bounds,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Counterexample {
    spec: String,
    assembly: Value,
    assembly_digest: String,
    source_revision: Option<String>,
    implementation_digests: BTreeMap<String, String>,
    decisions: Vec<Decision>,
    failure: ProbeFailure,
    trace: SystemTrace,
}

struct Engine {
    assembly: ProbeAssembly,
    assembly_value: Value,
    assembly_digest: String,
    instances: BTreeMap<String, RuntimeInstance>,
    routes: Vec<ResolvedRoute>,
    protocol_routes: Vec<ProtocolRoute>,
    base_dir: PathBuf,
    bounds: Bounds,
    timeout_seconds: u64,
    cache: BTreeMap<String, Value>,
}

#[derive(Clone)]
struct StepChoice {
    envelope_index: usize,
    action: StepAction,
    substitute_choices: BTreeMap<String, usize>,
}

#[derive(Clone)]
enum StepAction {
    Deliver,
    Fault(FaultKind),
}

pub(super) fn file_spec(path: &Path) -> Result<Option<String>> {
    if path == Path::new("-") {
        return Ok(None);
    }
    let value = load_json_or_yaml(path)?;
    Ok(value
        .get("spec")
        .and_then(Value::as_str)
        .map(str::to_string))
}

pub(super) fn verify_evidence(path: &Path, timeout_seconds: u64) -> Result<Option<String>> {
    let Some(spec) = file_spec(path)? else {
        return Ok(None);
    };
    match spec.as_str() {
        ASSEMBLY_SPEC => {
            let (assembly_value, base_dir) = load_input(path)?;
            validate_schema(
                &assembly_value,
                include_str!("../../../../../schemas/probe-assembly.schema.json"),
                "probe assembly",
            )?;
            let assembly: ProbeAssembly = serde_json::from_value(assembly_value.clone())
                .context("invalid canonical probe assembly")?;
            let mut engine = Engine::new(
                assembly,
                assembly_value,
                base_dir,
                None,
                None,
                None,
                timeout_seconds,
            )?;
            let (trace, _) = engine.explore_internal()?;
            if trace.result != "pass" {
                bail!(
                    "canonical probe assembly `{}` returned `{}`",
                    path.display(),
                    trace.result
                );
            }
            Ok(Some(format!(
                "probe assembly exhausted {} global states and {} completed schedules",
                trace.coverage.states, trace.coverage.schedules
            )))
        }
        COUNTEREXAMPLE_SPEC => {
            let options = AssemblyCliOptions {
                file: path.to_path_buf(),
                describe: false,
                explore: false,
                replay: true,
                max_steps: None,
                max_schedules: None,
                max_states: None,
                out: None,
                json: true,
                timeout_seconds,
            };
            let (report, reproduced) = replay_inner(&options)?;
            if reproduced {
                bail!(
                    "canonical probe counterexample `{}` still reproduces",
                    path.display()
                );
            }
            Ok(Some(format!(
                "probe counterexample replayed as {}",
                report
                    .get("result")
                    .and_then(Value::as_str)
                    .unwrap_or("invalid")
            )))
        }
        TRACE_SPEC => {
            let (value, _) = load_input(path)?;
            validate_schema(
                &value,
                include_str!("../../../../../schemas/probe-system-trace.schema.json"),
                "probe system trace",
            )?;
            let result = value
                .get("result")
                .and_then(Value::as_str)
                .unwrap_or("invalid");
            let mode = value
                .get("mode")
                .and_then(Value::as_str)
                .unwrap_or("invalid");
            let exhausted = value
                .get("exhausted")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if result != "pass" || mode != "exploration" || !exhausted {
                bail!(
                    "canonical probe trace `{}` records result `{result}`, mode `{mode}`, exhausted `{exhausted}`; verification requires a passing exhausted exploration",
                    path.display(),
                );
            }
            Ok(Some("passing probe system trace validated".to_string()))
        }
        _ => Ok(None),
    }
}

pub(super) fn run(options: AssemblyCliOptions) -> Result<i32> {
    if options.replay {
        return replay(options);
    }
    let (assembly_value, base_dir) = load_input(&options.file)?;
    validate_schema(
        &assembly_value,
        include_str!("../../../../../schemas/probe-assembly.schema.json"),
        "probe assembly",
    )?;
    let assembly: ProbeAssembly =
        serde_json::from_value(assembly_value.clone()).context("invalid probe assembly")?;
    let mut engine = Engine::new(
        assembly,
        assembly_value,
        base_dir,
        options.max_steps,
        options.max_schedules,
        options.max_states,
        options.timeout_seconds,
    )?;

    if options.describe {
        let description = engine.description();
        print_value(&description, options.json)?;
        return Ok(0);
    }

    let (trace, counterexample) = if options.explore {
        engine.explore()?
    } else {
        engine.deterministic()?
    };
    if let Some(path) = options.out.as_deref() {
        let value = counterexample
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?
            .unwrap_or(serde_json::to_value(&trace)?);
        write_artifact(path, &value)?;
    }
    if options.json {
        println!("{}", serde_json::to_string_pretty(&trace)?);
    } else {
        print_trace(&trace);
    }
    match trace.result.as_str() {
        "pass" => Ok(0),
        "fail" | "inconclusive" => Ok(1),
        result => bail!("probe engine produced unknown result `{result}`"),
    }
}

pub(super) fn explore_property_traces(
    file: &Path,
    max_steps: Option<usize>,
    max_schedules: Option<usize>,
    max_states: Option<usize>,
    timeout_seconds: u64,
) -> Result<PropertyExploration> {
    let (assembly_value, base_dir) = load_input(file)?;
    validate_schema(
        &assembly_value,
        include_str!("../../../../../schemas/probe-assembly.schema.json"),
        "probe assembly",
    )?;
    let assembly: ProbeAssembly =
        serde_json::from_value(assembly_value.clone()).context("invalid probe assembly")?;
    let mut engine = Engine::new(
        assembly,
        assembly_value,
        base_dir,
        max_steps,
        max_schedules,
        max_states,
        timeout_seconds,
    )?;
    engine.collect_property_traces()
}

fn replay(options: AssemblyCliOptions) -> Result<i32> {
    match replay_inner(&options) {
        Ok((report, reproduced)) => {
            print_value(&report, options.json)?;
            Ok(i32::from(reproduced))
        }
        Err(error) => {
            let report = json!({
                "spec": "rms/probe-replay-report/v0.1",
                "result": "invalid",
                "error": format!("{error:#}")
            });
            print_value(&report, options.json)?;
            Ok(2)
        }
    }
}

fn replay_inner(options: &AssemblyCliOptions) -> Result<(Value, bool)> {
    let (value, base_dir) = load_input(&options.file)?;
    validate_schema(
        &value,
        include_str!("../../../../../schemas/probe-counterexample.schema.json"),
        "probe counterexample",
    )?;
    let counterexample: Counterexample =
        serde_json::from_value(value).context("invalid probe counterexample")?;
    let assembly: ProbeAssembly = serde_json::from_value(counterexample.assembly.clone())
        .context("counterexample assembly is invalid")?;
    let mut engine = Engine::new(
        assembly,
        counterexample.assembly.clone(),
        base_dir,
        options.max_steps,
        options.max_schedules,
        options.max_states,
        options.timeout_seconds,
    )?;
    let (trace, _) = engine.run_forced(&counterexample.decisions)?;
    let reproduced = trace.failure.as_ref().is_some_and(|failure| {
        failure.check == counterexample.failure.check
            && failure.message == counterexample.failure.message
    });
    let status = if reproduced { "reproduced" } else { "resolved" };
    let current_digests = engine
        .instances
        .values()
        .map(|instance| (instance.id.clone(), instance.digest.clone()))
        .collect::<BTreeMap<_, _>>();
    let drift = current_digests != counterexample.implementation_digests;
    let report = json!({
        "spec": "rms/probe-replay-report/v0.1",
        "result": status,
        "source_drift": drift,
        "recorded_failure": counterexample.failure,
        "observed_failure": trace.failure,
        "trace": trace
    });
    Ok((report, reproduced))
}

impl Engine {
    fn new(
        assembly: ProbeAssembly,
        assembly_value: Value,
        base_dir: PathBuf,
        max_steps: Option<usize>,
        max_schedules: Option<usize>,
        max_states: Option<usize>,
        timeout_seconds: u64,
    ) -> Result<Self> {
        if assembly.spec != ASSEMBLY_SPEC {
            bail!("probe assembly must declare `spec: {ASSEMBLY_SPEC}`");
        }
        validate_unique_ids(&assembly)?;
        let mut instances = BTreeMap::new();
        for spec in &assembly.instances {
            let path = resolve_implementation(&base_dir, &spec.implementation)?;
            let binding = load_probe_binding(&path)?;
            let manifest_yaml = load_manifest(&path)?.value;
            let manifest = serde_json::to_value(&manifest_yaml)?;
            let module = manifest
                .get("module")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>")
                .to_string();
            let source =
                fs::read(&path).with_context(|| format!("failed to read `{}`", path.display()))?;
            let digest = sha256_bytes(&source);
            instances.insert(
                spec.id.clone(),
                RuntimeInstance {
                    id: spec.id.clone(),
                    implementation_path: path,
                    binding,
                    manifest,
                    module,
                    start: spec.start.clone(),
                    digest,
                },
            );
        }
        let bounds = Bounds {
            max_steps: max_steps
                .or(assembly.exploration.max_steps)
                .unwrap_or(DEFAULT_MAX_STEPS),
            max_schedules: max_schedules
                .or(assembly.exploration.max_schedules)
                .unwrap_or(DEFAULT_MAX_SCHEDULES),
            max_states: max_states
                .or(assembly.exploration.max_states)
                .unwrap_or(DEFAULT_MAX_STATES),
        };
        if bounds.max_steps == 0 || bounds.max_schedules == 0 || bounds.max_states == 0 {
            bail!("probe exploration bounds must be positive");
        }
        let assembly_digest = sha256_bytes(&serde_json::to_vec(&assembly_value)?);
        let mut engine = Self {
            assembly,
            assembly_value,
            assembly_digest,
            instances,
            routes: Vec::new(),
            protocol_routes: Vec::new(),
            base_dir,
            bounds,
            timeout_seconds,
            cache: BTreeMap::new(),
        };
        engine.routes = engine.resolve_dependency_routes()?;
        engine.protocol_routes = engine.resolve_protocol_routes()?;
        engine.validate_assembly()?;
        Ok(engine)
    }

    fn description(&self) -> Value {
        json!({
            "spec": "rms/probe-assembly-description/v0.1",
            "result": "ready",
            "assembly_digest": self.assembly_digest,
            "instances": self.instances.values().map(|instance| json!({
                "id": instance.id,
                "module": instance.module,
                "implementation": instance.implementation_path,
                "machine": instance.binding.machine,
                "probe_protocol": get_json_str(&instance.manifest, &["architecture", "probe", "protocol"]).unwrap_or("rms/machine-probe/v0.1")
            })).collect::<Vec<_>>(),
            "routes": self.routes,
            "protocol_routes": self.protocol_routes.iter().map(|route| json!({
                "id": route.id,
                "contract": route.contract,
                "source": route.source,
                "target": route.target,
                "message": route.message,
                "send_case": route.send_case,
                "receive_input": route.receive_input,
                "mapper": route.mapper,
                "mapper_instance": route.mapper_instance
            })).collect::<Vec<_>>(),
            "substitutes": self.assembly.substitutes,
            "faults": self.assembly.faults,
            "checks": self.assembly.checks,
            "bounds": self.bounds
        })
    }

    fn validate_assembly(&self) -> Result<()> {
        for stimulus in &self.assembly.stimuli {
            let instance = self.instances.get(&stimulus.target).ok_or_else(|| {
                anyhow!("stimulus targets unknown instance `{}`", stimulus.target)
            })?;
            validate_normalized_input(&stimulus.input)?;
            let kind = stimulus.input["kind"].as_str().unwrap_or("");
            let name = stimulus.input["name"].as_str().unwrap_or("");
            if !machine_declares_named_kind(&instance.manifest, kind, name, false) {
                bail!(
                    "stimulus `{name}` is not a declared {kind} input of instance `{}`",
                    stimulus.target
                );
            }
            let public = json_array(
                &instance.manifest,
                &["architecture", "public_behavior_bindings"],
            )
            .iter()
            .flat_map(|binding| json_array(binding, &["machine_inputs"]))
            .filter_map(Value::as_str)
            .any(|input| input == name);
            if !public {
                bail!(
                    "stimulus `{name}` is not a public machine input of instance `{}`",
                    stimulus.target
                );
            }
        }
        for route in &self.assembly.routing {
            if !self.instances.contains_key(&route.provider) {
                bail!(
                    "routing selection `{}` names unknown provider instance `{}`",
                    route.binding,
                    route.provider
                );
            }
            if !self.routes.iter().any(|resolved| {
                resolved.binding == route.binding
                    && resolved.target == route.provider
                    && route
                        .consumer
                        .as_deref()
                        .is_none_or(|consumer| resolved.source == consumer)
            }) {
                bail!(
                    "routing selection `{}` does not resolve a canonical probe bridge to `{}`",
                    route.binding,
                    route.provider
                );
            }
        }
        for route in &self.routes {
            self.validate_dependency_route(route)?;
        }
        for route in &self.routes {
            if let Some(mapper) = route.mapper.as_deref() {
                self.validate_mapper(&route.mapper_instance, mapper)?;
            }
        }
        for route in &self.protocol_routes {
            if let Some(mapper) = route.mapper.as_deref() {
                self.validate_mapper(&route.mapper_instance, mapper)?;
            }
        }
        for substitute in &self.assembly.substitutes {
            let source = self
                .instances
                .get(&substitute.source)
                .ok_or_else(|| anyhow!("substitute `{}` names unknown source", substitute.id))?;
            if !matches!(substitute.output.kind.as_str(), "command" | "effect")
                || !machine_declares_named_kind(
                    &source.manifest,
                    &substitute.output.kind,
                    &substitute.output.name,
                    true,
                )
            {
                bail!(
                    "substitute `{}` does not select a declared command/effect output of `{}`",
                    substitute.id,
                    substitute.source
                );
            }
            for outcome in &substitute.outcomes {
                validate_normalized_input(&outcome.input)?;
                let target = self.instances.get(&outcome.target).ok_or_else(|| {
                    anyhow!("substitute `{}` names unknown target", substitute.id)
                })?;
                let kind = outcome.input["kind"].as_str().unwrap_or("");
                let name = outcome.input["name"].as_str().unwrap_or("");
                if !matches!(kind, "observed-event" | "effect-result")
                    || !machine_declares_named_kind(&target.manifest, kind, name, false)
                {
                    bail!(
                        "substitute `{}` outcome `{name}` is not a declared observed-event/effect-result input of `{}`",
                        substitute.id,
                        outcome.target
                    );
                }
            }
        }
        for check in &self.assembly.checks {
            if let CheckAssertion::State { instance, .. } = &check.assert {
                if !self.instances.contains_key(instance) {
                    bail!("check `{}` names unknown instance `{instance}`", check.id);
                }
            }
        }
        for fault in &self.assembly.faults {
            let known = self.routes.iter().any(|route| route.id == fault.route)
                || self
                    .protocol_routes
                    .iter()
                    .any(|route| route.id == fault.route)
                || self
                    .assembly
                    .substitutes
                    .iter()
                    .any(|substitute| substitute.id == fault.route);
            if !known {
                bail!("fault names unknown route `{}`", fault.route);
            }
            if fault.kind == FaultKind::Delay && fault.delay.unwrap_or(0) == 0 {
                bail!("delay fault on `{}` requires a positive delay", fault.route);
            }
        }
        self.validate_output_closure()
    }

    fn validate_dependency_route(&self, route: &ResolvedRoute) -> Result<()> {
        let source = self
            .instances
            .get(&route.source)
            .ok_or_else(|| anyhow!("route `{}` has unknown source", route.id))?;
        let target = self
            .instances
            .get(&route.target)
            .ok_or_else(|| anyhow!("route `{}` has unknown target", route.id))?;
        if !machine_declares_named_kind(&source.manifest, &route.from.kind, &route.from.name, true)
        {
            bail!(
                "canonical bridge `{}` maps undeclared output `{}:{}` from `{}`",
                route.binding,
                route.from.kind,
                route.from.name,
                route.source
            );
        }
        if !machine_declares_named_kind(&target.manifest, &route.to.kind, &route.to.name, false) {
            bail!(
                "canonical bridge `{}` maps to undeclared input `{}:{}` on `{}`",
                route.binding,
                route.to.kind,
                route.to.name,
                route.target
            );
        }
        let request =
            matches!(route.from.kind.as_str(), "command" | "effect") && route.to.kind == "command";
        let outcome = matches!(route.from.kind.as_str(), "event" | "reply" | "rejection")
            && matches!(route.to.kind.as_str(), "observed-event" | "effect-result");
        if !request && !outcome {
            bail!(
                "canonical bridge `{}` has incompatible leg `{}:{} -> {}:{}`",
                route.binding,
                route.from.kind,
                route.from.name,
                route.to.kind,
                route.to.name
            );
        }
        if request {
            let public = json_array(
                &target.manifest,
                &["architecture", "public_behavior_bindings"],
            )
            .iter()
            .flat_map(|binding| json_array(binding, &["machine_inputs"]))
            .filter_map(Value::as_str)
            .any(|input| input == route.to.name);
            if !public {
                bail!(
                    "canonical bridge `{}` targets private provider input `{}`",
                    route.binding,
                    route.to.name
                );
            }
        }
        Ok(())
    }

    fn validate_output_closure(&self) -> Result<()> {
        for instance in self.instances.values() {
            for (field, kind) in [("commands", "command"), ("effects", "effect")] {
                for transition in json_array(
                    &instance.manifest,
                    &["architecture", "machine", "transitions"],
                ) {
                    let transition_case =
                        get_json_str(transition, &["case"]).unwrap_or("<missing>");
                    for output in json_array(transition, &[field])
                        .iter()
                        .filter_map(Value::as_str)
                    {
                        let routed = self.routes.iter().any(|route| {
                            route.source == instance.id
                                && route.from.kind == kind
                                && route.from.name == output
                        }) || self.assembly.substitutes.iter().any(|substitute| {
                            substitute.source == instance.id
                                && substitute.output.kind == kind
                                && substitute.output.name == output
                        }) || self.protocol_routes.iter().any(|route| {
                            route.source == instance.id && route.send_case == transition_case
                        });
                        if !routed {
                            bail!(
                            "instance `{}` can emit unresolved {kind} `{output}`; add a canonical probe bridge or explicit substitute",
                            instance.id
                        );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn resolve_dependency_routes(&self) -> Result<Vec<ResolvedRoute>> {
        let mut routes = Vec::new();
        for consumer in self.instances.values() {
            let bindings = json_array(
                &consumer.manifest,
                &["architecture", "dependency_behavior_bindings"],
            );
            for value in bindings {
                let binding: ManifestDependencyBinding = serde_json::from_value(value.clone())
                    .context("invalid dependency behavior binding")?;
                let Some(bridge) = binding.probe_bridge else {
                    continue;
                };
                let provider_module = binding.provider_module.as_deref().ok_or_else(|| {
                    anyhow!(
                        "probe bridge `{}` must resolve to a provider module",
                        binding.id
                    )
                })?;
                let candidates = self
                    .instances
                    .values()
                    .filter(|instance| instance.module == provider_module)
                    .collect::<Vec<_>>();
                let selected = self
                    .assembly
                    .routing
                    .iter()
                    .find(|selection| {
                        selection.binding == binding.id
                            && selection.consumer.as_deref() == Some(consumer.id.as_str())
                    })
                    .or_else(|| {
                        self.assembly.routing.iter().find(|selection| {
                            selection.binding == binding.id && selection.consumer.is_none()
                        })
                    })
                    .map(|selection| selection.provider.as_str());
                let provider = if let Some(selected) = selected {
                    candidates
                        .iter()
                        .find(|candidate| candidate.id == selected)
                        .copied()
                        .ok_or_else(|| {
                            anyhow!(
                                "routing selection `{}` does not name an instance of `{provider_module}`",
                                binding.id
                            )
                        })?
                } else {
                    match candidates.as_slice() {
                        [only] => *only,
                        [] => {
                            continue;
                        }
                        _ => bail!(
                            "probe bridge `{}` has multiple provider instances; select one in assembly routing",
                            binding.id
                        ),
                    }
                };
                routes.push(ResolvedRoute {
                    id: binding.id.clone(),
                    binding: binding.id.clone(),
                    source: consumer.id.clone(),
                    target: provider.id.clone(),
                    from: bridge.request.from,
                    to: bridge.request.to,
                    mapper: bridge.request.mapper,
                    mapper_instance: consumer.id.clone(),
                });
                for (index, outcome) in bridge.outcomes.into_iter().enumerate() {
                    routes.push(ResolvedRoute {
                        id: format!("{}/outcome/{index}", binding.id),
                        binding: binding.id.clone(),
                        source: provider.id.clone(),
                        target: consumer.id.clone(),
                        from: outcome.from,
                        to: outcome.to,
                        mapper: outcome.mapper,
                        mapper_instance: consumer.id.clone(),
                    });
                }
            }
        }
        routes.sort_by(|left, right| left.id.cmp(&right.id).then(left.source.cmp(&right.source)));
        Ok(routes)
    }

    fn resolve_protocol_routes(&self) -> Result<Vec<ProtocolRoute>> {
        let mut bindings = Vec::new();
        for instance in self.instances.values() {
            for value in json_array(&instance.manifest, &["architecture", "protocol_bindings"]) {
                let binding: ManifestProtocolBinding =
                    serde_json::from_value(value.clone()).context("invalid protocol binding")?;
                bindings.push((instance.id.clone(), binding));
            }
        }
        let mut routes = Vec::new();
        for (source, sender) in &bindings {
            for send in sender
                .mappings
                .iter()
                .filter(|mapping| mapping.direction == "send")
            {
                let receivers = bindings
                    .iter()
                    .flat_map(|(target, receiver)| {
                        receiver
                            .mappings
                            .iter()
                            .filter(move |mapping| {
                                *target != *source
                                    && receiver.contract == sender.contract
                                    && mapping.direction == "receive"
                                    && mapping.message == send.message
                            })
                            .map(move |mapping| (target, receiver, mapping))
                    })
                    .collect::<Vec<_>>();
                match receivers.as_slice() {
                    [(target, receiver, receive)] => {
                        let sender_model = self.load_protocol_model(source, &sender.contract)?;
                        let receiver_model =
                            self.load_protocol_model(target, &sender.contract)?;
                        if sender_model != receiver_model {
                            bail!(
                                "protocol contract `{}` differs between instances `{source}` and `{target}`",
                                sender.contract
                            );
                        }
                        let message = sender_model
                            .messages
                            .iter()
                            .find(|message| message.id == send.message)
                            .ok_or_else(|| {
                                anyhow!(
                                    "protocol binding `{}` maps undeclared message `{}`",
                                    sender.name,
                                    send.message
                                )
                            })?;
                        if message.from != sender.participant
                            || message.to != receiver.participant
                        {
                            bail!(
                                "protocol binding `{}` maps message `{}` from participant `{}` to `{}`, but the contract declares `{} -> {}`",
                                sender.name,
                                send.message,
                                sender.participant,
                                receiver.participant,
                                message.from,
                                message.to
                            );
                        }
                        let source_instance = self
                            .instances
                            .get(source)
                            .ok_or_else(|| anyhow!("protocol source instance disappeared"))?;
                        if !json_array(
                            &source_instance.manifest,
                            &["architecture", "machine", "transitions"],
                        )
                        .iter()
                        .any(|transition| {
                            get_json_str(transition, &["case"])
                                == Some(send.machine_case.as_str())
                        }) {
                            bail!(
                                "protocol send case `{}` is not a declared transition case of `{source}`",
                                send.machine_case
                            );
                        }
                        let target_instance = self
                            .instances
                            .get(*target)
                            .ok_or_else(|| anyhow!("protocol target instance disappeared"))?;
                        let receive_input = json_array(
                            &target_instance.manifest,
                            &["architecture", "machine", "transitions"],
                        )
                        .iter()
                        .find(|transition| {
                            get_json_str(transition, &["case"])
                                == Some(receive.machine_case.as_str())
                        })
                        .and_then(|transition| get_json_str(transition, &["on"]))
                        .ok_or_else(|| {
                            anyhow!(
                                "protocol receive case `{}` is not a declared transition case of `{}`",
                                receive.machine_case,
                                target
                            )
                        })?
                        .to_string();
                        let (mapper, mapper_instance) =
                            match (&send.mapper, &receive.mapper) {
                                (Some(send_mapper), Some(receive_mapper))
                                    if send_mapper != receive_mapper =>
                                {
                                    bail!(
                                        "protocol message `{}` declares conflicting send and receive mappers",
                                        send.message
                                    )
                                }
                                (Some(mapper), _) => (Some(mapper.clone()), source.clone()),
                                (None, Some(mapper)) => {
                                    (Some(mapper.clone()), (*target).clone())
                                }
                                (None, None) => (None, source.clone()),
                            };
                        routes.push(ProtocolRoute {
                            id: format!(
                                "protocol/{}/{}/{}",
                                sender.name, source, send.message
                            ),
                            contract: sender.contract.clone(),
                            source: source.clone(),
                            target: (*target).clone(),
                            send_case: send.machine_case.clone(),
                            receive_input,
                            message: send.message.clone(),
                            mapper,
                            mapper_instance,
                            model: sender_model,
                        });
                    }
                    [] => bail!(
                        "protocol `{}` message `{}` from participant `{}` has no receiver in the assembly",
                        sender.contract,
                        send.message,
                        sender.participant
                    ),
                    _ => bail!(
                        "protocol `{}` message `{}` has multiple receiver instances",
                        sender.contract,
                        send.message
                    ),
                }
            }
        }
        routes.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(routes)
    }

    fn initial_world(&mut self) -> Result<World> {
        let mut states = BTreeMap::new();
        let mut descriptions = BTreeMap::new();
        for instance in self.instances.values() {
            let description = self.describe_instance(instance)?;
            let state = if instance.start == Value::String("initial".to_string()) {
                description.get("initial_state").cloned().ok_or_else(|| {
                    anyhow!(
                        "probe description for `{}` has no initial_state",
                        instance.id
                    )
                })?
            } else {
                instance.start.clone()
            };
            validate_state_payload(&description, &state).with_context(|| {
                format!(
                    "instance `{}` has schema-invalid starting state",
                    instance.id
                )
            })?;
            states.insert(instance.id.clone(), state);
            descriptions.insert(instance.id.clone(), description);
        }
        for stimulus in &self.assembly.stimuli {
            validate_input_payload(
                descriptions
                    .get(&stimulus.target)
                    .ok_or_else(|| anyhow!("missing description for `{}`", stimulus.target))?,
                &stimulus.input,
            )
            .with_context(|| {
                format!(
                    "stimulus `{}` has schema-invalid payload data",
                    stimulus.id.as_deref().unwrap_or("<generated>")
                )
            })?;
        }
        for substitute in &self.assembly.substitutes {
            for outcome in &substitute.outcomes {
                validate_input_payload(
                    descriptions
                        .get(&outcome.target)
                        .ok_or_else(|| anyhow!("missing description for `{}`", outcome.target))?,
                    &outcome.input,
                )
                .with_context(|| {
                    format!(
                        "substitute `{}` outcome `{}` has schema-invalid payload data",
                        substitute.id,
                        outcome.id.as_deref().unwrap_or("<generated>")
                    )
                })?;
            }
        }
        let mut queue = self
            .assembly
            .stimuli
            .iter()
            .enumerate()
            .map(|(index, stimulus)| {
                let id = stimulus
                    .id
                    .clone()
                    .unwrap_or_else(|| format!("stimulus-{index}"));
                Envelope {
                    id: id.clone(),
                    idempotency_key: id.clone(),
                    route: format!("stimulus/{id}"),
                    target: stimulus.target.clone(),
                    at: stimulus.at,
                    input: stimulus.input.clone(),
                    source: None,
                    correlation_id: id,
                    causation_id: None,
                    attempt: 1,
                }
            })
            .collect::<Vec<_>>();
        sort_queue(&mut queue);
        let fault_remaining = self
            .assembly
            .faults
            .iter()
            .map(|fault| (fault_key(&fault.route, fault.kind), fault.budget))
            .collect();
        Ok(World {
            states,
            queue,
            time: 0,
            step: 0,
            timeline: Vec::new(),
            decisions: Vec::new(),
            message_counts: BTreeMap::new(),
            fault_remaining,
            satisfied_within: BTreeSet::new(),
            protocol_states: BTreeMap::new(),
        })
    }

    fn deterministic(&mut self) -> Result<(SystemTrace, Option<Counterexample>)> {
        let mut world = self.initial_world()?;
        let mut coverage = empty_coverage();
        let mut failure = None;
        while !world.queue.is_empty() && world.step < self.bounds.max_steps {
            advance_time(&mut world);
            if world.step > 0 || world.time > 0 {
                if let Some(expired) = self.evaluate_checks(&mut world, false)? {
                    failure = Some(expired);
                    break;
                }
            }
            let choice = StepChoice {
                envelope_index: first_enabled_index(&world)
                    .ok_or_else(|| anyhow!("scheduler has no enabled envelope"))?,
                action: StepAction::Deliver,
                substitute_choices: BTreeMap::new(),
            };
            failure = self.apply_choice(&mut world, &choice, false, &mut coverage)?;
            if failure.is_some() {
                break;
            }
        }
        if failure.is_none() {
            failure = self.evaluate_checks(&mut world, true)?;
        }
        coverage.states = world.step + 1;
        coverage.schedules = usize::from(world.queue.is_empty());
        let result = if failure.is_some() {
            "fail"
        } else if world.queue.is_empty() {
            "pass"
        } else {
            "inconclusive"
        };
        let trace = self.trace_from_world(
            &world,
            result,
            "deterministic",
            false,
            failure.clone(),
            coverage,
        );
        let counterexample = failure.map(|failure| self.counterexample(&world, failure, &trace));
        Ok((trace, counterexample))
    }

    fn run_forced(
        &mut self,
        decisions: &[Decision],
    ) -> Result<(SystemTrace, Option<Counterexample>)> {
        let mut world = self.initial_world()?;
        let mut coverage = empty_coverage();
        let mut failure = None;
        for decision in decisions {
            if world.queue.is_empty() || world.step >= self.bounds.max_steps {
                break;
            }
            advance_time(&mut world);
            if world.step > 0 || world.time > 0 {
                if let Some(expired) = self.evaluate_checks(&mut world, false)? {
                    failure = Some(expired);
                    break;
                }
            }
            let envelope_index = world
                .queue
                .iter()
                .position(|envelope| envelope.id == decision.envelope && envelope.at <= world.time)
                .ok_or_else(|| {
                    anyhow!(
                        "recorded decision references unavailable envelope `{}`",
                        decision.envelope
                    )
                })?;
            let action = match decision.action.as_str() {
                "deliver" => StepAction::Deliver,
                "delay" => StepAction::Fault(FaultKind::Delay),
                "duplicate" => StepAction::Fault(FaultKind::Duplicate),
                "drop" => StepAction::Fault(FaultKind::Drop),
                "timeout" => StepAction::Fault(FaultKind::Timeout),
                other => bail!("unknown recorded probe action `{other}`"),
            };
            let mut substitute_choices = parse_substitute_choices(decision.choice.as_deref())?;
            if let Some(index) = substitute_choices.remove("*") {
                let target = &world.queue[envelope_index].target;
                for substitute in self
                    .assembly
                    .substitutes
                    .iter()
                    .filter(|substitute| substitute.source == target.as_str())
                {
                    substitute_choices.insert(substitute.id.clone(), index);
                }
            }
            failure = self.apply_choice(
                &mut world,
                &StepChoice {
                    envelope_index,
                    action,
                    substitute_choices,
                },
                true,
                &mut coverage,
            )?;
            if failure.is_some() {
                break;
            }
        }
        if failure.is_none() && world.queue.is_empty() {
            failure = self.evaluate_checks(&mut world, true)?;
        }
        coverage.states = world.step + 1;
        coverage.schedules = usize::from(world.queue.is_empty());
        let result = if failure.is_some() {
            "fail"
        } else if world.queue.is_empty() {
            "pass"
        } else {
            "inconclusive"
        };
        let trace =
            self.trace_from_world(&world, result, "replay", false, failure.clone(), coverage);
        let counterexample = failure.map(|failure| self.counterexample(&world, failure, &trace));
        Ok((trace, counterexample))
    }

    fn explore(&mut self) -> Result<(SystemTrace, Option<Counterexample>)> {
        let (trace, counterexample) = self.explore_internal()?;
        let Some(counterexample) = counterexample else {
            return Ok((trace, None));
        };
        let minimized = self.minimize_counterexample(counterexample)?;
        Ok((minimized.trace.clone(), Some(minimized)))
    }

    fn explore_internal(&mut self) -> Result<(SystemTrace, Option<Counterexample>)> {
        let initial = self.initial_world()?;
        let mut frontier = VecDeque::from([initial]);
        let mut visited = BTreeSet::new();
        let mut coverage = empty_coverage();
        let mut last_terminal = None;
        while let Some(mut world) = frontier.pop_front() {
            if coverage.states >= self.bounds.max_states
                || coverage.schedules >= self.bounds.max_schedules
            {
                let terminal = last_terminal.unwrap_or(world);
                let trace = self.trace_from_world(
                    &terminal,
                    "inconclusive",
                    "exploration",
                    false,
                    None,
                    coverage,
                );
                return Ok((trace, None));
            }
            advance_time(&mut world);
            if world.step > 0 || world.time > 0 {
                if let Some(failure) = self.evaluate_checks(&mut world, false)? {
                    let trace = self.trace_from_world(
                        &world,
                        "fail",
                        "exploration",
                        false,
                        Some(failure.clone()),
                        coverage,
                    );
                    let counterexample = self.counterexample(&world, failure, &trace);
                    return Ok((trace, Some(counterexample)));
                }
            }
            let hash = world_hash(&world)?;
            if !visited.insert(hash) {
                continue;
            }
            coverage.states += 1;
            if world.queue.is_empty() {
                coverage.schedules += 1;
                if let Some(failure) = self.evaluate_checks(&mut world, true)? {
                    let trace = self.trace_from_world(
                        &world,
                        "fail",
                        "exploration",
                        false,
                        Some(failure.clone()),
                        coverage,
                    );
                    let counterexample = self.counterexample(&world, failure, &trace);
                    return Ok((trace, Some(counterexample)));
                }
                last_terminal = Some(world);
                continue;
            }
            if world.step >= self.bounds.max_steps {
                let trace = self.trace_from_world(
                    &world,
                    "inconclusive",
                    "exploration",
                    false,
                    None,
                    coverage,
                );
                return Ok((trace, None));
            }
            let choices = self.choices(&world);
            self.prefetch_transitions(&world, &choices)?;
            for choice in choices {
                let mut next = world.clone();
                let mut branch_coverage = coverage.clone();
                if let Some(failure) =
                    self.apply_choice(&mut next, &choice, true, &mut branch_coverage)?
                {
                    branch_coverage.states = coverage.states;
                    let trace = self.trace_from_world(
                        &next,
                        "fail",
                        "exploration",
                        false,
                        Some(failure.clone()),
                        branch_coverage,
                    );
                    let counterexample = self.counterexample(&next, failure, &trace);
                    return Ok((trace, Some(counterexample)));
                }
                coverage.transitions = coverage.transitions.max(branch_coverage.transitions);
                coverage
                    .transition_cases
                    .extend(branch_coverage.transition_cases);
                coverage.routes.extend(branch_coverage.routes);
                coverage.faults.extend(branch_coverage.faults);
                frontier.push_back(next);
            }
        }
        let terminal = last_terminal.unwrap_or(self.initial_world()?);
        let trace = self.trace_from_world(&terminal, "pass", "exploration", true, None, coverage);
        Ok((trace, None))
    }

    fn collect_property_traces(&mut self) -> Result<PropertyExploration> {
        let initial = self.initial_world()?;
        let mut frontier = VecDeque::from([initial]);
        let mut visited = BTreeSet::new();
        let mut coverage = empty_coverage();
        let mut traces = Vec::new();
        let mut exhausted = true;
        while let Some(mut world) = frontier.pop_front() {
            if coverage.states >= self.bounds.max_states
                || coverage.schedules >= self.bounds.max_schedules
            {
                exhausted = false;
                break;
            }
            advance_time(&mut world);
            if world.step > 0 || world.time > 0 {
                if let Some(failure) = self.evaluate_checks(&mut world, false)? {
                    let trace = self.trace_from_world(
                        &world,
                        "fail",
                        "exploration",
                        false,
                        Some(failure),
                        coverage.clone(),
                    );
                    traces.push(serde_json::to_value(trace)?);
                    continue;
                }
            }
            let hash = world_hash(&world)?;
            if !visited.insert(hash) {
                continue;
            }
            coverage.states += 1;
            if world.queue.is_empty() {
                coverage.schedules += 1;
                let failure = self.evaluate_checks(&mut world, true)?;
                let result = if failure.is_some() { "fail" } else { "pass" };
                let trace = self.trace_from_world(
                    &world,
                    result,
                    "exploration",
                    false,
                    failure,
                    coverage.clone(),
                );
                traces.push(serde_json::to_value(trace)?);
                continue;
            }
            if world.step >= self.bounds.max_steps {
                exhausted = false;
                let trace = self.trace_from_world(
                    &world,
                    "inconclusive",
                    "exploration",
                    false,
                    None,
                    coverage.clone(),
                );
                traces.push(serde_json::to_value(trace)?);
                continue;
            }
            let choices = self.choices(&world);
            self.prefetch_transitions(&world, &choices)?;
            for choice in choices {
                let mut next = world.clone();
                let mut branch_coverage = coverage.clone();
                if let Some(failure) =
                    self.apply_choice(&mut next, &choice, true, &mut branch_coverage)?
                {
                    let trace = self.trace_from_world(
                        &next,
                        "fail",
                        "exploration",
                        false,
                        Some(failure),
                        branch_coverage,
                    );
                    traces.push(serde_json::to_value(trace)?);
                    continue;
                }
                coverage.transitions = coverage.transitions.max(branch_coverage.transitions);
                coverage
                    .transition_cases
                    .extend(branch_coverage.transition_cases);
                coverage.routes.extend(branch_coverage.routes);
                coverage.faults.extend(branch_coverage.faults);
                frontier.push_back(next);
            }
        }
        if traces.is_empty() && exhausted {
            let world = self.initial_world()?;
            traces.push(serde_json::to_value(self.trace_from_world(
                &world,
                "pass",
                "exploration",
                false,
                None,
                coverage.clone(),
            ))?);
        }
        Ok(PropertyExploration {
            traces,
            exhausted,
            assembly_digest: self.assembly_digest.clone(),
            coverage: serde_json::to_value(&coverage)?,
            bounds: serde_json::to_value(&self.bounds)?,
        })
    }

    fn minimize_counterexample(&self, original: Counterexample) -> Result<Counterexample> {
        let expected_failure = original.failure.clone();
        let mut best = original;
        let mut assembly: ProbeAssembly =
            serde_json::from_value(best.assembly.clone()).context("invalid failure assembly")?;

        let mut index = 0;
        while assembly.stimuli.len() > 1 && index < assembly.stimuli.len() {
            let mut candidate = assembly.clone();
            candidate.stimuli.remove(index);
            if let Some(counterexample) =
                self.counterexample_for_candidate(candidate.clone(), &expected_failure)?
            {
                assembly = candidate;
                best = counterexample;
            } else {
                index += 1;
            }
        }

        let mut fault_index = 0;
        while fault_index < assembly.faults.len() {
            let mut candidate = assembly.clone();
            candidate.faults.remove(fault_index);
            if let Some(counterexample) =
                self.counterexample_for_candidate(candidate.clone(), &expected_failure)?
            {
                assembly = candidate;
                best = counterexample;
            } else {
                fault_index += 1;
            }
        }

        for fault_index in 0..assembly.faults.len() {
            let Some(delay) = assembly.faults[fault_index].delay else {
                continue;
            };
            for smaller in shrink_u64(delay) {
                let mut candidate = assembly.clone();
                candidate.faults[fault_index].delay = Some(smaller.max(1));
                if let Some(counterexample) =
                    self.counterexample_for_candidate(candidate.clone(), &expected_failure)?
                {
                    assembly = candidate;
                    best = counterexample;
                }
            }
        }

        for stimulus_index in 0..assembly.stimuli.len() {
            let data = assembly.stimuli[stimulus_index]
                .input
                .get("data")
                .cloned()
                .unwrap_or(Value::Null);
            for smaller in shrink_json(&data) {
                let mut candidate = assembly.clone();
                if let Some(input) = candidate.stimuli[stimulus_index].input.as_object_mut() {
                    input.insert("data".to_string(), smaller);
                }
                if let Some(counterexample) =
                    self.counterexample_for_candidate(candidate.clone(), &expected_failure)?
                {
                    assembly = candidate;
                    best = counterexample;
                }
            }
        }
        Ok(best)
    }

    fn counterexample_for_candidate(
        &self,
        candidate: ProbeAssembly,
        expected_failure: &ProbeFailure,
    ) -> Result<Option<Counterexample>> {
        let value = serde_json::to_value(&candidate)?;
        let mut engine = match Engine::new(
            candidate,
            value,
            self.base_dir.clone(),
            Some(self.bounds.max_steps),
            Some(self.bounds.max_schedules),
            Some(self.bounds.max_states),
            self.timeout_seconds,
        ) {
            Ok(engine) => engine,
            Err(_) => return Ok(None),
        };
        match engine.explore_internal() {
            Ok((_, Some(counterexample)))
                if counterexample.failure.check == expected_failure.check
                    && counterexample.failure.message == expected_failure.message =>
            {
                Ok(Some(counterexample))
            }
            Ok(_) | Err(_) => Ok(None),
        }
    }

    fn choices(&self, world: &World) -> Vec<StepChoice> {
        let mut choices = Vec::new();
        for (index, envelope) in world.queue.iter().enumerate() {
            if envelope.at > world.time {
                continue;
            }
            let substitutes = self
                .assembly
                .substitutes
                .iter()
                .filter(|substitute| substitute.source == envelope.target)
                .collect::<Vec<_>>();
            for substitute_choices in substitute_assignments(&substitutes) {
                choices.push(StepChoice {
                    envelope_index: index,
                    action: StepAction::Deliver,
                    substitute_choices,
                });
            }
            for fault in self
                .assembly
                .faults
                .iter()
                .filter(|fault| fault.route == envelope.route)
            {
                if world
                    .fault_remaining
                    .get(&fault_key(&fault.route, fault.kind))
                    .copied()
                    .unwrap_or(0)
                    > 0
                {
                    choices.push(StepChoice {
                        envelope_index: index,
                        action: StepAction::Fault(fault.kind),
                        substitute_choices: BTreeMap::new(),
                    });
                }
            }
        }
        choices.sort_by(|left, right| {
            let left_envelope = &world.queue[left.envelope_index];
            let right_envelope = &world.queue[right.envelope_index];
            envelope_order(left_envelope, right_envelope)
                .then(action_label(&left.action).cmp(action_label(&right.action)))
                .then(left.substitute_choices.cmp(&right.substitute_choices))
        });
        choices
    }

    fn apply_choice(
        &mut self,
        world: &mut World,
        choice: &StepChoice,
        explore_substitutes: bool,
        coverage: &mut Coverage,
    ) -> Result<Option<ProbeFailure>> {
        let mut envelope = world.queue.remove(choice.envelope_index);
        match choice.action {
            StepAction::Fault(kind) => {
                let key = fault_key(&envelope.route, kind);
                let remaining = world.fault_remaining.get(&key).copied().unwrap_or(0);
                if remaining == 0 {
                    bail!("fault budget `{key}` is exhausted");
                }
                world.fault_remaining.insert(key, remaining - 1);
                world.step += 1;
                world.decisions.push(Decision {
                    envelope: envelope.id.clone(),
                    action: fault_label(kind).to_string(),
                    choice: None,
                });
                coverage.faults.insert(fault_label(kind).to_string());
                let input = envelope.input.clone();
                let target = envelope.target.clone();
                let route = envelope.route.clone();
                match kind {
                    FaultKind::Delay => {
                        let delay = self
                            .assembly
                            .faults
                            .iter()
                            .find(|fault| fault.route == envelope.route && fault.kind == kind)
                            .and_then(|fault| fault.delay)
                            .unwrap_or(1);
                        envelope.at = envelope.at.saturating_add(delay);
                        world.queue.push(envelope.clone());
                        sort_queue(&mut world.queue);
                    }
                    FaultKind::Duplicate => {
                        let mut duplicate = envelope.clone();
                        duplicate.attempt += 1;
                        duplicate.id = format!("{}#{}", duplicate.id, duplicate.attempt);
                        world.queue.push(envelope.clone());
                        world.queue.push(duplicate);
                        sort_queue(&mut world.queue);
                    }
                    FaultKind::Drop | FaultKind::Timeout => {}
                }
                world.timeline.push(TimelineEntry {
                    step: world.step,
                    time: world.time,
                    action: fault_label(kind).to_string(),
                    envelope: envelope.id,
                    idempotency_key: envelope.idempotency_key,
                    route,
                    source: envelope.source,
                    target,
                    input,
                    correlation_id: envelope.correlation_id,
                    causation_id: envelope.causation_id,
                    attempt: envelope.attempt,
                    state_before: None,
                    state_after: None,
                    transition_case: None,
                    source_file: None,
                    source_function: None,
                    outputs: Vec::new(),
                });
            }
            StepAction::Deliver => {
                world.step += 1;
                let state_before = world
                    .states
                    .get(&envelope.target)
                    .cloned()
                    .ok_or_else(|| anyhow!("unknown target instance `{}`", envelope.target))?;
                let record =
                    self.evaluate_transition(&envelope.target, &state_before, &envelope.input)?;
                let state_after = record
                    .get("state_after")
                    .cloned()
                    .ok_or_else(|| anyhow!("transition record has no state_after"))?;
                let transition_case = get_json_str(&record, &["source", "branch"])
                    .unwrap_or("<missing>")
                    .to_string();
                let outputs = record_outputs(&record);
                world.decisions.push(Decision {
                    envelope: envelope.id.clone(),
                    action: "deliver".to_string(),
                    choice: encode_substitute_choices(
                        &self.assembly.substitutes,
                        &envelope.target,
                        &outputs,
                        &choice.substitute_choices,
                    ),
                });
                world
                    .states
                    .insert(envelope.target.clone(), state_after.clone());
                for output in &outputs {
                    *world.message_counts.entry(output.name.clone()).or_default() += 1;
                }
                coverage.transitions += 1;
                coverage
                    .transition_cases
                    .insert(format!("{}:{transition_case}", envelope.target));
                coverage.routes.insert(envelope.route.clone());
                world.timeline.push(TimelineEntry {
                    step: world.step,
                    time: world.time,
                    action: "deliver".to_string(),
                    envelope: envelope.id.clone(),
                    idempotency_key: envelope.idempotency_key.clone(),
                    route: envelope.route.clone(),
                    source: envelope.source.clone(),
                    target: envelope.target.clone(),
                    input: envelope.input.clone(),
                    correlation_id: envelope.correlation_id.clone(),
                    causation_id: envelope.causation_id.clone(),
                    attempt: envelope.attempt,
                    state_before: Some(state_before),
                    state_after: Some(state_after),
                    transition_case: Some(transition_case.clone()),
                    source_file: get_json_str(&record, &["source", "file"]).map(str::to_string),
                    source_function: get_json_str(&record, &["source", "function"])
                        .map(str::to_string),
                    outputs: outputs.clone(),
                });
                if let Some(failure) = self.advance_protocols(world, &envelope, &transition_case)? {
                    return Ok(Some(failure));
                }
                let routed = self.route_outputs(
                    &envelope,
                    &transition_case,
                    &outputs,
                    explore_substitutes,
                    &choice.substitute_choices,
                )?;
                for next in routed.into_iter().next().unwrap_or_default() {
                    *world
                        .message_counts
                        .entry(input_name(&next.input))
                        .or_default() += 1;
                    world.queue.push(next);
                }
                sort_queue(&mut world.queue);
            }
        }
        self.evaluate_checks(world, false)
    }

    fn load_protocol_model(&self, instance_id: &str, contract: &str) -> Result<ProtocolModel> {
        let instance = self
            .instances
            .get(instance_id)
            .ok_or_else(|| anyhow!("protocol participant `{instance_id}` is unknown"))?;
        let root = instance
            .implementation_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let path = root.join(contract);
        let value = load_json_or_yaml(&path).with_context(|| {
            format!("protocol contract `{contract}` for instance `{instance_id}` is not executable")
        })?;
        let semantics = value
            .get("semantics")
            .and_then(|value| value.get("protocol"))
            .cloned()
            .ok_or_else(|| {
                anyhow!(
                    "protocol contract `{}` has no semantics.protocol declaration",
                    path.display()
                )
            })?;
        let model: ProtocolModel = serde_json::from_value(semantics)
            .with_context(|| format!("invalid protocol model in `{}`", path.display()))?;
        let states = model.states.iter().cloned().collect::<BTreeSet<_>>();
        if states.len() != model.states.len()
            || !states.contains(&model.initial_state)
            || model
                .terminal_states
                .iter()
                .any(|state| !states.contains(state))
            || model.transitions.iter().any(|transition| {
                !states.contains(&transition.from) || !states.contains(&transition.to)
            })
        {
            bail!(
                "protocol contract `{}` has an invalid closed state model",
                path.display()
            );
        }
        Ok(model)
    }

    fn advance_protocols(
        &self,
        world: &mut World,
        cause: &Envelope,
        transition_case: &str,
    ) -> Result<Option<ProbeFailure>> {
        for route in self
            .protocol_routes
            .iter()
            .filter(|route| route.source == cause.target && route.send_case == transition_case)
        {
            let conversation = format!("{}#{}", route.contract, cause.correlation_id);
            let current = world
                .protocol_states
                .get(&conversation)
                .cloned()
                .unwrap_or_else(|| route.model.initial_state.clone());
            let Some(transition) =
                route.model.transitions.iter().find(|transition| {
                    transition.from == current && transition.on == route.message
                })
            else {
                return Ok(Some(ProbeFailure {
                    check: format!("protocol:{}", route.contract),
                    step: world.step,
                    time: world.time,
                    message: format!(
                        "message `{}` is illegal from protocol state `{current}`",
                        route.message
                    ),
                }));
            };
            world
                .protocol_states
                .insert(conversation, transition.to.clone());
        }
        Ok(None)
    }

    fn route_outputs(
        &self,
        cause: &Envelope,
        transition_case: &str,
        outputs: &[OutputValue],
        explore_substitutes: bool,
        substitute_choices: &BTreeMap<String, usize>,
    ) -> Result<Vec<Vec<Envelope>>> {
        let mut alternatives = vec![Vec::new()];
        for output in outputs {
            let mut routes = self
                .routes
                .iter()
                .filter(|route| {
                    route.source == cause.target
                        && route.from.kind == output.kind
                        && route.from.name == output.name
                })
                .collect::<Vec<_>>();
            if routes.len() > 1 {
                if let Some(source) = cause.source.as_deref() {
                    routes.retain(|route| route.target == source);
                }
            }
            let substitutes = self
                .assembly
                .substitutes
                .iter()
                .filter(|substitute| {
                    substitute.source == cause.target
                        && substitute.output.kind == output.kind
                        && substitute.output.name == output.name
                })
                .collect::<Vec<_>>();
            if routes.len() > 1 {
                bail!(
                    "output {}:{} from `{}` resolves to multiple canonical routes",
                    output.kind,
                    output.name,
                    cause.target
                );
            }
            if let Some(route) = routes.first() {
                let data = output.value.get("data").cloned().unwrap_or(Value::Null);
                let data = if let Some(mapper) = route.mapper.as_deref() {
                    self.execute_mapper(&route.mapper_instance, mapper, &data)?
                } else {
                    data
                };
                let next = envelope_from_route(cause, route, data, 0);
                for branch in &mut alternatives {
                    branch.push(next.clone());
                }
            } else if let Some(substitute) = substitutes.first() {
                let selected = if explore_substitutes {
                    substitute_choices
                        .get(&substitute.id)
                        .and_then(|index| substitute.outcomes.get(*index))
                        .unwrap_or(&substitute.outcomes[0])
                } else {
                    &substitute.outcomes[0]
                };
                let outcomes = std::slice::from_ref(selected);
                let mut expanded = Vec::new();
                for branch in &alternatives {
                    for (index, outcome) in outcomes.iter().enumerate() {
                        let mut branch = branch.clone();
                        branch.push(Envelope {
                            id: format!("{}:{}/{}", cause.id, substitute.id, index),
                            idempotency_key: format!("{}:{}/{}", cause.id, substitute.id, index),
                            route: substitute.id.clone(),
                            target: outcome.target.clone(),
                            at: cause.at.saturating_add(outcome.after),
                            input: outcome.input.clone(),
                            source: Some(cause.target.clone()),
                            correlation_id: cause.correlation_id.clone(),
                            causation_id: Some(cause.id.clone()),
                            attempt: 1,
                        });
                        expanded.push(branch);
                    }
                }
                alternatives = expanded;
            }
        }
        for route in self
            .protocol_routes
            .iter()
            .filter(|route| route.source == cause.target && route.send_case == transition_case)
        {
            let data = outputs
                .iter()
                .find(|output| output.name == route.message)
                .and_then(|output| output.value.get("data"))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let data = if let Some(mapper) = route.mapper.as_deref() {
                self.execute_mapper(&route.mapper_instance, mapper, &data)?
            } else {
                data
            };
            let target = self
                .instances
                .get(&route.target)
                .ok_or_else(|| anyhow!("protocol target disappeared"))?;
            let kind = input_kind(&target.manifest, &route.receive_input)?;
            let next = Envelope {
                id: format!("{}:{}", cause.id, route.message),
                idempotency_key: format!("{}:{}", cause.id, route.message),
                route: route.id.clone(),
                target: route.target.clone(),
                at: cause.at,
                input: json!({"kind": kind, "name": route.receive_input, "data": data}),
                source: Some(cause.target.clone()),
                correlation_id: cause.correlation_id.clone(),
                causation_id: Some(cause.id.clone()),
                attempt: 1,
            };
            for branch in &mut alternatives {
                branch.push(next.clone());
            }
        }
        Ok(alternatives)
    }

    fn evaluate_checks(&self, world: &mut World, quiescent: bool) -> Result<Option<ProbeFailure>> {
        for check in &self.assembly.checks {
            let satisfied = self.assertion_satisfied(&check.assert, world)?;
            match check.when {
                CheckWhen::Always if !satisfied => {
                    return Ok(Some(check_failure(
                        check,
                        world,
                        "always assertion is false",
                    )));
                }
                CheckWhen::Quiescent if quiescent && !satisfied => {
                    return Ok(Some(check_failure(
                        check,
                        world,
                        "quiescent assertion is false",
                    )));
                }
                CheckWhen::Within => {
                    if satisfied {
                        world.satisfied_within.insert(check.id.clone());
                        continue;
                    }
                    if world.satisfied_within.contains(&check.id) {
                        continue;
                    }
                    let step_expired = check.within_steps.is_some_and(|limit| world.step >= limit);
                    let time_expired = check.within_time.is_some_and(|limit| world.time >= limit);
                    if step_expired || time_expired || quiescent {
                        return Ok(Some(check_failure(
                            check,
                            world,
                            "bounded eventual assertion was not reached",
                        )));
                    }
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn assertion_satisfied(&self, assertion: &CheckAssertion, world: &World) -> Result<bool> {
        match assertion {
            CheckAssertion::State { instance, equals } => Ok(world
                .states
                .get(instance)
                .is_some_and(|state| state == equals)),
            CheckAssertion::QueueEmpty => Ok(world.queue.is_empty()),
            CheckAssertion::MessageCount {
                name,
                equals,
                min,
                max,
            } => {
                let observed = world.message_counts.get(name).copied().unwrap_or(0);
                Ok(equals.is_none_or(|value| observed == value)
                    && min.is_none_or(|value| observed >= value)
                    && max.is_none_or(|value| observed <= value))
            }
            CheckAssertion::Oracle { command, runner } => {
                self.execute_oracle(command, runner, world)
            }
        }
    }

    fn execute_oracle(&self, command: &str, runner: &str, world: &World) -> Result<bool> {
        let temp_root = std::env::temp_dir().join(format!(
            "rms-probe-oracle-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root)?;
        let snapshot_path = temp_root.join("snapshot.json");
        let trace_path = temp_root.join("trace.json");
        fs::write(
            &snapshot_path,
            serde_json::to_vec_pretty(&json!({
                "states": world.states,
                "queue": world.queue,
                "time": world.time,
                "step": world.step,
                "message_counts": world.message_counts
            }))?,
        )?;
        fs::write(&trace_path, serde_json::to_vec_pretty(&world.timeline)?)?;
        let snapshot_display = snapshot_path.display().to_string();
        let trace_display = trace_path.display().to_string();
        let output = execute_proof_command(
            &self.base_dir,
            command,
            &[
                ("RMS_PROBE_SNAPSHOT", snapshot_display.as_str()),
                ("RMS_PROBE_TRACE", trace_display.as_str()),
                ("RMS_PROBE_ORACLE", runner),
            ],
            self.timeout_seconds,
        );
        let _ = fs::remove_dir_all(&temp_root);
        let output = output?;
        if output.timed_out {
            bail!(
                "probe oracle `{runner}` exceeded {} second(s)",
                self.timeout_seconds
            );
        }
        Ok(output.status.success())
    }

    fn execute_mapper(&self, instance_id: &str, mapper_id: &str, data: &Value) -> Result<Value> {
        let instance = self
            .instances
            .get(instance_id)
            .ok_or_else(|| anyhow!("probe mapper source instance `{instance_id}` is unknown"))?;
        let mapper = json_array(&instance.manifest, &["architecture", "probe", "mappers"])
            .iter()
            .find_map(|value| {
                serde_json::from_value::<ManifestProbeMapper>(value.clone())
                    .ok()
                    .filter(|mapper| mapper.id == mapper_id)
            })
            .ok_or_else(|| {
                anyhow!(
                    "probe route mapper `{mapper_id}` is not declared by instance `{instance_id}`"
                )
            })?;
        let command = get_json_str(&instance.manifest, &["commands", &mapper.command])
            .ok_or_else(|| anyhow!("probe mapper command `{}` is not declared", mapper.command))?;
        let temp_root = std::env::temp_dir().join(format!(
            "rms-probe-mapper-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root)?;
        let request_path = temp_root.join("request.json");
        let output_path = temp_root.join("output.json");
        fs::write(
            &request_path,
            serde_json::to_vec_pretty(&json!({
                "spec": "rms/probe-mapper/v0.1",
                "mapper": mapper.id,
                "data": data
            }))?,
        )?;
        let request_display = request_path.display().to_string();
        let output_display = output_path.display().to_string();
        let process = execute_proof_command(
            instance
                .implementation_path
                .parent()
                .unwrap_or_else(|| Path::new(".")),
            command,
            &[
                ("RMS_PROBE_MAPPER_REQUEST", request_display.as_str()),
                ("RMS_PROBE_MAPPER_OUTPUT", output_display.as_str()),
                ("RMS_PROBE_MAPPER", mapper.runner.as_str()),
            ],
            self.timeout_seconds,
        );
        let result = match process {
            Ok(process) if process.timed_out => {
                Err(anyhow!("probe mapper `{mapper_id}` timed out"))
            }
            Ok(process) if !process.status.success() => Err(anyhow!(
                "probe mapper `{mapper_id}` failed with status {}: {}",
                process.status,
                process.stderr.trim()
            )),
            Ok(_) => load_json_or_yaml(&output_path),
            Err(error) => Err(error),
        };
        let _ = fs::remove_dir_all(&temp_root);
        result
    }

    fn validate_mapper(&self, instance_id: &str, mapper_id: &str) -> Result<()> {
        let instance = self
            .instances
            .get(instance_id)
            .ok_or_else(|| anyhow!("probe mapper source instance `{instance_id}` is unknown"))?;
        let mapper = json_array(&instance.manifest, &["architecture", "probe", "mappers"])
            .iter()
            .find_map(|value| {
                serde_json::from_value::<ManifestProbeMapper>(value.clone())
                    .ok()
                    .filter(|mapper| mapper.id == mapper_id)
            })
            .ok_or_else(|| {
                anyhow!(
                    "probe route mapper `{mapper_id}` is not declared by instance `{instance_id}`"
                )
            })?;
        if get_json_str(&instance.manifest, &["commands", &mapper.command]).is_none() {
            bail!("probe mapper command `{}` is not declared", mapper.command);
        }
        Ok(())
    }

    fn describe_instance(&self, instance: &RuntimeInstance) -> Result<Value> {
        let protocol = probe_protocol(&instance.manifest);
        let request = json!({"spec": protocol, "operation": "describe"});
        execute_binding_request(instance, &request, self.timeout_seconds, true)
    }

    fn evaluate_transition(
        &mut self,
        instance_id: &str,
        state: &Value,
        input: &Value,
    ) -> Result<Value> {
        let instance = self
            .instances
            .get(instance_id)
            .ok_or_else(|| anyhow!("unknown instance `{instance_id}`"))?;
        let key = transition_cache_key(&instance.digest, state, input)?;
        if let Some(record) = self.cache.get(&key) {
            return Ok(record.clone());
        }
        let protocol = probe_protocol(&instance.manifest);
        let request = json!({
            "spec": protocol,
            "operation": "run",
            "start": state,
            "steps": [{"input": input}]
        });
        let bundle = execute_binding_request(instance, &request, self.timeout_seconds, false)?;
        let record = bundle
            .get("records")
            .and_then(Value::as_array)
            .and_then(|records| records.first())
            .cloned()
            .ok_or_else(|| anyhow!("probe runner returned no transition record"))?;
        self.cache.insert(key, record.clone());
        Ok(record)
    }

    fn prefetch_transitions(&mut self, world: &World, choices: &[StepChoice]) -> Result<()> {
        let mut grouped: BTreeMap<String, (String, Vec<(String, Value, Value)>)> = BTreeMap::new();
        for choice in choices {
            if !matches!(choice.action, StepAction::Deliver) {
                continue;
            }
            let envelope = &world.queue[choice.envelope_index];
            let instance = self
                .instances
                .get(&envelope.target)
                .ok_or_else(|| anyhow!("unknown instance `{}`", envelope.target))?;
            if instance.binding.protocol != "rms/machine-probe/v0.2" {
                continue;
            }
            let state = world
                .states
                .get(&envelope.target)
                .cloned()
                .ok_or_else(|| anyhow!("missing state for `{}`", envelope.target))?;
            let key = transition_cache_key(&instance.digest, &state, &envelope.input)?;
            if self.cache.contains_key(&key) {
                continue;
            }
            let (_, cases) = grouped
                .entry(instance.digest.clone())
                .or_insert_with(|| (envelope.target.clone(), Vec::new()));
            if !cases.iter().any(|(existing, _, _)| existing == &key) {
                cases.push((key, state, envelope.input.clone()));
            }
        }
        for (_, (instance_id, cases)) in grouped {
            let instance = self
                .instances
                .get(&instance_id)
                .cloned()
                .ok_or_else(|| anyhow!("unknown instance `{instance_id}`"))?;
            let request = json!({
                "spec": "rms/machine-probe/v0.2",
                "operation": "evaluate",
                "cases": cases.iter().enumerate().map(|(index, (_, state, input))| json!({
                    "id": format!("case-{index}"),
                    "state": state,
                    "input": input
                })).collect::<Vec<_>>()
            });
            let output = execute_binding_request(&instance, &request, self.timeout_seconds, false)?;
            let results = output
                .get("results")
                .and_then(Value::as_array)
                .ok_or_else(|| anyhow!("v0.2 probe evaluation returned no results"))?;
            if results.len() != cases.len() {
                bail!(
                    "v0.2 probe evaluation returned {} results for {} cases",
                    results.len(),
                    cases.len()
                );
            }
            for ((key, _, _), result) in cases.into_iter().zip(results) {
                let record = result
                    .get("record")
                    .cloned()
                    .ok_or_else(|| anyhow!("v0.2 probe evaluation result has no record"))?;
                self.cache.insert(key, record);
            }
        }
        Ok(())
    }

    fn trace_from_world(
        &self,
        world: &World,
        result: &str,
        mode: &str,
        exhausted: bool,
        failure: Option<ProbeFailure>,
        coverage: Coverage,
    ) -> SystemTrace {
        SystemTrace {
            spec: TRACE_SPEC.to_string(),
            result: result.to_string(),
            mode: mode.to_string(),
            exhausted,
            assembly_digest: self.assembly_digest.clone(),
            source_revision: self.source_revision(),
            instances: self
                .instances
                .values()
                .map(|instance| InstanceResult {
                    id: instance.id.clone(),
                    module: instance.module.clone(),
                    implementation: instance.implementation_path.display().to_string(),
                    digest: instance.digest.clone(),
                    state: world
                        .states
                        .get(&instance.id)
                        .cloned()
                        .unwrap_or(Value::Null),
                })
                .collect(),
            timeline: world.timeline.clone(),
            protocols: world.protocol_states.clone(),
            checks: self
                .assembly
                .checks
                .iter()
                .map(|check| check.id.clone())
                .collect(),
            failure,
            coverage,
            bounds: self.bounds.clone(),
        }
    }

    fn counterexample(
        &self,
        world: &World,
        failure: ProbeFailure,
        trace: &SystemTrace,
    ) -> Counterexample {
        Counterexample {
            spec: COUNTEREXAMPLE_SPEC.to_string(),
            assembly: self.replayable_assembly_value(),
            assembly_digest: self.assembly_digest.clone(),
            source_revision: self.source_revision(),
            implementation_digests: self
                .instances
                .values()
                .map(|instance| (instance.id.clone(), instance.digest.clone()))
                .collect(),
            decisions: world.decisions.clone(),
            failure,
            trace: trace.clone(),
        }
    }

    fn replayable_assembly_value(&self) -> Value {
        let mut value = self.assembly_value.clone();
        if let Some(instances) = value.get_mut("instances").and_then(Value::as_array_mut) {
            for instance in instances {
                let Some(id) = instance.get("id").and_then(Value::as_str) else {
                    continue;
                };
                let Some(runtime) = self.instances.get(id) else {
                    continue;
                };
                if let Some(object) = instance.as_object_mut() {
                    object.insert(
                        "implementation".to_string(),
                        Value::String(runtime.implementation_path.display().to_string()),
                    );
                }
            }
        }
        value
    }

    fn source_revision(&self) -> Option<String> {
        self.instances
            .values()
            .find_map(|instance| git_revision(&instance.implementation_path))
            .or_else(|| git_revision(&self.base_dir))
    }
}

fn execute_binding_request(
    instance: &RuntimeInstance,
    request: &Value,
    timeout_seconds: u64,
    description: bool,
) -> Result<Value> {
    let temp_root = std::env::temp_dir().join(format!(
        "rms-assembly-probe-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&temp_root)
        .with_context(|| format!("failed to create `{}`", temp_root.display()))?;
    let request_path = temp_root.join("request.json");
    let output_path = temp_root.join("output.json");
    let result = (|| {
        fs::write(&request_path, serde_json::to_vec_pretty(request)?)
            .with_context(|| format!("failed to write `{}`", request_path.display()))?;
        let root = instance
            .implementation_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let request_display = request_path.display().to_string();
        let output_display = output_path.display().to_string();
        let process = execute_proof_command(
            root,
            &instance.binding.command,
            &[
                ("RMS_PROBE_REQUEST", request_display.as_str()),
                ("RMS_PROBE_OUTPUT", output_display.as_str()),
                ("RMS_PROBE_RUNNER", instance.binding.runner.as_str()),
            ],
            timeout_seconds,
        )?;
        if process.timed_out {
            bail!(
                "probe runner `{}` exceeded {} second(s)",
                instance.binding.runner,
                timeout_seconds
            );
        }
        if !process.status.success() {
            bail!(
                "probe runner `{}` failed with status {}: {}",
                instance.binding.runner,
                process.status,
                process.stderr.trim()
            );
        }
        let output_yaml = load_yaml_value(&output_path)?;
        let evaluation = request.get("operation").and_then(Value::as_str) == Some("evaluate");
        if description {
            validate_probe_description(&output_yaml, &instance.binding.implementation)?;
        } else if evaluation {
            if get_str(&output_yaml, &["spec"]) != Some("rms/machine-probe-evaluation/v0.2") {
                bail!("v0.2 probe evaluation returned an invalid spec");
            }
            let results = get_path(&output_yaml, &["results"])
                .and_then(serde_yaml::Value::as_sequence)
                .ok_or_else(|| anyhow!("v0.2 probe evaluation returned no results"))?;
            for (index, result) in results.iter().enumerate() {
                let record = get_path(result, &["record"])
                    .cloned()
                    .ok_or_else(|| anyhow!("v0.2 result {index} has no record"))?;
                let bundle = serde_yaml::to_value(json!({
                    "spec": "rms/trace-bundle/v0.1",
                    "machine": instance.binding.machine,
                    "records": [serde_json::to_value(record)?]
                }))?;
                fs::write(&output_path, serde_json::to_vec_pretty(&bundle)?)?;
                validate_probe_trace_shape(&bundle)?;
                let mut trace = super::build_trace_report(&output_path)?;
                apply_probe_trace_conformance(
                    &output_path,
                    &instance.binding.implementation,
                    &mut trace,
                );
                if trace_has_errors(&trace) {
                    bail!("v0.2 probe evaluation result {index} is nonconforming");
                }
            }
        } else {
            validate_probe_trace_shape(&output_yaml)?;
            let mut trace = super::build_trace_report(&output_path)?;
            apply_probe_trace_conformance(
                &output_path,
                &instance.binding.implementation,
                &mut trace,
            );
            if trace_has_errors(&trace) {
                bail!(
                    "probe runner `{}` returned a nonconforming transition",
                    instance.binding.runner
                );
            }
        }
        serde_json::to_value(output_yaml).context("failed to normalize probe runner output")
    })();
    let _ = fs::remove_dir_all(&temp_root);
    result
}

fn validate_unique_ids(assembly: &ProbeAssembly) -> Result<()> {
    let mut instance_ids = BTreeSet::new();
    for instance in &assembly.instances {
        if !instance_ids.insert(instance.id.as_str()) {
            bail!("duplicate probe instance id `{}`", instance.id);
        }
    }
    let mut stimulus_ids = BTreeSet::new();
    for (index, stimulus) in assembly.stimuli.iter().enumerate() {
        let id = stimulus
            .id
            .clone()
            .unwrap_or_else(|| format!("stimulus-{index}"));
        if !stimulus_ids.insert(id.clone()) {
            bail!("duplicate probe stimulus id `{id}`");
        }
    }
    let mut check_ids = BTreeSet::new();
    for check in &assembly.checks {
        if !check_ids.insert(check.id.as_str()) {
            bail!("duplicate probe check id `{}`", check.id);
        }
    }
    let mut substitute_ids = BTreeSet::new();
    for substitute in &assembly.substitutes {
        if !substitute_ids.insert(substitute.id.as_str()) {
            bail!("duplicate probe substitute id `{}`", substitute.id);
        }
    }
    let mut routing_keys = BTreeSet::new();
    for route in &assembly.routing {
        let key = (
            route.consumer.as_deref().unwrap_or("*"),
            route.binding.as_str(),
        );
        if !routing_keys.insert(key) {
            bail!(
                "duplicate probe routing selection for consumer `{}` and binding `{}`",
                key.0,
                key.1
            );
        }
    }
    let mut fault_keys = BTreeSet::new();
    for fault in &assembly.faults {
        if !fault_keys.insert((fault.route.as_str(), fault.kind)) {
            bail!(
                "duplicate probe fault declaration for route `{}` and kind `{}`",
                fault.route,
                fault_label(fault.kind)
            );
        }
    }
    Ok(())
}

fn resolve_implementation(base: &Path, source: &str) -> Result<PathBuf> {
    let path = base.join(source);
    let path = if path.is_dir() {
        path.join("implementation.yaml")
    } else {
        path
    };
    if !path.is_file() {
        bail!("probe implementation `{}` does not exist", path.display());
    }
    fs::canonicalize(&path).with_context(|| format!("failed to resolve `{}`", path.display()))
}

fn validate_normalized_input(input: &Value) -> Result<()> {
    let kind = input
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("probe input is missing kind"))?;
    if !matches!(kind, "command" | "observed-event" | "effect-result") {
        bail!("unsupported normalized probe input kind `{kind}`");
    }
    if input.get("name").and_then(Value::as_str).is_none() {
        bail!("probe input is missing name");
    }
    Ok(())
}

fn machine_declares_named_kind(manifest: &Value, kind: &str, name: &str, output: bool) -> bool {
    let field = match (output, kind) {
        (true, "command") | (false, "command") => "commands",
        (true, "event") => "events",
        (true, "effect") => "effects",
        (true, "reply") => "replies",
        (true, "rejection") => "rejections",
        (false, "observed-event") => "observed_events",
        (false, "effect-result") => "effect_results",
        _ => return false,
    };
    json_array(manifest, &["architecture", "machine", field])
        .iter()
        .filter_map(Value::as_str)
        .any(|candidate| candidate == name)
}

fn validate_input_payload(description: &Value, input: &Value) -> Result<()> {
    let kind = input
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("input is missing kind"))?;
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("input is missing name"))?;
    let declaration = description
        .get("inputs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|declaration| {
            declaration.get("kind").and_then(Value::as_str) == Some(kind)
                && declaration.get("name").and_then(Value::as_str) == Some(name)
        })
        .ok_or_else(|| anyhow!("input `{kind}:{name}` is absent from the probe description"))?;
    validate_payload_schema(
        declaration.get("data_schema"),
        input.get("data").unwrap_or(&Value::Null),
        &format!("input `{kind}:{name}`"),
    )
}

fn validate_state_payload(description: &Value, state: &Value) -> Result<()> {
    let name = state
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("state is missing name"))?;
    let declaration = description
        .get("states")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|declaration| declaration.get("name").and_then(Value::as_str) == Some(name))
        .ok_or_else(|| anyhow!("state `{name}` is absent from the probe description"))?;
    validate_payload_schema(
        declaration.get("data_schema"),
        state.get("data").unwrap_or(&Value::Null),
        &format!("state `{name}`"),
    )
}

fn validate_payload_schema(schema: Option<&Value>, data: &Value, label: &str) -> Result<()> {
    let Some(schema) = schema else {
        return Ok(());
    };
    let validator = validator_for(schema)
        .with_context(|| format!("{label} data schema could not be compiled"))?;
    let errors = validator
        .iter_errors(data)
        .map(|error| format!("{error} at `{}`", error.instance_path()))
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        bail!("{label} data is schema-invalid: {}", errors.join("; "));
    }
    Ok(())
}

fn record_outputs(record: &Value) -> Vec<OutputValue> {
    let Some(output) = record.get("output") else {
        return Vec::new();
    };
    let mut values = Vec::new();
    for (field, kind) in [
        ("events", "event"),
        ("commands", "command"),
        ("effects", "effect"),
    ] {
        for value in output
            .get(field)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(name) = value.get("name").and_then(Value::as_str) {
                values.push(OutputValue {
                    kind: kind.to_string(),
                    name: name.to_string(),
                    value: value.clone(),
                });
            }
        }
    }
    for (field, kind) in [("reply", "reply"), ("rejection", "rejection")] {
        if let Some(value) = output.get(field).filter(|value| !value.is_null()) {
            if let Some(name) = value.get("name").and_then(Value::as_str) {
                values.push(OutputValue {
                    kind: kind.to_string(),
                    name: name.to_string(),
                    value: value.clone(),
                });
            }
        }
    }
    values
}

fn envelope_from_route(
    cause: &Envelope,
    route: &ResolvedRoute,
    data: Value,
    delay: u64,
) -> Envelope {
    Envelope {
        id: format!("{}:{}", cause.id, route.id),
        idempotency_key: format!("{}:{}", cause.id, route.id),
        route: route.id.clone(),
        target: route.target.clone(),
        at: cause.at.saturating_add(delay),
        input: json!({
            "kind": route.to.kind,
            "name": route.to.name,
            "data": data
        }),
        source: Some(route.source.clone()),
        correlation_id: cause.correlation_id.clone(),
        causation_id: Some(cause.id.clone()),
        attempt: 1,
    }
}

fn check_failure(check: &CheckSpec, world: &World, detail: &str) -> ProbeFailure {
    ProbeFailure {
        check: check.id.clone(),
        step: world.step,
        time: world.time,
        message: detail.to_string(),
    }
}

fn empty_coverage() -> Coverage {
    Coverage {
        states: 0,
        schedules: 0,
        transitions: 0,
        transition_cases: BTreeSet::new(),
        routes: BTreeSet::new(),
        faults: BTreeSet::new(),
    }
}

fn advance_time(world: &mut World) {
    if world.queue.iter().all(|envelope| envelope.at > world.time) {
        if let Some(next) = world.queue.iter().map(|envelope| envelope.at).min() {
            world.time = next;
        }
    }
}

fn first_enabled_index(world: &World) -> Option<usize> {
    world
        .queue
        .iter()
        .position(|envelope| envelope.at <= world.time)
}

fn sort_queue(queue: &mut [Envelope]) {
    queue.sort_by(envelope_order);
}

fn envelope_order(left: &Envelope, right: &Envelope) -> std::cmp::Ordering {
    left.at
        .cmp(&right.at)
        .then(left.route.cmp(&right.route))
        .then(left.id.cmp(&right.id))
        .then(left.target.cmp(&right.target))
        .then(left.attempt.cmp(&right.attempt))
}

fn action_label(action: &StepAction) -> &'static str {
    match action {
        StepAction::Deliver => "deliver",
        StepAction::Fault(kind) => fault_label(*kind),
    }
}

fn fault_label(kind: FaultKind) -> &'static str {
    match kind {
        FaultKind::Delay => "delay",
        FaultKind::Duplicate => "duplicate",
        FaultKind::Drop => "drop",
        FaultKind::Timeout => "timeout",
    }
}

fn fault_key(route: &str, kind: FaultKind) -> String {
    format!("{route}:{}", fault_label(kind))
}

fn substitute_assignments(substitutes: &[&SubstituteSpec]) -> Vec<BTreeMap<String, usize>> {
    let mut assignments = vec![BTreeMap::new()];
    for substitute in substitutes
        .iter()
        .filter(|substitute| substitute.outcomes.len() > 1)
    {
        let mut expanded = Vec::new();
        for assignment in &assignments {
            for index in 0..substitute.outcomes.len() {
                let mut candidate = assignment.clone();
                candidate.insert(substitute.id.clone(), index);
                expanded.push(candidate);
            }
        }
        assignments = expanded;
    }
    assignments
}

fn encode_substitute_choices(
    substitutes: &[SubstituteSpec],
    source: &str,
    outputs: &[OutputValue],
    choices: &BTreeMap<String, usize>,
) -> Option<String> {
    let encoded = substitutes
        .iter()
        .filter(|substitute| {
            substitute.source == source
                && substitute.outcomes.len() > 1
                && outputs.iter().any(|output| {
                    output.kind == substitute.output.kind && output.name == substitute.output.name
                })
        })
        .filter_map(|substitute| {
            choices
                .get(&substitute.id)
                .map(|index| format!("{}={index}", substitute.id))
        })
        .collect::<Vec<_>>();
    (!encoded.is_empty()).then(|| encoded.join(","))
}

fn parse_substitute_choices(choice: Option<&str>) -> Result<BTreeMap<String, usize>> {
    let Some(choice) = choice else {
        return Ok(BTreeMap::new());
    };
    if let Some(index) = choice.strip_prefix("substitute-") {
        return Ok(BTreeMap::from([(
            "*".to_string(),
            index
                .parse::<usize>()
                .with_context(|| format!("invalid recorded substitute choice `{choice}`"))?,
        )]));
    }
    choice
        .split(',')
        .map(|item| {
            let (id, index) = item
                .split_once('=')
                .ok_or_else(|| anyhow!("invalid recorded substitute choice `{item}`"))?;
            Ok((
                id.to_string(),
                index
                    .parse::<usize>()
                    .with_context(|| format!("invalid recorded substitute index in `{item}`"))?,
            ))
        })
        .collect()
}

fn world_hash(world: &World) -> Result<String> {
    Ok(sha256_bytes(&serde_json::to_vec(&json!({
        "states": world.states,
        "queue": world.queue,
        "time": world.time,
        "step": world.step,
        "message_counts": world.message_counts,
        "fault_remaining": world.fault_remaining,
        "satisfied_within": world.satisfied_within,
        "protocol_states": world.protocol_states
    }))?))
}

fn transition_cache_key(digest: &str, state: &Value, input: &Value) -> Result<String> {
    Ok(sha256_bytes(&serde_json::to_vec(&json!({
        "implementation": digest,
        "state": state,
        "input": input
    }))?))
}

fn shrink_u64(value: u64) -> Vec<u64> {
    if value <= 1 {
        return Vec::new();
    }
    let mut values = vec![1, value / 2];
    values.sort_unstable();
    values.dedup();
    values
        .into_iter()
        .filter(|candidate| *candidate < value)
        .collect()
}

fn shrink_json(value: &Value) -> Vec<Value> {
    let mut candidates = Vec::new();
    match value {
        Value::Object(object) => {
            for key in object.keys() {
                let mut smaller = object.clone();
                smaller.remove(key);
                candidates.push(Value::Object(smaller));
            }
            for (key, nested) in object {
                for shrunk in shrink_json(nested) {
                    let mut smaller = object.clone();
                    smaller.insert(key.clone(), shrunk);
                    candidates.push(Value::Object(smaller));
                }
            }
        }
        Value::Array(items) if !items.is_empty() => {
            candidates.push(Value::Array(Vec::new()));
            candidates.push(Value::Array(items[..items.len() / 2].to_vec()));
        }
        Value::String(value) if !value.is_empty() => {
            candidates.push(Value::String(String::new()));
            candidates.push(Value::String(
                value.chars().take(value.chars().count() / 2).collect(),
            ));
        }
        Value::Number(number) if number.as_i64() != Some(0) => {
            candidates.push(json!(0));
        }
        Value::Bool(true) => candidates.push(Value::Bool(false)),
        _ => {}
    }
    candidates.sort_by(|left, right| {
        serde_json::to_string(left)
            .unwrap_or_default()
            .cmp(&serde_json::to_string(right).unwrap_or_default())
    });
    candidates.dedup();
    candidates
}

fn input_name(input: &Value) -> String {
    input
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("<missing>")
        .to_string()
}

fn input_kind(manifest: &Value, name: &str) -> Result<&'static str> {
    for (field, kind) in [
        ("commands", "command"),
        ("observed_events", "observed-event"),
        ("effect_results", "effect-result"),
    ] {
        if json_array(manifest, &["architecture", "machine", field])
            .iter()
            .filter_map(Value::as_str)
            .any(|candidate| candidate == name)
        {
            return Ok(kind);
        }
    }
    bail!("protocol receive case `{name}` is not a declared machine input")
}

fn probe_protocol(manifest: &Value) -> &str {
    get_json_str(manifest, &["architecture", "probe", "protocol"])
        .unwrap_or("rms/machine-probe/v0.1")
}

fn get_json_str<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str()
}

fn json_array<'a>(value: &'a Value, path: &[&str]) -> &'a [Value] {
    let mut current = value;
    for segment in path {
        let Some(next) = current.get(*segment) else {
            return &[];
        };
        current = next;
    }
    current.as_array().map(Vec::as_slice).unwrap_or(&[])
}

fn load_input(path: &Path) -> Result<(Value, PathBuf)> {
    if path == Path::new("-") {
        let mut source = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut source)
            .context("failed to read probe assembly from stdin")?;
        let yaml: serde_yaml::Value =
            serde_yaml::from_str(&source).context("failed to parse probe assembly from stdin")?;
        return Ok((
            serde_json::to_value(yaml)?,
            std::env::current_dir().context("failed to resolve current directory")?,
        ));
    }
    let value = load_json_or_yaml(path)?;
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to resolve current directory")?
            .join(path)
    };
    Ok((
        value,
        absolute
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf(),
    ))
}

fn load_json_or_yaml(path: &Path) -> Result<Value> {
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read `{}`", path.display()))?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&source)
        .with_context(|| format!("failed to parse `{}`", path.display()))?;
    serde_json::to_value(yaml).context("failed to normalize YAML")
}

fn validate_schema(value: &Value, source: &str, label: &str) -> Result<()> {
    let schema: Value = serde_json::from_str(source)
        .with_context(|| format!("embedded {label} schema is invalid"))?;
    let validator = validator_for(&schema)
        .with_context(|| format!("embedded {label} schema could not be compiled"))?;
    let errors = validator
        .iter_errors(value)
        .map(|error| format!("{error} at `{}`", error.instance_path()))
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        bail!("invalid {label}: {}", errors.join("; "));
    }
    Ok(())
}

fn write_artifact(path: &Path, value: &Value) -> Result<()> {
    let bytes = if path.extension().and_then(|value| value.to_str()) == Some("json") {
        serde_json::to_vec_pretty(value)?
    } else {
        serde_yaml::to_string(value)?.into_bytes()
    };
    fs::write(path, bytes).with_context(|| format!("failed to write `{}`", path.display()))
}

fn print_value(value: &Value, json_output: bool) -> Result<()> {
    if json_output {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        print!("{}", serde_yaml::to_string(value)?);
    }
    Ok(())
}

fn print_trace(trace: &SystemTrace) {
    println!("RMS probe assembly: {}", trace.result);
    println!(
        "coverage: {} states, {} schedules, {} transitions",
        trace.coverage.states, trace.coverage.schedules, trace.coverage.transitions
    );
    println!("timeline:");
    for entry in &trace.timeline {
        if entry.action == "deliver" {
            println!(
                "  {} @{} {} --{}--> {} [{}]",
                entry.step,
                entry.time,
                entry.source.as_deref().unwrap_or("external"),
                input_name(&entry.input),
                entry.target,
                entry.transition_case.as_deref().unwrap_or("<missing>")
            );
        } else {
            println!(
                "  {} @{} {} {} -> {}",
                entry.step, entry.time, entry.action, entry.envelope, entry.target
            );
        }
    }
    if let Some(failure) = &trace.failure {
        println!(
            "first_failure: {} at step {}: {}",
            failure.check, failure.step, failure.message
        );
    }
}

fn git_revision(root: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembly_schema_accepts_arbitrary_instance_count_and_rejects_unknown_fields() {
        let schema = include_str!("../../../../../schemas/probe-assembly.schema.json");
        let valid = json!({
            "spec": ASSEMBLY_SPEC,
            "instances": [
                {"id": "one", "implementation": "one/implementation.yaml"},
                {"id": "two", "implementation": "two/implementation.yaml"},
                {"id": "three", "implementation": "two/implementation.yaml"}
            ],
            "stimuli": [{
                "target": "one",
                "input": {"kind": "command", "name": "Start", "data": {}}
            }]
        });
        validate_schema(&valid, schema, "probe assembly").unwrap();
        let mut invalid = valid;
        invalid
            .as_object_mut()
            .unwrap()
            .insert("topology".to_string(), json!("guessed"));
        assert!(validate_schema(&invalid, schema, "probe assembly").is_err());
    }

    #[test]
    fn scheduler_orders_same_time_work_by_route_then_envelope_then_target() {
        let mut queue = vec![
            Envelope {
                id: "b".to_string(),
                idempotency_key: "b".to_string(),
                route: "z".to_string(),
                target: "a".to_string(),
                at: 0,
                input: json!({"kind":"command","name":"B"}),
                source: None,
                correlation_id: "b".to_string(),
                causation_id: None,
                attempt: 1,
            },
            Envelope {
                id: "c".to_string(),
                idempotency_key: "c".to_string(),
                route: "a".to_string(),
                target: "b".to_string(),
                at: 0,
                input: json!({"kind":"command","name":"C"}),
                source: None,
                correlation_id: "c".to_string(),
                causation_id: None,
                attempt: 1,
            },
            Envelope {
                id: "a".to_string(),
                idempotency_key: "a".to_string(),
                route: "a".to_string(),
                target: "c".to_string(),
                at: 0,
                input: json!({"kind":"command","name":"A"}),
                source: None,
                correlation_id: "a".to_string(),
                causation_id: None,
                attempt: 1,
            },
        ];
        sort_queue(&mut queue);
        assert_eq!(
            queue
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "c", "b"]
        );
    }

    #[test]
    fn canonical_probe_evidence_requires_exhausted_exploration() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repository root");
        let assembly_path = repository.join("examples/probes/series.yaml");
        assert!(verify_evidence(&assembly_path, 30)
            .expect("canonical assembly verification")
            .is_some());

        let (assembly_value, base_dir) = load_input(&assembly_path).unwrap();
        let assembly: ProbeAssembly = serde_json::from_value(assembly_value.clone()).unwrap();
        let mut engine =
            Engine::new(assembly, assembly_value, base_dir, None, None, None, 30).unwrap();
        let (deterministic, _) = engine.deterministic().unwrap();
        let path = std::env::temp_dir().join(format!(
            "rms-probe-evidence-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, serde_json::to_vec_pretty(&deterministic).unwrap()).unwrap();
        assert!(verify_evidence(&path, 30).is_err());

        let (explored, _) = engine.explore().unwrap();
        fs::write(&path, serde_json::to_vec_pretty(&explored).unwrap()).unwrap();
        assert!(verify_evidence(&path, 30).unwrap().is_some());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn every_declared_transport_fault_exposes_the_seeded_failure() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repository root");
        let path = repository.join("examples/probes/series-faults.yaml");
        let (value, base_dir) = load_input(&path).unwrap();
        let assembly: ProbeAssembly = serde_json::from_value(value).unwrap();
        for kind in [
            FaultKind::Delay,
            FaultKind::Duplicate,
            FaultKind::Drop,
            FaultKind::Timeout,
        ] {
            let mut candidate = assembly.clone();
            candidate.faults.retain(|fault| fault.kind == kind);
            let candidate_value = serde_json::to_value(&candidate).unwrap();
            let mut engine = Engine::new(
                candidate,
                candidate_value,
                base_dir.clone(),
                None,
                None,
                None,
                30,
            )
            .unwrap();
            let (trace, counterexample) = engine.explore().unwrap();
            assert_eq!(
                trace.result,
                "fail",
                "{} did not expose the seeded failure",
                fault_label(kind)
            );
            assert!(counterexample.is_some());
        }
    }

    #[test]
    fn v01_adapter_uses_the_one_transition_fallback() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repository root");
        let source = repository.join("examples/probe-topologies/source");
        let root = std::env::temp_dir().join(format!(
            "rms-v01-probe-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::copy(
            source.join("machine_probe.fixture"),
            root.join("machine_probe.fixture"),
        )
        .unwrap();
        let manifest = fs::read_to_string(source.join("implementation.fixture"))
            .unwrap()
            .replace("rms/machine-probe/v0.2", "rms/machine-probe/v0.1");
        fs::write(root.join("implementation.yaml"), manifest).unwrap();
        let assembly_value = json!({
            "spec": ASSEMBLY_SPEC,
            "instances": [{"id":"source","implementation":"implementation.yaml"}],
            "stimuli": [{
                "id":"start",
                "target":"source",
                "input":{"kind":"command","name":"Start","data":{}}
            }],
            "substitutes": [{
                "id":"work-substitute",
                "source":"source",
                "output":{"kind":"effect","name":"Work"},
                "outcomes":[{
                    "target":"source",
                    "input":{"kind":"effect-result","name":"Done","data":{}}
                }]
            }],
            "checks": [{
                "id":"ready",
                "when":"quiescent",
                "assert":{"kind":"state","instance":"source","equals":{"name":"Ready","data":{}}}
            }]
        });
        let assembly: ProbeAssembly = serde_json::from_value(assembly_value.clone()).unwrap();
        let mut engine =
            Engine::new(assembly, assembly_value, root.clone(), None, None, None, 30).unwrap();
        let (trace, _) = engine.explore().unwrap();
        assert_eq!(trace.result, "pass");
        assert_eq!(trace.coverage.transitions, 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn route_resolution_rejects_invented_missing_incompatible_and_ambiguous_wiring() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repository root");
        let path = repository.join("examples/probes/series.yaml");
        let (value, base_dir) = load_input(&path).unwrap();
        let valid: ProbeAssembly = serde_json::from_value(value).unwrap();

        let build = |assembly: ProbeAssembly| {
            let value = serde_json::to_value(&assembly).unwrap();
            Engine::new(assembly, value, base_dir.clone(), None, None, None, 30)
        };
        assert!(build(valid.clone()).is_ok());

        let mut invented = valid.clone();
        invented.routing.push(RoutingSelection {
            consumer: Some("source".to_string()),
            binding: "invented-route".to_string(),
            provider: "worker".to_string(),
        });
        assert!(build(invented).is_err());

        let mut missing = valid.clone();
        missing.instances.retain(|instance| instance.id != "worker");
        assert!(build(missing).is_err());

        let mut incompatible = valid.clone();
        incompatible.routing.push(RoutingSelection {
            consumer: Some("source".to_string()),
            binding: "work-provider".to_string(),
            provider: "source".to_string(),
        });
        assert!(build(incompatible).is_err());

        let mut ambiguous = valid;
        let mut second_worker = ambiguous
            .instances
            .iter()
            .find(|instance| instance.id == "worker")
            .cloned()
            .unwrap();
        second_worker.id = "worker-2".to_string();
        ambiguous.instances.push(second_worker);
        assert!(build(ambiguous).is_err());
    }

    #[test]
    fn breadth_first_exploration_returns_the_known_shortest_schedule_failure() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repository root");
        let path = repository.join("examples/probes/concurrent-order-failure.yaml");
        let (value, base_dir) = load_input(&path).unwrap();
        let assembly: ProbeAssembly = serde_json::from_value(value.clone()).unwrap();
        let mut engine = Engine::new(assembly, value, base_dir, None, None, None, 30).unwrap();
        let (trace, counterexample) = engine.explore().unwrap();
        assert_eq!(trace.result, "fail");
        assert_eq!(trace.failure.as_ref().map(|failure| failure.step), Some(2));
        assert_eq!(
            counterexample
                .as_ref()
                .map(|counterexample| counterexample.decisions.len()),
            Some(2)
        );
    }

    #[test]
    fn bound_exhaustion_is_inconclusive_never_pass() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repository root");
        let path = repository.join("examples/probes/series.yaml");
        let (value, base_dir) = load_input(&path).unwrap();
        let assembly: ProbeAssembly = serde_json::from_value(value.clone()).unwrap();
        let mut engine = Engine::new(assembly, value, base_dir, Some(1), None, None, 30).unwrap();
        let (trace, counterexample) = engine.explore().unwrap();
        assert_eq!(trace.result, "inconclusive");
        assert!(!trace.exhausted);
        assert!(counterexample.is_none());
    }

    #[test]
    fn repeated_explorations_serialize_identically() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repository root");
        let path = repository.join("examples/probes/fan-out.yaml");
        let (value, base_dir) = load_input(&path).unwrap();
        let run = || {
            let assembly: ProbeAssembly = serde_json::from_value(value.clone()).unwrap();
            let mut engine = Engine::new(
                assembly,
                value.clone(),
                base_dir.clone(),
                None,
                None,
                None,
                30,
            )
            .unwrap();
            let (trace, counterexample) = engine.explore().unwrap();
            assert!(counterexample.is_none());
            serde_json::to_vec(&trace).unwrap()
        };
        assert_eq!(run(), run());
    }
}
