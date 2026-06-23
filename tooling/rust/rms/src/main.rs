use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use serde_yaml::Value as YamlValue;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write as IoWrite;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use syn::visit::{self, Visit};
use syn::{
    Attribute, ExprMacro, ExprMethodCall, Fields, ImplItem, Item, ItemStruct, Meta, Type, UseTree,
    Visibility,
};
use toml::Value as TomlValue;
use walkdir::WalkDir;

const VALIDATOR_NAME: &str = "rms";
const VALIDATOR_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_RUN_ROOT: &str = ".rms/runs";
const WORKBENCH_CONFIG_PATH: &str = ".rms/config.yaml";
const CODEX_PLUGIN_PATH: &str = "integrations/codex/rms";
const CANONICAL_SKILLS: &[&str] = &[
    "inspect-module",
    "implement-change",
    "refactor-module",
    "prune-module",
    "add-module",
    "evolve-contract",
    "compose-modules",
    "verify-module",
];
const INIT_AGENT_SKILLS: &[(&str, &str)] = &[
    ("README.md", include_str!("../assets/skills/README.md")),
    (
        "add-module/SKILL.md",
        include_str!("../assets/skills/add-module/SKILL.md"),
    ),
    (
        "compose-modules/SKILL.md",
        include_str!("../assets/skills/compose-modules/SKILL.md"),
    ),
    (
        "evolve-contract/SKILL.md",
        include_str!("../assets/skills/evolve-contract/SKILL.md"),
    ),
    (
        "implement-change/SKILL.md",
        include_str!("../assets/skills/implement-change/SKILL.md"),
    ),
    (
        "inspect-module/SKILL.md",
        include_str!("../assets/skills/inspect-module/SKILL.md"),
    ),
    (
        "prune-module/SKILL.md",
        include_str!("../assets/skills/prune-module/SKILL.md"),
    ),
    (
        "refactor-module/SKILL.md",
        include_str!("../assets/skills/refactor-module/SKILL.md"),
    ),
    (
        "verify-module/SKILL.md",
        include_str!("../assets/skills/verify-module/SKILL.md"),
    ),
];

#[derive(Parser)]
#[command(name = "rms")]
#[command(version)]
#[command(about = "Reliable Modular Systems reference CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate RMS manifests and referenced artifacts.
    Validate {
        /// Root directory to scan when explicit paths are not supplied.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Validate a specific module manifest.
        #[arg(long)]
        module: Vec<PathBuf>,

        /// Validate a specific system manifest.
        #[arg(long)]
        system: Vec<PathBuf>,

        /// Validate a specific context map.
        #[arg(long = "context-map")]
        context_map: Vec<PathBuf>,

        /// Validate a specific contract manifest.
        #[arg(long)]
        contract: Vec<PathBuf>,

        /// Validate a specific implementation binding.
        #[arg(long)]
        implementation: Vec<PathBuf>,

        /// Validate a specific conformance report.
        #[arg(long)]
        conformance: Vec<PathBuf>,

        /// Emit machine-readable diagnostics.
        #[arg(long)]
        json: bool,
    },

    /// Print a concise module brief.
    Inspect {
        /// Path to module.yaml or *.module.yaml.
        module: PathBuf,
    },

    /// Explain a module in a human-readable form, optionally focused by a question.
    Explain {
        /// Optional module path followed by an optional question. If omitted, the module is inferred from --root when unambiguous.
        subject: Vec<String>,

        /// Explicit path to module.yaml or *.module.yaml.
        #[arg(long)]
        module: Option<PathBuf>,

        /// Repository or system root used to locate system/context/glossary files.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Use the default AI provider from .rms/config.yaml.
        #[arg(long)]
        ai: bool,

        /// Optional AI provider to answer using a bounded RMS prompt.
        #[arg(long)]
        provider: Option<Provider>,

        /// Save a run record under .rms/runs.
        #[arg(long)]
        record: bool,

        /// Directory where run records are written.
        #[arg(long)]
        run_root: Option<PathBuf>,

        /// Optional model name passed to the provider.
        #[arg(long)]
        model: Option<String>,

        /// Sandbox mode passed to Codex provider execution.
        #[arg(long)]
        sandbox: Option<CodexSandbox>,

        /// Writable scope passed to Codex provider execution.
        #[arg(long = "write-scope")]
        write_scope: Option<ProviderWriteScope>,
    },

    /// Check local RMS and optional AI-provider readiness.
    Diagnose {
        /// Repository or system root to inspect.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Emit a machine-readable readiness report.
        #[arg(long)]
        json: bool,
    },

    /// Render a versioned RMS workbench prompt for a module task.
    Prompt {
        /// Prompt workflow to render.
        kind: PromptKind,

        /// Path to module.yaml or *.module.yaml.
        module: PathBuf,

        /// Optional task description to include in the prompt.
        #[arg(long)]
        task: Option<String>,

        /// Repository or system root used to locate system/context/glossary files.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Git diff spec to include. For review prompts, omitted means current working diff.
        #[arg(long)]
        diff: Option<String>,

        /// Include a derived RMS impact prelude. Only supported for review prompts.
        #[arg(long)]
        impact: bool,

        /// Use the default AI provider from .rms/config.yaml.
        #[arg(long)]
        ai: bool,

        /// Optional AI provider to execute the rendered prompt.
        #[arg(long)]
        provider: Option<Provider>,

        /// Save a run record under .rms/runs even without provider execution.
        #[arg(long)]
        record: bool,

        /// Directory where run records are written.
        #[arg(long)]
        run_root: Option<PathBuf>,

        /// Optional model name passed to the provider.
        #[arg(long)]
        model: Option<String>,

        /// Sandbox mode passed to Codex provider execution.
        #[arg(long)]
        sandbox: Option<CodexSandbox>,

        /// Writable scope passed to Codex provider execution.
        #[arg(long = "write-scope")]
        write_scope: Option<ProviderWriteScope>,
    },

    /// Render an advisory RMS implementation plan prompt for a module task.
    Plan {
        /// Path to module.yaml or *.module.yaml.
        module: PathBuf,

        /// Task description to plan.
        #[arg(long)]
        task: String,

        /// Repository or system root used to locate system/context/glossary files.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Use the default AI provider from .rms/config.yaml.
        #[arg(long)]
        ai: bool,

        /// Optional AI provider to execute the rendered prompt.
        #[arg(long)]
        provider: Option<Provider>,

        /// Save a run record under .rms/runs even without provider execution.
        #[arg(long)]
        record: bool,

        /// Directory where run records are written.
        #[arg(long)]
        run_root: Option<PathBuf>,

        /// Optional model name passed to the provider.
        #[arg(long)]
        model: Option<String>,

        /// Sandbox mode passed to Codex provider execution.
        #[arg(long)]
        sandbox: Option<CodexSandbox>,

        /// Writable scope passed to Codex provider execution.
        #[arg(long = "write-scope")]
        write_scope: Option<ProviderWriteScope>,
    },

    /// Render an advisory RMS review prompt for the current or requested diff.
    Review {
        /// Path to module.yaml or *.module.yaml.
        module: PathBuf,

        /// Optional task or review focus.
        #[arg(long)]
        task: Option<String>,

        /// Repository or system root used to locate system/context/glossary files.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Git diff spec to include. Omitted means current working diff.
        #[arg(long)]
        diff: Option<String>,

        /// Include a derived RMS impact prelude before the diff.
        #[arg(long)]
        impact: bool,

        /// Use the default AI provider from .rms/config.yaml.
        #[arg(long)]
        ai: bool,

        /// Optional AI provider to execute the rendered prompt.
        #[arg(long)]
        provider: Option<Provider>,

        /// Save a run record under .rms/runs even without provider execution.
        #[arg(long)]
        record: bool,

        /// Directory where run records are written.
        #[arg(long)]
        run_root: Option<PathBuf>,

        /// Optional model name passed to the provider.
        #[arg(long)]
        model: Option<String>,

        /// Sandbox mode passed to Codex provider execution.
        #[arg(long)]
        sandbox: Option<CodexSandbox>,

        /// Writable scope passed to Codex provider execution.
        #[arg(long = "write-scope")]
        write_scope: Option<ProviderWriteScope>,
    },

    /// Classify RMS semantic impact for the current or requested git diff.
    Impact {
        /// Optional git diff spec. Omitted means staged, unstaged, and untracked working-tree paths.
        diff: Option<String>,

        /// Repository or system root used to discover RMS artifacts and read git state.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Run executable RMS checks selected from git impact analysis.
    Gate {
        /// Optional git diff spec. Omitted means staged, unstaged, and untracked working-tree paths.
        diff: Option<String>,

        /// Repository or system root used to discover RMS artifacts and read git state.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Print the selected checks without running them.
        #[arg(long)]
        dry_run: bool,

        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Render an advisory RMS refactor prompt for behavior-preserving module changes.
    Refactor {
        /// Path to module.yaml or *.module.yaml.
        module: PathBuf,

        /// Refactor task description.
        #[arg(long)]
        task: String,

        /// Repository or system root used to locate system/context/glossary files.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Use the default AI provider from .rms/config.yaml.
        #[arg(long)]
        ai: bool,

        /// Optional AI provider to execute the rendered prompt.
        #[arg(long)]
        provider: Option<Provider>,

        /// Save a run record under .rms/runs even without provider execution.
        #[arg(long)]
        record: bool,

        /// Directory where run records are written.
        #[arg(long)]
        run_root: Option<PathBuf>,

        /// Optional model name passed to the provider.
        #[arg(long)]
        model: Option<String>,

        /// Sandbox mode passed to Codex provider execution.
        #[arg(long)]
        sandbox: Option<CodexSandbox>,

        /// Writable scope passed to Codex provider execution.
        #[arg(long = "write-scope")]
        write_scope: Option<ProviderWriteScope>,
    },

    /// Render an advisory RMS implementation prompt for a module change.
    Implement {
        /// Path to module.yaml or *.module.yaml.
        module: PathBuf,

        /// Implementation task description.
        #[arg(long)]
        task: String,

        /// Repository or system root used to locate system/context/glossary files.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Use the default AI provider from .rms/config.yaml.
        #[arg(long)]
        ai: bool,

        /// Optional AI provider to execute the rendered prompt.
        #[arg(long)]
        provider: Option<Provider>,

        /// Save a run record under .rms/runs even without provider execution.
        #[arg(long)]
        record: bool,

        /// Directory where run records are written.
        #[arg(long)]
        run_root: Option<PathBuf>,

        /// Optional model name passed to the provider.
        #[arg(long)]
        model: Option<String>,

        /// Sandbox mode passed to Codex provider execution.
        #[arg(long)]
        sandbox: Option<CodexSandbox>,

        /// Writable scope passed to Codex provider execution.
        #[arg(long = "write-scope")]
        write_scope: Option<ProviderWriteScope>,
    },

    /// Render an advisory RMS contract-evolution prompt for public surface changes.
    #[command(name = "evolve-contract", alias = "evolve")]
    EvolveContract {
        /// Path to module.yaml or *.module.yaml.
        module: PathBuf,

        /// Contract evolution task description.
        #[arg(long)]
        task: String,

        /// Repository or system root used to locate system/context/glossary files.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Use the default AI provider from .rms/config.yaml.
        #[arg(long)]
        ai: bool,

        /// Optional AI provider to execute the rendered prompt.
        #[arg(long)]
        provider: Option<Provider>,

        /// Save a run record under .rms/runs even without provider execution.
        #[arg(long)]
        record: bool,

        /// Directory where run records are written.
        #[arg(long)]
        run_root: Option<PathBuf>,

        /// Optional model name passed to the provider.
        #[arg(long)]
        model: Option<String>,

        /// Sandbox mode passed to Codex provider execution.
        #[arg(long)]
        sandbox: Option<CodexSandbox>,

        /// Writable scope passed to Codex provider execution.
        #[arg(long = "write-scope")]
        write_scope: Option<ProviderWriteScope>,
    },

    /// Render an advisory RMS evidence prompt for proving a changed promise.
    Evidence {
        /// Path to module.yaml or *.module.yaml.
        module: PathBuf,

        /// Evidence task description.
        #[arg(long)]
        task: String,

        /// Repository or system root used to locate system/context/glossary files.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Use the default AI provider from .rms/config.yaml.
        #[arg(long)]
        ai: bool,

        /// Optional AI provider to execute the rendered prompt.
        #[arg(long)]
        provider: Option<Provider>,

        /// Save a run record under .rms/runs even without provider execution.
        #[arg(long)]
        record: bool,

        /// Directory where run records are written.
        #[arg(long)]
        run_root: Option<PathBuf>,

        /// Optional model name passed to the provider.
        #[arg(long)]
        model: Option<String>,

        /// Sandbox mode passed to Codex provider execution.
        #[arg(long)]
        sandbox: Option<CodexSandbox>,

        /// Writable scope passed to Codex provider execution.
        #[arg(long = "write-scope")]
        write_scope: Option<ProviderWriteScope>,
    },

    /// Inspect saved RMS workbench run records.
    Run {
        #[command(subcommand)]
        command: RunCommands,
    },

    /// Manage RMS workbench configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Run release-readiness checks for the RMS workbench and adapter package.
    Release {
        #[command(subcommand)]
        command: ReleaseCommands,
    },

    /// Build a bounded agent context packet for a module.
    Context {
        /// Path to module.yaml or *.module.yaml.
        module: PathBuf,

        /// Optional task description to include in the packet.
        #[arg(long)]
        task: Option<String>,

        /// Repository or system root used to locate system/context/glossary files.
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },

    /// Generate an interactive module atlas from canonical RMS artifacts.
    Atlas {
        /// Path to module.yaml or *.module.yaml.
        module: PathBuf,

        /// Repository or system root used to resolve canonical context files.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Output directory. Defaults to <root>/dist/rms-atlas/<module-name>.
        #[arg(long)]
        output: Option<PathBuf>,

        /// Replace an existing output directory.
        #[arg(long)]
        force: bool,
    },

    /// Emit a conformance report for a module.
    Conformance {
        /// Path to module.yaml or *.module.yaml.
        module: PathBuf,

        /// Optional implementation binding.
        #[arg(long)]
        implementation: Option<PathBuf>,

        /// Optional output file. Prints to stdout when omitted.
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Compare two module manifests and classify compatibility impact.
    CheckCompat {
        /// Previously accepted module manifest.
        old: PathBuf,

        /// Candidate replacement module manifest.
        new: PathBuf,

        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Check whether discovered modules can compose through declared public contracts.
    Compose {
        /// Root directory containing RMS system, context map, and module manifests.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Run the verification command declared by an implementation binding.
    Verify {
        /// Path to implementation.yaml.
        implementation: PathBuf,

        /// Print the command without executing it.
        #[arg(long)]
        dry_run: bool,
    },

    /// Assemble a portable RMS module package directory.
    Package {
        /// Path to module.yaml or *.module.yaml.
        module: PathBuf,

        /// Output directory. Defaults to dist/<module>-<version>.rms.
        #[arg(long)]
        output: Option<PathBuf>,

        /// Replace an existing output directory.
        #[arg(long)]
        force: bool,
    },

    /// Verify a portable RMS module package directory.
    VerifyPackage {
        /// Path to a package directory containing PACKAGE.json.
        package: PathBuf,

        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Scaffold a new RMS system in a directory.
    Init {
        /// Directory to initialize.
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Stable system name.
        #[arg(long)]
        name: String,

        /// One-sentence system purpose.
        #[arg(long)]
        purpose: String,

        /// Initial system version.
        #[arg(long, default_value = "0.1.0")]
        version: String,

        /// Initial bounded context names.
        #[arg(long = "context")]
        context: Vec<String>,
    },

    /// Scaffold a new RMS module directory.
    AddModule {
        /// Directory where module artifacts should be created.
        path: PathBuf,

        /// Stable module name.
        #[arg(long)]
        name: String,

        /// One-sentence module purpose.
        #[arg(long)]
        purpose: String,

        /// Module kind.
        #[arg(long, default_value = "module")]
        kind: String,

        /// Declared RMS profiles. `core` is added automatically when omitted.
        #[arg(long = "profile")]
        profile: Vec<String>,

        /// Optional implementation binding to scaffold. Currently supports `rust`, `swift`, and `executable`.
        #[arg(long)]
        binding: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize)]
struct Diagnostic {
    severity: Severity,
    check: String,
    path: String,
    message: String,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum PromptKind {
    Explain,
    Plan,
    Review,
    Refactor,
    Implement,
    #[value(name = "evolve-contract", alias = "evolve")]
    EvolveContract,
    Prune,
    Evidence,
    Drift,
}

#[derive(Subcommand)]
enum RunCommands {
    /// List saved workbench run records.
    List {
        /// Repository or system root used to locate the run directory.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Directory where run records are stored.
        #[arg(long)]
        run_root: Option<PathBuf>,
    },

    /// Inspect one saved workbench run record.
    Inspect {
        /// Run id or path to a run directory.
        run: PathBuf,

        /// Repository or system root used when resolving a run id.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Directory where run records are stored.
        #[arg(long)]
        run_root: Option<PathBuf>,
    },

    /// Inspect the newest saved workbench run record.
    Latest {
        /// Repository or system root used when resolving the run directory.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Directory where run records are stored.
        #[arg(long)]
        run_root: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Create a default .rms/config.yaml.
    Init {
        /// Repository or system root where .rms/config.yaml is written.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Default provider to write.
        #[arg(long, default_value = "codex")]
        provider: Provider,

        /// Optional Codex model to write.
        #[arg(long)]
        model: Option<String>,

        /// Run-record directory to write.
        #[arg(long, default_value = ".rms/runs")]
        run_root: PathBuf,

        /// Overwrite an existing config file.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ReleaseCommands {
    /// Run the canonical release-readiness gate.
    Check {
        /// Repository root to check.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Skip `cargo package`; useful when checking offline.
        #[arg(long)]
        skip_cargo_package: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Provider {
    None,
    Codex,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CodexSandbox {
    #[value(name = "read-only")]
    ReadOnly,
    #[value(name = "workspace-write")]
    WorkspaceWrite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum ProviderWriteScope {
    Module,
    Root,
}

#[derive(Clone, Debug)]
struct PromptRunOptions {
    provider: Provider,
    record: bool,
    run_root: PathBuf,
    model: Option<String>,
    sandbox: CodexSandbox,
    write_scope: ProviderWriteScope,
}

#[derive(Clone, Debug)]
struct RawPromptRunOptions {
    ai: bool,
    provider: Option<Provider>,
    record: bool,
    run_root: Option<PathBuf>,
    model: Option<String>,
    sandbox: Option<CodexSandbox>,
    write_scope: Option<ProviderWriteScope>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct WorkbenchConfig {
    ai: AiConfig,
    runs: RunsConfig,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct AiConfig {
    default_provider: Option<String>,
    codex: CodexConfig,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct CodexConfig {
    model: Option<String>,
    sandbox: Option<String>,
    write_scope: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RunsConfig {
    directory: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct LoadedWorkbenchConfig {
    path: PathBuf,
    value: WorkbenchConfig,
}

#[derive(Debug, Serialize)]
struct DiagnoseReport {
    validator: &'static str,
    version: &'static str,
    root: String,
    repository: Vec<ReadinessItem>,
    config: ConfigReadiness,
    manifest_counts: BTreeMap<String, usize>,
    validation: ValidationReadiness,
    native_tools: Vec<CommandReadiness>,
    ai_providers: Vec<CommandReadiness>,
    run_records: RunRecordReadiness,
    guidance: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ReadinessItem {
    name: String,
    path: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct ConfigReadiness {
    path: String,
    status: String,
    default_provider: Option<String>,
    codex_model: Option<String>,
    codex_sandbox: Option<String>,
    codex_write_scope: Option<String>,
    run_directory: String,
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct ValidationReadiness {
    status: String,
    errors: usize,
    warnings: usize,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize)]
struct CommandReadiness {
    command: String,
    status: String,
    detail: Option<String>,
}

#[derive(Debug, Serialize)]
struct RunRecordReadiness {
    directory: String,
    status: String,
    message: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ImpactReport {
    result: ImpactResult,
    root: String,
    diff: Option<String>,
    source_revision: Option<String>,
    changed_paths: Vec<ImpactPath>,
    affected_modules: Vec<ImpactModuleImpact>,
    findings: Vec<ImpactFinding>,
    recommended_checks: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ImpactModuleImpact {
    name: String,
    manifest: String,
    implementation: Option<String>,
    changed_paths: Vec<String>,
    categories: Vec<ImpactCategory>,
}

#[derive(Clone, Debug)]
struct GatePlan {
    report: GateReport,
    actions: Vec<GateCheckAction>,
}

#[derive(Clone, Debug)]
enum GateCheckAction {
    ValidateRoot,
    ComposeRoot,
    VerifyImplementation(PathBuf),
}

#[derive(Clone, Debug, Serialize)]
struct GateReport {
    result: GateResult,
    root: String,
    diff: Option<String>,
    source_revision: Option<String>,
    impact_result: ImpactResult,
    affected_modules: Vec<String>,
    executable_checks: Vec<GateCheck>,
    manual_checks: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum GateResult {
    Pending,
    Pass,
    Fail,
}

#[derive(Clone, Debug, Serialize)]
struct GateCheck {
    command: String,
    status: GateCheckStatus,
    message: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum GateCheckStatus {
    Pending,
    Pass,
    Fail,
}

impl PromptKind {
    fn label(self) -> &'static str {
        match self {
            PromptKind::Plan => "plan",
            PromptKind::Explain => "explain",
            PromptKind::Review => "review",
            PromptKind::Refactor => "refactor",
            PromptKind::Implement => "implement",
            PromptKind::EvolveContract => "evolve-contract",
            PromptKind::Prune => "prune",
            PromptKind::Evidence => "evidence",
            PromptKind::Drift => "drift",
        }
    }

    fn prompt_id(self) -> &'static str {
        match self {
            PromptKind::Plan => "rms.plan@v1",
            PromptKind::Explain => "rms.explain@v1",
            PromptKind::Review => "rms.review@v1",
            PromptKind::Refactor => "rms.refactor@v1",
            PromptKind::Implement => "rms.implement@v1",
            PromptKind::EvolveContract => "rms.evolve-contract@v1",
            PromptKind::Prune => "rms.prune@v1",
            PromptKind::Evidence => "rms.evidence@v1",
            PromptKind::Drift => "rms.drift@v1",
        }
    }

    fn default_task(self) -> Option<&'static str> {
        match self {
            PromptKind::Review => Some("review the diff for RMS conformance"),
            PromptKind::Explain => Some("explain how this module works"),
            PromptKind::Drift => {
                Some("identify drift between canonical RMS artifacts and implementation reality")
            }
            _ => None,
        }
    }

    fn includes_diff_by_default(self) -> bool {
        matches!(self, PromptKind::Review)
    }

    fn workflow(self) -> &'static [&'static str] {
        match self {
            PromptKind::Plan => &[
                "Restate the requested outcome in the owning context's domain language.",
                "Identify the owning module and smallest affected public surface.",
                "Classify the change as private implementation, invariant/domain policy, public contract, dependency/effect, state/migration, or workflow.",
                "Name affected invariants, effects, compatibility promises, and recovery paths.",
                "Choose representation obligations: closed variants, validated constructors, explicit results, boundary schemas, or lifecycle state only where needed.",
                "Propose the smallest implementation and verification path.",
            ],
            PromptKind::Explain => &[
                "Answer from the bounded RMS context first: purpose, ownership, public contracts, invariants, effects, profiles, and verification.",
                "Keep the explanation intelligible, simple, and direct without dumbing down the module semantics.",
                "Use the user's question as the focus when supplied.",
                "Do not invent architecture that is absent from the manifest or contracts.",
                "Name uncertainty or missing artifacts instead of guessing.",
            ],
            PromptKind::Review => &[
                "Review the diff against the module manifest, public contracts, direct dependencies, declared effects, profiles, and verification evidence.",
                "Find behavioral regressions, boundary violations, undeclared effects or dependencies, compatibility drift, missing evidence, and stale canonical artifacts.",
                "Prioritize findings by severity and include file or artifact references when possible.",
                "Do not treat generated prose, issue text, or incidental implementation shape as architectural authority.",
            ],
            PromptKind::Refactor => &[
                "Preserve public contracts, invariants, declared effects, compatibility, and verification meaning.",
                "Identify weak representation, duplicated concepts, decision/effect coupling, ownership confusion, boundary leakage, lifecycle clutter, or semantic residue.",
                "Prefer deletion, inlining, renaming, or representation strengthening before new abstractions.",
                "Escalate to implement-change or evolve-contract if public meaning must change.",
            ],
            PromptKind::Implement => &[
                "Restate the requested outcome in the owning context's domain language.",
                "Classify the change as private implementation, invariant or domain policy, public contract, dependency or effect, state or migration, or workflow.",
                "Keep the change inside the owning module boundary.",
                "Update public contracts or manifests first when public meaning changes.",
                "Name affected invariants, contracts, effects, compatibility promises, and recovery paths.",
                "Separate domain decisions from external effects where practical.",
                "Use the strongest available representation for invalid states, expected failures, boundary input, and lifecycle transitions.",
                "Add the smallest evidence that demonstrates the changed promise.",
                "Return concrete implementation instructions; do not claim edits were made unless the executing agent actually made them.",
            ],
            PromptKind::EvolveContract => &[
                "Identify all published contract versions and known consumers.",
                "Classify compatibility impact across shape, meaning, failures, authorization, idempotency, ordering, consistency, timeout, retry, stored state, and operations.",
                "Preserve the existing version when compatibility can be maintained cleanly.",
                "Introduce a new version for breaking changes and define migration, coexistence, translation, and deprecation behavior.",
            ],
            PromptKind::Prune => &[
                "Build a semantic root set from manifests, contracts, invariants, effects, profiles, compatibility policy, operational recovery, implementation binding, and verification evidence.",
                "Classify candidate artifacts by current semantic role.",
                "Delete, merge, inline, rename, or document residue before introducing replacements.",
                "Do not hide a semantic change as pruning.",
            ],
            PromptKind::Evidence => &[
                "Identify the changed promise and the evidence category it belongs to: law, contract, scenario, boundary, runtime, reconciliation, or migration.",
                "Prefer the smallest evidence that strongly demonstrates the promise.",
                "Include negative evidence for impossible variants, invalid constructors, malformed boundary input, and illegal transitions when applicable.",
                "Name the manifest paths or implementation binding entries that should reference the evidence.",
            ],
            PromptKind::Drift => &[
                "Compare manifest purpose, ownership, contracts, effects, profiles, compatibility policy, verification evidence, and glossary language against implementation reality.",
                "Identify contradictions among canonical artifacts as architectural drift, not a prompt-precedence problem.",
                "Separate deterministic validation failures from semantic suspicions requiring human review.",
                "Propose the smallest artifact or implementation correction for each drift item.",
            ],
        }
    }

    fn expected_output(self) -> &'static [&'static str] {
        match self {
            PromptKind::Plan => &[
                "Owning module and affected contract surface.",
                "Change classification and compatibility impact.",
                "Affected invariants, effects, dependencies, profiles, and recovery paths.",
                "Implementation outline inside the owning boundary.",
                "Focused verification plan and commands.",
            ],
            PromptKind::Explain => &[
                "Intelligible plain-language explanation of how the module works.",
                "Important public contracts and owned decisions.",
                "Effects, invariants, and verification evidence that shape behavior.",
                "Any missing or ambiguous canonical artifacts relevant to the question.",
            ],
            PromptKind::Review => &[
                "Findings first, ordered by severity.",
                "Each finding names the violated RMS artifact or missing evidence.",
                "Open questions or assumptions.",
                "Brief change summary only after findings.",
            ],
            PromptKind::Refactor => &[
                "Public semantics preserved.",
                "Internal shape changes proposed.",
                "Boundary, dependency, and effect impact.",
                "Verification needed to prove compatibility.",
            ],
            PromptKind::Implement => &[
                "Requested outcome in owning-context language.",
                "Change classification and compatibility impact.",
                "Concrete implementation steps.",
                "Contract/manifest updates required before code changes.",
                "Representation choices for invalid states, failures, boundary schemas, or lifecycle transitions.",
                "Verification and conformance evidence.",
            ],
            PromptKind::EvolveContract => &[
                "Compatibility classification.",
                "Versioning decision.",
                "Migration, coexistence, translation, and deprecation plan.",
                "Provider and consumer evidence updates.",
            ],
            PromptKind::Prune => &[
                "Semantic root set.",
                "Artifacts to delete, merge, inline, rename, retain, or defer.",
                "Compatibility and evidence impact.",
                "Removal conditions for retained residue.",
            ],
            PromptKind::Evidence => &[
                "Evidence category and rationale.",
                "Positive and negative cases.",
                "Manifest or implementation binding references to update.",
                "Native command expected to run the evidence.",
            ],
            PromptKind::Drift => &[
                "Confirmed drift.",
                "Suspected drift requiring review.",
                "Canonical artifact or implementation source of each mismatch.",
                "Smallest correction path.",
            ],
        }
    }

    fn deterministic_checks(self) -> &'static [&'static str] {
        match self {
            PromptKind::Explain | PromptKind::Plan | PromptKind::Evidence | PromptKind::Drift => {
                &["rms validate --root <root>", "rms compose --root <root>"]
            }
            PromptKind::Review
            | PromptKind::Refactor
            | PromptKind::Implement
            | PromptKind::Prune => &[
                "rms validate --root <root>",
                "rms verify <implementation.yaml>",
                "rms compose --root <root>",
            ],
            PromptKind::EvolveContract => &[
                "rms check-compat <old-module.yaml> <new-module.yaml>",
                "rms validate --root <root>",
                "rms compose --root <root>",
            ],
        }
    }
}

impl Provider {
    fn label(self) -> &'static str {
        match self {
            Provider::None => "none",
            Provider::Codex => "codex",
        }
    }
}

impl CodexSandbox {
    fn as_str(self) -> &'static str {
        match self {
            CodexSandbox::ReadOnly => "read-only",
            CodexSandbox::WorkspaceWrite => "workspace-write",
        }
    }
}

impl ProviderWriteScope {
    fn as_str(self) -> &'static str {
        match self {
            ProviderWriteScope::Module => "module",
            ProviderWriteScope::Root => "root",
        }
    }
}

fn resolve_prompt_run_options(root: &Path, raw: RawPromptRunOptions) -> Result<PromptRunOptions> {
    if raw.ai && raw.provider.is_some() {
        bail!("use either `--ai` or `--provider`, not both");
    }

    let config = load_workbench_config(root)?;
    let config_value = config.as_ref().map(|loaded| &loaded.value);

    let provider = if raw.ai {
        let configured = config_value
            .and_then(|config| config.ai.default_provider.as_deref())
            .ok_or_else(|| {
                anyhow!(
                    "`--ai` requires `{}` with `ai.default_provider`",
                    root.join(WORKBENCH_CONFIG_PATH).display()
                )
            })?;
        let provider = parse_config_provider(configured, "ai.default_provider")?;
        if provider == Provider::None {
            bail!("`--ai` requires a non-`none` `ai.default_provider`");
        }
        provider
    } else {
        raw.provider.unwrap_or(Provider::None)
    };

    let run_root = raw
        .run_root
        .or_else(|| config_value.and_then(|config| config.runs.directory.clone()))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RUN_ROOT));

    let model = raw.model.or_else(|| {
        if provider == Provider::Codex {
            config_value.and_then(|config| config.ai.codex.model.clone())
        } else {
            None
        }
    });

    let sandbox = if let Some(sandbox) = raw.sandbox {
        sandbox
    } else if provider == Provider::Codex {
        config_value
            .and_then(|config| config.ai.codex.sandbox.as_deref())
            .map(|value| parse_config_sandbox(value, "ai.codex.sandbox"))
            .transpose()?
            .unwrap_or(CodexSandbox::ReadOnly)
    } else {
        CodexSandbox::ReadOnly
    };

    let write_scope = if let Some(write_scope) = raw.write_scope {
        write_scope
    } else if provider == Provider::Codex {
        config_value
            .and_then(|config| config.ai.codex.write_scope.as_deref())
            .map(|value| parse_config_write_scope(value, "ai.codex.write_scope"))
            .transpose()?
            .unwrap_or_else(|| {
                if matches!(sandbox, CodexSandbox::WorkspaceWrite) {
                    ProviderWriteScope::Module
                } else {
                    ProviderWriteScope::Root
                }
            })
    } else {
        ProviderWriteScope::Root
    };

    Ok(PromptRunOptions {
        provider,
        record: raw.record,
        run_root,
        model,
        sandbox,
        write_scope,
    })
}

fn resolve_run_root(root: &Path, explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    Ok(load_workbench_config(root)?
        .and_then(|loaded| loaded.value.runs.directory)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RUN_ROOT)))
}

fn load_workbench_config(root: &Path) -> Result<Option<LoadedWorkbenchConfig>> {
    let path = root.join(WORKBENCH_CONFIG_PATH);
    if !path.exists() {
        return Ok(None);
    }
    let source = fs::read_to_string(&path)
        .with_context(|| format!("failed to read `{}`", path.display()))?;
    let value: WorkbenchConfig = serde_yaml::from_str(&source)
        .with_context(|| format!("failed to parse `{}`", path.display()))?;
    Ok(Some(LoadedWorkbenchConfig { path, value }))
}

fn run_config_init(
    root: &Path,
    provider: Provider,
    model: Option<&str>,
    run_root: &Path,
    force: bool,
) -> Result<()> {
    let path = root.join(WORKBENCH_CONFIG_PATH);
    if path.exists() && !force {
        bail!(
            "workbench config already exists at `{}`; pass `--force` to overwrite",
            path.display()
        );
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create `{}`", parent.display()))?;
    }
    let rendered = render_workbench_config(provider, model, run_root);
    fs::write(&path, rendered).with_context(|| format!("failed to write `{}`", path.display()))?;
    println!("created {}", path.display());
    Ok(())
}

fn run_release_check(root: &Path, skip_cargo_package: bool) -> Result<()> {
    println!("# RMS Release Check");
    println!();
    println!("Root: {}", root.display());
    println!();

    let rms_exe = std::env::current_exe().with_context(|| "failed to locate current rms binary")?;

    run_release_metadata_check(root)?;
    run_release_step(
        "cargo fmt",
        command_with_args("cargo", &["fmt", "--all", "--check"], root),
    )?;
    run_release_step(
        "cargo test",
        command_with_args("cargo", &["test", "--workspace", "--locked"], root),
    )?;
    run_release_step(
        "rms validate",
        command_with_args(
            &rms_exe,
            &["validate", "--root", root.to_string_lossy().as_ref()],
            root,
        ),
    )?;
    run_release_step(
        "rms verify rms-cli",
        command_with_args(
            &rms_exe,
            &["verify", "tooling/rust/rms/implementation.yaml"],
            root,
        ),
    )?;
    run_release_step(
        "rms compose examples/minimal",
        command_with_args(&rms_exe, &["compose", "--root", "examples/minimal"], root),
    )?;
    run_release_step(
        "rms compose examples/rust",
        command_with_args(&rms_exe, &["compose", "--root", "examples/rust"], root),
    )?;
    run_release_step(
        "rms compose examples/swift",
        command_with_args(&rms_exe, &["compose", "--root", "examples/swift"], root),
    )?;
    run_release_step(
        "rms check-compat smoke",
        command_with_args(
            &rms_exe,
            &[
                "check-compat",
                "examples/rust/module.yaml",
                "examples/rust/module.yaml",
            ],
            root,
        ),
    )?;
    run_release_package_smoke(root, &rms_exe)?;
    run_release_scaffold_roundtrip(root, &rms_exe)?;
    run_release_step(
        "example Rust binding tests",
        command_with_args(
            "cargo",
            &[
                "test",
                "--manifest-path",
                "examples/rust/Cargo.toml",
                "--locked",
            ],
            root,
        ),
    )?;
    run_release_binary_smoke(root)?;
    if !skip_cargo_package {
        run_release_step(
            "cargo package",
            command_with_args(
                "cargo",
                &[
                    "package",
                    "--manifest-path",
                    "tooling/rust/rms/Cargo.toml",
                    "--allow-dirty",
                    "--no-verify",
                ],
                root,
            ),
        )?;
    }
    run_release_plugin_check(root)?;

    println!();
    println!("pass: release check");
    Ok(())
}

fn run_release_metadata_check(root: &Path) -> Result<()> {
    println!("## release metadata");
    validate_release_metadata(root)?;
    println!("pass");
    println!();
    Ok(())
}

fn run_release_scaffold_roundtrip(root: &Path, rms_exe: &Path) -> Result<()> {
    let temp = std::env::temp_dir().join(format!(
        "rms-release-scaffold-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&temp).with_context(|| format!("failed to create `{}`", temp.display()))?;
    let app = temp.join("app");
    let app_arg = app.to_string_lossy().to_string();
    let widget = app.join("modules/widget");
    let widget_arg = widget.to_string_lossy().to_string();
    let swift_widget = app.join("modules/swift-widget");
    let swift_widget_arg = swift_widget.to_string_lossy().to_string();
    let widget_manifest = widget.join("Cargo.toml");
    let widget_manifest_arg = widget_manifest.to_string_lossy().to_string();
    let executable_widget = app.join("modules/executable-widget");
    let executable_widget_arg = executable_widget.to_string_lossy().to_string();
    let executable_implementation = executable_widget.join("implementation.yaml");
    let executable_implementation_arg = executable_implementation.to_string_lossy().to_string();

    let result = (|| -> Result<()> {
        run_release_step(
            "rms init scaffold",
            command_with_args(
                rms_exe,
                &[
                    "init",
                    app_arg.as_str(),
                    "--name",
                    "app",
                    "--purpose",
                    "Try RMS",
                    "--context",
                    "core",
                ],
                root,
            ),
        )?;
        run_release_step(
            "rms add-module rust scaffold",
            command_with_args(
                rms_exe,
                &[
                    "add-module",
                    widget_arg.as_str(),
                    "--name",
                    "widget",
                    "--purpose",
                    "Own widgets",
                    "--kind",
                    "library",
                    "--binding",
                    "rust",
                ],
                root,
            ),
        )?;
        run_release_step(
            "rms add-module swift scaffold",
            command_with_args(
                rms_exe,
                &[
                    "add-module",
                    swift_widget_arg.as_str(),
                    "--name",
                    "swift-widget",
                    "--purpose",
                    "Own Swift widgets",
                    "--kind",
                    "library",
                    "--binding",
                    "swift",
                ],
                root,
            ),
        )?;
        run_release_step(
            "rms add-module executable scaffold",
            command_with_args(
                rms_exe,
                &[
                    "add-module",
                    executable_widget_arg.as_str(),
                    "--name",
                    "executable-widget",
                    "--purpose",
                    "Own executable widgets",
                    "--kind",
                    "adapter",
                    "--profile",
                    "boundary",
                    "--binding",
                    "executable",
                ],
                root,
            ),
        )?;
        run_release_step(
            "rms validate scaffold",
            command_with_args(rms_exe, &["validate", "--root", app_arg.as_str()], root),
        )?;
        run_release_step(
            "rms compose scaffold",
            command_with_args(rms_exe, &["compose", "--root", app_arg.as_str()], root),
        )?;
        run_release_step(
            "cargo generate-lockfile scaffold",
            command_with_args(
                "cargo",
                &[
                    "generate-lockfile",
                    "--manifest-path",
                    widget_manifest_arg.as_str(),
                ],
                root,
            ),
        )?;
        run_release_step(
            "cargo test scaffold rust binding",
            command_with_args(
                "cargo",
                &[
                    "test",
                    "--manifest-path",
                    widget_manifest_arg.as_str(),
                    "--locked",
                ],
                root,
            ),
        )?;
        run_release_step(
            "rms verify scaffold executable binding",
            command_with_args(
                rms_exe,
                &["verify", executable_implementation_arg.as_str()],
                root,
            ),
        )?;
        Ok(())
    })();

    let _ = fs::remove_dir_all(&temp);
    result
}

fn run_release_package_smoke(root: &Path, rms_exe: &Path) -> Result<()> {
    let temp = std::env::temp_dir().join(format!(
        "rms-release-package-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&temp).with_context(|| format!("failed to create `{}`", temp.display()))?;
    let output = temp.join("rust-example.rms");
    let output_arg = output.to_string_lossy().to_string();
    let result = run_release_step(
        "rms package smoke",
        command_with_args(
            rms_exe,
            &[
                "package",
                "examples/rust/module.yaml",
                "--output",
                output_arg.as_str(),
            ],
            root,
        ),
    )
    .and_then(|()| {
        for required in ["PACKAGE.json", "conformance-report.json"] {
            let path = output.join(required);
            if !path.exists() {
                bail!("package smoke missing `{}`", path.display());
            }
        }
        Ok(())
    })
    .and_then(|()| {
        run_release_step(
            "rms verify-package smoke",
            command_with_args(rms_exe, &["verify-package", output_arg.as_str()], root),
        )
    });
    let _ = fs::remove_dir_all(&temp);
    result
}

fn run_release_plugin_check(root: &Path) -> Result<()> {
    println!("## codex plugin wrapper");
    validate_codex_plugin_sync(root)?;
    println!("pass");
    println!();
    Ok(())
}

fn run_release_binary_smoke(root: &Path) -> Result<()> {
    run_release_step(
        "release binary build",
        command_with_args(
            "cargo",
            &["build", "--release", "-p", "rms", "--locked"],
            root,
        ),
    )?;

    let binary = release_binary_path(root);
    if !binary.exists() {
        bail!("release binary smoke missing `{}`", binary.display());
    }

    run_release_step(
        "release binary diagnose",
        command_with_args(&binary, &["diagnose", "--root", ".", "--json"], root),
    )?;
    run_release_step(
        "release binary validate minimal",
        command_with_args(&binary, &["validate", "--root", "examples/minimal"], root),
    )?;
    run_release_install_smoke(root, &binary)?;
    Ok(())
}

fn release_binary_path(root: &Path) -> PathBuf {
    root.join("target")
        .join("release")
        .join(format!("rms{}", std::env::consts::EXE_SUFFIX))
}

fn run_release_install_smoke(root: &Path, binary: &Path) -> Result<()> {
    let temp = std::env::temp_dir().join(format!(
        "rms-release-install-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    ));
    let bin_dir = temp.join("bin");
    fs::create_dir_all(&bin_dir)
        .with_context(|| format!("failed to create `{}`", bin_dir.display()))?;
    let installed_binary = bin_dir.join(format!("rms{}", std::env::consts::EXE_SUFFIX));
    fs::copy(binary, &installed_binary).with_context(|| {
        format!(
            "failed to copy `{}` to `{}`",
            binary.display(),
            installed_binary.display()
        )
    })?;
    make_executable(&installed_binary)?;

    let result = (|| -> Result<()> {
        run_release_step(
            "clean-room installed binary diagnose",
            command_with_path(
                "rms",
                &["diagnose", "--root", ".", "--json"],
                root,
                &bin_dir,
            )?,
        )?;
        run_release_step(
            "clean-room installed binary validate minimal",
            command_with_path(
                "rms",
                &["validate", "--root", "examples/minimal"],
                root,
                &bin_dir,
            )?,
        )?;
        Ok(())
    })();

    let _ = fs::remove_dir_all(&temp);
    result
}

fn command_with_path(
    program: &str,
    args: &[&str],
    root: &Path,
    first_path: &Path,
) -> Result<Command> {
    let mut paths = vec![first_path.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    let path_value = std::env::join_paths(paths).with_context(|| "failed to build PATH")?;
    let mut command = command_with_args(program, args, root);
    command.env("PATH", path_value);
    Ok(command)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .with_context(|| format!("failed to read metadata for `{}`", path.display()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).with_context(|| {
        format!(
            "failed to set executable permissions on `{}`",
            path.display()
        )
    })
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

fn validate_release_metadata(root: &Path) -> Result<()> {
    let cargo_path = root.join("tooling/rust/rms/Cargo.toml");
    let cargo = load_toml(&cargo_path)?;
    let cargo_package = cargo
        .get("package")
        .and_then(TomlValue::as_table)
        .ok_or_else(|| anyhow!("`{}` missing [package]", cargo_path.display()))?;
    let cargo_name = cargo_package
        .get("name")
        .and_then(TomlValue::as_str)
        .ok_or_else(|| anyhow!("`{}` missing package.name", cargo_path.display()))?;
    let cargo_version = cargo_package
        .get("version")
        .and_then(TomlValue::as_str)
        .ok_or_else(|| anyhow!("`{}` missing package.version", cargo_path.display()))?;
    if cargo_name != "rms" {
        bail!("release metadata expected Cargo package name `rms`, found `{cargo_name}`");
    }

    let module_path = root.join("tooling/rust/rms/module.yaml");
    let module = load_manifest(&module_path)?;
    let module_version = get_str(&module.value, &["module", "version"])
        .ok_or_else(|| anyhow!("`{}` missing module.version", module_path.display()))?;
    if module_version != cargo_version {
        bail!(
            "release metadata version drift: Cargo package is `{cargo_version}` but rms-cli module is `{module_version}`"
        );
    }

    let plugin_path = root
        .join(CODEX_PLUGIN_PATH)
        .join(".codex-plugin/plugin.json");
    let plugin_source = fs::read_to_string(&plugin_path)
        .with_context(|| format!("failed to read `{}`", plugin_path.display()))?;
    let plugin: JsonValue = serde_json::from_str(&plugin_source)
        .with_context(|| format!("failed to parse `{}`", plugin_path.display()))?;
    let plugin_version = plugin
        .get("version")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| anyhow!("`{}` missing version", plugin_path.display()))?;
    if plugin_version != cargo_version {
        bail!(
            "release metadata version drift: Cargo package is `{cargo_version}` but Codex plugin is `{plugin_version}`"
        );
    }

    Ok(())
}

fn command_with_args(program: impl AsRef<Path>, args: &[&str], root: &Path) -> Command {
    let mut command = Command::new(program.as_ref());
    command.args(args).current_dir(root);
    command
}

fn run_release_step(label: &str, mut command: Command) -> Result<()> {
    println!("## {label}");
    let status = command
        .status()
        .with_context(|| format!("failed to start release check step `{label}`"))?;
    if !status.success() {
        bail!(
            "release check step `{label}` failed with status {}",
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string())
        );
    }
    println!("pass");
    println!();
    Ok(())
}

fn validate_codex_plugin_sync(root: &Path) -> Result<()> {
    let plugin_root = root.join(CODEX_PLUGIN_PATH);
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let manifest_source = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read `{}`", manifest_path.display()))?;
    let manifest: JsonValue = serde_json::from_str(&manifest_source)
        .with_context(|| format!("failed to parse `{}`", manifest_path.display()))?;
    for field in ["name", "version", "description", "skills"] {
        if manifest.get(field).is_none() {
            bail!("Codex plugin manifest missing `{field}`");
        }
    }
    if manifest.get("skills").and_then(JsonValue::as_str) != Some("./skills/") {
        bail!("Codex plugin manifest `skills` must be `./skills/`");
    }

    for skill in CANONICAL_SKILLS {
        let canonical = root.join("skills").join(skill).join("SKILL.md");
        let packaged = plugin_root.join("skills").join(skill).join("SKILL.md");
        let canonical_source = fs::read_to_string(&canonical)
            .with_context(|| format!("failed to read `{}`", canonical.display()))?;
        let packaged_source = fs::read_to_string(&packaged)
            .with_context(|| format!("failed to read `{}`", packaged.display()))?;
        if canonical_source != packaged_source {
            bail!("Codex plugin skill `{skill}` is out of sync with canonical `skills/{skill}`");
        }
    }

    let mut packaged_skills = BTreeSet::new();
    let plugin_skills_dir = plugin_root.join("skills");
    for entry in fs::read_dir(&plugin_skills_dir)
        .with_context(|| format!("failed to read `{}`", plugin_skills_dir.display()))?
        .filter_map(Result::ok)
    {
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                packaged_skills.insert(name.to_string());
            }
        }
    }
    let expected = CANONICAL_SKILLS
        .iter()
        .map(|skill| (*skill).to_string())
        .collect::<BTreeSet<_>>();
    if packaged_skills != expected {
        bail!("Codex plugin skill set does not match canonical skill set");
    }
    Ok(())
}

fn render_workbench_config(provider: Provider, model: Option<&str>, run_root: &Path) -> String {
    let mut out = String::new();
    out.push_str("ai:\n");
    let _ = writeln!(out, "  default_provider: {}", provider.label());
    out.push_str("  codex:\n");
    if let Some(model) = model.filter(|value| !value.trim().is_empty()) {
        let _ = writeln!(out, "    model: {}", yaml_quote(model));
    }
    out.push_str("    sandbox: read-only\n");
    out.push_str("runs:\n");
    let _ = writeln!(
        out,
        "  directory: {}",
        yaml_quote(&run_root.display().to_string())
    );
    out
}

fn parse_config_provider(value: &str, field: &str) -> Result<Provider> {
    match value {
        "none" => Ok(Provider::None),
        "codex" => Ok(Provider::Codex),
        other => bail!("unsupported `{field}` value `{other}`; expected `none` or `codex`"),
    }
}

fn parse_config_sandbox(value: &str, field: &str) -> Result<CodexSandbox> {
    match value {
        "read-only" => Ok(CodexSandbox::ReadOnly),
        "workspace-write" => Ok(CodexSandbox::WorkspaceWrite),
        other => bail!(
            "unsupported `{field}` value `{other}`; expected `read-only` or `workspace-write`"
        ),
    }
}

fn parse_config_write_scope(value: &str, field: &str) -> Result<ProviderWriteScope> {
    match value {
        "module" => Ok(ProviderWriteScope::Module),
        "root" => Ok(ProviderWriteScope::Root),
        other => bail!("unsupported `{field}` value `{other}`; expected `module` or `root`"),
    }
}

#[derive(Debug)]
struct LoadedManifest {
    path: PathBuf,
    value: YamlValue,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate {
            root,
            module,
            system,
            context_map,
            contract,
            implementation,
            conformance,
            json,
        } => run_validate(ValidateRequest {
            root,
            module,
            system,
            context_map,
            contract,
            implementation,
            conformance,
            json,
        }),
        Commands::Inspect { module } => {
            let manifest = load_manifest(&module)?;
            print_module_brief(&manifest);
            Ok(())
        }
        Commands::Explain {
            subject,
            module,
            root,
            ai,
            provider,
            record,
            run_root,
            model,
            sandbox,
            write_scope,
        } => {
            let options = resolve_prompt_run_options(
                &root,
                RawPromptRunOptions {
                    ai,
                    provider,
                    record,
                    run_root,
                    model,
                    sandbox,
                    write_scope,
                },
            )?;
            run_explain(&subject, module.as_deref(), &root, &options)
        }
        Commands::Diagnose { root, json } => run_diagnose(&root, json),
        Commands::Prompt {
            kind,
            module,
            task,
            root,
            diff,
            impact,
            ai,
            provider,
            record,
            run_root,
            model,
            sandbox,
            write_scope,
        } => {
            let options = resolve_prompt_run_options(
                &root,
                RawPromptRunOptions {
                    ai,
                    provider,
                    record,
                    run_root,
                    model,
                    sandbox,
                    write_scope,
                },
            )?;
            run_prompt(
                kind,
                &module,
                &root,
                task.as_deref(),
                diff.as_deref(),
                impact,
                &options,
            )
        }
        Commands::Plan {
            module,
            task,
            root,
            ai,
            provider,
            record,
            run_root,
            model,
            sandbox,
            write_scope,
        } => {
            let options = resolve_prompt_run_options(
                &root,
                RawPromptRunOptions {
                    ai,
                    provider,
                    record,
                    run_root,
                    model,
                    sandbox,
                    write_scope,
                },
            )?;
            run_prompt(
                PromptKind::Plan,
                &module,
                &root,
                Some(&task),
                None,
                false,
                &options,
            )
        }
        Commands::Review {
            module,
            task,
            root,
            diff,
            impact,
            ai,
            provider,
            record,
            run_root,
            model,
            sandbox,
            write_scope,
        } => {
            let options = resolve_prompt_run_options(
                &root,
                RawPromptRunOptions {
                    ai,
                    provider,
                    record,
                    run_root,
                    model,
                    sandbox,
                    write_scope,
                },
            )?;
            run_prompt(
                PromptKind::Review,
                &module,
                &root,
                task.as_deref(),
                diff.as_deref(),
                impact,
                &options,
            )
        }
        Commands::Impact { diff, root, json } => run_impact(&root, diff.as_deref(), json),
        Commands::Gate {
            diff,
            root,
            dry_run,
            json,
        } => run_gate(&root, diff.as_deref(), dry_run, json),
        Commands::Refactor {
            module,
            task,
            root,
            ai,
            provider,
            record,
            run_root,
            model,
            sandbox,
            write_scope,
        } => {
            let options = resolve_prompt_run_options(
                &root,
                RawPromptRunOptions {
                    ai,
                    provider,
                    record,
                    run_root,
                    model,
                    sandbox,
                    write_scope,
                },
            )?;
            run_prompt(
                PromptKind::Refactor,
                &module,
                &root,
                Some(&task),
                None,
                false,
                &options,
            )
        }
        Commands::Implement {
            module,
            task,
            root,
            ai,
            provider,
            record,
            run_root,
            model,
            sandbox,
            write_scope,
        } => {
            let options = resolve_prompt_run_options(
                &root,
                RawPromptRunOptions {
                    ai,
                    provider,
                    record,
                    run_root,
                    model,
                    sandbox,
                    write_scope,
                },
            )?;
            run_prompt(
                PromptKind::Implement,
                &module,
                &root,
                Some(&task),
                None,
                false,
                &options,
            )
        }
        Commands::EvolveContract {
            module,
            task,
            root,
            ai,
            provider,
            record,
            run_root,
            model,
            sandbox,
            write_scope,
        } => {
            let options = resolve_prompt_run_options(
                &root,
                RawPromptRunOptions {
                    ai,
                    provider,
                    record,
                    run_root,
                    model,
                    sandbox,
                    write_scope,
                },
            )?;
            run_prompt(
                PromptKind::EvolveContract,
                &module,
                &root,
                Some(&task),
                None,
                false,
                &options,
            )
        }
        Commands::Evidence {
            module,
            task,
            root,
            ai,
            provider,
            record,
            run_root,
            model,
            sandbox,
            write_scope,
        } => {
            let options = resolve_prompt_run_options(
                &root,
                RawPromptRunOptions {
                    ai,
                    provider,
                    record,
                    run_root,
                    model,
                    sandbox,
                    write_scope,
                },
            )?;
            run_prompt(
                PromptKind::Evidence,
                &module,
                &root,
                Some(&task),
                None,
                false,
                &options,
            )
        }
        Commands::Run { command } => match command {
            RunCommands::List { root, run_root } => {
                let run_root = resolve_run_root(&root, run_root)?;
                run_list_runs(&root, &run_root)
            }
            RunCommands::Inspect {
                run,
                root,
                run_root,
            } => {
                let run_root = resolve_run_root(&root, run_root)?;
                run_inspect_run(&run, &root, &run_root)
            }
            RunCommands::Latest { root, run_root } => {
                let run_root = resolve_run_root(&root, run_root)?;
                run_latest_run(&root, &run_root)
            }
        },
        Commands::Config { command } => match command {
            ConfigCommands::Init {
                root,
                provider,
                model,
                run_root,
                force,
            } => run_config_init(&root, provider, model.as_deref(), &run_root, force),
        },
        Commands::Release { command } => match command {
            ReleaseCommands::Check {
                root,
                skip_cargo_package,
            } => run_release_check(&root, skip_cargo_package),
        },
        Commands::Context { module, task, root } => {
            let manifest = load_manifest(&module)?;
            print_context_packet(&manifest, &root, task.as_deref())?;
            Ok(())
        }
        Commands::Atlas {
            module,
            root,
            output,
            force,
        } => run_atlas(&module, &root, output.as_deref(), force),
        Commands::Conformance {
            module,
            implementation,
            output,
        } => {
            let report = build_conformance_report(&module, implementation.as_deref())?;
            let rendered = serde_json::to_string_pretty(&report)?;
            if let Some(path) = output {
                fs::write(path, rendered)?;
            } else {
                println!("{rendered}");
            }
            Ok(())
        }
        Commands::CheckCompat { old, new, json } => run_check_compat(&old, &new, json),
        Commands::Compose { root, json } => run_compose(&root, json),
        Commands::Verify {
            implementation,
            dry_run,
        } => run_verify(&implementation, dry_run),
        Commands::Package {
            module,
            output,
            force,
        } => run_package(&module, output.as_deref(), force),
        Commands::VerifyPackage { package, json } => run_verify_package(&package, json),
        Commands::Init {
            path,
            name,
            purpose,
            version,
            context,
        } => run_init(&path, &name, &purpose, &version, &context),
        Commands::AddModule {
            path,
            name,
            purpose,
            kind,
            profile,
            binding,
        } => run_add_module(&path, &name, &purpose, &kind, &profile, binding.as_deref()),
    }
}

struct ValidateRequest {
    root: PathBuf,
    module: Vec<PathBuf>,
    system: Vec<PathBuf>,
    context_map: Vec<PathBuf>,
    contract: Vec<PathBuf>,
    implementation: Vec<PathBuf>,
    conformance: Vec<PathBuf>,
    json: bool,
}

fn run_validate(request: ValidateRequest) -> Result<()> {
    let diagnostics = collect_validation_diagnostics(
        &request.root,
        request.module,
        request.system,
        request.context_map,
        request.contract,
        request.implementation,
        request.conformance,
    )?;

    if request.json {
        println!("{}", serde_json::to_string_pretty(&diagnostics)?);
    } else if diagnostics.is_empty() {
        println!("pass: no RMS validation diagnostics");
    } else {
        for diagnostic in &diagnostics {
            println!(
                "{} [{}] {}: {}",
                severity_label(diagnostic.severity),
                diagnostic.check,
                diagnostic.path,
                diagnostic.message
            );
        }
    }

    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        bail!("RMS validation failed");
    }

    Ok(())
}

fn collect_validation_diagnostics(
    root: &Path,
    modules: Vec<PathBuf>,
    systems: Vec<PathBuf>,
    context_maps: Vec<PathBuf>,
    contracts: Vec<PathBuf>,
    implementations: Vec<PathBuf>,
    conformance_reports: Vec<PathBuf>,
) -> Result<Vec<Diagnostic>> {
    let targets = discover_targets(
        root,
        modules,
        systems,
        context_maps,
        contracts,
        implementations,
        conformance_reports,
    )?;
    let mut diagnostics = Vec::new();

    for path in targets {
        match load_manifest(&path) {
            Ok(manifest) => validate_loaded_manifest(&manifest, &mut diagnostics),
            Err(error) => diagnostics.push(Diagnostic {
                severity: Severity::Error,
                check: "manifest.parse".to_string(),
                path: path.display().to_string(),
                message: error.to_string(),
            }),
        }
    }

    Ok(diagnostics)
}

fn run_diagnose(root: &Path, json_output: bool) -> Result<()> {
    let report = build_diagnose_report(root)?;
    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    print_diagnose_report(&report);
    Ok(())
}

fn build_diagnose_report(root: &Path) -> Result<DiagnoseReport> {
    let targets = discover_targets(
        root,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )?;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for target in &targets {
        let spec = load_manifest(target)
            .ok()
            .and_then(|manifest| get_str(&manifest.value, &["spec"]).map(str::to_string))
            .unwrap_or_else(|| "<unreadable>".to_string());
        *counts.entry(spec).or_default() += 1;
    }

    let diagnostics = collect_validation_diagnostics(
        root,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )?;
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .count();

    let config = diagnose_config(root);
    let run_records = diagnose_run_records(root, &config.run_directory);

    Ok(DiagnoseReport {
        validator: VALIDATOR_NAME,
        version: VALIDATOR_VERSION,
        root: root.display().to_string(),
        repository: vec![
            file_readiness("system.yaml", &root.join("system.yaml")),
            file_readiness("context-map.yaml", &root.join("context-map.yaml")),
            file_readiness("AGENTS.md", &root.join("AGENTS.md")),
            file_readiness(WORKBENCH_CONFIG_PATH, &root.join(WORKBENCH_CONFIG_PATH)),
        ],
        config,
        manifest_counts: counts,
        validation: ValidationReadiness {
            status: if diagnostics.is_empty() {
                "pass".to_string()
            } else {
                "review-required".to_string()
            },
            errors,
            warnings,
            diagnostics,
        },
        native_tools: vec![
            command_readiness("git", &["--version"]),
            command_readiness("cargo", &["--version"]),
            command_readiness("swift", &["--version"]),
        ],
        ai_providers: vec![
            command_readiness("codex", &["--version"]),
            command_readiness("claude", &["--version"]),
        ],
        run_records,
        guidance: vec![
            "Use `rms explain <module>` before asking broad questions about a module.".to_string(),
            "Use `rms config init` when you want checked-in or local workbench provider defaults.".to_string(),
            "Use `rms explain --ai` or `--provider codex` only when provider execution is intended.".to_string(),
            "Use `rms implement`, `rms evolve-contract`, and `rms evidence` for bounded agent guidance.".to_string(),
            "Use `rms context <module> --task ...` before implementation work.".to_string(),
            format!("Use `rms validate --root {}` before completion.", root.display()),
            format!(
                "Use `rms gate --root {}` to run git-impact-selected RMS checks.",
                root.display()
            ),
            "Use `rms verify <implementation.yaml>` when an implementation binding declares verification.".to_string(),
        ],
    })
}

fn print_diagnose_report(report: &DiagnoseReport) {
    println!("# RMS Diagnose");
    println!();
    println!("RMS CLI: {} {}", report.validator, report.version);
    println!("Root: {}", report.root);
    println!();

    println!("## Repository");
    for item in &report.repository {
        println!("{}: {}", item.name, item.status);
    }
    let total_manifests: usize = report.manifest_counts.values().sum();
    println!(
        "RMS manifests: {}",
        if total_manifests == 0 {
            "<none>".to_string()
        } else {
            total_manifests.to_string()
        }
    );
    for (spec, count) in &report.manifest_counts {
        println!("- {spec}: {count}");
    }
    println!();

    println!("## Config");
    println!("{}: {}", report.config.path, report.config.status);
    if let Some(message) = &report.config.message {
        println!("Message: {message}");
    }
    println!(
        "Default provider: {}",
        report
            .config
            .default_provider
            .as_deref()
            .unwrap_or("<none>")
    );
    println!(
        "Codex model: {}",
        report
            .config
            .codex_model
            .as_deref()
            .unwrap_or("<provider-default>")
    );
    println!(
        "Codex sandbox: {}",
        report
            .config
            .codex_sandbox
            .as_deref()
            .unwrap_or("read-only")
    );
    println!(
        "Codex write scope: {}",
        report
            .config
            .codex_write_scope
            .as_deref()
            .unwrap_or("<default>")
    );
    println!("Run directory: {}", report.config.run_directory);
    println!();

    println!("## Validation");
    println!("Status: {}", report.validation.status);
    if !report.validation.diagnostics.is_empty() {
        println!("Errors: {}", report.validation.errors);
        println!("Warnings: {}", report.validation.warnings);
        for diagnostic in report.validation.diagnostics.iter().take(12) {
            println!(
                "- {} [{}] {}: {}",
                severity_label(diagnostic.severity),
                diagnostic.check,
                diagnostic.path,
                diagnostic.message
            );
        }
        if report.validation.diagnostics.len() > 12 {
            println!(
                "- ... {} more diagnostics",
                report.validation.diagnostics.len() - 12
            );
        }
    }
    println!();

    println!("## Native Tools");
    for command in &report.native_tools {
        print_command_readiness(command);
    }
    println!();

    println!("## AI Providers");
    for command in &report.ai_providers {
        print_command_readiness(command);
    }
    println!();

    println!("## Run Records");
    println!(
        "{}: {}",
        report.run_records.directory, report.run_records.status
    );
    if let Some(message) = &report.run_records.message {
        println!("Message: {message}");
    }
    println!();

    println!("## Agent Guidance");
    for item in &report.guidance {
        println!("- {item}");
    }
}

fn file_readiness(label: &str, path: &Path) -> ReadinessItem {
    ReadinessItem {
        name: label.to_string(),
        path: path.display().to_string(),
        status: if path.exists() {
            "present".to_string()
        } else {
            "missing".to_string()
        },
    }
}

fn diagnose_config(root: &Path) -> ConfigReadiness {
    let path = root.join(WORKBENCH_CONFIG_PATH);
    match load_workbench_config(root) {
        Ok(Some(loaded)) => {
            let default_provider = match loaded.value.ai.default_provider.as_deref() {
                Some(value) => match parse_config_provider(value, "ai.default_provider") {
                    Ok(provider) => Some(provider.label().to_string()),
                    Err(error) => {
                        return ConfigReadiness {
                            path: loaded.path.display().to_string(),
                            status: "invalid".to_string(),
                            default_provider: Some(value.to_string()),
                            codex_model: loaded.value.ai.codex.model,
                            codex_sandbox: loaded.value.ai.codex.sandbox,
                            codex_write_scope: loaded.value.ai.codex.write_scope,
                            run_directory: loaded
                                .value
                                .runs
                                .directory
                                .unwrap_or_else(|| PathBuf::from(DEFAULT_RUN_ROOT))
                                .display()
                                .to_string(),
                            message: Some(error.to_string()),
                        };
                    }
                },
                None => None,
            };
            if let Some(value) = loaded.value.ai.codex.sandbox.as_deref() {
                if let Err(error) = parse_config_sandbox(value, "ai.codex.sandbox") {
                    return ConfigReadiness {
                        path: loaded.path.display().to_string(),
                        status: "invalid".to_string(),
                        default_provider,
                        codex_model: loaded.value.ai.codex.model,
                        codex_sandbox: Some(value.to_string()),
                        codex_write_scope: loaded.value.ai.codex.write_scope,
                        run_directory: loaded
                            .value
                            .runs
                            .directory
                            .unwrap_or_else(|| PathBuf::from(DEFAULT_RUN_ROOT))
                            .display()
                            .to_string(),
                        message: Some(error.to_string()),
                    };
                }
            }
            if let Some(value) = loaded.value.ai.codex.write_scope.as_deref() {
                if let Err(error) = parse_config_write_scope(value, "ai.codex.write_scope") {
                    return ConfigReadiness {
                        path: loaded.path.display().to_string(),
                        status: "invalid".to_string(),
                        default_provider,
                        codex_model: loaded.value.ai.codex.model,
                        codex_sandbox: loaded.value.ai.codex.sandbox,
                        codex_write_scope: Some(value.to_string()),
                        run_directory: loaded
                            .value
                            .runs
                            .directory
                            .unwrap_or_else(|| PathBuf::from(DEFAULT_RUN_ROOT))
                            .display()
                            .to_string(),
                        message: Some(error.to_string()),
                    };
                }
            }
            ConfigReadiness {
                path: loaded.path.display().to_string(),
                status: "present".to_string(),
                default_provider,
                codex_model: loaded.value.ai.codex.model,
                codex_sandbox: loaded.value.ai.codex.sandbox,
                codex_write_scope: loaded.value.ai.codex.write_scope,
                run_directory: loaded
                    .value
                    .runs
                    .directory
                    .unwrap_or_else(|| PathBuf::from(DEFAULT_RUN_ROOT))
                    .display()
                    .to_string(),
                message: None,
            }
        }
        Ok(None) => ConfigReadiness {
            path: path.display().to_string(),
            status: "missing".to_string(),
            default_provider: None,
            codex_model: None,
            codex_sandbox: None,
            codex_write_scope: None,
            run_directory: DEFAULT_RUN_ROOT.to_string(),
            message: Some("optional config is not present".to_string()),
        },
        Err(error) => ConfigReadiness {
            path: path.display().to_string(),
            status: "invalid".to_string(),
            default_provider: None,
            codex_model: None,
            codex_sandbox: None,
            codex_write_scope: None,
            run_directory: DEFAULT_RUN_ROOT.to_string(),
            message: Some(error.to_string()),
        },
    }
}

fn diagnose_run_records(root: &Path, configured_run_directory: &str) -> RunRecordReadiness {
    let directory = root.join(configured_run_directory);
    if directory.is_dir() {
        return RunRecordReadiness {
            directory: directory.display().to_string(),
            status: "present".to_string(),
            message: None,
        };
    }
    if directory.exists() {
        return RunRecordReadiness {
            directory: directory.display().to_string(),
            status: "not-directory".to_string(),
            message: Some("configured run record path exists but is not a directory".to_string()),
        };
    }
    let parent = directory.parent().unwrap_or(root);
    RunRecordReadiness {
        directory: directory.display().to_string(),
        status: if parent.exists() {
            "missing-will-be-created".to_string()
        } else {
            "parent-missing".to_string()
        },
        message: Some("run records are created when `--record` or a provider is used".to_string()),
    }
}

fn run_explain(
    subject: &[String],
    explicit_module: Option<&Path>,
    root: &Path,
    options: &PromptRunOptions,
) -> Result<()> {
    let (module, question) = resolve_explain_subject(subject, explicit_module, root)?;
    if options.provider == Provider::None && !options.record {
        let manifest = load_manifest(&module)?;
        return print_module_explanation(&manifest, root, question.as_deref());
    }

    run_prompt(
        PromptKind::Explain,
        &module,
        root,
        question.as_deref(),
        None,
        false,
        options,
    )
}

fn resolve_explain_subject(
    subject: &[String],
    explicit_module: Option<&Path>,
    root: &Path,
) -> Result<(PathBuf, Option<String>)> {
    if let Some(module) = explicit_module {
        return Ok((module.to_path_buf(), join_question(subject)));
    }

    if let Some(first) = subject.first() {
        let candidate = Path::new(first);
        if candidate.exists() && is_module_yaml_manifest(candidate) {
            return Ok((candidate.to_path_buf(), join_question(&subject[1..])));
        }
    }

    let module = infer_single_module(root)?;
    Ok((module, join_question(subject)))
}

fn join_question(parts: &[String]) -> Option<String> {
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn infer_single_module(root: &Path) -> Result<PathBuf> {
    let direct = root.join("module.yaml");
    if direct.exists() && is_module_yaml_manifest(&direct) {
        return Ok(direct);
    }

    let mut modules = Vec::new();
    for entry in WalkDir::new(root)
        .max_depth(3)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if (file_name == "module.yaml" || file_name.ends_with(".module.yaml"))
            && is_module_yaml_manifest(path)
        {
            modules.push(path.to_path_buf());
        }
    }

    match modules.len() {
        0 => bail!(
            "could not infer module from `{}`; pass `--module <module.yaml>` or run from a module directory",
            root.display()
        ),
        1 => Ok(modules.remove(0)),
        _ => bail!(
            "found multiple modules under `{}`; pass `--module <module.yaml>`",
            root.display()
        ),
    }
}

fn is_module_yaml_manifest(path: &Path) -> bool {
    if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
        return false;
    }
    let Ok(source) = fs::read_to_string(path) else {
        return false;
    };
    source
        .lines()
        .take(5)
        .any(|line| line.trim() == "spec: rms/module/v0.1")
}

fn run_prompt(
    kind: PromptKind,
    module: &Path,
    root: &Path,
    task: Option<&str>,
    diff: Option<&str>,
    impact: bool,
    options: &PromptRunOptions,
) -> Result<()> {
    let manifest = load_manifest(module)?;
    let mut rendered = render_workbench_prompt(&manifest, root, kind, task, diff, impact)?;
    if options.provider != Provider::None {
        rendered.push_str(&render_provider_execution_scope(&manifest, root, options));
    }

    let run_dir = if options.record || options.provider != Provider::None {
        Some(write_prompt_run_record(
            &manifest, root, kind, task, diff, impact, &rendered, options,
        )?)
    } else {
        None
    };

    match options.provider {
        Provider::None => {
            println!("{rendered}");
            if let Some(run_dir) = run_dir {
                eprintln!("run record: {}", run_dir.display());
            }
        }
        Provider::Codex => {
            let run_dir =
                run_dir.ok_or_else(|| anyhow!("provider execution requires run record"))?;
            execute_codex_provider(root, &manifest, &rendered, &run_dir, options)?;
            println!("run record: {}", run_dir.display());
            println!("response: {}", run_dir.join("response.md").display());
        }
    }

    Ok(())
}

fn write_prompt_run_record(
    manifest: &LoadedManifest,
    root: &Path,
    kind: PromptKind,
    task: Option<&str>,
    diff: Option<&str>,
    impact: bool,
    prompt: &str,
    options: &PromptRunOptions,
) -> Result<PathBuf> {
    let run_id = run_id(kind, manifest);
    let run_dir = root.join(&options.run_root).join(&run_id);
    fs::create_dir_all(&run_dir)
        .with_context(|| format!("failed to create run record `{}`", run_dir.display()))?;

    fs::write(run_dir.join("prompt.md"), prompt)
        .with_context(|| format!("failed to write `{}`", run_dir.join("prompt.md").display()))?;

    let request =
        render_run_request_yaml(manifest, root, kind, task, diff, impact, options, &run_id);
    fs::write(run_dir.join("request.yaml"), request).with_context(|| {
        format!(
            "failed to write `{}`",
            run_dir.join("request.yaml").display()
        )
    })?;

    let diagnostics = collect_validation_diagnostics(
        root,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )?;
    let checks = json!({
        "validator": VALIDATOR_NAME,
        "validator_version": VALIDATOR_VERSION,
        "validation": diagnostics,
    });
    fs::write(
        run_dir.join("checks.json"),
        serde_json::to_string_pretty(&checks)?,
    )
    .with_context(|| {
        format!(
            "failed to write `{}`",
            run_dir.join("checks.json").display()
        )
    })?;

    Ok(run_dir)
}

fn execute_codex_provider(
    root: &Path,
    manifest: &LoadedManifest,
    prompt: &str,
    run_dir: &Path,
    options: &PromptRunOptions,
) -> Result<()> {
    let response_path = run_dir.join("response.md");
    let provider_response_path = provider_response_path(run_dir)?;
    let stdout_path = run_dir.join("provider.stdout.log");
    let stderr_path = run_dir.join("provider.stderr.log");
    let execution_root = provider_execution_root(root, manifest, options);

    let mut command = Command::new("codex");
    command
        .arg("exec")
        .arg("--cd")
        .arg(&execution_root)
        .arg("--sandbox")
        .arg(options.sandbox.as_str())
        .arg("--output-last-message")
        .arg(&provider_response_path);

    if let Some(model) = &options.model {
        command.arg("--model").arg(model);
    }
    command.arg("-");

    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| "failed to start `codex exec` provider")?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(prompt.as_bytes())
            .with_context(|| "failed to write prompt to `codex exec` stdin")?;
    }

    let output = child
        .wait_with_output()
        .with_context(|| "failed to wait for `codex exec` provider")?;
    fs::write(&stdout_path, &output.stdout)
        .with_context(|| format!("failed to write `{}`", stdout_path.display()))?;
    fs::write(&stderr_path, &output.stderr)
        .with_context(|| format!("failed to write `{}`", stderr_path.display()))?;

    if !output.status.success() {
        bail!(
            "`codex exec` failed with status {}; see `{}` and `{}`",
            output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string()),
            stdout_path.display(),
            stderr_path.display()
        );
    }

    if !response_path.exists() {
        fs::write(
            &response_path,
            String::from_utf8_lossy(&output.stdout).as_ref(),
        )
        .with_context(|| format!("failed to write `{}`", response_path.display()))?;
    }

    Ok(())
}

fn render_provider_execution_scope(
    manifest: &LoadedManifest,
    root: &Path,
    options: &PromptRunOptions,
) -> String {
    let execution_root = provider_execution_root(root, manifest, options);
    let module_root = module_execution_root(root, manifest);
    let mut out = String::new();
    out.push_str("\n## Provider Execution Scope\n");
    let _ = writeln!(out, "- Provider: {}", options.provider.label());
    let _ = writeln!(out, "- Sandbox: {}", options.sandbox.as_str());
    let _ = writeln!(out, "- Write scope: {}", options.write_scope.as_str());
    let _ = writeln!(out, "- Execution root: {}", execution_root.display());
    match (options.sandbox, options.write_scope) {
        (CodexSandbox::ReadOnly, _) => {
            out.push_str("- Filesystem writes are not permitted in this provider run.\n");
        }
        (CodexSandbox::WorkspaceWrite, ProviderWriteScope::Module) => {
            let _ = writeln!(
                out,
                "- Edit only files under the owning module directory `{}`. If the task requires changing system, context, glossary, or another module, stop and report the required scope expansion.",
                module_root.display()
            );
        }
        (CodexSandbox::WorkspaceWrite, ProviderWriteScope::Root) => {
            out.push_str("- Repository-root writes are permitted. Still preserve RMS module ownership and update canonical artifacts before implementation when public meaning changes.\n");
        }
    }
    out
}

fn provider_execution_root(
    root: &Path,
    manifest: &LoadedManifest,
    options: &PromptRunOptions,
) -> PathBuf {
    if matches!(
        (options.sandbox, options.write_scope),
        (CodexSandbox::WorkspaceWrite, ProviderWriteScope::Module)
    ) {
        module_execution_root(root, manifest)
    } else {
        root.to_path_buf()
    }
}

fn module_execution_root(root: &Path, manifest: &LoadedManifest) -> PathBuf {
    let module_dir = manifest.path.parent().unwrap_or_else(|| Path::new("."));
    if module_dir.is_absolute() {
        module_dir.to_path_buf()
    } else {
        root.join(module_dir)
    }
}

fn provider_response_path(run_dir: &Path) -> Result<PathBuf> {
    absolute_path(&run_dir.join("response.md"))
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .with_context(|| "failed to resolve current directory")?
            .join(path))
    }
}

fn run_list_runs(root: &Path, run_root: &Path) -> Result<()> {
    let directory = root.join(run_root);
    println!("# RMS Run Records");
    println!();
    println!("Path: {}", directory.display());
    println!();

    if !directory.exists() {
        println!("<no run records>");
        return Ok(());
    }

    let runs = collect_run_dirs(&directory)?;

    if runs.is_empty() {
        println!("<no run records>");
        return Ok(());
    }

    for run in runs {
        print_run_list_item(&run)?;
    }

    Ok(())
}

fn run_inspect_run(run: &Path, root: &Path, run_root: &Path) -> Result<()> {
    let run_dir = resolve_run_dir(run, root, run_root);
    if !run_dir.exists() {
        bail!("run record does not exist: `{}`", run_dir.display());
    }

    print_run_record(&run_dir)
}

fn run_latest_run(root: &Path, run_root: &Path) -> Result<()> {
    let directory = root.join(run_root);
    let Some(run_dir) = latest_run_dir(&directory)? else {
        bail!("no run records found in `{}`", directory.display());
    };
    print_run_record(&run_dir)
}

fn latest_run_dir(directory: &Path) -> Result<Option<PathBuf>> {
    let runs = collect_run_dirs(directory)?;
    Ok(runs.into_iter().next())
}

fn collect_run_dirs(directory: &Path) -> Result<Vec<PathBuf>> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut runs = Vec::new();
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read run directory `{}`", directory.display()))?
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_dir() {
            runs.push(path);
        }
    }
    runs.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
    Ok(runs)
}

fn print_run_record(run_dir: &Path) -> Result<()> {
    println!("# RMS Run Record");
    println!();
    println!("Path: {}", run_dir.display());
    println!();

    print_run_request(&run_dir)?;
    print_run_files(&run_dir)?;
    print_run_checks(&run_dir)?;
    print_run_response(&run_dir)?;

    Ok(())
}

fn print_run_list_item(run_dir: &Path) -> Result<()> {
    let request = load_optional_yaml(&run_dir.join("request.yaml"))?;
    let run_id = request
        .as_ref()
        .and_then(|value| get_str(value, &["run_id"]))
        .or_else(|| run_dir.file_name().and_then(|name| name.to_str()))
        .unwrap_or("<unknown>");
    let prompt = request
        .as_ref()
        .and_then(|value| get_str(value, &["prompt"]))
        .unwrap_or("<unknown>");
    let provider = request
        .as_ref()
        .and_then(|value| get_str(value, &["provider"]))
        .unwrap_or("<unknown>");
    let task = request
        .as_ref()
        .and_then(|value| get_str(value, &["task"]))
        .unwrap_or("<none>");
    let response = if run_dir.join("response.md").exists() {
        "response"
    } else {
        "no-response"
    };
    println!("- {run_id}: {prompt} provider={provider} {response}");
    println!("  task: {task}");
    println!("  path: {}", run_dir.display());
    Ok(())
}

fn print_run_request(run_dir: &Path) -> Result<()> {
    println!("## Request");
    let Some(request) = load_optional_yaml(&run_dir.join("request.yaml"))? else {
        println!("- <missing request.yaml>");
        println!();
        return Ok(());
    };
    for field in [
        "run_id",
        "prompt",
        "provider",
        "module",
        "root",
        "task",
        "diff",
        "model",
        "sandbox",
        "source_revision",
    ] {
        if let Some(value) = get_str(&request, &[field]) {
            println!("- {field}: {value}");
        }
    }
    println!();
    Ok(())
}

fn print_run_files(run_dir: &Path) -> Result<()> {
    println!("## Files");
    for name in [
        "request.yaml",
        "prompt.md",
        "checks.json",
        "response.md",
        "provider.stdout.log",
        "provider.stderr.log",
    ] {
        let path = run_dir.join(name);
        if path.exists() {
            let size = fs::metadata(&path)
                .map(|metadata| metadata.len())
                .unwrap_or(0);
            println!("- {name}: {} bytes ({})", size, path.display());
        }
    }
    println!();
    Ok(())
}

fn print_run_checks(run_dir: &Path) -> Result<()> {
    println!("## Checks");
    let path = run_dir.join("checks.json");
    if !path.exists() {
        println!("- <missing checks.json>");
        println!();
        return Ok(());
    }
    let source = fs::read_to_string(&path)
        .with_context(|| format!("failed to read `{}`", path.display()))?;
    let value: JsonValue = serde_json::from_str(&source)
        .with_context(|| format!("failed to parse `{}`", path.display()))?;
    let diagnostics = value
        .get("validation")
        .and_then(JsonValue::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    println!(
        "- validator: {} {}",
        value
            .get("validator")
            .and_then(JsonValue::as_str)
            .unwrap_or("<unknown>"),
        value
            .get("validator_version")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
    );
    println!("- validation diagnostics: {diagnostics}");
    println!();
    Ok(())
}

fn print_run_response(run_dir: &Path) -> Result<()> {
    let path = run_dir.join("response.md");
    if !path.exists() {
        return Ok(());
    }
    println!("## Response");
    let response = fs::read_to_string(&path)
        .with_context(|| format!("failed to read `{}`", path.display()))?;
    print!("{}", truncate_for_prompt(&response, 12_000));
    if !response.ends_with('\n') {
        println!();
    }
    Ok(())
}

fn resolve_run_dir(run: &Path, root: &Path, run_root: &Path) -> PathBuf {
    if run.exists() || run.components().count() > 1 {
        run.to_path_buf()
    } else {
        root.join(run_root).join(run)
    }
}

fn load_optional_yaml(path: &Path) -> Result<Option<YamlValue>> {
    if !path.exists() {
        return Ok(None);
    }
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read `{}`", path.display()))?;
    let value = serde_yaml::from_str(&source)
        .with_context(|| format!("failed to parse YAML `{}`", path.display()))?;
    Ok(Some(value))
}

fn run_id(kind: PromptKind, manifest: &LoadedManifest) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let module = get_str(&manifest.value, &["module", "name"]).unwrap_or("module");
    format!(
        "{}-{}-{}",
        timestamp,
        kind.label(),
        sanitize_run_segment(module)
    )
}

fn sanitize_run_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "run".to_string()
    } else {
        trimmed.to_string()
    }
}

fn render_run_request_yaml(
    manifest: &LoadedManifest,
    root: &Path,
    kind: PromptKind,
    task: Option<&str>,
    diff: Option<&str>,
    impact: bool,
    options: &PromptRunOptions,
    run_id: &str,
) -> String {
    let mut out = String::new();
    let task = task.or_else(|| kind.default_task()).unwrap_or("");
    let _ = writeln!(out, "run_id: {}", yaml_quote(run_id));
    let _ = writeln!(out, "prompt: {}", yaml_quote(kind.prompt_id()));
    let _ = writeln!(out, "provider: {}", yaml_quote(options.provider.label()));
    let _ = writeln!(
        out,
        "module: {}",
        yaml_quote(&manifest.path.display().to_string())
    );
    let _ = writeln!(out, "root: {}", yaml_quote(&root.display().to_string()));
    let _ = writeln!(out, "task: {}", yaml_quote(task));
    if let Some(diff) = diff {
        let _ = writeln!(out, "diff: {}", yaml_quote(diff));
    }
    if impact {
        let _ = writeln!(out, "impact: true");
    }
    if let Some(model) = &options.model {
        let _ = writeln!(out, "model: {}", yaml_quote(model));
    }
    let _ = writeln!(out, "sandbox: {}", yaml_quote(options.sandbox.as_str()));
    let _ = writeln!(
        out,
        "write_scope: {}",
        yaml_quote(options.write_scope.as_str())
    );
    let _ = writeln!(
        out,
        "execution_root: {}",
        yaml_quote(
            &provider_execution_root(root, manifest, options)
                .display()
                .to_string()
        )
    );
    if let Some(revision) = source_revision(root) {
        let _ = writeln!(out, "source_revision: {}", yaml_quote(&revision));
    }
    out
}

fn render_workbench_prompt(
    manifest: &LoadedManifest,
    root: &Path,
    kind: PromptKind,
    task: Option<&str>,
    diff: Option<&str>,
    impact: bool,
) -> Result<String> {
    if impact && kind != PromptKind::Review {
        bail!("`--impact` is only supported for review prompts");
    }
    let effective_task = task
        .or_else(|| kind.default_task())
        .ok_or_else(|| anyhow!("{} prompts require `--task`", kind.label()))?;
    let include_diff = kind.includes_diff_by_default() || diff.is_some();
    let impact_report = if impact {
        let changed_paths = read_git_changed_paths(root, diff)?;
        Some(build_impact_report(root, diff, &changed_paths)?)
    } else {
        None
    };
    let diff_text = if include_diff {
        Some(read_git_diff(root, diff)?)
    } else {
        None
    };

    let mut out = String::new();
    writeln!(out, "# RMS Workbench Prompt")?;
    writeln!(out)?;
    writeln!(out, "Prompt: {}", kind.prompt_id())?;
    writeln!(
        out,
        "Mode: advisory; no edits are performed by this command"
    )?;
    writeln!(out, "Module: {}", manifest.path.display())?;
    writeln!(out, "Task: {effective_task}")?;
    writeln!(out)?;

    writeln!(out, "## Operating Rule")?;
    writeln!(out, "Use RMS canonical artifacts as the source of architectural truth. Do not infer ownership, effects, contracts, compatibility, or verification obligations from incidental code shape when the manifest or contracts say otherwise.")?;
    writeln!(out)?;

    append_prompt_context(&mut out, manifest, root)?;
    writeln!(out)?;

    writeln!(out, "## Workflow")?;
    for item in kind.workflow() {
        writeln!(out, "- {item}")?;
    }
    writeln!(out)?;

    writeln!(out, "## Expected Output")?;
    for item in kind.expected_output() {
        writeln!(out, "- {item}")?;
    }
    writeln!(out)?;

    writeln!(out, "## Deterministic Checks")?;
    for item in kind.deterministic_checks() {
        writeln!(out, "- {item}")?;
    }

    if let Some(report) = &impact_report {
        append_impact_prompt(&mut out, report)?;
    }

    if let Some(diff_text) = diff_text {
        writeln!(out)?;
        writeln!(out, "## Diff")?;
        if diff_text.trim().is_empty() {
            writeln!(out, "<no diff content detected>")?;
        } else {
            writeln!(out, "```diff")?;
            writeln!(out, "{diff_text}")?;
            writeln!(out, "```")?;
        }
    }

    Ok(out)
}

fn append_impact_prompt(out: &mut String, report: &ImpactReport) -> Result<()> {
    const MAX_IMPACT_ITEMS: usize = 40;

    writeln!(out)?;
    writeln!(out, "## Impact")?;
    writeln!(out, "Derived from git changed paths and RMS artifacts. Treat this as review evidence, not architectural authority.")?;
    writeln!(out, "- Result: {}", impact_result_label(report.result))?;
    if let Some(revision) = &report.source_revision {
        writeln!(out, "- Source revision: {revision}")?;
    }
    if let Some(diff) = &report.diff {
        writeln!(out, "- Diff: {diff}")?;
    } else {
        writeln!(out, "- Diff: working tree")?;
    }
    writeln!(
        out,
        "- Affected modules: {}",
        if report.affected_modules.is_empty() {
            "<none>".to_string()
        } else {
            report
                .affected_modules
                .iter()
                .map(|module| module.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    )?;

    writeln!(out)?;
    writeln!(out, "### Impact Paths")?;
    if report.changed_paths.is_empty() {
        writeln!(out, "- <none>")?;
    } else {
        for path in report.changed_paths.iter().take(MAX_IMPACT_ITEMS) {
            writeln!(
                out,
                "- {} [{}] {}{}",
                path.status,
                impact_category_label(path.category),
                path.path,
                path.module
                    .as_ref()
                    .map(|module| format!(" ({module})"))
                    .unwrap_or_default()
            )?;
        }
        append_impact_truncation_note(out, report.changed_paths.len(), MAX_IMPACT_ITEMS)?;
    }

    writeln!(out)?;
    writeln!(out, "### Impact Findings")?;
    if report.findings.is_empty() {
        writeln!(out, "- <none>")?;
    } else {
        for finding in report.findings.iter().take(MAX_IMPACT_ITEMS) {
            writeln!(
                out,
                "- {} [{}] {}{}: {}",
                impact_result_label(finding.severity),
                finding.check,
                finding.path.as_deref().unwrap_or("<none>"),
                finding
                    .module
                    .as_ref()
                    .map(|module| format!(" ({module})"))
                    .unwrap_or_default(),
                finding.message
            )?;
        }
        append_impact_truncation_note(out, report.findings.len(), MAX_IMPACT_ITEMS)?;
    }

    writeln!(out)?;
    writeln!(out, "### Impact Checks")?;
    if report.recommended_checks.is_empty() {
        writeln!(out, "- <none>")?;
    } else {
        for check in &report.recommended_checks {
            writeln!(out, "- {check}")?;
        }
    }

    Ok(())
}

fn append_impact_truncation_note(out: &mut String, total: usize, limit: usize) -> Result<()> {
    if total > limit {
        writeln!(out, "- [{} additional items omitted]", total - limit)?;
    }
    Ok(())
}

fn append_prompt_context(out: &mut String, manifest: &LoadedManifest, root: &Path) -> Result<()> {
    writeln!(out, "## Bounded RMS Context")?;
    writeln!(
        out,
        "- Name: {} {}",
        get_str(&manifest.value, &["module", "name"]).unwrap_or("<unknown>"),
        get_str(&manifest.value, &["module", "version"]).unwrap_or("")
    )?;
    writeln!(
        out,
        "- Kind: {}",
        get_str(&manifest.value, &["module", "kind"]).unwrap_or("<missing>")
    )?;
    writeln!(
        out,
        "- Purpose: {}",
        get_str(&manifest.value, &["module", "purpose"]).unwrap_or("<missing>")
    )?;
    append_prompt_string_list(
        out,
        "Profiles",
        &get_string_array(&manifest.value, &["profiles"]),
    )?;
    append_prompt_owned_terms(out, &manifest.value)?;
    append_prompt_contract_groups(out, "Provides", get_path(&manifest.value, &["provides"]))?;
    append_prompt_contract_groups(out, "Requires", get_path(&manifest.value, &["requires"]))?;
    append_prompt_invariants(out, &manifest.value)?;
    append_prompt_effects(out, &manifest.value)?;
    writeln!(
        out,
        "- Compatibility: {}",
        get_str(&manifest.value, &["compatibility", "policy"]).unwrap_or("<missing>")
    )?;
    append_prompt_verification(out, &manifest.value)?;
    append_prompt_change_protocols(out, &manifest.value)?;

    writeln!(out)?;
    writeln!(out, "### Canonical Files")?;
    for file_name in ["system.yaml", "context-map.yaml", "GLOSSARY.md"] {
        let path = root.join(file_name);
        if path.exists() {
            writeln!(out, "- {}", path.display())?;
        }
    }
    for reference in referenced_paths(&manifest.value) {
        let path = manifest
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&reference);
        writeln!(out, "- {}", path.display())?;
    }

    Ok(())
}

fn append_prompt_string_list(out: &mut String, label: &str, items: &[String]) -> Result<()> {
    writeln!(
        out,
        "- {label}: {}",
        if items.is_empty() {
            "<none>".to_string()
        } else {
            items.join(", ")
        }
    )?;
    Ok(())
}

fn append_prompt_owned_terms(out: &mut String, value: &YamlValue) -> Result<()> {
    writeln!(out, "- Ownership:")?;
    if let Some(owns) = get_path(value, &["owns"]).and_then(YamlValue::as_mapping) {
        for (key, values) in owns {
            let label = key.as_str().unwrap_or("<unknown>");
            let items = values
                .as_sequence()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(YamlValue::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            writeln!(
                out,
                "  - {label}: {}",
                if items.is_empty() { "<none>" } else { &items }
            )?;
        }
    } else {
        writeln!(out, "  - <missing>")?;
    }
    Ok(())
}

fn append_prompt_contract_groups(
    out: &mut String,
    label: &str,
    value: Option<&YamlValue>,
) -> Result<()> {
    writeln!(out, "- {label}:")?;
    let Some(groups) = value.and_then(YamlValue::as_mapping) else {
        writeln!(out, "  - <missing>")?;
        return Ok(());
    };
    for (group, items) in groups {
        let group = group.as_str().unwrap_or("<unknown>");
        writeln!(out, "  - {group}:")?;
        if let Some(items) = items.as_sequence() {
            for item in items {
                match item {
                    YamlValue::String(name) => writeln!(out, "    - {name}")?,
                    YamlValue::Mapping(mapping) => {
                        let name = mapping
                            .get(YamlValue::String("name".to_string()))
                            .and_then(YamlValue::as_str)
                            .unwrap_or("<unnamed>");
                        let contract = mapping
                            .get(YamlValue::String("contract".to_string()))
                            .and_then(YamlValue::as_str)
                            .unwrap_or("<no contract>");
                        writeln!(out, "    - {name} ({contract})")?;
                    }
                    _ => writeln!(out, "    - <unsupported reference>")?,
                }
            }
        }
    }
    Ok(())
}

fn append_prompt_invariants(out: &mut String, value: &YamlValue) -> Result<()> {
    writeln!(out, "- Invariants:")?;
    let Some(invariants) = get_path(value, &["invariants"]).and_then(YamlValue::as_sequence) else {
        writeln!(out, "  - <missing>")?;
        return Ok(());
    };
    if invariants.is_empty() {
        writeln!(out, "  - <none declared>")?;
        return Ok(());
    }
    for invariant in invariants {
        writeln!(
            out,
            "  - {}: {}",
            get_str(invariant, &["id"]).unwrap_or("<missing-id>"),
            get_str(invariant, &["statement"]).unwrap_or("<missing statement>")
        )?;
    }
    Ok(())
}

fn append_prompt_effects(out: &mut String, value: &YamlValue) -> Result<()> {
    writeln!(out, "- Effects:")?;
    let Some(effects) = get_path(value, &["effects"]).and_then(YamlValue::as_sequence) else {
        writeln!(out, "  - <missing>")?;
        return Ok(());
    };
    if effects.is_empty() {
        writeln!(out, "  - <none declared>")?;
        return Ok(());
    }
    for effect in effects {
        writeln!(
            out,
            "  - {} ({})",
            get_str(effect, &["name"]).unwrap_or("<unnamed>"),
            get_str(effect, &["kind"]).unwrap_or("<unknown-kind>")
        )?;
    }
    Ok(())
}

fn append_prompt_verification(out: &mut String, value: &YamlValue) -> Result<()> {
    writeln!(out, "- Verification:")?;
    for category in ["laws", "contracts", "scenarios", "boundaries"] {
        let items = get_string_array(value, &["verification", category]);
        writeln!(
            out,
            "  - {category}: {}",
            if items.is_empty() {
                "<none>".to_string()
            } else {
                items.join(", ")
            }
        )?;
    }
    Ok(())
}

fn append_prompt_change_protocols(out: &mut String, value: &YamlValue) -> Result<()> {
    let Some(protocols) = change_protocol_items(value) else {
        return Ok(());
    };
    if protocols.is_empty() {
        return Ok(());
    }

    writeln!(out, "- Change Protocols:")?;
    for protocol in protocols {
        let id = get_str(protocol, &["id"]).unwrap_or("<missing-id>");
        let applies_when = get_str(protocol, &["applies_when"]).unwrap_or("<missing applies_when>");
        writeln!(out, "  - {id}: {applies_when}")?;
        if let Some(classification) = get_str(protocol, &["classification"]) {
            writeln!(out, "    classification: {classification}")?;
        }
        let required_updates = get_string_array(protocol, &["required_updates"]);
        if !required_updates.is_empty() {
            writeln!(out, "    required updates:")?;
            for update in required_updates {
                writeln!(out, "      - {update}")?;
            }
        }
        let verify = get_string_array(protocol, &["verify"]);
        if !verify.is_empty() {
            writeln!(out, "    verify:")?;
            for command in verify {
                writeln!(out, "      - {command}")?;
            }
        }
    }
    Ok(())
}

fn read_git_diff(root: &Path, diff: Option<&str>) -> Result<String> {
    let mut command = Command::new("git");
    command.current_dir(root).arg("diff");
    if let Some(diff) = diff {
        command.arg(diff);
    }
    let output = match command.output() {
        Ok(output) => output,
        Err(error) => return Ok(format!("diff unavailable: {error}")),
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Ok(format!(
            "diff unavailable: git exited with status {}{}",
            output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string()),
            if stderr.trim().is_empty() {
                String::new()
            } else {
                format!(": {}", stderr.trim())
            }
        ));
    }
    Ok(truncate_for_prompt(
        String::from_utf8_lossy(&output.stdout).as_ref(),
        16_000,
    ))
}

fn truncate_for_prompt(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        return value.to_string();
    }
    let mut end = 0;
    for (index, character) in value.char_indices() {
        let next = index + character.len_utf8();
        if next > limit {
            break;
        }
        end = next;
    }
    let mut truncated = value[..end].to_string();
    truncated.push_str("\n[truncated for prompt]\n");
    truncated
}

fn run_impact(root: &Path, diff: Option<&str>, json_output: bool) -> Result<()> {
    let changed_paths = read_git_changed_paths(root, diff)?;
    let report = build_impact_report(root, diff, &changed_paths)?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_impact_report(&report);
    }

    Ok(())
}

fn run_gate(root: &Path, diff: Option<&str>, dry_run: bool, json_output: bool) -> Result<()> {
    let changed_paths = read_git_changed_paths(root, diff)?;
    let impact = build_impact_report(root, diff, &changed_paths)?;
    let mut plan = build_gate_plan(root, diff, &impact);

    if !dry_run {
        for index in 0..plan.actions.len() {
            match run_gate_action(root, &plan.actions[index]) {
                Ok(message) => {
                    plan.report.executable_checks[index].status = GateCheckStatus::Pass;
                    plan.report.executable_checks[index].message = Some(message);
                }
                Err(error) => {
                    plan.report.executable_checks[index].status = GateCheckStatus::Fail;
                    plan.report.executable_checks[index].message = Some(error.to_string());
                }
            }
        }
        plan.report.result = if plan
            .report
            .executable_checks
            .iter()
            .any(|check| check.status == GateCheckStatus::Fail)
        {
            GateResult::Fail
        } else {
            GateResult::Pass
        };
    }

    if json_output {
        println!("{}", serde_json::to_string_pretty(&plan.report)?);
    } else {
        print_gate_report(&plan.report);
    }

    if plan.report.result == GateResult::Fail {
        bail!("RMS gate failed");
    }

    Ok(())
}

fn build_gate_plan(root: &Path, diff: Option<&str>, impact: &ImpactReport) -> GatePlan {
    let mut executable_checks = Vec::new();
    let mut actions = Vec::new();
    let mut seen_executable = BTreeSet::new();
    let mut manual_checks = BTreeSet::new();

    if impact.result != ImpactResult::NoRmsImpact && !impact.changed_paths.is_empty() {
        push_gate_check(
            &mut executable_checks,
            &mut actions,
            &mut seen_executable,
            format!("rms validate --root {}", root.display()),
            GateCheckAction::ValidateRoot,
        );

        if impact.changed_paths.iter().any(|path| {
            matches!(
                path.category,
                ImpactCategory::SystemManifest
                    | ImpactCategory::ContextMap
                    | ImpactCategory::ModuleManifest
                    | ImpactCategory::Contract
                    | ImpactCategory::Operations
                    | ImpactCategory::Glossary
            )
        }) {
            push_gate_check(
                &mut executable_checks,
                &mut actions,
                &mut seen_executable,
                format!("rms compose --root {}", root.display()),
                GateCheckAction::ComposeRoot,
            );
        }

        let diff_arg = diff
            .map(|diff| format!(" --diff {diff}"))
            .unwrap_or_default();
        for module in &impact.affected_modules {
            if module
                .categories
                .iter()
                .any(|category| gate_category_requires_verification(*category))
            {
                if let Some(implementation) = &module.implementation {
                    push_gate_check(
                        &mut executable_checks,
                        &mut actions,
                        &mut seen_executable,
                        format!("rms verify {implementation}"),
                        GateCheckAction::VerifyImplementation(PathBuf::from(implementation)),
                    );
                } else {
                    manual_checks.insert(format!(
                        "Add or identify an implementation binding before verifying {}",
                        module.name
                    ));
                }
            }

            if module
                .categories
                .iter()
                .any(|category| gate_category_requires_review(*category))
            {
                manual_checks.insert(format!(
                    "rms review {} --impact{}",
                    module.manifest, diff_arg
                ));
            }

            if module.categories.iter().any(|category| {
                matches!(
                    category,
                    ImpactCategory::ModuleManifest | ImpactCategory::Contract
                )
            }) {
                manual_checks.insert(format!(
                    "rms check-compat <previous {}> {}",
                    module.manifest, module.manifest
                ));
            }
        }

        for path in &impact.changed_paths {
            if path.module.is_none() && gate_category_requires_review(path.category) {
                manual_checks.insert(format!(
                    "Review {} [{}] for RMS conformance",
                    path.path,
                    impact_category_label(path.category)
                ));
            }
        }
    }

    let result = if executable_checks.is_empty() {
        GateResult::Pass
    } else {
        GateResult::Pending
    };

    GatePlan {
        report: GateReport {
            result,
            root: root.display().to_string(),
            diff: diff.map(ToString::to_string),
            source_revision: impact.source_revision.clone(),
            impact_result: impact.result,
            affected_modules: impact
                .affected_modules
                .iter()
                .map(|module| module.name.clone())
                .collect(),
            executable_checks,
            manual_checks: manual_checks.into_iter().collect(),
        },
        actions,
    }
}

fn push_gate_check(
    checks: &mut Vec<GateCheck>,
    actions: &mut Vec<GateCheckAction>,
    seen: &mut BTreeSet<String>,
    command: String,
    action: GateCheckAction,
) {
    if seen.insert(command.clone()) {
        checks.push(GateCheck {
            command,
            status: GateCheckStatus::Pending,
            message: None,
        });
        actions.push(action);
    }
}

fn gate_category_requires_verification(category: ImpactCategory) -> bool {
    matches!(
        category,
        ImpactCategory::Source
            | ImpactCategory::ImplementationBinding
            | ImpactCategory::VerificationEvidence
    )
}

fn gate_category_requires_review(category: ImpactCategory) -> bool {
    matches!(
        category,
        ImpactCategory::SystemManifest
            | ImpactCategory::ContextMap
            | ImpactCategory::ModuleManifest
            | ImpactCategory::Contract
            | ImpactCategory::ImplementationBinding
            | ImpactCategory::Operations
            | ImpactCategory::Glossary
    )
}

fn run_gate_action(root: &Path, action: &GateCheckAction) -> Result<String> {
    match action {
        GateCheckAction::ValidateRoot => {
            let diagnostics = collect_validation_diagnostics(
                root,
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
            )?;
            let errors = diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity == Severity::Error)
                .count();
            let warnings = diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity == Severity::Warning)
                .count();
            if errors > 0 {
                bail!("{errors} RMS validation error(s)");
            }
            Ok(format!("{warnings} warning(s), no validation errors"))
        }
        GateCheckAction::ComposeRoot => {
            let report = compose_system(root)?;
            if report.result == ComposeResult::Fail {
                bail!(
                    "composition failed with {} finding(s)",
                    report.findings.len()
                );
            }
            Ok(format!(
                "composition {}",
                compose_result_label(report.result)
            ))
        }
        GateCheckAction::VerifyImplementation(implementation) => {
            run_verify_captured(&root.join(implementation))
        }
    }
}

fn run_verify_captured(implementation: &Path) -> Result<String> {
    let manifest = load_manifest(implementation)?;
    let command = get_str(&manifest.value, &["commands", "verify"])
        .ok_or_else(|| anyhow!("implementation binding does not declare `commands.verify`"))?;
    let root = implementation.parent().unwrap_or_else(|| Path::new("."));
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(root)
        .output()
        .with_context(|| format!("failed to run verify command `{command}`"))?;

    if !output.status.success() {
        bail!(
            "verify command failed with status {}{}",
            exit_status_label(output.status),
            command_output_excerpt(&output)
        );
    }

    Ok(format!("verify command passed: {command}"))
}

fn exit_status_label(status: std::process::ExitStatus) -> String {
    status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "signal".to_string())
}

fn command_output_excerpt(output: &std::process::Output) -> String {
    let mut details = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        details.push(format!(
            "stdout: {}",
            truncate_for_prompt(stdout.trim(), 1_000).trim()
        ));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        details.push(format!(
            "stderr: {}",
            truncate_for_prompt(stderr.trim(), 1_000).trim()
        ));
    }
    if details.is_empty() {
        String::new()
    } else {
        format!(" ({})", details.join("; "))
    }
}

fn print_gate_report(report: &GateReport) {
    println!("RMS gate: {}", gate_result_label(report.result));
    println!("RMS impact: {}", impact_result_label(report.impact_result));
    println!("Root: {}", report.root);
    if let Some(diff) = &report.diff {
        println!("Diff: {diff}");
    } else {
        println!("Diff: working tree");
    }
    if let Some(revision) = &report.source_revision {
        println!("Source revision: {revision}");
    }
    print_string_list("Affected modules", &report.affected_modules);

    if report.executable_checks.is_empty() && report.manual_checks.is_empty() {
        println!("No RMS gate checks required.");
        return;
    }

    if !report.executable_checks.is_empty() {
        println!();
        println!("## Executable Checks");
        for check in &report.executable_checks {
            println!(
                "- {}: {}{}",
                gate_check_status_label(check.status),
                check.command,
                check
                    .message
                    .as_ref()
                    .map(|message| format!(" ({message})"))
                    .unwrap_or_default()
            );
        }
    }

    if !report.manual_checks.is_empty() {
        println!();
        print_string_list("Manual obligations", &report.manual_checks);
    }
}

fn gate_result_label(result: GateResult) -> &'static str {
    match result {
        GateResult::Pending => "pending",
        GateResult::Pass => "pass",
        GateResult::Fail => "fail",
    }
}

fn gate_check_status_label(status: GateCheckStatus) -> &'static str {
    match status {
        GateCheckStatus::Pending => "pending",
        GateCheckStatus::Pass => "pass",
        GateCheckStatus::Fail => "fail",
    }
}

#[derive(Clone, Debug)]
struct ChangedPath {
    status: String,
    path: String,
}

#[derive(Clone, Debug)]
struct ImpactModuleMetadata {
    name: String,
    manifest: PathBuf,
    implementation: Option<PathBuf>,
    base: PathBuf,
    source_root: Option<PathBuf>,
    contract_refs: BTreeSet<PathBuf>,
    evidence_refs: BTreeSet<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ImpactResult {
    NoRmsImpact,
    ImplementationOnly,
    EvidenceReview,
    ReviewRequired,
}

#[derive(Clone, Debug, Serialize)]
struct ImpactPath {
    path: String,
    status: String,
    category: ImpactCategory,
    module: Option<String>,
    module_manifest: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ImpactCategory {
    SystemManifest,
    ContextMap,
    ModuleManifest,
    Contract,
    ImplementationBinding,
    Source,
    VerificationEvidence,
    Operations,
    Glossary,
    ConformanceReport,
    WorkbenchConfig,
    Other,
}

#[derive(Clone, Debug, Serialize)]
struct ImpactFinding {
    severity: ImpactResult,
    check: String,
    path: Option<String>,
    module: Option<String>,
    message: String,
}

fn read_git_changed_paths(root: &Path, diff: Option<&str>) -> Result<Vec<ChangedPath>> {
    let mut paths = BTreeMap::<String, ChangedPath>::new();

    if let Some(diff) = diff {
        collect_git_name_status(
            root,
            &["diff", "--relative", "--name-status", diff],
            &mut paths,
        )?;
    } else {
        collect_git_name_status(root, &["diff", "--relative", "--name-status"], &mut paths)?;
        collect_git_name_status(
            root,
            &["diff", "--relative", "--cached", "--name-status"],
            &mut paths,
        )?;
        collect_git_untracked_paths(root, &mut paths)?;
    }

    Ok(paths.into_values().collect())
}

fn collect_git_name_status(
    root: &Path,
    args: &[&str],
    paths: &mut BTreeMap<String, ChangedPath>,
) -> Result<()> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .with_context(|| "failed to start git while reading changed paths")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Not a git repository") || stderr.contains("not a git repository") {
            bail!(
                "git repository required to read changed paths; run `git init` or run deterministic checks directly (`rms validate --root {}`, `rms compose --root {}`, and `rms verify <implementation.yaml>`)",
                root.display(),
                root.display()
            );
        }
        bail!(
            "git changed-path query failed with status {}{}",
            output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string()),
            if stderr.trim().is_empty() {
                String::new()
            } else {
                format!(": {}", stderr.trim())
            }
        );
    }

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() < 2 {
            continue;
        }
        let status = fields[0];
        let path = if status.starts_with('R') || status.starts_with('C') {
            fields.last().copied().unwrap_or(fields[1])
        } else {
            fields[1]
        };
        insert_changed_path(root, paths, status, path);
    }

    Ok(())
}

fn collect_git_untracked_paths(
    root: &Path,
    paths: &mut BTreeMap<String, ChangedPath>,
) -> Result<()> {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "ls-files",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
            ".",
        ])
        .output()
        .with_context(|| "failed to start git while reading untracked paths")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git untracked-path query failed with status {}{}",
            output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string()),
            if stderr.trim().is_empty() {
                String::new()
            } else {
                format!(": {}", stderr.trim())
            }
        );
    }

    for path in String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|path| !path.is_empty())
    {
        insert_changed_path(root, paths, "??", path);
    }

    Ok(())
}

fn insert_changed_path(
    root: &Path,
    paths: &mut BTreeMap<String, ChangedPath>,
    status: &str,
    path: &str,
) {
    let path = display_path(&root_relative_path(root, Path::new(path)));
    paths
        .entry(path.clone())
        .and_modify(|existing| {
            if !existing.status.split('/').any(|item| item == status) {
                existing.status.push('/');
                existing.status.push_str(status);
            }
        })
        .or_insert_with(|| ChangedPath {
            status: status.to_string(),
            path,
        });
}

fn build_impact_report(
    root: &Path,
    diff: Option<&str>,
    changed_paths: &[ChangedPath],
) -> Result<ImpactReport> {
    let modules = discover_impact_modules(root)?;
    Ok(build_impact_report_from_modules(
        root,
        diff,
        changed_paths,
        &modules,
    ))
}

fn build_impact_report_from_modules(
    root: &Path,
    diff: Option<&str>,
    changed_paths: &[ChangedPath],
    modules: &[ImpactModuleMetadata],
) -> ImpactReport {
    let mut report_paths = Vec::new();
    let mut findings = Vec::new();
    let mut module_paths = BTreeMap::<String, BTreeSet<String>>::new();
    let mut module_categories = BTreeMap::<String, BTreeSet<ImpactCategory>>::new();
    let mut overall = ImpactResult::NoRmsImpact;

    for changed_path in changed_paths {
        let normalized_path = normalize_relative_path(Path::new(&changed_path.path));
        let (category, module) = classify_impact_path(&normalized_path, modules);
        let module_name = module.map(|module| module.name.clone());
        let module_manifest = module.map(|module| display_path(&module.manifest));
        let severity = impact_for_category(category, module.is_some());
        overall = overall.max(severity);

        let path_display = display_path(&normalized_path);
        if let Some(module_name) = &module_name {
            module_paths
                .entry(module_name.clone())
                .or_default()
                .insert(path_display.clone());
            module_categories
                .entry(module_name.clone())
                .or_default()
                .insert(category);
        }

        if severity != ImpactResult::NoRmsImpact {
            findings.push(impact_finding(
                severity,
                impact_check_for_category(category),
                Some(path_display.clone()),
                module_name.clone(),
                impact_message_for_category(category),
            ));
        }

        report_paths.push(ImpactPath {
            path: path_display,
            status: changed_path.status.clone(),
            category,
            module: module_name,
            module_manifest,
        });
    }

    let affected_modules = modules
        .iter()
        .filter_map(|module| {
            let changed_paths = module_paths.remove(&module.name)?;
            let categories = module_categories
                .remove(&module.name)
                .unwrap_or_default()
                .into_iter()
                .collect();
            Some(ImpactModuleImpact {
                name: module.name.clone(),
                manifest: display_path(&module.manifest),
                implementation: module
                    .implementation
                    .as_ref()
                    .map(|path| display_path(path)),
                changed_paths: changed_paths.into_iter().collect(),
                categories,
            })
        })
        .collect::<Vec<_>>();

    let recommended_checks =
        impact_recommended_checks(root, diff, &report_paths, &affected_modules);

    ImpactReport {
        result: overall,
        root: root.display().to_string(),
        diff: diff.map(ToString::to_string),
        source_revision: source_revision(root),
        changed_paths: report_paths,
        affected_modules,
        findings,
        recommended_checks,
    }
}

fn discover_impact_modules(root: &Path) -> Result<Vec<ImpactModuleMetadata>> {
    let targets = discover_targets(root, vec![], vec![], vec![], vec![], vec![], vec![])?;
    let mut module_manifests = Vec::new();
    let mut implementation_manifests = Vec::new();

    for target in targets {
        let manifest = load_manifest(&target)?;
        match get_str(&manifest.value, &["spec"]) {
            Some("rms/module/v0.1") => module_manifests.push(manifest),
            Some("rms/implementation/v0.1") => implementation_manifests.push(manifest),
            _ => {}
        }
    }

    let mut modules = Vec::new();
    for manifest in module_manifests {
        let name = get_str(&manifest.value, &["module", "name"])
            .unwrap_or("<unknown>")
            .to_string();
        let manifest_path = root_relative_path(root, &manifest.path);
        let base = manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let implementation = implementation_manifests.iter().find(|implementation| {
            get_str(&implementation.value, &["module"]) == Some(name.as_str())
                && root_relative_path(root, &implementation.path)
                    .parent()
                    .is_some_and(|implementation_base| implementation_base == base)
        });
        let implementation_path =
            implementation.map(|implementation| root_relative_path(root, &implementation.path));
        let source_root = implementation.and_then(|implementation| {
            get_str(&implementation.value, &["source", "root"])
                .map(|source_root| normalize_relative_path(base.join(source_root)))
        });

        let mut contract_refs = BTreeSet::new();
        let mut evidence_refs = BTreeSet::new();
        for reference in referenced_paths(&manifest.value) {
            let path = normalize_relative_path(base.join(&reference));
            if path_has_component(&path, "verification") {
                evidence_refs.insert(path);
            } else if path_has_component(&path, "contracts") {
                contract_refs.insert(path);
            }
        }

        modules.push(ImpactModuleMetadata {
            name,
            manifest: manifest_path,
            implementation: implementation_path,
            base,
            source_root,
            contract_refs,
            evidence_refs,
        });
    }

    modules.sort_by(|left, right| {
        component_count(&right.base)
            .cmp(&component_count(&left.base))
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(modules)
}

fn classify_impact_path<'a>(
    path: &Path,
    modules: &'a [ImpactModuleMetadata],
) -> (ImpactCategory, Option<&'a ImpactModuleMetadata>) {
    for module in modules {
        if path == module.manifest {
            return (ImpactCategory::ModuleManifest, Some(module));
        }
        if module
            .implementation
            .as_ref()
            .is_some_and(|implementation| path == implementation)
        {
            return (ImpactCategory::ImplementationBinding, Some(module));
        }
        let Some(remainder) = path_under_base(path, &module.base) else {
            continue;
        };
        if first_component_is(remainder, "verification") {
            return (ImpactCategory::VerificationEvidence, Some(module));
        }
        if path_matches_any_reference(path, &module.evidence_refs) {
            return (ImpactCategory::VerificationEvidence, Some(module));
        }
        if path_matches_any_reference(path, &module.contract_refs) {
            return (ImpactCategory::Contract, Some(module));
        }
        if first_component_is(remainder, "contracts") {
            return (ImpactCategory::Contract, Some(module));
        }
        if first_component_is(remainder, "ops") {
            return (ImpactCategory::Operations, Some(module));
        }
        if is_system_manifest_path(remainder) {
            return (ImpactCategory::SystemManifest, Some(module));
        }
        if is_context_map_path(remainder) {
            return (ImpactCategory::ContextMap, Some(module));
        }
        if is_glossary_path(remainder) {
            return (ImpactCategory::Glossary, Some(module));
        }
        if is_conformance_report_path(remainder) {
            return (ImpactCategory::ConformanceReport, Some(module));
        }
        if is_workbench_config_path(remainder) {
            return (ImpactCategory::WorkbenchConfig, Some(module));
        }
        if module
            .source_root
            .as_ref()
            .is_some_and(|source_root| path_under_base(path, source_root).is_some())
        {
            return (ImpactCategory::Source, Some(module));
        }
        if !remainder.as_os_str().is_empty() {
            return (ImpactCategory::Other, Some(module));
        }
    }

    if is_system_manifest_path(path) {
        (ImpactCategory::SystemManifest, None)
    } else if is_context_map_path(path) {
        (ImpactCategory::ContextMap, None)
    } else if is_glossary_path(path) {
        (ImpactCategory::Glossary, None)
    } else if is_conformance_report_path(path) {
        (ImpactCategory::ConformanceReport, None)
    } else if is_workbench_config_path(path) {
        (ImpactCategory::WorkbenchConfig, None)
    } else {
        (ImpactCategory::Other, None)
    }
}

fn path_under_base<'a>(path: &'a Path, base: &Path) -> Option<&'a Path> {
    if base.as_os_str().is_empty() {
        Some(path)
    } else {
        path.strip_prefix(base).ok()
    }
}

fn path_matches_any_reference(path: &Path, references: &BTreeSet<PathBuf>) -> bool {
    references
        .iter()
        .any(|reference| path == reference || path_under_base(path, reference).is_some())
}

fn impact_for_category(category: ImpactCategory, has_module: bool) -> ImpactResult {
    match category {
        ImpactCategory::SystemManifest
        | ImpactCategory::ContextMap
        | ImpactCategory::ModuleManifest
        | ImpactCategory::Contract
        | ImpactCategory::ImplementationBinding
        | ImpactCategory::Operations
        | ImpactCategory::Glossary => ImpactResult::ReviewRequired,
        ImpactCategory::VerificationEvidence | ImpactCategory::ConformanceReport => {
            ImpactResult::EvidenceReview
        }
        ImpactCategory::Source => ImpactResult::ImplementationOnly,
        ImpactCategory::Other if has_module => ImpactResult::ImplementationOnly,
        ImpactCategory::WorkbenchConfig | ImpactCategory::Other => ImpactResult::NoRmsImpact,
    }
}

fn impact_check_for_category(category: ImpactCategory) -> &'static str {
    match category {
        ImpactCategory::SystemManifest => "system.changed",
        ImpactCategory::ContextMap => "context-map.changed",
        ImpactCategory::ModuleManifest => "module-manifest.changed",
        ImpactCategory::Contract => "contract.changed",
        ImpactCategory::ImplementationBinding => "implementation-binding.changed",
        ImpactCategory::Source => "source.changed",
        ImpactCategory::VerificationEvidence => "verification-evidence.changed",
        ImpactCategory::Operations => "operations.changed",
        ImpactCategory::Glossary => "glossary.changed",
        ImpactCategory::ConformanceReport => "conformance-report.changed",
        ImpactCategory::WorkbenchConfig => "workbench-config.changed",
        ImpactCategory::Other => "path.changed",
    }
}

fn impact_message_for_category(category: ImpactCategory) -> &'static str {
    match category {
        ImpactCategory::SystemManifest => {
            "system-level artifact changed; validate composition and compatibility assumptions"
        }
        ImpactCategory::ContextMap => {
            "context relationship artifact changed; review module composition boundaries"
        }
        ImpactCategory::ModuleManifest => {
            "module manifest changed; review public surface, effects, dependencies, and compatibility"
        }
        ImpactCategory::Contract => {
            "public contract path changed; classify compatibility before accepting the diff"
        }
        ImpactCategory::ImplementationBinding => {
            "implementation binding changed; review build, verification, dependencies, and source boundary declarations"
        }
        ImpactCategory::Source => {
            "implementation source changed; run declared verification for the affected module"
        }
        ImpactCategory::VerificationEvidence => {
            "verification evidence changed; confirm it still proves the declared promise"
        }
        ImpactCategory::Operations => {
            "operational artifact changed; review recovery, reconciliation, observability, and runtime checks"
        }
        ImpactCategory::Glossary => {
            "domain language changed; review affected contracts and module terminology"
        }
        ImpactCategory::ConformanceReport => {
            "conformance evidence changed; confirm it is tied to the intended source revision"
        }
        ImpactCategory::WorkbenchConfig => {
            "workbench configuration changed; treat as operational input, not module semantics"
        }
        ImpactCategory::Other => "path changed near an RMS module but has no specialized category",
    }
}

fn impact_finding(
    severity: ImpactResult,
    check: impl Into<String>,
    path: Option<String>,
    module: Option<String>,
    message: impl Into<String>,
) -> ImpactFinding {
    ImpactFinding {
        severity,
        check: check.into(),
        path,
        module,
        message: message.into(),
    }
}

fn impact_recommended_checks(
    root: &Path,
    diff: Option<&str>,
    paths: &[ImpactPath],
    modules: &[ImpactModuleImpact],
) -> Vec<String> {
    let mut checks = BTreeSet::new();
    if paths.is_empty() {
        return Vec::new();
    }
    checks.insert(format!("rms validate --root {}", root.display()));

    if paths.iter().any(|path| {
        matches!(
            path.category,
            ImpactCategory::SystemManifest
                | ImpactCategory::ContextMap
                | ImpactCategory::ModuleManifest
                | ImpactCategory::Contract
                | ImpactCategory::Operations
                | ImpactCategory::Glossary
        )
    }) {
        checks.insert(format!("rms compose --root {}", root.display()));
    }

    for module in modules {
        let diff_arg = diff
            .map(|diff| format!(" --diff {diff}"))
            .unwrap_or_default();
        checks.insert(format!("rms review {}{}", module.manifest, diff_arg));

        if module.categories.iter().any(|category| {
            matches!(
                category,
                ImpactCategory::Source
                    | ImpactCategory::ImplementationBinding
                    | ImpactCategory::VerificationEvidence
            )
        }) {
            if let Some(implementation) = &module.implementation {
                checks.insert(format!("rms verify {implementation}"));
            }
        }

        if module.categories.iter().any(|category| {
            matches!(
                category,
                ImpactCategory::ModuleManifest | ImpactCategory::Contract
            )
        }) {
            checks.insert(format!(
                "rms check-compat <previous {}> {}",
                module.manifest, module.manifest
            ));
        }
    }

    checks.into_iter().collect()
}

fn print_impact_report(report: &ImpactReport) {
    println!("RMS impact: {}", impact_result_label(report.result));
    println!("Root: {}", report.root);
    if let Some(diff) = &report.diff {
        println!("Diff: {diff}");
    } else {
        println!("Diff: working tree");
    }
    if let Some(revision) = &report.source_revision {
        println!("Source revision: {revision}");
    }
    print_string_list(
        "Affected modules",
        &report
            .affected_modules
            .iter()
            .map(|module| module.name.clone())
            .collect::<Vec<_>>(),
    );

    if report.changed_paths.is_empty() {
        println!("No changed paths detected.");
        return;
    }

    println!();
    println!("## Changed Paths");
    for path in &report.changed_paths {
        println!(
            "- {} [{}] {}{}",
            path.status,
            impact_category_label(path.category),
            path.path,
            path.module
                .as_ref()
                .map(|module| format!(" ({module})"))
                .unwrap_or_default()
        );
    }

    if !report.findings.is_empty() {
        println!();
        println!("## Findings");
        for finding in &report.findings {
            let path = finding.path.as_deref().unwrap_or("<none>");
            let module = finding.module.as_deref().unwrap_or("<none>");
            println!(
                "- {} [{}] path={} module={}: {}",
                impact_result_label(finding.severity),
                finding.check,
                path,
                module,
                finding.message
            );
        }
    }

    if !report.recommended_checks.is_empty() {
        println!();
        print_string_list("Recommended checks", &report.recommended_checks);
    }
}

fn impact_result_label(result: ImpactResult) -> &'static str {
    match result {
        ImpactResult::NoRmsImpact => "no-rms-impact",
        ImpactResult::ImplementationOnly => "implementation-only",
        ImpactResult::EvidenceReview => "evidence-review",
        ImpactResult::ReviewRequired => "review-required",
    }
}

fn impact_category_label(category: ImpactCategory) -> &'static str {
    match category {
        ImpactCategory::SystemManifest => "system-manifest",
        ImpactCategory::ContextMap => "context-map",
        ImpactCategory::ModuleManifest => "module-manifest",
        ImpactCategory::Contract => "contract",
        ImpactCategory::ImplementationBinding => "implementation-binding",
        ImpactCategory::Source => "source",
        ImpactCategory::VerificationEvidence => "verification-evidence",
        ImpactCategory::Operations => "operations",
        ImpactCategory::Glossary => "glossary",
        ImpactCategory::ConformanceReport => "conformance-report",
        ImpactCategory::WorkbenchConfig => "workbench-config",
        ImpactCategory::Other => "other",
    }
}

fn root_relative_path(root: &Path, path: &Path) -> PathBuf {
    let path = normalize_relative_path(path);
    let root = normalize_relative_path(root);
    if !root.as_os_str().is_empty() {
        if let Ok(stripped) = path.strip_prefix(&root) {
            return normalize_relative_path(stripped);
        }
    }
    path
}

fn normalize_relative_path(path: impl AsRef<Path>) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.as_ref().components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => normalized.push(".."),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn display_path(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        path.display().to_string()
    }
}

fn component_count(path: &Path) -> usize {
    path.components().count()
}

fn first_component_is(path: &Path, expected: &str) -> bool {
    path.components()
        .next()
        .is_some_and(|component| matches!(component, Component::Normal(part) if part == expected))
}

fn file_name_is(path: &Path, expected: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == expected)
}

fn is_system_manifest_path(path: &Path) -> bool {
    file_name_is(path, "system.yaml")
}

fn is_context_map_path(path: &Path) -> bool {
    file_name_is(path, "context-map.yaml")
}

fn is_glossary_path(path: &Path) -> bool {
    file_name_is(path, "GLOSSARY.md")
}

fn is_conformance_report_path(path: &Path) -> bool {
    file_name_is(path, "conformance-report.json")
}

fn is_workbench_config_path(path: &Path) -> bool {
    path.components()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|window| {
            matches!(
                window,
                [Component::Normal(parent), Component::Normal(file)]
                    if *parent == ".rms" && *file == "config.yaml"
            )
        })
}

fn discover_targets(
    root: &Path,
    modules: Vec<PathBuf>,
    systems: Vec<PathBuf>,
    context_maps: Vec<PathBuf>,
    contracts: Vec<PathBuf>,
    implementations: Vec<PathBuf>,
    conformance_reports: Vec<PathBuf>,
) -> Result<Vec<PathBuf>> {
    let mut explicit = Vec::new();
    explicit.extend(modules);
    explicit.extend(systems);
    explicit.extend(context_maps);
    explicit.extend(contracts);
    explicit.extend(implementations);
    explicit.extend(conformance_reports);

    if !explicit.is_empty() {
        return Ok(explicit);
    }

    let mut found = BTreeSet::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file_name == "conformance-report.json" || is_supported_yaml_manifest(path) {
            found.insert(path.to_path_buf());
        }
    }

    Ok(found.into_iter().collect())
}

fn is_supported_yaml_manifest(path: &Path) -> bool {
    if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
        return false;
    }
    let Ok(source) = fs::read_to_string(path) else {
        return false;
    };
    source.lines().take(5).any(|line| {
        matches!(
            line.trim(),
            "spec: rms/system/v0.1"
                | "spec: rms/module/v0.1"
                | "spec: rms/contract/v0.1"
                | "spec: rms/context-map/v0.1"
                | "spec: rms/implementation/v0.1"
        )
    })
}

fn validate_loaded_manifest(manifest: &LoadedManifest, diagnostics: &mut Vec<Diagnostic>) {
    let spec = get_str(&manifest.value, &["spec"]);
    validate_against_embedded_schema(manifest, diagnostics);

    match spec {
        Some("rms/system/v0.1") => validate_system(manifest, diagnostics),
        Some("rms/module/v0.1") => validate_module(manifest, diagnostics),
        Some("rms/contract/v0.1") => validate_contract(manifest, diagnostics),
        Some("rms/context-map/v0.1") => validate_context_map(manifest, diagnostics),
        Some("rms/implementation/v0.1") => validate_implementation(manifest, diagnostics),
        Some("rms/conformance/v0.1") => {}
        Some(other) => diagnostics.push(error(
            "manifest.spec",
            &manifest.path,
            format!("unsupported spec identifier `{other}`"),
        )),
        None => diagnostics.push(error(
            "manifest.spec",
            &manifest.path,
            "missing required `spec` field",
        )),
    }

    scan_for_secret_like_keys(&manifest.value, &manifest.path, diagnostics);
}

fn validate_against_embedded_schema(manifest: &LoadedManifest, diagnostics: &mut Vec<Diagnostic>) {
    let Some(spec) = get_str(&manifest.value, &["spec"]) else {
        return;
    };
    let Some(schema_source) = schema_for_spec(spec) else {
        return;
    };

    let schema: JsonValue = match serde_json::from_str(schema_source) {
        Ok(schema) => schema,
        Err(error) => {
            diagnostics.push(error_diagnostic(
                "schema.internal.parse",
                &manifest.path,
                format!("embedded schema for `{spec}` could not be parsed: {error}"),
            ));
            return;
        }
    };

    let instance = match serde_json::to_value(&manifest.value) {
        Ok(instance) => instance,
        Err(error) => {
            diagnostics.push(error_diagnostic(
                "schema.instance.convert",
                &manifest.path,
                format!("manifest could not be converted to JSON for schema validation: {error}"),
            ));
            return;
        }
    };

    let validator = match jsonschema::validator_for(&schema) {
        Ok(validator) => validator,
        Err(error) => {
            diagnostics.push(error_diagnostic(
                "schema.internal.compile",
                &manifest.path,
                format!("embedded schema for `{spec}` could not be compiled: {error}"),
            ));
            return;
        }
    };

    for validation_error in validator.iter_errors(&instance) {
        diagnostics.push(error_diagnostic(
            "schema.validate",
            &manifest.path,
            format!(
                "{} at `{}`",
                validation_error,
                validation_error.instance_path()
            ),
        ));
    }
}

fn schema_for_spec(spec: &str) -> Option<&'static str> {
    match spec {
        "rms/system/v0.1" => Some(include_str!("../../../../schemas/system.schema.json")),
        "rms/module/v0.1" => Some(include_str!("../../../../schemas/module.schema.json")),
        "rms/contract/v0.1" => Some(include_str!("../../../../schemas/contract.schema.json")),
        "rms/context-map/v0.1" => Some(include_str!("../../../../schemas/context-map.schema.json")),
        "rms/implementation/v0.1" => Some(include_str!(
            "../../../../schemas/implementation.schema.json"
        )),
        "rms/conformance/v0.1" => Some(include_str!("../../../../schemas/conformance.schema.json")),
        _ => None,
    }
}

fn validate_system(manifest: &LoadedManifest, diagnostics: &mut Vec<Diagnostic>) {
    require_str(manifest, diagnostics, "system.name", &["system", "name"]);
    require_str(
        manifest,
        diagnostics,
        "system.version",
        &["system", "version"],
    );
    require_str(
        manifest,
        diagnostics,
        "system.purpose",
        &["system", "purpose"],
    );
    require_array(manifest, diagnostics, "contexts", &["contexts"]);
    require_array(
        manifest,
        diagnostics,
        "public_interfaces",
        &["public_interfaces"],
    );
    require_array(manifest, diagnostics, "invariants", &["invariants"]);
    require_str(
        manifest,
        diagnostics,
        "compatibility.policy",
        &["compatibility", "policy"],
    );

    check_contract_refs(manifest, diagnostics, &["public_interfaces"]);
    check_optional_path(manifest, diagnostics, &["glossary"]);
    check_optional_path(manifest, diagnostics, &["context_map"]);
}

fn validate_module(manifest: &LoadedManifest, diagnostics: &mut Vec<Diagnostic>) {
    require_str(manifest, diagnostics, "module.name", &["module", "name"]);
    require_str(
        manifest,
        diagnostics,
        "module.version",
        &["module", "version"],
    );
    require_str(manifest, diagnostics, "module.kind", &["module", "kind"]);
    require_str(
        manifest,
        diagnostics,
        "module.purpose",
        &["module", "purpose"],
    );

    for required in [
        "profiles",
        "owns",
        "provides",
        "requires",
        "invariants",
        "effects",
        "compatibility",
        "verification",
    ] {
        if get_path(&manifest.value, &[required]).is_none() {
            diagnostics.push(error(
                format!("module.required.{required}"),
                &manifest.path,
                format!("missing required `{required}` section"),
            ));
        }
    }

    let profiles = get_string_array(&manifest.value, &["profiles"]);
    if !profiles.iter().any(|profile| profile == "core") {
        diagnostics.push(error(
            "profiles.core",
            &manifest.path,
            "`profiles` must include `core`",
        ));
    }
    for profile in &profiles {
        if !matches!(
            profile.as_str(),
            "core" | "stateful" | "distributed" | "workflow" | "boundary"
        ) {
            diagnostics.push(error(
                "profiles.allowed",
                &manifest.path,
                format!("unknown profile `{profile}`"),
            ));
        }
    }

    require_str(
        manifest,
        diagnostics,
        "compatibility.policy",
        &["compatibility", "policy"],
    );
    require_verification_categories(manifest, diagnostics);
    check_contract_refs(manifest, diagnostics, &["provides"]);
    check_contract_refs(manifest, diagnostics, &["requires"]);
    check_invariant_evidence(manifest, diagnostics);
    check_verification_paths(manifest, diagnostics);
    check_change_protocols(manifest, diagnostics);
    check_profile_obligations(manifest, diagnostics, &profiles);
}

fn validate_contract(manifest: &LoadedManifest, diagnostics: &mut Vec<Diagnostic>) {
    require_str(manifest, diagnostics, "contract.name", &["name"]);
    require_str(manifest, diagnostics, "contract.kind", &["kind"]);
    require_str(manifest, diagnostics, "contract.meaning", &["meaning"]);

    for field in ["preconditions", "postconditions"] {
        validate_contract_assumptions(manifest, diagnostics, field);
    }
    validate_contract_named_statements(manifest, diagnostics, "failure_categories");
}

fn validate_contract_assumptions(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    field: &str,
) {
    let Some(items) = get_path(&manifest.value, &[field]).and_then(YamlValue::as_sequence) else {
        return;
    };

    let mut ids = BTreeSet::new();
    for item in items {
        let Some(id) = get_str(item, &["id"]) else {
            continue;
        };
        if !ids.insert(id.to_string()) {
            diagnostics.push(error(
                format!("contract.{field}.duplicate-id"),
                &manifest.path,
                format!("duplicate `{field}` id `{id}`"),
            ));
        }
        if let Some(path) = get_str(item, &["verified_by"]) {
            check_relative_ref(
                manifest,
                diagnostics,
                format!("references.contract.{field}.verified-by"),
                path,
                "referenced contract assumption evidence does not exist",
            );
        }
    }
}

fn validate_contract_named_statements(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    field: &str,
) {
    let Some(items) = get_path(&manifest.value, &[field]).and_then(YamlValue::as_sequence) else {
        return;
    };

    let mut ids = BTreeSet::new();
    for item in items {
        let Some(id) = get_str(item, &["id"]) else {
            continue;
        };
        if !ids.insert(id.to_string()) {
            diagnostics.push(error(
                format!("contract.{field}.duplicate-id"),
                &manifest.path,
                format!("duplicate `{field}` id `{id}`"),
            ));
        }
    }
}

fn validate_context_map(manifest: &LoadedManifest, diagnostics: &mut Vec<Diagnostic>) {
    if get_path(&manifest.value, &["contexts"]).is_none()
        && get_path(&manifest.value, &["relationships"]).is_none()
    {
        diagnostics.push(error(
            "context-map.content",
            &manifest.path,
            "context map should declare `contexts`, `relationships`, or both",
        ));
    }
    check_contract_refs(manifest, diagnostics, &["relationships"]);
}

fn validate_implementation(manifest: &LoadedManifest, diagnostics: &mut Vec<Diagnostic>) {
    require_str(manifest, diagnostics, "module", &["module"]);
    require_str(manifest, diagnostics, "binding", &["binding"]);
    require_str(manifest, diagnostics, "source.root", &["source", "root"]);
    require_str(
        manifest,
        diagnostics,
        "source.public_entrypoint",
        &["source", "public_entrypoint"],
    );
    require_str(
        manifest,
        diagnostics,
        "commands.build",
        &["commands", "build"],
    );
    require_str(
        manifest,
        diagnostics,
        "commands.verify",
        &["commands", "verify"],
    );

    check_optional_path(manifest, diagnostics, &["source", "root"]);
    check_optional_path(manifest, diagnostics, &["source", "public_entrypoint"]);
    validate_semantic_function_declarations(manifest, diagnostics);

    match get_str(&manifest.value, &["binding"]) {
        Some("rust") => validate_rust_implementation(manifest, diagnostics),
        Some("swift") => validate_swift_implementation(manifest, diagnostics),
        Some("executable") => validate_executable_implementation(manifest, diagnostics),
        _ => {}
    }
}

fn validate_semantic_function_declarations(
    implementation: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(functions) = semantic_function_items(implementation) else {
        return;
    };

    let base = implementation
        .path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let module_manifest = load_binding_module_manifest(implementation, base);
    let invariant_ids = module_manifest
        .as_ref()
        .map(module_invariant_ids)
        .unwrap_or_default();
    let mut ids = BTreeSet::new();

    for function in functions {
        if let Some(id) = get_str(function, &["id"]) {
            if !ids.insert(id.to_string()) {
                diagnostics.push(error(
                    "implementation.semantic-functions.duplicate-id",
                    &implementation.path,
                    format!("duplicate semantic function id `{id}`"),
                ));
            }
        }

        for contract in get_string_array(function, &["discharges", "contracts"]) {
            check_relative_ref(
                implementation,
                diagnostics,
                "references.semantic-functions.contract",
                &contract,
                "semantic function references a contract that does not exist",
            );
        }

        for invariant in get_string_array(function, &["discharges", "invariants"]) {
            if module_manifest.is_some() && !invariant_ids.contains(&invariant) {
                diagnostics.push(error(
                    "implementation.semantic-functions.invariant",
                    &implementation.path,
                    format!(
                        "semantic function references undeclared module invariant `{invariant}`"
                    ),
                ));
            }
        }

        for category in ["laws", "contracts", "scenarios", "boundaries"] {
            for path in get_string_array(function, &["evidence", category]) {
                check_relative_ref(
                    implementation,
                    diagnostics,
                    format!("references.semantic-functions.evidence.{category}"),
                    &path,
                    "semantic function references evidence that does not exist",
                );
            }
        }
    }
}

fn semantic_function_items(manifest: &LoadedManifest) -> Option<&[YamlValue]> {
    get_path(&manifest.value, &["semantic_functions"])?
        .as_sequence()
        .map(Vec::as_slice)
}

fn module_invariant_ids(manifest: &LoadedManifest) -> BTreeSet<String> {
    get_path(&manifest.value, &["invariants"])
        .and_then(YamlValue::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| get_str(item, &["id"]))
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn validate_rust_implementation(manifest: &LoadedManifest, diagnostics: &mut Vec<Diagnostic>) {
    let base = manifest.path.parent().unwrap_or_else(|| Path::new("."));
    let Some(source_root_ref) = get_str(&manifest.value, &["source", "root"]) else {
        return;
    };
    let Some(public_entrypoint_ref) = get_str(&manifest.value, &["source", "public_entrypoint"])
    else {
        return;
    };

    let source_root = base.join(source_root_ref);
    let public_entrypoint = base.join(public_entrypoint_ref);

    if public_entrypoint.extension().and_then(|ext| ext.to_str()) != Some("rs") {
        diagnostics.push(error(
            "implementation.rust.public-entrypoint",
            &manifest.path,
            "`source.public_entrypoint` must point to a Rust source file",
        ));
    }

    if public_entrypoint.exists()
        && source_root.exists()
        && !public_entrypoint.starts_with(&source_root)
    {
        diagnostics.push(error(
            "implementation.rust.public-entrypoint",
            &manifest.path,
            "`source.public_entrypoint` must be inside `source.root` for Rust bindings",
        ));
    }

    let cargo_manifest = rust_cargo_manifest_path(manifest, base, &source_root);
    if !cargo_manifest.exists() {
        diagnostics.push(error(
            "implementation.rust.cargo-manifest",
            &manifest.path,
            format!(
                "Rust binding requires a Cargo manifest at `{}`",
                display_relative(base, &cargo_manifest)
            ),
        ));
        return;
    }

    let cargo = match load_toml(&cargo_manifest) {
        Ok(cargo) => cargo,
        Err(load_error) => {
            diagnostics.push(error(
                "implementation.rust.cargo-manifest.parse",
                &manifest.path,
                format!(
                    "failed to parse Cargo manifest `{}`: {load_error}",
                    display_relative(base, &cargo_manifest)
                ),
            ));
            return;
        }
    };

    validate_cargo_package_shape(manifest, diagnostics, &cargo_manifest, &cargo);
    validate_declared_rust_package(manifest, diagnostics, &cargo);
    validate_rust_dependency_allowlist(manifest, diagnostics, &cargo);
    validate_rust_public_modules(manifest, diagnostics, &public_entrypoint);
    validate_rust_source_boundaries(manifest, diagnostics, &source_root);
    validate_rust_typing(manifest, diagnostics, base, &source_root);
}

fn validate_swift_implementation(manifest: &LoadedManifest, diagnostics: &mut Vec<Diagnostic>) {
    let base = manifest.path.parent().unwrap_or_else(|| Path::new("."));
    let Some(source_root_ref) = get_str(&manifest.value, &["source", "root"]) else {
        return;
    };
    let Some(public_entrypoint_ref) = get_str(&manifest.value, &["source", "public_entrypoint"])
    else {
        return;
    };

    let source_root = base.join(source_root_ref);
    let public_entrypoint = base.join(public_entrypoint_ref);

    if public_entrypoint.extension().and_then(|ext| ext.to_str()) != Some("swift") {
        diagnostics.push(error(
            "implementation.swift.public-entrypoint",
            &manifest.path,
            "`source.public_entrypoint` must point to a Swift source file",
        ));
    }

    if public_entrypoint.exists()
        && source_root.exists()
        && !public_entrypoint.starts_with(&source_root)
    {
        diagnostics.push(error(
            "implementation.swift.public-entrypoint",
            &manifest.path,
            "`source.public_entrypoint` must be inside `source.root` for Swift bindings",
        ));
    }

    let package_manifest = swift_package_manifest_path(manifest, base);
    if !package_manifest.exists() {
        diagnostics.push(error(
            "implementation.swift.package-manifest",
            &manifest.path,
            format!(
                "Swift binding requires a package manifest at `{}`",
                display_relative(base, &package_manifest)
            ),
        ));
        return;
    }

    let package_source = match fs::read_to_string(&package_manifest) {
        Ok(source) => source,
        Err(read_error) => {
            diagnostics.push(error(
                "implementation.swift.package-manifest.read",
                &manifest.path,
                format!(
                    "failed to read Swift package manifest `{}`: {read_error}",
                    display_relative(base, &package_manifest)
                ),
            ));
            return;
        }
    };

    validate_swift_package_shape(manifest, diagnostics, &package_manifest, &package_source);
    validate_declared_swift_package(manifest, diagnostics, &package_source);
    validate_declared_swift_target(manifest, diagnostics, &source_root);
    validate_swift_source_boundaries(manifest, diagnostics, &source_root);
    validate_swift_typing(manifest, diagnostics, base, &source_root);
}

fn validate_executable_implementation(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let base = manifest.path.parent().unwrap_or_else(|| Path::new("."));
    let Some(source_root_ref) = get_str(&manifest.value, &["source", "root"]) else {
        return;
    };
    let Some(public_entrypoint_ref) = get_str(&manifest.value, &["source", "public_entrypoint"])
    else {
        return;
    };

    let source_root = base.join(source_root_ref);
    let public_entrypoint = base.join(public_entrypoint_ref);

    if public_entrypoint.exists()
        && source_root.exists()
        && !public_entrypoint.starts_with(&source_root)
    {
        diagnostics.push(error(
            "implementation.executable.public-entrypoint",
            &manifest.path,
            "`source.public_entrypoint` must be inside `source.root` for executable bindings",
        ));
    }

    if get_str(&manifest.value, &["toolchain", "runner"]).is_none() {
        diagnostics.push(warning(
            "implementation.executable.runner.declared",
            &manifest.path,
            "executable bindings should declare `toolchain.runner` to name the command environment",
        ));
    }
}

fn swift_package_manifest_path(manifest: &LoadedManifest, base: &Path) -> PathBuf {
    if let Some(path) = get_str(&manifest.value, &["toolchain", "package_manifest"]) {
        base.join(path)
    } else {
        base.join("Package.swift")
    }
}

fn validate_swift_package_shape(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    package_manifest: &Path,
    package_source: &str,
) {
    if !package_source.contains("import PackageDescription") || !package_source.contains("Package(")
    {
        diagnostics.push(error(
            "implementation.swift.package-manifest.shape",
            &manifest.path,
            format!(
                "Swift package manifest `{}` must import PackageDescription and declare `Package(...)`",
                package_manifest.display()
            ),
        ));
    }

    if parse_swift_package_name(package_source).is_none() {
        diagnostics.push(error(
            "implementation.swift.package.name",
            &manifest.path,
            "Swift package bindings must declare a package `name`",
        ));
    }
}

fn validate_declared_swift_package(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    package_source: &str,
) {
    let declared_package = get_str(&manifest.value, &["toolchain", "package"]);
    let package_name = parse_swift_package_name(package_source);

    match (declared_package, package_name.as_deref()) {
        (Some(declared), Some(actual)) if declared != actual => diagnostics.push(error(
            "implementation.swift.package.match",
            &manifest.path,
            format!("`toolchain.package` declares `{declared}` but Swift package is `{actual}`"),
        )),
        (None, Some(actual)) => diagnostics.push(warning(
            "implementation.swift.package.declared",
            &manifest.path,
            format!("Swift binding should declare `toolchain.package: {actual}`"),
        )),
        _ => {}
    }
}

fn validate_declared_swift_target(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    source_root: &Path,
) {
    let Some(target) = get_str(&manifest.value, &["toolchain", "target"]) else {
        diagnostics.push(warning(
            "implementation.swift.target.declared",
            &manifest.path,
            "Swift binding should declare `toolchain.target`",
        ));
        return;
    };

    if source_root.exists()
        && source_root
            .file_name()
            .and_then(|name| name.to_str())
            .is_none_or(|name| name != target)
    {
        diagnostics.push(warning(
            "implementation.swift.target.source-root",
            &manifest.path,
            format!("`source.root` usually points at `Sources/{target}` for Swift package targets"),
        ));
    }
}

fn validate_swift_source_boundaries(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    source_root: &Path,
) {
    if !source_root.exists() {
        return;
    }

    let allowed_external_modules: BTreeSet<_> = get_string_array(
        &manifest.value,
        &["dependencies", "allowed_external_modules"],
    )
    .into_iter()
    .collect();
    let allowed_public_reexports: BTreeSet<_> = get_string_array(
        &manifest.value,
        &["architecture", "allowed_public_reexports"],
    )
    .into_iter()
    .collect();
    let target = get_str(&manifest.value, &["toolchain", "target"]);

    for path in swift_source_files(source_root) {
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(read_error) => {
                diagnostics.push(error(
                    "implementation.swift.source.read",
                    &manifest.path,
                    format!(
                        "failed to read Swift source `{}`: {read_error}",
                        path.display()
                    ),
                ));
                continue;
            }
        };

        for import in collect_swift_imports(&source) {
            if is_swift_standard_module(&import.module) || target == Some(import.module.as_str()) {
                continue;
            }

            if !allowed_external_modules.contains(&import.module) {
                diagnostics.push(error(
                    "implementation.swift.imports.declared",
                    &manifest.path,
                    format!(
                        "Swift source `{}` imports external module `{}` not declared in `dependencies.allowed_external_modules`",
                        path.display(),
                        import.module
                    ),
                ));
            }

            if import.is_public_reexport && !allowed_public_reexports.contains(&import.module) {
                diagnostics.push(error(
                    "implementation.swift.reexports.external",
                    &manifest.path,
                    format!(
                        "Swift source `{}` publicly re-exports module `{}` without `architecture.allowed_public_reexports`",
                        path.display(),
                        import.module
                    ),
                ));
            }
        }
    }
}

fn validate_swift_typing(
    implementation: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    base: &Path,
    source_root: &Path,
) {
    if !source_root.exists() {
        return;
    }

    let module_manifest = load_binding_module_manifest(implementation, base);
    let mut summary = SwiftTypingSummary::default();

    for path in swift_source_files(source_root) {
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(read_error) => {
                diagnostics.push(error(
                    "implementation.swift.typing.source.read",
                    &implementation.path,
                    format!(
                        "failed to read Swift source `{}`: {read_error}",
                        path.display()
                    ),
                ));
                continue;
            }
        };

        inspect_swift_typing_file(implementation, diagnostics, &path, &source, &mut summary);
    }

    validate_swift_constructor_evidence(implementation, diagnostics, &summary);
    validate_swift_stateful_representation(
        implementation,
        diagnostics,
        module_manifest.as_ref(),
        &summary,
    );
}

fn rust_cargo_manifest_path(manifest: &LoadedManifest, base: &Path, source_root: &Path) -> PathBuf {
    if let Some(path) = get_str(&manifest.value, &["toolchain", "cargo_manifest"]) {
        base.join(path)
    } else {
        source_root.join("Cargo.toml")
    }
}

fn validate_cargo_package_shape(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    cargo_manifest: &Path,
    cargo: &TomlValue,
) {
    let has_package = cargo.get("package").and_then(TomlValue::as_table).is_some();
    let has_workspace = cargo
        .get("workspace")
        .and_then(TomlValue::as_table)
        .is_some();

    if !has_package && !has_workspace {
        diagnostics.push(error(
            "implementation.rust.cargo-manifest.shape",
            &manifest.path,
            format!(
                "Cargo manifest `{}` must declare `[package]` or `[workspace]`",
                cargo_manifest.display()
            ),
        ));
    }

    if let Some(package) = cargo.get("package").and_then(TomlValue::as_table) {
        if package.get("name").and_then(TomlValue::as_str).is_none() {
            diagnostics.push(error(
                "implementation.rust.package.name",
                &manifest.path,
                "Rust package bindings must declare `package.name`",
            ));
        }

        if package.get("edition").and_then(TomlValue::as_str).is_none() {
            diagnostics.push(warning(
                "implementation.rust.package.edition",
                &manifest.path,
                "Rust package should declare `package.edition` explicitly",
            ));
        }
    }
}

fn validate_declared_rust_package(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    cargo: &TomlValue,
) {
    let declared_package = get_str(&manifest.value, &["toolchain", "package"]);
    let cargo_package = cargo
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(TomlValue::as_str);

    match (declared_package, cargo_package) {
        (Some(declared), Some(actual)) if declared != actual => diagnostics.push(error(
            "implementation.rust.package.match",
            &manifest.path,
            format!("`toolchain.package` declares `{declared}` but Cargo package is `{actual}`"),
        )),
        (None, Some(actual)) => diagnostics.push(warning(
            "implementation.rust.package.declared",
            &manifest.path,
            format!("Rust binding should declare `toolchain.package: {actual}`"),
        )),
        (None, None)
            if cargo
                .get("workspace")
                .and_then(TomlValue::as_table)
                .is_some() =>
        {
            diagnostics.push(warning(
                "implementation.rust.package.declared",
                &manifest.path,
                "workspace Rust bindings should declare `toolchain.package`",
            ));
        }
        _ => {}
    }
}

fn validate_rust_dependency_allowlist(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    cargo: &TomlValue,
) {
    let dependencies = collect_rust_dependencies(cargo);
    if dependencies.is_empty() {
        return;
    }

    let allowed = get_string_array(
        &manifest.value,
        &["dependencies", "allowed_external_crates"],
    );
    if allowed.is_empty() {
        diagnostics.push(warning(
            "implementation.rust.dependencies.allowlist",
            &manifest.path,
            "Rust binding should declare `dependencies.allowed_external_crates` to make crate dependencies explicit",
        ));
        return;
    }

    let allowed: BTreeSet<_> = allowed.into_iter().collect();
    for dependency in dependencies {
        if !allowed.contains(&dependency) {
            diagnostics.push(error(
                "implementation.rust.dependencies.allowlist",
                &manifest.path,
                format!(
                    "Cargo dependency `{dependency}` is not declared in `dependencies.allowed_external_crates`"
                ),
            ));
        }
    }
}

fn validate_rust_public_modules(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    public_entrypoint: &Path,
) {
    let allowed_modules = get_string_array(&manifest.value, &["architecture", "public_modules"]);
    if allowed_modules.is_empty() || !public_entrypoint.exists() {
        return;
    }

    let allowed: BTreeSet<_> = allowed_modules.into_iter().collect();
    let source = match fs::read_to_string(public_entrypoint) {
        Ok(source) => source,
        Err(read_error) => {
            diagnostics.push(error(
                "implementation.rust.public-modules.read",
                &manifest.path,
                format!(
                    "failed to read public entrypoint `{}`: {read_error}",
                    public_entrypoint.display()
                ),
            ));
            return;
        }
    };

    for module in public_modules_declared_in_source(&source) {
        if !allowed.contains(&module) {
            diagnostics.push(error(
                "implementation.rust.public-modules",
                &manifest.path,
                format!(
                    "public module `{module}` is not declared in `architecture.public_modules`"
                ),
            ));
        }
    }
}

fn validate_rust_source_boundaries(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    source_root: &Path,
) {
    if !source_root.exists() {
        return;
    }

    let allowed_external_crates: BTreeSet<_> = get_string_array(
        &manifest.value,
        &["dependencies", "allowed_external_crates"],
    )
    .into_iter()
    .collect();
    let public_modules: BTreeSet<_> =
        get_string_array(&manifest.value, &["architecture", "public_modules"])
            .into_iter()
            .collect();
    let mut local_modules = local_module_roots(source_root);
    local_modules.extend(public_modules.iter().cloned());
    let allowed_public_reexports: BTreeSet<_> = get_string_array(
        &manifest.value,
        &["architecture", "allowed_public_reexports"],
    )
    .into_iter()
    .collect();

    for path in rust_source_files(source_root) {
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(read_error) => {
                diagnostics.push(error(
                    "implementation.rust.source.read",
                    &manifest.path,
                    format!(
                        "failed to read Rust source `{}`: {read_error}",
                        path.display()
                    ),
                ));
                continue;
            }
        };
        let parsed = match syn::parse_file(&source) {
            Ok(parsed) => parsed,
            Err(parse_error) => {
                diagnostics.push(error(
                    "implementation.rust.source.parse",
                    &manifest.path,
                    format!(
                        "failed to parse Rust source `{}`: {parse_error}",
                        path.display()
                    ),
                ));
                continue;
            }
        };

        for import in collect_rust_imports(&parsed) {
            match rust_import_root_kind(&import.root, &local_modules) {
                RustImportRootKind::External => {
                    if !allowed_external_crates.contains(&import.root) {
                        diagnostics.push(error(
                            "implementation.rust.imports.declared",
                            &manifest.path,
                            format!(
                                "Rust source `{}` imports external crate `{}` not declared in `dependencies.allowed_external_crates`",
                                path.display(),
                                import.root
                            ),
                        ));
                    }
                    if import.is_public && !allowed_public_reexports.contains(&import.root) {
                        diagnostics.push(error(
                            "implementation.rust.reexports.external",
                            &manifest.path,
                            format!(
                                "Rust source `{}` publicly re-exports external crate `{}` without `architecture.allowed_public_reexports`",
                                path.display(),
                                import.root
                            ),
                        ));
                    }
                }
                RustImportRootKind::LocalModule => {
                    if import.is_public
                        && !public_modules.is_empty()
                        && !public_modules.contains(&import.root)
                    {
                        diagnostics.push(error(
                            "implementation.rust.reexports.private-module",
                            &manifest.path,
                            format!(
                                "Rust source `{}` publicly re-exports local module `{}` that is not declared in `architecture.public_modules`",
                                path.display(),
                                import.root
                            ),
                        ));
                    }
                }
                RustImportRootKind::Standard | RustImportRootKind::SelfQualified => {}
            }
        }
    }
}

fn validate_rust_typing(
    implementation: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    base: &Path,
    source_root: &Path,
) {
    if !source_root.exists() {
        return;
    }

    let module_manifest = load_binding_module_manifest(implementation, base);
    let mut summary = RustTypingSummary::default();

    for path in rust_source_files(source_root) {
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(read_error) => {
                diagnostics.push(error(
                    "implementation.rust.typing.source.read",
                    &implementation.path,
                    format!(
                        "failed to read Rust source `{}`: {read_error}",
                        path.display()
                    ),
                ));
                continue;
            }
        };
        let parsed = match syn::parse_file(&source) {
            Ok(parsed) => parsed,
            Err(parse_error) => {
                diagnostics.push(error(
                    "implementation.rust.typing.source.parse",
                    &implementation.path,
                    format!(
                        "failed to parse Rust source `{}`: {parse_error}",
                        path.display()
                    ),
                ));
                continue;
            }
        };

        inspect_rust_typing_file(implementation, diagnostics, &path, &parsed, &mut summary);
    }

    validate_rust_constructor_evidence(implementation, diagnostics, &summary);
    validate_rust_stateful_representation(
        implementation,
        diagnostics,
        module_manifest.as_ref(),
        &summary,
    );
    validate_rust_semantic_function_symbols(implementation, diagnostics, &summary);
}

fn inspect_rust_typing_file(
    implementation: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    path: &Path,
    parsed: &syn::File,
    summary: &mut RustTypingSummary,
) {
    let allowed_public_field_structs: BTreeSet<_> = get_string_array(
        &implementation.value,
        &["architecture", "allowed_public_field_structs"],
    )
    .into_iter()
    .collect();
    let allowed_primitive_aliases: BTreeSet<_> = get_string_array(
        &implementation.value,
        &["architecture", "allowed_primitive_type_aliases"],
    )
    .into_iter()
    .collect();
    let allow_panics =
        get_bool(&implementation.value, &["architecture", "allow_panics"]).unwrap_or(false);

    for item in &parsed.items {
        if has_cfg_test_attr(item_attrs(item)) {
            continue;
        }

        match item {
            Item::Struct(item_struct) => {
                inspect_rust_struct_typing(
                    implementation,
                    diagnostics,
                    path,
                    item_struct,
                    &allowed_public_field_structs,
                    summary,
                );
            }
            Item::Enum(item_enum) => {
                if matches!(item_enum.vis, Visibility::Public(_)) {
                    summary.public_types.insert(item_enum.ident.to_string());
                } else {
                    summary.private_types.insert(item_enum.ident.to_string());
                }
            }
            Item::Type(item_type) => {
                if matches!(item_type.vis, Visibility::Public(_)) {
                    summary.public_types.insert(item_type.ident.to_string());
                    if is_primitive_alias_target(&item_type.ty)
                        && !allowed_primitive_aliases.contains(&item_type.ident.to_string())
                    {
                        diagnostics.push(error(
                            "implementation.rust.typing.primitive-alias",
                            &implementation.path,
                            format!(
                                "public type alias `{}` in `{}` points at a primitive; prefer a newtype/opaque value or declare it in `architecture.allowed_primitive_type_aliases`",
                                item_type.ident,
                                path.display()
                            ),
                        ));
                    }
                } else {
                    summary.private_types.insert(item_type.ident.to_string());
                }
            }
            Item::Impl(item_impl) => collect_rust_impl_methods(item_impl, summary),
            Item::Fn(item_fn) => {
                summary.functions.insert(item_fn.sig.ident.to_string());
            }
            _ => {}
        }

        if !allow_panics {
            let mut visitor = RustFailureVisitor::default();
            visitor.visit_item(item);
            for failure in visitor.failures {
                diagnostics.push(error(
                    "implementation.rust.typing.failure-discipline",
                    &implementation.path,
                    format!(
                        "Rust source `{}` uses `{}` in non-test domain code; prefer explicit result/error types for expected failures",
                        path.display(),
                        failure
                    ),
                ));
            }
        }
    }
}

fn inspect_rust_struct_typing(
    implementation: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    path: &Path,
    item_struct: &ItemStruct,
    allowed_public_field_structs: &BTreeSet<String>,
    summary: &mut RustTypingSummary,
) {
    let struct_name = item_struct.ident.to_string();
    if matches!(item_struct.vis, Visibility::Public(_)) {
        summary.public_types.insert(struct_name.clone());
    } else {
        summary.private_types.insert(struct_name.clone());
    }

    let public_fields = public_struct_field_count(item_struct);
    if public_fields > 0 && !allowed_public_field_structs.contains(&struct_name) {
        diagnostics.push(error(
            "implementation.rust.typing.public-fields",
            &implementation.path,
            format!(
                "public struct `{struct_name}` in `{}` exposes {public_fields} public field(s); prefer private fields plus validated constructors/accessors or declare it in `architecture.allowed_public_field_structs`",
                path.display()
            ),
        ));
    }

    if matches!(item_struct.vis, Visibility::Public(_))
        && private_struct_field_count(item_struct) > 0
    {
        summary
            .public_structs_with_private_fields
            .insert(struct_name);
    }
}

fn validate_rust_constructor_evidence(
    implementation: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    summary: &RustTypingSummary,
) {
    let allowed_missing_constructors: BTreeSet<_> = get_string_array(
        &implementation.value,
        &["architecture", "allowed_missing_constructors"],
    )
    .into_iter()
    .collect();

    for struct_name in &summary.public_structs_with_private_fields {
        if allowed_missing_constructors.contains(struct_name) {
            continue;
        }
        let Some(methods) = summary.impl_methods.get(struct_name) else {
            diagnostics.push(warning(
                "implementation.rust.typing.constructor",
                &implementation.path,
                format!(
                    "public struct `{struct_name}` has private fields but no public constructor evidence; add `new`/`try_new`/`parse`, or if it is produced only by a query/projector declare `architecture.allowed_missing_constructors` and evidence the producer"
                ),
            ));
            continue;
        };
        if !methods.iter().any(|method| is_constructor_like(method)) {
            diagnostics.push(warning(
                "implementation.rust.typing.constructor",
                &implementation.path,
                format!(
                    "public struct `{struct_name}` has private fields but no constructor-like method (`new`, `try_new`, `parse`, `from_*`); add one, or if it is produced only by a query/projector declare `architecture.allowed_missing_constructors` and evidence the producer"
                ),
            ));
        }
    }
}

fn validate_rust_stateful_representation(
    implementation: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    module_manifest: Option<&LoadedManifest>,
    summary: &RustTypingSummary,
) {
    let Some(module_manifest) = module_manifest else {
        return;
    };
    let profiles = get_string_array(&module_manifest.value, &["profiles"]);
    if !profiles.iter().any(|profile| profile == "stateful") {
        return;
    }

    let state_type = get_str(&implementation.value, &["architecture", "state_type"]);
    let transition_function = get_str(
        &implementation.value,
        &["architecture", "transition_function"],
    );

    if state_type.is_none() && transition_function.is_none() {
        diagnostics.push(error(
            "implementation.rust.typing.stateful-representation",
            &implementation.path,
            "stateful Rust bindings must declare `architecture.state_type` or `architecture.transition_function`",
        ));
        return;
    }

    if let Some(state_type) = state_type {
        if !summary.public_types.contains(state_type) && !summary.private_types.contains(state_type)
        {
            diagnostics.push(error(
                "implementation.rust.typing.state-type",
                &implementation.path,
                format!("declared `architecture.state_type` `{state_type}` was not found in Rust source"),
            ));
        }
    }

    if let Some(transition_function) = transition_function {
        if !summary.functions.contains(transition_function) {
            diagnostics.push(error(
                "implementation.rust.typing.transition-function",
                &implementation.path,
                format!("declared `architecture.transition_function` `{transition_function}` was not found in Rust source"),
            ));
        }
    }
}

fn validate_rust_semantic_function_symbols(
    implementation: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    summary: &RustTypingSummary,
) {
    let Some(functions) = semantic_function_items(implementation) else {
        return;
    };

    for function in functions {
        let Some(symbol) = get_str(function, &["symbol"]) else {
            continue;
        };

        if !rust_symbol_exists(summary, symbol) {
            diagnostics.push(error(
                "implementation.rust.semantic-functions.symbol",
                &implementation.path,
                format!("semantic function symbol `{symbol}` was not found in Rust source"),
            ));
        }
    }
}

fn rust_symbol_exists(summary: &RustTypingSummary, symbol: &str) -> bool {
    if summary.functions.contains(symbol) {
        return true;
    }

    let parts: Vec<_> = symbol.split("::").filter(|part| !part.is_empty()).collect();
    if parts.is_empty() {
        return false;
    }

    if let Some(function_name) = parts.last() {
        if summary.functions.contains(*function_name) {
            return true;
        }
    }

    if parts.len() >= 2 {
        let type_name = parts[parts.len() - 2];
        let method_name = parts[parts.len() - 1];
        if summary
            .impl_methods
            .get(type_name)
            .is_some_and(|methods| methods.contains(method_name))
        {
            return true;
        }
    }

    false
}

fn collect_rust_dependencies(cargo: &TomlValue) -> BTreeSet<String> {
    let mut dependencies = BTreeSet::new();
    for table_name in ["dependencies", "dev-dependencies", "build-dependencies"] {
        collect_dependency_table(cargo.get(table_name), &mut dependencies);
    }

    if let Some(targets) = cargo.get("target").and_then(TomlValue::as_table) {
        for target in targets.values() {
            collect_dependency_table(target.get("dependencies"), &mut dependencies);
            collect_dependency_table(target.get("dev-dependencies"), &mut dependencies);
            collect_dependency_table(target.get("build-dependencies"), &mut dependencies);
        }
    }

    dependencies
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RustImport {
    root: String,
    is_public: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RustImportRootKind {
    External,
    LocalModule,
    Standard,
    SelfQualified,
}

fn rust_import_root_kind(root: &str, local_modules: &BTreeSet<String>) -> RustImportRootKind {
    match root {
        "std" | "core" | "alloc" => RustImportRootKind::Standard,
        "crate" | "self" | "super" => RustImportRootKind::SelfQualified,
        _ if local_modules.contains(root) => RustImportRootKind::LocalModule,
        _ => RustImportRootKind::External,
    }
}

fn rust_source_files(source_root: &Path) -> Vec<PathBuf> {
    WalkDir::new(source_root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| !path_has_component(path, "target"))
        .filter(|path| !path_has_component(path, ".git"))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"))
        .collect()
}

fn path_has_component(path: &Path, component: &str) -> bool {
    path.components()
        .any(|part| matches!(part, Component::Normal(value) if value == component))
}

fn collect_rust_imports(file: &syn::File) -> Vec<RustImport> {
    let mut imports = Vec::new();
    for item in &file.items {
        match item {
            Item::Use(item_use) => {
                let is_public = matches!(item_use.vis, Visibility::Public(_));
                collect_use_tree_roots(&item_use.tree, None, is_public, &mut imports);
            }
            Item::ExternCrate(extern_crate) => {
                imports.push(RustImport {
                    root: extern_crate.ident.to_string(),
                    is_public: matches!(extern_crate.vis, Visibility::Public(_)),
                });
            }
            _ => {}
        }
    }
    imports
}

#[derive(Default)]
struct RustTypingSummary {
    public_types: BTreeSet<String>,
    private_types: BTreeSet<String>,
    public_structs_with_private_fields: BTreeSet<String>,
    impl_methods: std::collections::BTreeMap<String, BTreeSet<String>>,
    functions: BTreeSet<String>,
}

#[derive(Default)]
struct RustFailureVisitor {
    failures: Vec<String>,
}

impl<'ast> Visit<'ast> for RustFailureVisitor {
    fn visit_item(&mut self, item: &'ast Item) {
        if has_cfg_test_attr(item_attrs(item)) {
            return;
        }
        visit::visit_item(self, item);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method = node.method.to_string();
        if matches!(method.as_str(), "unwrap" | "expect") {
            self.failures.push(format!(".{method}()"));
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_macro(&mut self, node: &'ast ExprMacro) {
        if let Some(name) = node
            .mac
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
        {
            if matches!(name.as_str(), "panic" | "todo" | "unimplemented") {
                self.failures.push(format!("{name}!"));
            }
        }
        visit::visit_expr_macro(self, node);
    }
}

fn collect_rust_impl_methods(item_impl: &syn::ItemImpl, summary: &mut RustTypingSummary) {
    let Some(type_name) = rust_type_name(&item_impl.self_ty) else {
        return;
    };

    let methods = summary.impl_methods.entry(type_name).or_default();
    for item in &item_impl.items {
        if let ImplItem::Fn(function) = item {
            if matches!(function.vis, Visibility::Public(_)) {
                methods.insert(function.sig.ident.to_string());
            }
        }
    }
}

fn rust_type_name(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

fn is_constructor_like(method: &str) -> bool {
    method == "new"
        || method == "try_new"
        || method == "parse"
        || method == "from_validated"
        || method.starts_with("from_")
}

fn public_struct_field_count(item_struct: &ItemStruct) -> usize {
    match &item_struct.fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .filter(|field| matches!(field.vis, Visibility::Public(_)))
            .count(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .filter(|field| matches!(field.vis, Visibility::Public(_)))
            .count(),
        Fields::Unit => 0,
    }
}

fn private_struct_field_count(item_struct: &ItemStruct) -> usize {
    match &item_struct.fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .filter(|field| !matches!(field.vis, Visibility::Public(_)))
            .count(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .filter(|field| !matches!(field.vis, Visibility::Public(_)))
            .count(),
        Fields::Unit => 0,
    }
}

fn is_primitive_alias_target(ty: &Type) -> bool {
    let Some(name) = rust_type_name(ty) else {
        return false;
    };
    matches!(
        name.as_str(),
        "String"
            | "str"
            | "bool"
            | "char"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "f32"
            | "f64"
    )
}

fn item_attrs(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::ExternCrate(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::TraitAlias(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Union(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

fn has_cfg_test_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("test") {
            return true;
        }
        match &attr.meta {
            Meta::List(list) if list.path.is_ident("cfg") => {
                list.tokens.to_string().contains("test")
            }
            _ => false,
        }
    })
}

fn collect_use_tree_roots(
    tree: &UseTree,
    prefix: Option<String>,
    is_public: bool,
    imports: &mut Vec<RustImport>,
) {
    match tree {
        UseTree::Path(path) => {
            let root = prefix.unwrap_or_else(|| path.ident.to_string());
            collect_use_tree_roots(&path.tree, Some(root), is_public, imports);
        }
        UseTree::Name(name) => imports.push(RustImport {
            root: prefix.unwrap_or_else(|| name.ident.to_string()),
            is_public,
        }),
        UseTree::Rename(rename) => imports.push(RustImport {
            root: prefix.unwrap_or_else(|| rename.ident.to_string()),
            is_public,
        }),
        UseTree::Glob(_) => {
            if let Some(root) = prefix {
                imports.push(RustImport {
                    root: root.to_string(),
                    is_public,
                });
            }
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree_roots(item, prefix.clone(), is_public, imports);
            }
        }
    }
}

fn local_module_roots(source_root: &Path) -> BTreeSet<String> {
    let mut roots = BTreeSet::new();
    let Ok(entries) = fs::read_dir(source_root) else {
        return roots;
    };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                if stem != "lib" && stem != "main" {
                    roots.insert(stem.to_string());
                }
            }
        } else if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                roots.insert(name.to_string());
            }
        }
    }

    roots
}

fn collect_dependency_table(value: Option<&TomlValue>, dependencies: &mut BTreeSet<String>) {
    let Some(table) = value.and_then(TomlValue::as_table) else {
        return;
    };
    for name in table.keys() {
        dependencies.insert(name.to_string());
    }
}

fn public_modules_declared_in_source(source: &str) -> BTreeSet<String> {
    let mut modules = BTreeSet::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("pub mod ") else {
            continue;
        };
        let module = rest
            .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
            .next()
            .unwrap_or_default();
        if !module.is_empty() {
            modules.insert(module.to_string());
        }
    }
    modules
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SwiftImport {
    module: String,
    is_public_reexport: bool,
}

#[derive(Default)]
struct SwiftTypingSummary {
    public_types: BTreeSet<String>,
    private_types: BTreeSet<String>,
    public_structs_with_private_fields: BTreeSet<String>,
    public_structs_with_constructors: BTreeSet<String>,
    functions: BTreeSet<String>,
}

#[derive(Default)]
struct SwiftStructScan {
    name: String,
    depth: i32,
    private_fields: usize,
    public_fields: usize,
    has_constructor: bool,
}

fn parse_swift_package_name(source: &str) -> Option<String> {
    find_keyed_quoted_value(source, "name:")
}

fn find_keyed_quoted_value(source: &str, key: &str) -> Option<String> {
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        let Some(index) = trimmed.find(key) else {
            continue;
        };
        let after_key = &trimmed[index + key.len()..];
        let start = after_key.find('"')?;
        let after_quote = &after_key[start + 1..];
        let end = after_quote.find('"')?;
        return Some(after_quote[..end].to_string());
    }
    None
}

fn swift_source_files(source_root: &Path) -> Vec<PathBuf> {
    WalkDir::new(source_root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| !path_has_component(path, ".build"))
        .filter(|path| !path_has_component(path, ".git"))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("swift"))
        .collect()
}

fn collect_swift_imports(source: &str) -> Vec<SwiftImport> {
    let mut imports = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }

        let (rest, is_public_reexport) =
            if let Some(rest) = trimmed.strip_prefix("@_exported import ") {
                (rest, true)
            } else if let Some(rest) = trimmed.strip_prefix("import ") {
                (rest, false)
            } else {
                continue;
            };

        let rest = strip_swift_import_qualifier(rest.trim_start());
        let module = rest
            .split(|character: char| {
                !(character.is_ascii_alphanumeric() || character == '_' || character == '.')
            })
            .next()
            .unwrap_or_default()
            .split('.')
            .next()
            .unwrap_or_default();
        if !module.is_empty() {
            imports.push(SwiftImport {
                module: module.to_string(),
                is_public_reexport,
            });
        }
    }
    imports
}

fn strip_swift_import_qualifier(value: &str) -> &str {
    for qualifier in ["class ", "struct ", "enum ", "protocol ", "func ", "var "] {
        if let Some(rest) = value.strip_prefix(qualifier) {
            return rest.trim_start();
        }
    }
    value
}

fn is_swift_standard_module(module: &str) -> bool {
    matches!(
        module,
        "Swift"
            | "Foundation"
            | "Dispatch"
            | "Darwin"
            | "Glibc"
            | "UIKit"
            | "AppKit"
            | "SwiftUI"
            | "Combine"
            | "Observation"
            | "XCTest"
    )
}

fn inspect_swift_typing_file(
    implementation: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    path: &Path,
    source: &str,
    summary: &mut SwiftTypingSummary,
) {
    let allowed_public_field_structs: BTreeSet<_> = get_string_array(
        &implementation.value,
        &["architecture", "allowed_public_field_structs"],
    )
    .into_iter()
    .collect();
    let allowed_primitive_aliases: BTreeSet<_> = get_string_array(
        &implementation.value,
        &["architecture", "allowed_primitive_type_aliases"],
    )
    .into_iter()
    .collect();
    let allow_traps =
        get_bool(&implementation.value, &["architecture", "allow_traps"]).unwrap_or(false);
    let mut current_struct: Option<SwiftStructScan> = None;

    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }

        if !allow_traps {
            for trap in swift_traps_in_line(trimmed) {
                diagnostics.push(error(
                    "implementation.swift.typing.failure-discipline",
                    &implementation.path,
                    format!(
                        "Swift source `{}` uses `{trap}` in domain code; prefer explicit result/error types for expected failures",
                        path.display()
                    ),
                ));
            }
        }

        if let Some((name, target)) = parse_swift_public_typealias(trimmed) {
            summary.public_types.insert(name.clone());
            if is_swift_primitive_type(target) && !allowed_primitive_aliases.contains(name.as_str())
            {
                diagnostics.push(error(
                    "implementation.swift.typing.primitive-alias",
                    &implementation.path,
                    format!(
                        "public typealias `{name}` in `{}` points at a primitive; prefer an opaque value type or declare it in `architecture.allowed_primitive_type_aliases`",
                        path.display()
                    ),
                ));
            }
        }

        if let Some(name) = parse_swift_declaration_name(trimmed, "enum") {
            if has_swift_public_modifier(trimmed) {
                summary.public_types.insert(name);
            } else {
                summary.private_types.insert(name);
            }
        }
        if let Some(name) = parse_swift_function_name(trimmed) {
            summary.functions.insert(name);
        }

        if let Some(scan) = current_struct.as_mut() {
            if is_swift_public_field(trimmed) {
                scan.public_fields += 1;
            } else if is_swift_stored_field(trimmed) {
                scan.private_fields += 1;
            }
            if is_swift_constructor_like(trimmed) {
                scan.has_constructor = true;
            }
            scan.depth += swift_brace_delta(trimmed);
            if scan.depth <= 0 {
                if let Some(finished_scan) = current_struct.take() {
                    finish_swift_struct_scan(
                        implementation,
                        diagnostics,
                        path,
                        &allowed_public_field_structs,
                        summary,
                        finished_scan,
                    );
                }
            }
            continue;
        }

        if let Some(name) = parse_swift_declaration_name(trimmed, "struct") {
            if has_swift_public_modifier(trimmed) {
                summary.public_types.insert(name.clone());
                current_struct = Some(SwiftStructScan {
                    name,
                    depth: swift_brace_delta(trimmed).max(1),
                    private_fields: 0,
                    public_fields: 0,
                    has_constructor: false,
                });
            } else {
                summary.private_types.insert(name);
            }
        }
    }

    if let Some(scan) = current_struct {
        finish_swift_struct_scan(
            implementation,
            diagnostics,
            path,
            &allowed_public_field_structs,
            summary,
            scan,
        );
    }
}

fn finish_swift_struct_scan(
    implementation: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    path: &Path,
    allowed_public_field_structs: &BTreeSet<String>,
    summary: &mut SwiftTypingSummary,
    scan: SwiftStructScan,
) {
    if scan.public_fields > 0 && !allowed_public_field_structs.contains(&scan.name) {
        diagnostics.push(error(
            "implementation.swift.typing.public-fields",
            &implementation.path,
            format!(
                "public struct `{}` in `{}` exposes {} public field(s); prefer private fields plus validated initializers/accessors or declare it in `architecture.allowed_public_field_structs`",
                scan.name,
                path.display(),
                scan.public_fields
            ),
        ));
    }

    if scan.private_fields > 0 {
        summary
            .public_structs_with_private_fields
            .insert(scan.name.clone());
    }
    if scan.has_constructor {
        summary.public_structs_with_constructors.insert(scan.name);
    }
}

fn validate_swift_constructor_evidence(
    implementation: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    summary: &SwiftTypingSummary,
) {
    let allowed_missing_constructors: BTreeSet<_> = get_string_array(
        &implementation.value,
        &["architecture", "allowed_missing_constructors"],
    )
    .into_iter()
    .collect();

    for struct_name in &summary.public_structs_with_private_fields {
        if allowed_missing_constructors.contains(struct_name)
            || summary
                .public_structs_with_constructors
                .contains(struct_name)
        {
            continue;
        }
        diagnostics.push(warning(
            "implementation.swift.typing.constructor",
            &implementation.path,
            format!(
                "public struct `{struct_name}` has private fields but no public initializer/factory evidence; add `init`, `new`, `parse`, or if it is produced only by a query/projector declare `architecture.allowed_missing_constructors` and evidence the producer"
            ),
        ));
    }
}

fn validate_swift_stateful_representation(
    implementation: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    module_manifest: Option<&LoadedManifest>,
    summary: &SwiftTypingSummary,
) {
    let Some(module_manifest) = module_manifest else {
        return;
    };
    let profiles = get_string_array(&module_manifest.value, &["profiles"]);
    if !profiles.iter().any(|profile| profile == "stateful") {
        return;
    }

    let state_type = get_str(&implementation.value, &["architecture", "state_type"]);
    let transition_function = get_str(
        &implementation.value,
        &["architecture", "transition_function"],
    );

    if state_type.is_none() && transition_function.is_none() {
        diagnostics.push(error(
            "implementation.swift.typing.stateful-representation",
            &implementation.path,
            "stateful Swift bindings must declare `architecture.state_type` or `architecture.transition_function`",
        ));
        return;
    }

    if let Some(state_type) = state_type {
        if !summary.public_types.contains(state_type) && !summary.private_types.contains(state_type)
        {
            diagnostics.push(error(
                "implementation.swift.typing.state-type",
                &implementation.path,
                format!(
                    "declared `architecture.state_type` `{state_type}` was not found in Swift source"
                ),
            ));
        }
    }

    if let Some(transition_function) = transition_function {
        if !summary.functions.contains(transition_function) {
            diagnostics.push(error(
                "implementation.swift.typing.transition-function",
                &implementation.path,
                format!("declared `architecture.transition_function` `{transition_function}` was not found in Swift source"),
            ));
        }
    }
}

fn parse_swift_public_typealias(line: &str) -> Option<(String, &str)> {
    if !has_swift_public_modifier(line) {
        return None;
    }
    let rest = strip_swift_modifiers(line);
    let rest = rest.strip_prefix("typealias ")?;
    let (name, target) = rest.split_once('=')?;
    Some((swift_identifier(name.trim())?.to_string(), target.trim()))
}

fn parse_swift_function_name(line: &str) -> Option<String> {
    let rest = strip_swift_modifiers(line);
    let rest = rest.strip_prefix("func ")?;
    Some(swift_identifier(rest)?.to_string())
}

fn parse_swift_declaration_name(line: &str, kind: &str) -> Option<String> {
    let rest = strip_swift_modifiers(line);
    let rest = rest.strip_prefix(kind)?.trim_start();
    Some(swift_identifier(rest)?.to_string())
}

fn strip_swift_modifiers(mut line: &str) -> &str {
    loop {
        let trimmed = line.trim_start();
        let Some((token, rest)) = trimmed.split_once(char::is_whitespace) else {
            return trimmed;
        };
        if matches!(
            token,
            "public"
                | "open"
                | "internal"
                | "private"
                | "fileprivate"
                | "final"
                | "static"
                | "mutating"
                | "nonmutating"
        ) {
            line = rest;
        } else {
            return trimmed;
        }
    }
}

fn has_swift_public_modifier(line: &str) -> bool {
    line.split_whitespace()
        .take_while(|token| !matches!(*token, "struct" | "enum" | "class" | "typealias" | "func"))
        .any(|token| matches!(token, "public" | "open"))
}

fn swift_identifier(value: &str) -> Option<&str> {
    let identifier = value
        .trim_start()
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .next()
        .unwrap_or_default();
    if identifier.is_empty() {
        None
    } else {
        Some(identifier)
    }
}

fn is_swift_public_field(line: &str) -> bool {
    let rest = strip_swift_modifiers(line);
    has_swift_public_modifier(line)
        && (rest.starts_with("let ") || rest.starts_with("var "))
        && !line.contains('{')
}

fn is_swift_stored_field(line: &str) -> bool {
    let rest = strip_swift_modifiers(line);
    (rest.starts_with("let ") || rest.starts_with("var ")) && !line.contains('{')
}

fn is_swift_constructor_like(line: &str) -> bool {
    let rest = strip_swift_modifiers(line);
    has_swift_public_modifier(line)
        && (rest.starts_with("init(")
            || rest.starts_with("init?")
            || rest.starts_with("init!")
            || rest.starts_with("static func new(")
            || rest.starts_with("static func parse(")
            || rest.starts_with("static func from"))
}

fn swift_brace_delta(line: &str) -> i32 {
    let open = line.chars().filter(|character| *character == '{').count() as i32;
    let close = line.chars().filter(|character| *character == '}').count() as i32;
    open - close
}

fn swift_traps_in_line(line: &str) -> Vec<&'static str> {
    let mut traps = Vec::new();
    if line.contains("fatalError(") {
        traps.push("fatalError");
    }
    if line.contains("preconditionFailure(") {
        traps.push("preconditionFailure");
    }
    if line.contains("try!") {
        traps.push("try!");
    }
    if line.contains("as!") {
        traps.push("as!");
    }
    traps
}

fn is_swift_primitive_type(value: &str) -> bool {
    let Some(name) = swift_identifier(value.trim_start_matches('[').trim_start()) else {
        return false;
    };
    matches!(
        name,
        "String"
            | "Substring"
            | "Bool"
            | "Character"
            | "Int"
            | "Int8"
            | "Int16"
            | "Int32"
            | "Int64"
            | "UInt"
            | "UInt8"
            | "UInt16"
            | "UInt32"
            | "UInt64"
            | "Float"
            | "Double"
            | "Decimal"
            | "Date"
            | "UUID"
            | "Data"
    )
}

fn load_toml(path: &Path) -> Result<TomlValue> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read TOML `{}`", path.display()))?;
    contents
        .parse::<TomlValue>()
        .with_context(|| format!("failed to parse TOML `{}`", path.display()))
}

fn display_relative(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .components()
        .filter(|component| !matches!(component, Component::CurDir))
        .collect::<PathBuf>()
        .display()
        .to_string()
}

fn load_binding_module_manifest(
    implementation: &LoadedManifest,
    base: &Path,
) -> Option<LoadedManifest> {
    let declared_module = get_str(&implementation.value, &["module"])?;
    let direct = base.join("module.yaml");
    if let Ok(manifest) = load_manifest(&direct) {
        if get_str(&manifest.value, &["module", "name"]) == Some(declared_module) {
            return Some(manifest);
        }
    }

    let entries = fs::read_dir(base).ok()?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".module.yaml"))
        {
            if let Ok(manifest) = load_manifest(&path) {
                if get_str(&manifest.value, &["module", "name"]) == Some(declared_module) {
                    return Some(manifest);
                }
            }
        }
    }

    None
}

fn require_verification_categories(manifest: &LoadedManifest, diagnostics: &mut Vec<Diagnostic>) {
    for category in ["laws", "contracts", "scenarios", "boundaries"] {
        require_array(
            manifest,
            diagnostics,
            format!("verification.{category}"),
            &["verification", category],
        );
    }
}

fn check_profile_obligations(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    profiles: &[String],
) {
    if profiles.iter().any(|profile| profile == "stateful")
        && get_path(&manifest.value, &["state"]).is_none()
    {
        diagnostics.push(error(
            "profile.stateful",
            &manifest.path,
            "stateful modules must declare a `state` section",
        ));
    }

    if profiles.iter().any(|profile| profile == "distributed")
        && get_path(&manifest.value, &["operations", "reconciliation"]).is_none()
    {
        diagnostics.push(error(
            "profile.distributed.reconciliation",
            &manifest.path,
            "distributed modules must declare reconciliation or repair operations",
        ));
    }

    if profiles.iter().any(|profile| profile == "workflow")
        && get_path(&manifest.value, &["workflow"]).is_none()
    {
        diagnostics.push(error(
            "profile.workflow",
            &manifest.path,
            "workflow modules must declare a `workflow` section",
        ));
    }

    if profiles.iter().any(|profile| profile == "boundary")
        && get_path(&manifest.value, &["boundary"]).is_none()
    {
        diagnostics.push(error(
            "profile.boundary",
            &manifest.path,
            "boundary modules must declare a `boundary` section",
        ));
    }
}

fn check_contract_refs(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    path: &[&str],
) {
    let Some(value) = get_path(&manifest.value, path) else {
        return;
    };
    collect_contract_paths(value, &mut |contract| {
        check_relative_ref(
            manifest,
            diagnostics,
            "references.contract",
            contract,
            "referenced contract does not exist",
        );
    });
}

fn check_invariant_evidence(manifest: &LoadedManifest, diagnostics: &mut Vec<Diagnostic>) {
    let Some(invariants) =
        get_path(&manifest.value, &["invariants"]).and_then(YamlValue::as_sequence)
    else {
        return;
    };
    for invariant in invariants {
        if let Some(path) = get_str(invariant, &["verified_by"]) {
            check_relative_ref(
                manifest,
                diagnostics,
                "references.invariant-evidence",
                path,
                "referenced invariant evidence does not exist",
            );
        }
    }
}

fn check_verification_paths(manifest: &LoadedManifest, diagnostics: &mut Vec<Diagnostic>) {
    let Some(verification) = get_path(&manifest.value, &["verification"]) else {
        return;
    };
    for category in [
        "laws",
        "contracts",
        "scenarios",
        "boundaries",
        "runtime",
        "reconciliation",
    ] {
        let Some(paths) = get_path(verification, &[category]).and_then(YamlValue::as_sequence)
        else {
            continue;
        };
        for path in paths.iter().filter_map(YamlValue::as_str) {
            check_relative_ref(
                manifest,
                diagnostics,
                format!("references.verification.{category}"),
                path,
                "referenced verification evidence does not exist",
            );
        }
    }
}

fn check_change_protocols(manifest: &LoadedManifest, diagnostics: &mut Vec<Diagnostic>) {
    let Some(protocols) = change_protocol_items(&manifest.value) else {
        return;
    };

    let mut ids = BTreeSet::new();
    for (index, protocol) in protocols.iter().enumerate() {
        let Some(id) = get_str(protocol, &["id"]) else {
            diagnostics.push(error(
                "change-protocol.id",
                &manifest.path,
                format!("change protocol at index {index} is missing `id`"),
            ));
            continue;
        };
        if !ids.insert(id.to_string()) {
            diagnostics.push(error(
                "change-protocol.duplicate-id",
                &manifest.path,
                format!("duplicate change protocol id `{id}`"),
            ));
        }
        if get_str(protocol, &["applies_when"]).is_none() {
            diagnostics.push(error(
                "change-protocol.applies-when",
                &manifest.path,
                format!("change protocol `{id}` is missing `applies_when`"),
            ));
        }
        collect_change_protocol_reference_paths(protocol, &mut |reference| {
            check_relative_ref(
                manifest,
                diagnostics,
                "references.change-protocol",
                reference,
                "change protocol references an artifact that does not exist",
            );
        });
    }
}

fn check_optional_path(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    path: &[&str],
) {
    if let Some(reference) = get_str(&manifest.value, path) {
        check_relative_ref(
            manifest,
            diagnostics,
            format!("references.{}", path.join(".")),
            reference,
            "referenced path does not exist",
        );
    }
}

fn check_relative_ref(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    check: impl Into<String>,
    reference: &str,
    message: &str,
) {
    if reference.starts_with("http://")
        || reference.starts_with("https://")
        || reference.starts_with("urn:")
    {
        return;
    }

    let base = manifest.path.parent().unwrap_or_else(|| Path::new("."));
    let full_path = base.join(reference);
    if !full_path.exists() {
        diagnostics.push(error(
            check,
            &manifest.path,
            format!("{message}: `{reference}`"),
        ));
    }
}

fn collect_contract_paths(value: &YamlValue, emit: &mut impl FnMut(&str)) {
    match value {
        YamlValue::Mapping(mapping) => {
            if let Some(contract) = mapping
                .get(YamlValue::String("contract".to_string()))
                .and_then(YamlValue::as_str)
            {
                emit(contract);
            }
            for child in mapping.values() {
                collect_contract_paths(child, emit);
            }
        }
        YamlValue::Sequence(items) => {
            for item in items {
                collect_contract_paths(item, emit);
            }
        }
        _ => {}
    }
}

fn change_protocol_items(value: &YamlValue) -> Option<&[YamlValue]> {
    get_path(value, &["x-change-protocols"])?
        .as_sequence()
        .map(Vec::as_slice)
}

fn collect_change_protocol_reference_paths(value: &YamlValue, emit: &mut impl FnMut(&str)) {
    let Some(references) = get_path(value, &["references"]) else {
        return;
    };
    collect_string_leaf_values(references, emit);
}

fn collect_string_leaf_values(value: &YamlValue, emit: &mut impl FnMut(&str)) {
    match value {
        YamlValue::String(reference) => emit(reference),
        YamlValue::Sequence(items) => {
            for item in items {
                collect_string_leaf_values(item, emit);
            }
        }
        YamlValue::Mapping(mapping) => {
            for child in mapping.values() {
                collect_string_leaf_values(child, emit);
            }
        }
        _ => {}
    }
}

fn scan_for_secret_like_keys(value: &YamlValue, path: &Path, diagnostics: &mut Vec<Diagnostic>) {
    fn walk(
        value: &YamlValue,
        key_path: &mut Vec<String>,
        path: &Path,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        match value {
            YamlValue::Mapping(mapping) => {
                for (key, child) in mapping {
                    let key_text = key.as_str().unwrap_or("<non-string-key>").to_string();
                    key_path.push(key_text.clone());
                    let normalized = key_text.to_ascii_lowercase().replace('-', "_");
                    if matches!(
                        normalized.as_str(),
                        "secret" | "secrets" | "password" | "token" | "access_token" | "api_key"
                    ) {
                        diagnostics.push(error(
                            "security.secret-key",
                            path,
                            format!(
                                "canonical artifacts must not contain secret-bearing fields: `{}`",
                                key_path.join(".")
                            ),
                        ));
                    }
                    walk(child, key_path, path, diagnostics);
                    key_path.pop();
                }
            }
            YamlValue::Sequence(items) => {
                for item in items {
                    walk(item, key_path, path, diagnostics);
                }
            }
            _ => {}
        }
    }

    walk(value, &mut Vec::new(), path, diagnostics);
}

fn print_module_brief(manifest: &LoadedManifest) {
    println!("# RMS Module Brief");
    println!();
    println!("Path: {}", manifest.path.display());
    println!(
        "Module: {} {}",
        get_str(&manifest.value, &["module", "name"]).unwrap_or("<unknown>"),
        get_str(&manifest.value, &["module", "version"]).unwrap_or("")
    );
    println!(
        "Purpose: {}",
        get_str(&manifest.value, &["module", "purpose"]).unwrap_or("<missing>")
    );
    println!(
        "Kind: {}",
        get_str(&manifest.value, &["module", "kind"]).unwrap_or("<missing>")
    );
    print_string_list(
        "Profiles",
        &get_string_array(&manifest.value, &["profiles"]),
    );
    print_owned_terms(&manifest.value);
    print_contract_groups("Provides", get_path(&manifest.value, &["provides"]));
    print_contract_groups("Requires", get_path(&manifest.value, &["requires"]));
    print_invariants(&manifest.value);
    print_effects(&manifest.value);
    println!(
        "Compatibility: {}",
        get_str(&manifest.value, &["compatibility", "policy"]).unwrap_or("<missing>")
    );
    print_verification(&manifest.value);
    print_change_protocols(&manifest.value);
}

fn print_module_explanation(
    manifest: &LoadedManifest,
    root: &Path,
    question: Option<&str>,
) -> Result<()> {
    println!("# RMS Module Explanation");
    println!();
    println!("Path: {}", manifest.path.display());
    if let Some(question) = question {
        println!("Question: {question}");
    }
    println!();

    println!("## What This Module Is");
    println!(
        "{} {} is a {}.",
        get_str(&manifest.value, &["module", "name"]).unwrap_or("<unknown>"),
        get_str(&manifest.value, &["module", "version"]).unwrap_or(""),
        get_str(&manifest.value, &["module", "kind"]).unwrap_or("<missing-kind>")
    );
    println!(
        "Purpose: {}",
        get_str(&manifest.value, &["module", "purpose"]).unwrap_or("<missing>")
    );
    print_string_list(
        "Profiles",
        &get_string_array(&manifest.value, &["profiles"]),
    );

    print_owned_terms(&manifest.value);

    print_contract_groups("Provides", get_path(&manifest.value, &["provides"]));
    print_contract_groups("Requires", get_path(&manifest.value, &["requires"]));

    print_invariants(&manifest.value);
    print_effects(&manifest.value);
    println!(
        "Compatibility: {}",
        get_str(&manifest.value, &["compatibility", "policy"]).unwrap_or("<missing>")
    );
    print_verification(&manifest.value);
    print_change_protocols(&manifest.value);

    println!();
    println!("## Before Changing It");
    println!("- Keep changes inside this module's ownership boundary.");
    println!("- Change public contracts first when public meaning changes.");
    println!(
        "- Declare new dependencies, effects, profiles, and recovery paths before relying on them."
    );
    println!("- Add only the smallest evidence that strongly demonstrates the changed promise.");

    let module_base = manifest.path.parent().unwrap_or_else(|| Path::new("."));
    let module_name = get_str(&manifest.value, &["module", "name"]).unwrap_or("");
    if !module_name.is_empty() {
        if let Some(implementation) = sibling_implementation_manifest(module_base, module_name)? {
            println!(
                "- Implementation binding: {}",
                implementation.path.display()
            );
            if let Some(command) = get_str(&implementation.value, &["commands", "verify"]) {
                println!("- Verification command: {command}");
            }
        }
    }

    println!();
    println!("## Useful Commands");
    println!("- rms inspect {}", manifest.path.display());
    println!(
        "- rms context {} --root {} --task \"<task>\"",
        manifest.path.display(),
        root.display()
    );
    println!("- rms validate --root {}", root.display());
    if !module_name.is_empty() && module_base.join("implementation.yaml").exists() {
        println!(
            "- rms verify {}",
            module_base.join("implementation.yaml").display()
        );
    }

    if let Some(question) = question {
        println!();
        println!("## Question Focus");
        print_question_focus(&manifest.value, question);
    }

    Ok(())
}

fn print_context_packet(manifest: &LoadedManifest, root: &Path, task: Option<&str>) -> Result<()> {
    println!("# RMS Context Packet");
    println!();
    if let Some(task) = task {
        println!("Task: {task}");
        println!();
    }

    print_module_brief(manifest);

    println!();
    println!("## Canonical Files");
    for file_name in ["system.yaml", "context-map.yaml", "GLOSSARY.md"] {
        let path = root.join(file_name);
        if path.exists() {
            println!("- {}", path.display());
        }
    }

    println!();
    println!("## Public References");
    for reference in referenced_paths(&manifest.value) {
        let path = manifest
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&reference);
        println!("- {}", path.display());
    }

    println!();
    println!("## Working Rules");
    println!("- Preserve the owning module boundary.");
    println!("- Update public contracts before implementation when public meaning changes.");
    println!("- Declare new dependencies, effects, profile obligations, and recovery paths.");
    println!("- Verify declared laws, contracts, scenarios, and boundaries that the task affects.");
    Ok(())
}

#[derive(Debug, Serialize)]
struct AtlasDocument {
    spec: &'static str,
    source: AtlasSource,
    module: AtlasModuleSummary,
    layers: Vec<AtlasLayer>,
    nodes: Vec<AtlasNode>,
    edges: Vec<AtlasEdge>,
    traces: Vec<AtlasTraceProjection>,
    tours: Vec<AtlasTour>,
    annotations: Vec<AtlasAnnotation>,
    interaction: AtlasInteraction,
}

#[derive(Debug, Serialize)]
struct AtlasSource {
    root: String,
    module_manifest: String,
    generated_by: &'static str,
    generated_version: &'static str,
    source_revision: Option<String>,
}

#[derive(Debug, Serialize)]
struct AtlasModuleSummary {
    id: String,
    name: String,
    version: String,
    kind: String,
    purpose: String,
    profiles: Vec<String>,
    compatibility: String,
}

#[derive(Debug, Serialize)]
struct AtlasLayer {
    id: &'static str,
    label: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct AtlasNode {
    id: String,
    kind: String,
    layer: String,
    label: String,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    source_refs: Vec<AtlasSourceRef>,
    clauses: Vec<AtlasContractClause>,
    emphasis: u8,
}

#[derive(Clone, Debug, Serialize)]
struct AtlasContractClause {
    id: String,
    kind: String,
    label: String,
    statement: String,
    source_refs: Vec<AtlasSourceRef>,
}

#[derive(Clone, Debug, Serialize)]
struct AtlasEdge {
    id: String,
    kind: String,
    from: String,
    to: String,
    label: String,
    source_refs: Vec<AtlasSourceRef>,
}

#[derive(Clone, Debug, Serialize)]
struct AtlasSourceRef {
    role: String,
    path: String,
}

#[derive(Debug, Serialize)]
struct AtlasTraceProjection {
    id: String,
    label: String,
    intent: &'static str,
    entry_node_id: String,
    summary: String,
    steps: Vec<AtlasTraceStep>,
    gaps: Vec<AtlasTraceGap>,
    source_refs: Vec<AtlasSourceRef>,
}

#[derive(Debug, Serialize)]
struct AtlasTraceStep {
    id: String,
    role: &'static str,
    title: String,
    body: String,
    reading: AtlasTraceReading,
    node_ids: Vec<String>,
    edge_ids: Vec<String>,
    confidence: &'static str,
    source_refs: Vec<AtlasSourceRef>,
}

#[derive(Debug, Serialize)]
struct AtlasTraceReading {
    promise: String,
    before: String,
    after: String,
    failure: String,
    evidence: String,
    impact: String,
    justification: Vec<AtlasJustificationStep>,
}

#[derive(Debug, Serialize)]
struct AtlasJustificationStep {
    node_id: String,
    label: String,
    role: String,
    detail: String,
}

#[derive(Debug, Serialize)]
struct AtlasTraceGap {
    id: String,
    title: String,
    body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggested_artifact: Option<String>,
    source_refs: Vec<AtlasSourceRef>,
}

#[derive(Debug, Serialize)]
struct AtlasTour {
    id: &'static str,
    title: &'static str,
    steps: Vec<AtlasTourStep>,
}

#[derive(Debug, Serialize)]
struct AtlasTourStep {
    node_id: String,
    title: String,
    body: String,
}

#[derive(Debug, Serialize)]
struct AtlasAnnotation {
    node_id: String,
    text: String,
    source_refs: Vec<AtlasSourceRef>,
}

#[derive(Debug, Serialize)]
struct AtlasInteraction {
    default_focus: String,
    supports_live_reconciliation: bool,
    agent_generation_policy: &'static str,
}

#[derive(Debug)]
struct AtlasNamedReference {
    name: String,
    contract: Option<String>,
}

fn run_atlas(module: &Path, root: &Path, output: Option<&Path>, force: bool) -> Result<()> {
    let manifest = load_manifest(module)?;
    let mut diagnostics = Vec::new();
    validate_loaded_manifest(&manifest, &mut diagnostics);
    if let Some(diagnostic) = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity == Severity::Error)
    {
        bail!(
            "target module failed validation: {} [{}] {}",
            diagnostic.path,
            diagnostic.check,
            diagnostic.message
        );
    }

    let atlas = build_module_atlas(&manifest, root)?;
    let atlas_json = serde_json::to_string_pretty(&atlas)?;
    let module_name = get_str(&manifest.value, &["module", "name"]).unwrap_or("module");
    let output_dir = output.map(Path::to_path_buf).unwrap_or_else(|| {
        root.join("dist/rms-atlas")
            .join(semantic_id_segment(module_name))
    });

    if output_dir.exists() {
        if !force {
            bail!(
                "atlas output already exists at `{}`; pass `--force` to replace it",
                output_dir.display()
            );
        }
        if output_dir.is_dir() {
            fs::remove_dir_all(&output_dir)
                .with_context(|| format!("failed to remove `{}`", output_dir.display()))?;
        } else {
            fs::remove_file(&output_dir)
                .with_context(|| format!("failed to remove `{}`", output_dir.display()))?;
        }
    }

    fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create `{}`", output_dir.display()))?;
    fs::write(output_dir.join("atlas.json"), &atlas_json).with_context(|| {
        format!(
            "failed to write `{}`",
            output_dir.join("atlas.json").display()
        )
    })?;
    fs::write(
        output_dir.join("index.html"),
        render_atlas_html(&atlas_json),
    )
    .with_context(|| {
        format!(
            "failed to write `{}`",
            output_dir.join("index.html").display()
        )
    })?;

    println!("atlas: {}", output_dir.join("index.html").display());
    println!("data: {}", output_dir.join("atlas.json").display());
    Ok(())
}

fn build_module_atlas(manifest: &LoadedManifest, root: &Path) -> Result<AtlasDocument> {
    let module_name = get_str(&manifest.value, &["module", "name"]).unwrap_or("module");
    let module_id = stable_atlas_id("module", module_name);
    let module_base = manifest.path.parent().unwrap_or_else(|| Path::new("."));
    let module_ref = atlas_source_ref(root, &manifest.path, "module-manifest");
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node_ids = BTreeSet::new();

    push_atlas_node(
        &mut nodes,
        &mut node_ids,
        AtlasNode {
            id: module_id.clone(),
            kind: "module".to_string(),
            layer: "overview".to_string(),
            label: module_name.to_string(),
            summary: get_str(&manifest.value, &["module", "purpose"])
                .unwrap_or("RMS module")
                .to_string(),
            group: Some(
                get_str(&manifest.value, &["module", "kind"])
                    .unwrap_or("module")
                    .to_string(),
            ),
            source_refs: vec![module_ref.clone()],
            clauses: module_summary_clauses(&manifest.value, std::slice::from_ref(&module_ref)),
            emphasis: 5,
        },
    );

    for profile in get_string_array(&manifest.value, &["profiles"]) {
        let id = stable_atlas_id("profile", &profile);
        push_atlas_node(
            &mut nodes,
            &mut node_ids,
            AtlasNode {
                id: id.clone(),
                kind: "profile".to_string(),
                layer: "overview".to_string(),
                label: profile.clone(),
                summary: format!("Declared RMS profile `{profile}`."),
                group: Some("profiles".to_string()),
                source_refs: vec![module_ref.clone()],
                clauses: Vec::new(),
                emphasis: 2,
            },
        );
        push_atlas_edge(
            &mut edges,
            "declares-profile",
            &module_id,
            &id,
            "declares",
            vec![module_ref.clone()],
        );
    }

    if let Some(owns) = get_path(&manifest.value, &["owns"]).and_then(YamlValue::as_mapping) {
        for (group, values) in owns {
            let group = group.as_str().unwrap_or("ownership");
            let Some(values) = values.as_sequence() else {
                continue;
            };
            for value in values.iter().filter_map(YamlValue::as_str) {
                let id = stable_atlas_id(&format!("owns-{group}"), value);
                push_atlas_node(
                    &mut nodes,
                    &mut node_ids,
                    AtlasNode {
                        id: id.clone(),
                        kind: "owned".to_string(),
                        layer: "ownership".to_string(),
                        label: value.to_string(),
                        summary: format!("Owned {group} in the {module_name} module."),
                        group: Some(group.to_string()),
                        source_refs: vec![module_ref.clone()],
                        clauses: Vec::new(),
                        emphasis: 3,
                    },
                );
                push_atlas_edge(
                    &mut edges,
                    "owns",
                    &module_id,
                    &id,
                    group,
                    vec![module_ref.clone()],
                );
            }
        }
    }

    for group in ["commands", "queries", "events", "capabilities"] {
        for item in atlas_named_references(&manifest.value, &["provides", group]) {
            let id = stable_atlas_id(&format!("provides-{group}"), &item.name);
            let mut source_refs = vec![module_ref.clone()];
            if let Some(contract) = &item.contract {
                source_refs.push(atlas_source_ref(
                    root,
                    &module_base.join(contract),
                    "contract",
                ));
            }
            let clauses = item
                .contract
                .as_deref()
                .map(|contract| contract_clauses(root, &module_base.join(contract)))
                .unwrap_or_default();
            let summary = item
                .contract
                .as_deref()
                .and_then(|contract| contract_meaning(&module_base.join(contract)))
                .unwrap_or_else(|| format!("Public {group} surface provided by {module_name}."));
            push_atlas_node(
                &mut nodes,
                &mut node_ids,
                AtlasNode {
                    id: id.clone(),
                    kind: "public-surface".to_string(),
                    layer: "public-surface".to_string(),
                    label: item.name,
                    summary,
                    group: Some(group.to_string()),
                    source_refs: source_refs.clone(),
                    clauses,
                    emphasis: if group == "commands" { 4 } else { 3 },
                },
            );
            push_atlas_edge(&mut edges, "provides", &module_id, &id, group, source_refs);
        }
    }

    for item in atlas_named_references(&manifest.value, &["requires", "modules"]) {
        let id = stable_atlas_id("requires-module", &item.name);
        push_atlas_node(
            &mut nodes,
            &mut node_ids,
            AtlasNode {
                id: id.clone(),
                kind: "required-module".to_string(),
                layer: "dependencies".to_string(),
                label: item.name,
                summary: format!("Declared module dependency for {module_name}."),
                group: Some("modules".to_string()),
                source_refs: vec![module_ref.clone()],
                clauses: Vec::new(),
                emphasis: 3,
            },
        );
        push_atlas_edge(
            &mut edges,
            "requires",
            &module_id,
            &id,
            "requires module",
            vec![module_ref.clone()],
        );
    }

    for item in atlas_named_references(&manifest.value, &["requires", "capabilities"]) {
        let id = stable_atlas_id("requires-capability", &item.name);
        let mut source_refs = vec![module_ref.clone()];
        if let Some(contract) = &item.contract {
            source_refs.push(atlas_source_ref(
                root,
                &module_base.join(contract),
                "contract",
            ));
        }
        let clauses = item
            .contract
            .as_deref()
            .map(|contract| contract_clauses(root, &module_base.join(contract)))
            .unwrap_or_default();
        let summary = item
            .contract
            .as_deref()
            .and_then(|contract| contract_meaning(&module_base.join(contract)))
            .unwrap_or_else(|| format!("Required capability for {module_name}."));
        push_atlas_node(
            &mut nodes,
            &mut node_ids,
            AtlasNode {
                id: id.clone(),
                kind: "required-capability".to_string(),
                layer: "dependencies".to_string(),
                label: item.name,
                summary,
                group: Some("capabilities".to_string()),
                source_refs: source_refs.clone(),
                clauses,
                emphasis: 3,
            },
        );
        push_atlas_edge(
            &mut edges,
            "requires",
            &module_id,
            &id,
            "requires capability",
            source_refs,
        );
    }

    if let Some(invariants) =
        get_path(&manifest.value, &["invariants"]).and_then(YamlValue::as_sequence)
    {
        for invariant in invariants {
            let invariant_id = get_str(invariant, &["id"]).unwrap_or("invariant");
            let id = stable_atlas_id("invariant", invariant_id);
            let mut source_refs = vec![module_ref.clone()];
            if let Some(path) = get_str(invariant, &["verified_by"]) {
                source_refs.push(atlas_source_ref(root, &module_base.join(path), "evidence"));
            }
            push_atlas_node(
                &mut nodes,
                &mut node_ids,
                AtlasNode {
                    id: id.clone(),
                    kind: "invariant".to_string(),
                    layer: "constraints".to_string(),
                    label: invariant_id.to_string(),
                    summary: get_str(invariant, &["statement"])
                        .unwrap_or("Declared module invariant.")
                        .to_string(),
                    group: get_str(invariant, &["enforced_by"]).map(ToString::to_string),
                    source_refs: source_refs.clone(),
                    clauses: invariant_clauses(invariant, &source_refs),
                    emphasis: 4,
                },
            );
            push_atlas_edge(
                &mut edges,
                "constrains",
                &id,
                &module_id,
                "constrains",
                source_refs.clone(),
            );

            if let Some(path) = get_str(invariant, &["verified_by"]) {
                let evidence_id = stable_atlas_id("verification", path);
                push_verification_node(
                    &mut nodes,
                    &mut node_ids,
                    root,
                    module_base,
                    &evidence_id,
                    path,
                    "invariant evidence",
                    3,
                );
                push_atlas_edge(
                    &mut edges,
                    "verifies",
                    &evidence_id,
                    &id,
                    "verifies",
                    vec![atlas_source_ref(root, &module_base.join(path), "evidence")],
                );
            }
        }
    }

    if let Some(effects) = get_path(&manifest.value, &["effects"]).and_then(YamlValue::as_sequence)
    {
        for effect in effects {
            let name = get_str(effect, &["name"]).unwrap_or("effect");
            let id = stable_atlas_id("effect", name);
            let summary = summarize_effect(effect);
            let source_refs = vec![module_ref.clone()];
            push_atlas_node(
                &mut nodes,
                &mut node_ids,
                AtlasNode {
                    id: id.clone(),
                    kind: "effect".to_string(),
                    layer: "effects".to_string(),
                    label: name.to_string(),
                    summary,
                    group: get_str(effect, &["kind"]).map(ToString::to_string),
                    source_refs: source_refs.clone(),
                    clauses: effect_clauses(effect, &source_refs),
                    emphasis: 4,
                },
            );
            push_atlas_edge(
                &mut edges,
                "has-effect",
                &module_id,
                &id,
                "effect",
                vec![module_ref.clone()],
            );
        }
    }

    if let Some(state) = get_path(&manifest.value, &["state"]) {
        let id = stable_atlas_id("state", module_name);
        let mut source_refs = vec![module_ref.clone()];
        if let Some(model) = get_str(state, &["model"]) {
            source_refs.push(atlas_source_ref(
                root,
                &module_base.join(model),
                "state-model",
            ));
        }
        push_atlas_node(
            &mut nodes,
            &mut node_ids,
            AtlasNode {
                id: id.clone(),
                kind: "state".to_string(),
                layer: "lifecycle".to_string(),
                label: "state model".to_string(),
                summary: summarize_state(state),
                group: get_str(state, &["consistency_boundary"]).map(ToString::to_string),
                source_refs: source_refs.clone(),
                clauses: state_clauses(state, &source_refs),
                emphasis: 3,
            },
        );
        push_atlas_edge(
            &mut edges,
            "describes-lifecycle",
            &id,
            &module_id,
            "lifecycle",
            source_refs,
        );
    }

    if let Some(boundary) = get_path(&manifest.value, &["boundary"]) {
        let id = stable_atlas_id("boundary", module_name);
        let source_refs = vec![module_ref.clone()];
        push_atlas_node(
            &mut nodes,
            &mut node_ids,
            AtlasNode {
                id: id.clone(),
                kind: "boundary".to_string(),
                layer: "public-surface".to_string(),
                label: "boundary".to_string(),
                summary: summarize_boundary(boundary),
                group: Some("boundary".to_string()),
                source_refs: source_refs.clone(),
                clauses: boundary_clauses(boundary, &source_refs),
                emphasis: 4,
            },
        );
        push_atlas_edge(
            &mut edges,
            "defines-boundary",
            &module_id,
            &id,
            "boundary",
            vec![module_ref.clone()],
        );
    }

    let compatibility = get_str(&manifest.value, &["compatibility", "policy"])
        .unwrap_or("<missing>")
        .to_string();
    let compatibility_id = stable_atlas_id("compatibility", &compatibility);
    let compatibility_source_refs = vec![module_ref.clone()];
    push_atlas_node(
        &mut nodes,
        &mut node_ids,
        AtlasNode {
            id: compatibility_id.clone(),
            kind: "compatibility".to_string(),
            layer: "overview".to_string(),
            label: compatibility.clone(),
            summary: "Declared compatibility policy for replacing or evolving this module."
                .to_string(),
            group: Some("compatibility".to_string()),
            source_refs: compatibility_source_refs.clone(),
            clauses: compatibility_clauses(
                get_path(&manifest.value, &["compatibility"]),
                &compatibility_source_refs,
            ),
            emphasis: 3,
        },
    );
    push_atlas_edge(
        &mut edges,
        "declares-compatibility",
        &module_id,
        &compatibility_id,
        "compatibility",
        vec![module_ref.clone()],
    );

    for category in ["laws", "contracts", "scenarios", "boundaries"] {
        for path in get_string_array(&manifest.value, &["verification", category]) {
            let id = stable_atlas_id("verification", &format!("{category}:{path}"));
            push_verification_node(
                &mut nodes,
                &mut node_ids,
                root,
                module_base,
                &id,
                &path,
                category,
                3,
            );
            push_atlas_edge(
                &mut edges,
                "verifies",
                &id,
                &module_id,
                category,
                vec![atlas_source_ref(
                    root,
                    &module_base.join(&path),
                    "verification",
                )],
            );
        }
    }

    if let Some(operations) =
        get_path(&manifest.value, &["operations"]).and_then(YamlValue::as_mapping)
    {
        for (key, value) in operations {
            let name = key.as_str().unwrap_or("operation");
            let id = stable_atlas_id("operation", name);
            let source_refs = atlas_operation_source_refs(root, module_base, &module_ref, value);
            push_atlas_node(
                &mut nodes,
                &mut node_ids,
                AtlasNode {
                    id: id.clone(),
                    kind: "operation".to_string(),
                    layer: "operations".to_string(),
                    label: name.to_string(),
                    summary: summarize_operation(value),
                    group: Some("operations".to_string()),
                    source_refs: source_refs.clone(),
                    clauses: operation_clauses(name, value, &source_refs),
                    emphasis: 3,
                },
            );
            push_atlas_edge(
                &mut edges,
                "operates",
                &module_id,
                &id,
                "operation",
                source_refs,
            );
        }
    }

    let traces = build_atlas_traces(&nodes, &edges, &module_id);
    let tours = build_atlas_tours(&nodes, &module_id);

    Ok(AtlasDocument {
        spec: "rms/atlas/v0.1",
        source: AtlasSource {
            root: root.display().to_string(),
            module_manifest: atlas_source_ref(root, &manifest.path, "module-manifest").path,
            generated_by: VALIDATOR_NAME,
            generated_version: VALIDATOR_VERSION,
            source_revision: source_revision(root),
        },
        module: AtlasModuleSummary {
            id: module_id.clone(),
            name: module_name.to_string(),
            version: get_str(&manifest.value, &["module", "version"])
                .unwrap_or("")
                .to_string(),
            kind: get_str(&manifest.value, &["module", "kind"])
                .unwrap_or("")
                .to_string(),
            purpose: get_str(&manifest.value, &["module", "purpose"])
                .unwrap_or("")
                .to_string(),
            profiles: get_string_array(&manifest.value, &["profiles"]),
            compatibility,
        },
        layers: atlas_layers(),
        nodes,
        edges,
        traces,
        tours,
        annotations: Vec::new(),
        interaction: AtlasInteraction {
            default_focus: module_id,
            supports_live_reconciliation: true,
            agent_generation_policy:
                "Agents may add annotations and guided trace prose only for existing semantic IDs; topology remains derived from RMS artifacts.",
        },
    })
}

fn atlas_layers() -> Vec<AtlasLayer> {
    vec![
        AtlasLayer {
            id: "overview",
            label: "Overview",
        },
        AtlasLayer {
            id: "ownership",
            label: "Ownership",
        },
        AtlasLayer {
            id: "public-surface",
            label: "Public Surface",
        },
        AtlasLayer {
            id: "dependencies",
            label: "Dependencies",
        },
        AtlasLayer {
            id: "effects",
            label: "Effects",
        },
        AtlasLayer {
            id: "constraints",
            label: "Constraints",
        },
        AtlasLayer {
            id: "lifecycle",
            label: "Lifecycle",
        },
        AtlasLayer {
            id: "verification",
            label: "Verification",
        },
        AtlasLayer {
            id: "operations",
            label: "Operations",
        },
    ]
}

fn push_atlas_node(nodes: &mut Vec<AtlasNode>, node_ids: &mut BTreeSet<String>, node: AtlasNode) {
    if node_ids.insert(node.id.clone()) {
        nodes.push(node);
    }
}

fn push_verification_node(
    nodes: &mut Vec<AtlasNode>,
    node_ids: &mut BTreeSet<String>,
    root: &Path,
    module_base: &Path,
    id: &str,
    path: &str,
    group: &str,
    emphasis: u8,
) {
    let absolute_path = module_base.join(path);
    let source_refs = vec![atlas_source_ref(root, &absolute_path, "verification")];
    push_atlas_node(
        nodes,
        node_ids,
        AtlasNode {
            id: id.to_string(),
            kind: "verification".to_string(),
            layer: "verification".to_string(),
            label: path_label(path),
            summary: evidence_summary(&absolute_path),
            group: Some(group.to_string()),
            source_refs: source_refs.clone(),
            clauses: vec![atlas_clause(
                "evidence",
                group,
                evidence_summary(&absolute_path),
                &source_refs,
            )],
            emphasis,
        },
    );
}

fn push_atlas_edge(
    edges: &mut Vec<AtlasEdge>,
    kind: &str,
    from: &str,
    to: &str,
    label: &str,
    source_refs: Vec<AtlasSourceRef>,
) {
    edges.push(AtlasEdge {
        id: stable_atlas_id("edge", &format!("{kind}:{from}:{to}")),
        kind: kind.to_string(),
        from: from.to_string(),
        to: to.to_string(),
        label: label.to_string(),
        source_refs,
    });
}

fn atlas_named_references(value: &YamlValue, path: &[&str]) -> Vec<AtlasNamedReference> {
    get_path(value, path)
        .and_then(YamlValue::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| match item {
                    YamlValue::String(name) => Some(AtlasNamedReference {
                        name: name.to_string(),
                        contract: None,
                    }),
                    YamlValue::Mapping(_) => {
                        get_str(item, &["name"]).map(|name| AtlasNamedReference {
                            name: name.to_string(),
                            contract: get_str(item, &["contract"]).map(ToString::to_string),
                        })
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn module_summary_clauses(
    value: &YamlValue,
    source_refs: &[AtlasSourceRef],
) -> Vec<AtlasContractClause> {
    let mut clauses = Vec::new();
    if let Some(purpose) = get_str(value, &["module", "purpose"]) {
        clauses.push(atlas_clause("module", "purpose", purpose, source_refs));
    }
    if let Some(kind) = get_str(value, &["module", "kind"]) {
        clauses.push(atlas_clause("module", "kind", kind, source_refs));
    }
    let profiles = get_string_array(value, &["profiles"]);
    if !profiles.is_empty() {
        clauses.push(atlas_clause(
            "module",
            "profiles",
            profiles.join(", "),
            source_refs,
        ));
    }
    clauses
}

fn contract_clauses(root: &Path, path: &Path) -> Vec<AtlasContractClause> {
    let Ok(manifest) = load_manifest(path) else {
        return Vec::new();
    };
    let source_refs = vec![atlas_source_ref(root, path, "contract")];
    let contract_kind = get_str(&manifest.value, &["kind"]).unwrap_or("contract");
    let mut clauses = Vec::new();
    if let Some(meaning) = get_str(&manifest.value, &["meaning"]) {
        clauses.push(atlas_clause("contract", "meaning", meaning, &source_refs));
    }
    if let Some(kind) = get_str(&manifest.value, &["kind"]) {
        clauses.push(atlas_clause("contract", "kind", kind, &source_refs));
    }
    push_contract_section_clauses(
        &mut clauses,
        &manifest.value,
        "preconditions",
        "precondition",
        "precondition",
        &source_refs,
    );
    push_contract_section_clauses(
        &mut clauses,
        &manifest.value,
        "postconditions",
        "postcondition",
        "postcondition",
        &source_refs,
    );
    push_contract_section_clauses(
        &mut clauses,
        &manifest.value,
        "failure_categories",
        "failure",
        "failure",
        &source_refs,
    );
    if contract_kind == "command" {
        for (path, label) in [
            ("preconditions", "preconditions"),
            ("postconditions", "postconditions"),
            ("failure_categories", "failure categories"),
        ] {
            if get_path(&manifest.value, &[path])
                .and_then(YamlValue::as_sequence)
                .is_none_or(Vec::is_empty)
            {
                clauses.push(atlas_clause(
                    "gap",
                    label,
                    "Not declared by this command contract.",
                    &source_refs,
                ));
            }
        }
    }
    if let Some(policy) = get_str(&manifest.value, &["compatibility", "policy"]) {
        clauses.push(atlas_clause(
            "compatibility",
            "policy",
            policy,
            &source_refs,
        ));
    }
    clauses
}

fn push_contract_section_clauses(
    clauses: &mut Vec<AtlasContractClause>,
    value: &YamlValue,
    path: &str,
    kind: &str,
    label_prefix: &str,
    source_refs: &[AtlasSourceRef],
) {
    let Some(items) = get_path(value, &[path]).and_then(YamlValue::as_sequence) else {
        return;
    };
    for item in items {
        let label = get_str(item, &["id"])
            .map(|id| format!("{label_prefix}: {id}"))
            .unwrap_or_else(|| label_prefix.to_string());
        let statement = get_str(item, &["statement"])
            .map(ToString::to_string)
            .unwrap_or_else(|| atlas_yaml_inline(item, 6));
        clauses.push(atlas_clause(kind, label, statement, source_refs));
    }
}

fn invariant_clauses(
    invariant: &YamlValue,
    source_refs: &[AtlasSourceRef],
) -> Vec<AtlasContractClause> {
    let mut clauses = Vec::new();
    if let Some(statement) = get_str(invariant, &["statement"]) {
        clauses.push(atlas_clause(
            "invariant",
            "statement",
            statement,
            source_refs,
        ));
    }
    if let Some(enforced_by) = get_str(invariant, &["enforced_by"]) {
        clauses.push(atlas_clause(
            "invariant",
            "enforced by",
            enforced_by,
            source_refs,
        ));
    }
    if let Some(verified_by) = get_str(invariant, &["verified_by"]) {
        clauses.push(atlas_clause(
            "invariant",
            "verified by",
            verified_by,
            source_refs,
        ));
    }
    clauses
}

fn effect_clauses(effect: &YamlValue, source_refs: &[AtlasSourceRef]) -> Vec<AtlasContractClause> {
    let mut clauses = Vec::new();
    for key in ["kind", "capability"] {
        if let Some(value) = get_str(effect, &[key]) {
            clauses.push(atlas_clause("effect", key, value, source_refs));
        }
    }
    if let Some(semantics) = get_path(effect, &["semantics"]).and_then(YamlValue::as_mapping) {
        for (key, value) in semantics {
            let Some(key) = key.as_str() else {
                continue;
            };
            clauses.push(atlas_clause(
                "effect semantics",
                key,
                atlas_yaml_inline(value, 6),
                source_refs,
            ));
        }
    }
    clauses
}

fn state_clauses(state: &YamlValue, source_refs: &[AtlasSourceRef]) -> Vec<AtlasContractClause> {
    [
        "model",
        "consistency_boundary",
        "concurrency",
        "persistence",
        "migration_policy",
    ]
    .iter()
    .filter_map(|key| {
        get_str(state, &[*key]).map(|value| atlas_clause("state", *key, value, source_refs))
    })
    .collect()
}

fn boundary_clauses(
    boundary: &YamlValue,
    source_refs: &[AtlasSourceRef],
) -> Vec<AtlasContractClause> {
    let mut clauses = Vec::new();
    let accepted = get_string_array(boundary, &["accepted_contracts"]);
    if !accepted.is_empty() {
        clauses.push(atlas_clause(
            "boundary",
            "accepted contracts",
            accepted.join(", "),
            source_refs,
        ));
    }
    for key in [
        "validation",
        "authorization",
        "malformed_input",
        "deprecation",
    ] {
        if let Some(value) = get_str(boundary, &[key]) {
            clauses.push(atlas_clause("boundary", key, value, source_refs));
        }
    }
    clauses
}

fn compatibility_clauses(
    compatibility: Option<&YamlValue>,
    source_refs: &[AtlasSourceRef],
) -> Vec<AtlasContractClause> {
    let Some(compatibility) = compatibility else {
        return vec![atlas_clause(
            "gap",
            "compatibility policy",
            "No compatibility policy is declared.",
            source_refs,
        )];
    };
    let Some(mapping) = compatibility.as_mapping() else {
        return Vec::new();
    };
    mapping
        .iter()
        .filter_map(|(key, value)| {
            Some(atlas_clause(
                "compatibility",
                key.as_str()?,
                atlas_yaml_inline(value, 6),
                source_refs,
            ))
        })
        .collect()
}

fn operation_clauses(
    name: &str,
    value: &YamlValue,
    source_refs: &[AtlasSourceRef],
) -> Vec<AtlasContractClause> {
    match value {
        YamlValue::Mapping(mapping) => mapping
            .iter()
            .filter_map(|(key, value)| {
                Some(atlas_clause(
                    "operation",
                    key.as_str()?,
                    atlas_yaml_inline(value, 6),
                    source_refs,
                ))
            })
            .collect(),
        YamlValue::Sequence(items) => vec![atlas_clause(
            "operation",
            name,
            items
                .iter()
                .map(|item| atlas_yaml_inline(item, 6))
                .collect::<Vec<_>>()
                .join(", "),
            source_refs,
        )],
        YamlValue::String(value) => vec![atlas_clause("operation", name, value, source_refs)],
        _ => Vec::new(),
    }
}

fn atlas_clause(
    kind: impl Into<String>,
    label: impl Into<String>,
    statement: impl Into<String>,
    source_refs: &[AtlasSourceRef],
) -> AtlasContractClause {
    let kind = kind.into();
    let label = label.into();
    let statement = statement.into();
    AtlasContractClause {
        id: stable_atlas_id("clause", &format!("{kind}:{label}:{statement}")),
        kind,
        label,
        statement,
        source_refs: source_refs.to_vec(),
    }
}

fn atlas_yaml_inline(value: &YamlValue, limit: usize) -> String {
    match value {
        YamlValue::Null => "null".to_string(),
        YamlValue::Bool(value) => value.to_string(),
        YamlValue::Number(value) => value.to_string(),
        YamlValue::String(value) => value.to_string(),
        YamlValue::Sequence(items) => {
            let mut parts = items
                .iter()
                .take(limit)
                .map(|item| atlas_yaml_inline(item, limit))
                .collect::<Vec<_>>();
            if items.len() > limit {
                parts.push("...".to_string());
            }
            parts.join(", ")
        }
        YamlValue::Mapping(mapping) => {
            let mut parts = mapping
                .iter()
                .filter_map(|(key, value)| {
                    Some(format!(
                        "{}: {}",
                        key.as_str()?,
                        atlas_yaml_inline(value, limit)
                    ))
                })
                .take(limit)
                .collect::<Vec<_>>();
            if mapping.len() > limit {
                parts.push("...".to_string());
            }
            parts.join("; ")
        }
        _ => serde_yaml::to_string(value)
            .map(|rendered| {
                rendered
                    .lines()
                    .map(str::trim)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_else(|_| "<structured>".to_string()),
    }
}

fn contract_meaning(path: &Path) -> Option<String> {
    let manifest = load_manifest(path).ok()?;
    get_str(&manifest.value, &["meaning"]).map(ToString::to_string)
}

fn summarize_effect(effect: &YamlValue) -> String {
    let mut parts = Vec::new();
    if let Some(kind) = get_str(effect, &["kind"]) {
        parts.push(format!("kind: {kind}"));
    }
    if let Some(capability) = get_str(effect, &["capability"]) {
        parts.push(format!("capability: {capability}"));
    }
    if let Some(semantics) = get_path(effect, &["semantics"]) {
        let summary = summarize_yaml_mapping(semantics, 4);
        if !summary.is_empty() {
            parts.push(summary);
        }
    }
    if parts.is_empty() {
        "Declared external effect.".to_string()
    } else {
        parts.join("; ")
    }
}

fn summarize_state(state: &YamlValue) -> String {
    let mut parts = Vec::new();
    for key in [
        "model",
        "consistency_boundary",
        "concurrency",
        "persistence",
        "migration_policy",
    ] {
        if let Some(value) = get_str(state, &[key]) {
            parts.push(format!("{key}: {value}"));
        }
    }
    if parts.is_empty() {
        "Declared lifecycle or state model.".to_string()
    } else {
        parts.join("; ")
    }
}

fn summarize_boundary(boundary: &YamlValue) -> String {
    let mut parts = Vec::new();
    for key in [
        "validation",
        "authorization",
        "malformed_input",
        "deprecation",
    ] {
        if let Some(value) = get_str(boundary, &[key]) {
            parts.push(format!("{key}: {value}"));
        }
    }
    let accepted = get_string_array(boundary, &["accepted_contracts"]);
    if !accepted.is_empty() {
        parts.push(format!("accepted contracts: {}", accepted.join(", ")));
    }
    if parts.is_empty() {
        "Declared boundary behavior.".to_string()
    } else {
        parts.join("; ")
    }
}

fn summarize_operation(value: &YamlValue) -> String {
    match value {
        YamlValue::Mapping(_) => {
            let summary = summarize_yaml_mapping(value, 6);
            if summary.is_empty() {
                "Declared operational semantics.".to_string()
            } else {
                summary
            }
        }
        YamlValue::Sequence(items) => {
            let paths = items
                .iter()
                .filter_map(YamlValue::as_str)
                .take(6)
                .collect::<Vec<_>>();
            if paths.is_empty() {
                "Declared operational references.".to_string()
            } else {
                paths.join("; ")
            }
        }
        YamlValue::String(value) => value.to_string(),
        _ => "Declared operational semantics.".to_string(),
    }
}

fn atlas_operation_source_refs(
    root: &Path,
    module_base: &Path,
    module_ref: &AtlasSourceRef,
    value: &YamlValue,
) -> Vec<AtlasSourceRef> {
    let mut refs = vec![module_ref.clone()];
    collect_operation_path_refs(root, module_base, value, &mut refs);
    refs
}

fn collect_operation_path_refs(
    root: &Path,
    module_base: &Path,
    value: &YamlValue,
    refs: &mut Vec<AtlasSourceRef>,
) {
    match value {
        YamlValue::String(path) if path.contains('/') || path.contains('.') => {
            let source_ref = atlas_source_ref(root, &module_base.join(path), "operation");
            if !refs.iter().any(|existing| {
                existing.role == source_ref.role && existing.path == source_ref.path
            }) {
                refs.push(source_ref);
            }
        }
        YamlValue::Sequence(items) => {
            for item in items {
                collect_operation_path_refs(root, module_base, item, refs);
            }
        }
        YamlValue::Mapping(mapping) => {
            for value in mapping.values() {
                collect_operation_path_refs(root, module_base, value, refs);
            }
        }
        _ => {}
    }
}

fn summarize_yaml_mapping(value: &YamlValue, limit: usize) -> String {
    let Some(mapping) = value.as_mapping() else {
        return String::new();
    };
    mapping
        .iter()
        .filter_map(|(key, value)| {
            Some(format!(
                "{}: {}",
                key.as_str()?,
                value
                    .as_str()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "<structured>".to_string())
            ))
        })
        .take(limit)
        .collect::<Vec<_>>()
        .join("; ")
}

fn evidence_summary(path: &Path) -> String {
    if path.is_dir() {
        let count = WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .count();
        return format!("{count} evidence artifact(s) under this path.");
    }
    if path.exists() {
        "Evidence artifact referenced by the module.".to_string()
    } else {
        "Evidence path referenced by the module.".to_string()
    }
}

fn path_label(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn atlas_source_ref(root: &Path, path: &Path, role: &str) -> AtlasSourceRef {
    AtlasSourceRef {
        role: role.to_string(),
        path: display_path(&root_relative_path(root, path)),
    }
}

fn atlas_nodes_for_kind<'a>(nodes: &'a [AtlasNode], kind: &str) -> Vec<&'a AtlasNode> {
    nodes.iter().filter(|node| node.kind == kind).collect()
}

fn atlas_nodes_for_group<'a>(
    nodes: &'a [AtlasNode],
    layer: &str,
    group: &str,
) -> Vec<&'a AtlasNode> {
    nodes
        .iter()
        .filter(|node| node.layer == layer && node.group.as_deref() == Some(group))
        .collect()
}

fn atlas_take_nodes<'a>(groups: &[&[&'a AtlasNode]], limit: usize) -> Vec<&'a AtlasNode> {
    let mut result = Vec::new();
    for group in groups {
        for node in *group {
            if !result
                .iter()
                .any(|existing: &&AtlasNode| existing.id == node.id)
            {
                result.push(*node);
            }
            if result.len() >= limit {
                return result;
            }
        }
    }
    result
}

fn atlas_node_ids(nodes: &[&AtlasNode]) -> Vec<String> {
    nodes.iter().map(|node| node.id.clone()).collect()
}

fn build_atlas_traces(
    nodes: &[AtlasNode],
    edges: &[AtlasEdge],
    module_id: &str,
) -> Vec<AtlasTraceProjection> {
    let Some(module) = nodes.iter().find(|node| node.id == module_id) else {
        return Vec::new();
    };

    let commands = atlas_nodes_for_group(nodes, "public-surface", "commands");
    let entries = if commands.is_empty() {
        vec![module]
    } else {
        commands
    };
    let boundaries = atlas_nodes_for_kind(nodes, "boundary");
    let invariants = atlas_nodes_for_kind(nodes, "invariant");
    let states = atlas_nodes_for_kind(nodes, "state");
    let effects = atlas_nodes_for_kind(nodes, "effect");
    let events = atlas_nodes_for_group(nodes, "public-surface", "events");
    let operations = atlas_nodes_for_kind(nodes, "operation");
    let verification = atlas_nodes_for_kind(nodes, "verification");
    let compatibility = atlas_nodes_for_kind(nodes, "compatibility");

    entries
        .into_iter()
        .map(|entry| {
            build_atlas_trace_for_entry(
                module,
                entry,
                edges,
                &boundaries,
                &invariants,
                &states,
                &effects,
                &events,
                &operations,
                &verification,
                &compatibility,
            )
        })
        .collect()
}

fn build_atlas_trace_for_entry(
    module: &AtlasNode,
    entry: &AtlasNode,
    edges: &[AtlasEdge],
    boundaries: &[&AtlasNode],
    invariants: &[&AtlasNode],
    states: &[&AtlasNode],
    effects: &[&AtlasNode],
    events: &[&AtlasNode],
    operations: &[&AtlasNode],
    verification: &[&AtlasNode],
    compatibility: &[&AtlasNode],
) -> AtlasTraceProjection {
    let mut steps = Vec::new();
    let mut gaps = Vec::new();
    let entry_nodes = if entry.id == module.id {
        vec![module]
    } else {
        vec![entry, module]
    };
    let entry_refs = collect_atlas_source_refs(&entry_nodes);

    steps.push(atlas_trace_step(
        "stimulus",
        "Stimulus",
        &format!("Start at {}", entry.label),
        if entry.id == module.id {
            format!(
                "`{}` is the module being projected from canonical RMS artifacts.",
                module.label
            )
        } else {
            format!(
                "`{}` enters through the declared public surface. {}",
                entry.label, entry.summary
            )
        },
        &entry_nodes,
        "direct",
        edges,
    ));

    if boundaries.is_empty() {
        gaps.push(atlas_trace_gap(
            "boundary",
            "No boundary node",
            "The manifest does not declare boundary behavior for this trace.",
            Some("module.boundary".to_string()),
            &entry_refs,
        ));
    } else {
        steps.push(atlas_trace_step(
            "boundary",
            "Boundary",
            "Cross the declared boundary",
            "Validation, authorization, accepted contracts, and deprecation policy define how external input is allowed to enter.",
            boundaries,
            "direct",
            edges,
        ));
    }

    let matched_invariants = atlas_semantic_matches(entry, invariants, 2, true);
    if matched_invariants.is_empty() {
        if invariants.is_empty() {
            gaps.push(atlas_trace_gap(
                "rule",
                "No invariant node",
                "No declared invariant was available to constrain this trace.",
                Some("module.invariants".to_string()),
                &entry_refs,
            ));
        } else {
            gaps.push(atlas_trace_gap(
                "rule",
                "No explicit rule link",
                "Invariants exist, but the canonical artifacts do not explicitly connect one to this entry. The atlas will not invent that edge.",
                Some("contract preconditions or invariant linkage".to_string()),
                &entry_refs,
            ));
        }
    } else {
        steps.push(atlas_trace_step(
            "rule",
            "Rule",
            "Apply the governing invariant",
            format!(
                "The trace is constrained by {}.",
                atlas_label_list(&matched_invariants)
            ),
            &matched_invariants,
            "inferred",
            edges,
        ));
    }

    if states.is_empty() {
        gaps.push(atlas_trace_gap(
            "state",
            "No lifecycle state",
            "No state model was declared for this module.",
            Some("module.state".to_string()),
            &entry_refs,
        ));
    } else {
        steps.push(atlas_trace_step(
            "state",
            "State",
            "Update or inspect lifecycle state",
            "The declared state model is the consistency boundary for lifecycle, persistence, concurrency, and migration semantics.",
            states,
            "direct",
            edges,
        ));
    }

    let effect_matches = atlas_semantic_matches(entry, effects, 1, false);
    let (effect_nodes, effect_confidence) = if !effect_matches.is_empty() {
        (effect_matches, "inferred")
    } else if effects.len() == 1 {
        (effects.to_vec(), "module-level")
    } else {
        (Vec::new(), "gap")
    };
    if effect_nodes.is_empty() {
        gaps.push(atlas_trace_gap(
            "effect",
            "No effect link",
            "No declared effect could be tied to this trace from the current artifacts.",
            Some("effect semantics or command-to-effect linkage".to_string()),
            &entry_refs,
        ));
    } else {
        steps.push(atlas_trace_step(
            "effect",
            "Effect",
            "Touch external truth",
            format!(
                "{} shapes idempotency, ordering, retry, timeout, or other effect semantics.",
                atlas_label_list(&effect_nodes)
            ),
            &effect_nodes,
            effect_confidence,
            edges,
        ));
    }

    let event_matches = atlas_semantic_matches(entry, events, 1, false);
    let (event_nodes, event_confidence) = if !event_matches.is_empty() {
        (event_matches, "inferred")
    } else if events.len() == 1 {
        (events.to_vec(), "module-level")
    } else {
        (Vec::new(), "gap")
    };
    if event_nodes.is_empty() {
        gaps.push(atlas_trace_gap(
            "outcome",
            "No outcome event",
            "No public event was declared as the observable outcome for this trace.",
            Some("provided event or contract outcome semantics".to_string()),
            &entry_refs,
        ));
    } else {
        steps.push(atlas_trace_step(
            "outcome",
            "Outcome",
            "Publish or expose the outcome",
            format!(
                "{} is the declared public outcome surface related to this trace.",
                atlas_label_list(&event_nodes)
            ),
            &event_nodes,
            event_confidence,
            edges,
        ));
    }

    let operation_matches = atlas_semantic_matches(entry, operations, 1, false);
    let (operation_nodes, operation_confidence) = if !operation_matches.is_empty() {
        (operation_matches, "inferred")
    } else if !operations.is_empty() && !effect_nodes.is_empty() {
        (operations.to_vec(), "module-level")
    } else {
        (Vec::new(), "gap")
    };
    if !operation_nodes.is_empty() {
        steps.push(atlas_trace_step(
            "operate",
            "Operate",
            "Recover and observe",
            "Declared observability, runtime checks, reconciliation, or runbooks tell a human how this behavior is operated.",
            &operation_nodes,
            operation_confidence,
            edges,
        ));
    }

    let proof_nodes = atlas_proof_nodes_for(&matched_invariants, verification, edges);
    if proof_nodes.is_empty() {
        if verification.is_empty() {
            gaps.push(atlas_trace_gap(
                "proof",
                "No proof lane",
                "The module does not declare verification evidence for this trace.",
                Some("module.verification".to_string()),
                &entry_refs,
            ));
        } else {
            steps.push(atlas_trace_step(
                "proof",
                "Proof",
                "Check module-level evidence",
                "Verification exists, but the current artifacts do not tie a specific evidence path to this trace.",
                &atlas_take_nodes(&[verification], 4),
                "module-level",
                edges,
            ));
        }
    } else {
        steps.push(atlas_trace_step(
            "proof",
            "Proof",
            "Verify the protected promise",
            format!(
                "{} backs the rule step through declared evidence links.",
                atlas_label_list(&proof_nodes)
            ),
            &proof_nodes,
            "direct",
            edges,
        ));
    }

    if !compatibility.is_empty() && entry.id != module.id {
        steps.push(atlas_trace_step(
            "compatibility",
            "Compatibility",
            "Respect public evolution policy",
            "A public entrypoint is compatibility-sensitive; changing it must preserve or deliberately evolve the declared policy.",
            compatibility,
            "direct",
            edges,
        ));
    }

    let source_refs = collect_atlas_trace_source_refs(&steps, &gaps);
    AtlasTraceProjection {
        id: stable_atlas_id("trace", &entry.label),
        label: if entry.id == module.id {
            format!("{} orientation", module.label)
        } else {
            entry.label.clone()
        },
        intent: if entry.id == module.id {
            "orient"
        } else {
            "change-risk"
        },
        entry_node_id: entry.id.clone(),
        summary: if entry.id == module.id {
            format!(
                "See what {} owns, promises, depends on, and proves.",
                module.label
            )
        } else {
            format!(
                "A {} request moves through checks, rules, stored truth, outside effects, visible results, recovery, and proof.",
                human_identifier(&entry.label)
            )
        },
        steps,
        gaps,
        source_refs,
    }
}

fn atlas_trace_step(
    id: &str,
    role: &'static str,
    title: &str,
    body: impl Into<String>,
    nodes: &[&AtlasNode],
    confidence: &'static str,
    edges: &[AtlasEdge],
) -> AtlasTraceStep {
    let node_ids = atlas_node_ids(nodes);
    let body = body.into();
    AtlasTraceStep {
        id: id.to_string(),
        role,
        title: title.to_string(),
        body: body.clone(),
        reading: atlas_trace_reading(role, title, &body, nodes, confidence),
        edge_ids: atlas_trace_edge_ids(edges, &node_ids),
        node_ids,
        confidence,
        source_refs: collect_atlas_source_refs(nodes),
    }
}

fn atlas_trace_reading(
    role: &str,
    title: &str,
    body: &str,
    nodes: &[&AtlasNode],
    confidence: &str,
) -> AtlasTraceReading {
    let clauses = nodes
        .iter()
        .flat_map(|node| node.clauses.iter())
        .collect::<Vec<_>>();
    let role_key = role.to_ascii_lowercase();
    AtlasTraceReading {
        promise: atlas_trace_promise(&role_key, title, body, &clauses),
        before: atlas_trace_before(&role_key, &clauses),
        after: atlas_trace_after(&role_key, body, &clauses),
        failure: atlas_trace_failure(&role_key, &clauses),
        evidence: atlas_trace_evidence(nodes, confidence),
        impact: atlas_trace_impact(&role_key),
        justification: atlas_trace_justification(nodes),
    }
}

fn atlas_trace_promise(
    role: &str,
    title: &str,
    body: &str,
    clauses: &[&AtlasContractClause],
) -> String {
    match role {
        "stimulus" => first_clause(clauses, |clause| {
            clause.kind == "contract" && clause.label == "meaning"
        })
        .unwrap_or_else(|| body.to_string()),
        "boundary" => sentence_join(
            ["accepted contracts", "validation"]
                .iter()
                .filter_map(|label| first_labeled_readable_clause(clauses, label))
                .collect(),
        )
        .unwrap_or_else(|| body.to_string()),
        "rule" => sentence_join(clause_statements(
            clauses,
            |clause| clause.kind == "invariant" && clause.label == "statement",
            3,
        ))
        .unwrap_or_else(|| body.to_string()),
        "state" => sentence_join(readable_clauses(
            clauses,
            |clause| {
                clause.kind == "state"
                    && matches!(
                        clause.label.as_str(),
                        "model" | "consistency_boundary" | "concurrency" | "persistence"
                    )
            },
            3,
        ))
        .unwrap_or_else(|| body.to_string()),
        "effect" => sentence_join(readable_clauses(
            clauses,
            |clause| {
                matches!(
                    clause.label.as_str(),
                    "idempotency" | "ordering" | "timeout" | "retry" | "compensation"
                )
            },
            4,
        ))
        .unwrap_or_else(|| body.to_string()),
        "outcome" => first_clause(clauses, |clause| {
            clause.kind == "contract" && clause.label == "meaning"
        })
        .map(human_outcome_meaning)
        .unwrap_or_else(|| body.to_string()),
        "operate" => sentence_join(readable_clauses(
            clauses,
            |clause| clause.kind == "operation",
            4,
        ))
        .unwrap_or_else(|| body.to_string()),
        "proof" => atlas_evidence_promise(clauses).unwrap_or_else(|| body.to_string()),
        "compatibility" => sentence_join(readable_clauses(
            clauses,
            |clause| clause.kind == "compatibility",
            3,
        ))
        .unwrap_or_else(|| body.to_string()),
        _ => title.to_string(),
    }
}

fn atlas_evidence_promise(clauses: &[&AtlasContractClause]) -> Option<String> {
    let count = clauses
        .iter()
        .filter(|clause| clause.kind == "evidence")
        .count();
    (count > 0).then(|| {
        let noun = if count == 1 {
            "evidence artifact"
        } else {
            "evidence artifacts"
        };
        format!("{count} declared {noun} back this promise.")
    })
}

fn atlas_trace_before(role: &str, clauses: &[&AtlasContractClause]) -> String {
    match role {
        "stimulus" => first_clause(clauses, |clause| clause.kind == "precondition")
            .or_else(|| missing_clause(clauses, "preconditions"))
            .unwrap_or_else(|| "No entry preconditions are declared for this stage.".to_string()),
        "boundary" => sentence_join(
            ["accepted contracts", "authorization", "validation"]
                .iter()
                .filter_map(|label| first_labeled_readable_clause(clauses, label))
                .collect(),
        )
        .unwrap_or_else(|| "No boundary entry conditions are declared.".to_string()),
        "rule" => sentence_join(clause_statements(
            clauses,
            |clause| clause.kind == "invariant",
            2,
        ))
        .unwrap_or_else(|| "No governing rule is declared for this stage.".to_string()),
        "state" => first_labeled_readable_clause(clauses, "consistency_boundary")
            .unwrap_or_else(|| "No state consistency boundary is declared.".to_string()),
        "effect" => sentence_join(
            ["idempotency", "ordering"]
                .iter()
                .filter_map(|label| first_labeled_readable_clause(clauses, label))
                .collect(),
        )
        .unwrap_or_else(|| "No effect preconditions are declared.".to_string()),
        "outcome" => {
            "The preceding contract stages determine whether an observable outcome exists."
                .to_string()
        }
        "operate" => first_labeled_readable_clause(clauses, "observability").unwrap_or_else(|| {
            "Operation starts from declared observability or runbook support.".to_string()
        }),
        "proof" => {
            "The selected claim must be backed by declared verification evidence.".to_string()
        }
        "compatibility" => {
            "A public surface or stored behavior is being read as compatibility-sensitive."
                .to_string()
        }
        _ => "No before-state is declared for this stage.".to_string(),
    }
}

fn atlas_trace_after(role: &str, body: &str, clauses: &[&AtlasContractClause]) -> String {
    match role {
        "stimulus" => first_clause(clauses, |clause| clause.kind == "postcondition")
            .or_else(|| missing_clause(clauses, "postconditions"))
            .unwrap_or_else(|| body.to_string()),
        "boundary" => first_labeled_readable_clause(clauses, "malformed_input")
            .map(|value| format!("Accepted input continues. {value}"))
            .unwrap_or_else(|| body.to_string()),
        "rule" => sentence_join(clause_statements(
            clauses,
            |clause| clause.kind == "invariant",
            2,
        ))
        .unwrap_or_else(|| body.to_string()),
        "state" => sentence_join(
            ["persistence", "migration_policy"]
                .iter()
                .filter_map(|label| first_labeled_readable_clause(clauses, label))
                .collect(),
        )
        .unwrap_or_else(|| body.to_string()),
        "effect" => sentence_join(
            ["consistency", "compensation", "reconciliation"]
                .iter()
                .filter_map(|label| first_labeled_readable_clause(clauses, label))
                .collect(),
        )
        .unwrap_or_else(|| body.to_string()),
        "outcome" => body.to_string(),
        "operate" => sentence_join(
            ["runtime_checks", "reconciliation", "runbooks"]
                .iter()
                .filter_map(|label| first_labeled_readable_clause(clauses, label))
                .collect(),
        )
        .unwrap_or_else(|| body.to_string()),
        "proof" => atlas_evidence_promise(clauses).unwrap_or_else(|| body.to_string()),
        "compatibility" => sentence_join(readable_clauses(
            clauses,
            |clause| clause.kind == "compatibility",
            3,
        ))
        .unwrap_or_else(|| body.to_string()),
        _ => body.to_string(),
    }
}

fn atlas_trace_failure(role: &str, clauses: &[&AtlasContractClause]) -> String {
    match role {
        "stimulus" => first_clause(clauses, |clause| clause.kind == "failure")
            .or_else(|| missing_clause(clauses, "failure categories"))
            .unwrap_or_else(|| "No failure categories are declared for this command.".to_string()),
        "boundary" => first_labeled_readable_clause(clauses, "malformed_input")
            .unwrap_or_else(|| "No malformed-input behavior is declared.".to_string()),
        "effect" => sentence_join(
            ["timeout", "retry", "compensation", "reconciliation"]
                .iter()
                .filter_map(|label| first_labeled_readable_clause(clauses, label))
                .collect(),
        )
        .unwrap_or_else(|| "No external-effect failure semantics are declared.".to_string()),
        "proof" => {
            "If evidence is absent or stale, trust in this stage becomes a review obligation."
                .to_string()
        }
        _ => first_clause(clauses, |clause| clause.kind == "failure")
            .unwrap_or_else(|| "No stage-specific failure category is declared.".to_string()),
    }
}

fn atlas_trace_evidence(nodes: &[&AtlasNode], confidence: &str) -> String {
    let sources = collect_atlas_source_refs(nodes);
    let labels = nodes
        .iter()
        .map(|node| format!("`{}`", node.label))
        .collect::<Vec<_>>()
        .join(", ");
    if sources.is_empty() {
        format!("{confidence} projection from {labels}.")
    } else {
        format!(
            "{confidence} projection from {labels}; {} source reference(s).",
            sources.len()
        )
    }
}

fn atlas_trace_impact(role: &str) -> String {
    match role {
        "stimulus" => "Changing this entrypoint can change the public reason the behavior exists.",
        "boundary" => "Changing this stage can alter validation, authorization, or accepted input.",
        "rule" => "Changing this stage can weaken or move domain authority.",
        "state" => {
            "Changing this stage can affect lifecycle, persistence, concurrency, or migrations."
        }
        "effect" => {
            "Changing this stage can affect external truth, retries, idempotency, or compensation."
        }
        "outcome" => "Changing this stage can affect subscribers, queries, or observable facts.",
        "operate" => {
            "Changing this stage can affect recovery, reconciliation, or incident handling."
        }
        "proof" => "Changing this stage can leave promises unverified.",
        "compatibility" => "Changing this stage can break existing consumers or stored state.",
        _ => "Review this semantic stage before changing it.",
    }
    .to_string()
}

fn atlas_trace_justification(nodes: &[&AtlasNode]) -> Vec<AtlasJustificationStep> {
    nodes
        .iter()
        .map(|node| AtlasJustificationStep {
            node_id: node.id.clone(),
            label: node.label.clone(),
            role: format!("{}/{}", node.layer, node.kind),
            detail: node
                .clauses
                .iter()
                .find(|clause| clause.kind != "gap")
                .map(|clause| clause.statement.clone())
                .unwrap_or_else(|| node.summary.clone()),
        })
        .collect()
}

fn first_clause(
    clauses: &[&AtlasContractClause],
    predicate: impl Fn(&AtlasContractClause) -> bool,
) -> Option<String> {
    clauses
        .iter()
        .find(|clause| predicate(clause))
        .map(|clause| clause.statement.clone())
}

fn first_labeled_readable_clause(clauses: &[&AtlasContractClause], label: &str) -> Option<String> {
    clauses
        .iter()
        .find(|clause| clause.label == label)
        .map(|clause| readable_clause(clause))
}

fn missing_clause(clauses: &[&AtlasContractClause], label: &str) -> Option<String> {
    clauses
        .iter()
        .find(|clause| clause.kind == "gap" && clause.label == label)
        .map(|clause| format!("Missing {label}: {}", clause.statement))
}

fn readable_clauses(
    clauses: &[&AtlasContractClause],
    predicate: impl Fn(&AtlasContractClause) -> bool,
    limit: usize,
) -> Vec<String> {
    clauses
        .iter()
        .filter(|clause| predicate(clause))
        .take(limit)
        .map(|clause| readable_clause(clause))
        .collect()
}

fn readable_clause(clause: &AtlasContractClause) -> String {
    let label = clause.label.replace('_', " ");
    let statement = clause.statement.trim();
    if label.starts_with("precondition") || label.starts_with("postcondition") {
        return sentence(statement);
    }
    match label.as_str() {
        "accepted contracts" => sentence(format!(
            "Only these public contracts can enter here: {}",
            human_list(statement)
        )),
        "validation" if statement == "reject-before-domain-entry" => {
            "Invalid input is rejected before it reaches domain logic.".to_string()
        }
        "validation" => sentence(format!(
            "Validation follows {}",
            human_identifier(statement)
        )),
        "authorization" => sentence(format!(
            "Callers need {} authority",
            human_identifier(statement)
        )),
        "malformed input" if statement == "reject-with-stable-error" => {
            "Malformed input gets a stable error.".to_string()
        }
        "malformed input" => sentence(format!(
            "Malformed input is handled as {}",
            human_identifier(statement)
        )),
        "deprecation" => sentence(format!(
            "Deprecation follows {}",
            human_identifier(statement)
        )),
        "model" if looks_like_path(statement) => {
            sentence(format!("The lifecycle is described in {statement}"))
        }
        "model" => sentence(format!(
            "The lifecycle follows {}",
            human_identifier(statement)
        )),
        "consistency boundary" if statement == "one-payment" => {
            "Each payment is decided inside its own consistency boundary.".to_string()
        }
        "consistency boundary" => sentence(format!(
            "The consistency boundary is {}",
            human_identifier(statement)
        )),
        "concurrency" if statement == "optimistic-version" => {
            "Concurrent changes are guarded by version checks.".to_string()
        }
        "concurrency" => sentence(format!(
            "Concurrency is handled by {}",
            human_identifier(statement)
        )),
        "persistence" if statement == "durable-ledger" => {
            "Accepted changes are recorded in the durable ledger.".to_string()
        }
        "persistence" => sentence(format!("Persistence uses {}", human_identifier(statement))),
        "migration policy" => sentence(format!(
            "Stored state changes through {} migrations",
            human_identifier(statement)
        )),
        "idempotency" => sentence(format!(
            "Use {} to avoid applying the same operation twice",
            human_reference(statement)
        )),
        "ordering" if statement == "per-payment" => {
            "Keep provider operations ordered for each payment.".to_string()
        }
        "ordering" => sentence(format!(
            "Ordering is enforced {}",
            human_identifier(statement)
        )),
        "timeout" if statement == "unknown-outcome" => {
            "A timeout leaves the result unknown until retry or reconciliation settles it."
                .to_string()
        }
        "timeout" => sentence(format!(
            "Timeouts are treated as {}",
            human_identifier(statement)
        )),
        "retry" if statement == "same-idempotency-key-only" => {
            "Retries must use the same idempotency key.".to_string()
        }
        "retry" => sentence(format!("Retries follow {}", human_identifier(statement))),
        "consistency" if statement == "per-payment" => {
            "External consistency is managed per payment.".to_string()
        }
        "consistency" => sentence(format!(
            "External consistency follows {}",
            human_identifier(statement)
        )),
        "compensation" if statement == "refund-payment" => {
            "If compensation is needed, it is done by refunding the payment.".to_string()
        }
        "compensation" => sentence(format!("Compensation uses {}", human_identifier(statement))),
        "reconciliation" if statement == "required" => {
            "Reconciliation is required when provider truth is uncertain.".to_string()
        }
        "reconciliation" if looks_like_path(statement) => {
            sentence(format!("Reconciliation is handled through {statement}"))
        }
        "reconciliation" => sentence(format!(
            "Reconciliation follows {}",
            human_identifier(statement)
        )),
        "correlation" => sentence(format!(
            "Operators can find this flow by {}",
            human_reference(statement)
        )),
        "causation" => sentence(format!(
            "Follow-up work is tied back to {}",
            human_reference(statement)
        )),
        "runtime checks" => sentence(format!("Runtime checks live at {statement}")),
        "runbooks" => sentence(format!("The recovery runbook is {statement}")),
        "policy" => sentence(format!(
            "Public changes follow the {} policy",
            human_identifier(statement)
        )),
        "events" => sentence(format!(
            "Integration events are {}",
            human_identifier(statement)
        )),
        "migration" => sentence(format!(
            "Migration requires {}",
            human_identifier(statement)
        )),
        "stored state" => sentence(format!(
            "Stored state changes require {}",
            human_identifier(statement)
        )),
        _ if statement.is_empty() => label,
        _ => sentence(statement),
    }
}

fn clause_statements(
    clauses: &[&AtlasContractClause],
    predicate: impl Fn(&AtlasContractClause) -> bool,
    limit: usize,
) -> Vec<String> {
    clauses
        .iter()
        .filter(|clause| predicate(clause))
        .take(limit)
        .map(|clause| clause.statement.clone())
        .collect()
}

fn sentence_join(parts: Vec<String>) -> Option<String> {
    let parts = parts
        .into_iter()
        .filter_map(|part| {
            let part = part.trim();
            (!part.is_empty()).then(|| sentence(part))
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn sentence(value: impl AsRef<str>) -> String {
    let value = value.as_ref().trim();
    if value.ends_with('.') || value.ends_with('!') || value.ends_with('?') {
        value.to_string()
    } else {
        format!("{value}.")
    }
}

fn human_identifier(value: &str) -> String {
    value
        .trim()
        .trim_matches('`')
        .replace(['-', '_', '.'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn human_reference(value: &str) -> String {
    let identifier = human_identifier(value);
    if identifier.ends_with(" id") || identifier.ends_with(" key") {
        format!("the {identifier}")
    } else {
        identifier
    }
}

fn human_outcome_meaning(value: String) -> String {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix("Published fact for ") {
        sentence(format!("It publishes a fact for {rest}"))
    } else {
        sentence(value)
    }
}

fn human_list(value: &str) -> String {
    value
        .split(',')
        .map(|item| human_identifier(item.trim()))
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

fn atlas_trace_gap(
    id: &str,
    title: &str,
    body: &str,
    suggested_artifact: Option<String>,
    source_refs: &[AtlasSourceRef],
) -> AtlasTraceGap {
    AtlasTraceGap {
        id: id.to_string(),
        title: title.to_string(),
        body: body.to_string(),
        suggested_artifact,
        source_refs: source_refs.to_vec(),
    }
}

fn atlas_semantic_matches<'a>(
    entry: &AtlasNode,
    candidates: &[&'a AtlasNode],
    min_score: usize,
    prefer_action: bool,
) -> Vec<&'a AtlasNode> {
    let entry_terms = atlas_semantic_terms(&format!("{} {}", entry.label, entry.summary));
    let action = atlas_action_term(&entry.label);
    let mut scored = candidates
        .iter()
        .filter_map(|node| {
            let node_terms = atlas_semantic_terms(&format!(
                "{} {} {}",
                node.label,
                node.summary,
                node.group.as_deref().unwrap_or("")
            ));
            let overlap = entry_terms.intersection(&node_terms).count();
            let action_match = action
                .as_ref()
                .is_some_and(|action| node_terms.contains(action));
            let score = overlap + usize::from(action_match) * 10;
            if (prefer_action && action_match) || (!prefer_action && overlap >= min_score) {
                Some((score, *node))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.label.cmp(&right.1.label))
    });
    scored.into_iter().map(|(_, node)| node).collect()
}

fn atlas_proof_nodes_for<'a>(
    invariants: &[&AtlasNode],
    verification: &[&'a AtlasNode],
    edges: &[AtlasEdge],
) -> Vec<&'a AtlasNode> {
    let invariant_ids = invariants
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut proof_nodes = Vec::new();
    for edge in edges {
        if edge.kind != "verifies" || !invariant_ids.contains(edge.to.as_str()) {
            continue;
        }
        if let Some(node) = verification.iter().find(|node| node.id == edge.from) {
            if !proof_nodes
                .iter()
                .any(|existing: &&AtlasNode| existing.id == node.id)
            {
                proof_nodes.push(*node);
            }
        }
    }
    proof_nodes
}

fn atlas_label_list(nodes: &[&AtlasNode]) -> String {
    nodes
        .iter()
        .map(|node| format!("`{}`", node.label))
        .collect::<Vec<_>>()
        .join(", ")
}

fn atlas_semantic_terms(value: &str) -> BTreeSet<String> {
    let mut terms = BTreeSet::new();
    let mut current = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            current.push(character.to_ascii_lowercase());
        } else {
            push_atlas_semantic_term(&mut terms, &mut current);
        }
    }
    push_atlas_semantic_term(&mut terms, &mut current);
    terms
}

fn push_atlas_semantic_term(terms: &mut BTreeSet<String>, current: &mut String) {
    if let Some(term) = normalize_atlas_semantic_token(current) {
        terms.insert(term);
    }
    current.clear();
}

fn atlas_action_term(label: &str) -> Option<String> {
    label
        .split(|character: char| !character.is_ascii_alphanumeric())
        .find_map(normalize_atlas_semantic_token)
}

fn normalize_atlas_semantic_token(token: &str) -> Option<String> {
    let token = token.trim().to_ascii_lowercase();
    if token.len() < 3 || is_atlas_stopword(&token) {
        return None;
    }
    let normalized = if token.len() > 4 && token.ends_with('s') && !token.ends_with("ss") {
        token.trim_end_matches('s').to_string()
    } else {
        token
    };
    (!is_atlas_stopword(&normalized)).then_some(normalized)
}

fn is_atlas_stopword(token: &str) -> bool {
    matches!(
        token,
        "and"
            | "the"
            | "for"
            | "from"
            | "with"
            | "this"
            | "that"
            | "into"
            | "only"
            | "module"
            | "public"
            | "surface"
            | "declared"
            | "request"
            | "valid"
            | "value"
            | "kind"
            | "contract"
            | "contracts"
    )
}

fn atlas_trace_edge_ids(edges: &[AtlasEdge], node_ids: &[String]) -> Vec<String> {
    edges
        .iter()
        .filter(|edge| node_ids.contains(&edge.from) && node_ids.contains(&edge.to))
        .map(|edge| edge.id.clone())
        .collect()
}

fn collect_atlas_source_refs(nodes: &[&AtlasNode]) -> Vec<AtlasSourceRef> {
    let mut refs = Vec::new();
    for node in nodes {
        for source_ref in &node.source_refs {
            push_atlas_source_ref(&mut refs, source_ref.clone());
        }
    }
    refs
}

fn collect_atlas_trace_source_refs(
    steps: &[AtlasTraceStep],
    gaps: &[AtlasTraceGap],
) -> Vec<AtlasSourceRef> {
    let mut refs = Vec::new();
    for step in steps {
        for source_ref in &step.source_refs {
            push_atlas_source_ref(&mut refs, source_ref.clone());
        }
    }
    for gap in gaps {
        for source_ref in &gap.source_refs {
            push_atlas_source_ref(&mut refs, source_ref.clone());
        }
    }
    refs
}

fn push_atlas_source_ref(refs: &mut Vec<AtlasSourceRef>, source_ref: AtlasSourceRef) {
    if !refs
        .iter()
        .any(|existing| existing.role == source_ref.role && existing.path == source_ref.path)
    {
        refs.push(source_ref);
    }
}

fn build_atlas_tours(nodes: &[AtlasNode], module_id: &str) -> Vec<AtlasTour> {
    let mut steps = Vec::new();
    for layer in [
        "overview",
        "ownership",
        "public-surface",
        "dependencies",
        "effects",
        "constraints",
        "lifecycle",
        "verification",
        "operations",
    ] {
        if let Some(node) = nodes.iter().find(|node| {
            if layer == "overview" {
                node.id == module_id
            } else {
                node.layer == layer
            }
        }) {
            steps.push(AtlasTourStep {
                node_id: node.id.clone(),
                title: node.label.clone(),
                body: node.summary.clone(),
            });
        }
    }

    vec![AtlasTour {
        id: "human-overview",
        title: "Human Overview",
        steps,
    }]
}

fn stable_atlas_id(kind: &str, value: &str) -> String {
    format!(
        "{}:{}",
        semantic_id_segment(kind),
        semantic_id_segment(value)
    )
}

fn semantic_id_segment(value: &str) -> String {
    let mut output = String::new();
    let mut previous_dash = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            output.push(character);
            previous_dash = false;
        } else if !previous_dash {
            output.push('-');
            previous_dash = true;
        }
    }
    let trimmed = output.trim_matches('-');
    if trimmed.is_empty() {
        "item".to_string()
    } else {
        trimmed.to_string()
    }
}

fn render_atlas_html(atlas_json: &str) -> String {
    ATLAS_HTML_TEMPLATE.replace("__ATLAS_JSON__", &html_script_json(atlas_json))
}

fn html_script_json(value: &str) -> String {
    value.replace("</script", "<\\/script")
}

const ATLAS_HTML_TEMPLATE: &str = include_str!("atlas_template.html");

fn build_conformance_report(module: &Path, implementation: Option<&Path>) -> Result<JsonValue> {
    let manifest = load_manifest(module)?;
    let mut diagnostics = Vec::new();
    validate_loaded_manifest(&manifest, &mut diagnostics);

    let implementation_name = if let Some(path) = implementation {
        let implementation_manifest = load_manifest(path)?;
        validate_loaded_manifest(&implementation_manifest, &mut diagnostics);
        get_str(&implementation_manifest.value, &["binding"])
            .unwrap_or("unknown")
            .to_string()
    } else {
        "not-supplied".to_string()
    };

    let checks: Vec<JsonValue> = diagnostics
        .iter()
        .map(|diagnostic| {
            json!({
                "id": diagnostic.check,
                "category": diagnostic_category(&diagnostic.check),
                "result": if diagnostic.severity == Severity::Error { "fail" } else { "skipped" },
                "evidence": diagnostic.path,
                "note": diagnostic.message,
            })
        })
        .collect();

    let result = if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        "fail"
    } else if implementation.is_none() || has_empty_verification_category(&manifest.value) {
        "partial"
    } else {
        "pass"
    };

    Ok(json!({
        "spec": "rms/conformance/v0.1",
        "subject": {
            "module": get_str(&manifest.value, &["module", "name"]).unwrap_or("unknown"),
            "version": get_str(&manifest.value, &["module", "version"]).unwrap_or("unknown"),
            "implementation": implementation_name,
        },
        "source": {
            "revision": source_revision(module.parent().unwrap_or_else(|| Path::new(".")))
                .unwrap_or_else(|| "unknown".to_string()),
        },
        "profiles": get_string_array(&manifest.value, &["profiles"]),
        "validator": {
            "name": VALIDATOR_NAME,
            "version": VALIDATOR_VERSION,
        },
        "result": result,
        "checks": if checks.is_empty() {
            vec![json!({
                "id": "manifest.core",
                "category": "manifest",
                "result": "pass",
                "evidence": module.display().to_string(),
            })]
        } else {
            checks
        },
    }))
}

fn diagnostic_category(check: &str) -> &'static str {
    match check.split('.').next().unwrap_or("other") {
        "schema" | "manifest" | "module" | "context-map" | "implementation" => "manifest",
        "references" => {
            if check.contains("contract") {
                "contracts"
            } else if check.contains("verification") || check.contains("invariant") {
                "laws"
            } else {
                "other"
            }
        }
        "profiles" | "profile" => "profiles",
        "compatibility" => "compatibility",
        "effects" => "effects",
        "security" => "security",
        "contracts" => "contracts",
        "dependencies" => "dependencies",
        "operations" => "operations",
        "ownership" => "ownership",
        _ => "other",
    }
}

fn run_verify(implementation: &Path, dry_run: bool) -> Result<()> {
    let manifest = load_manifest(implementation)?;
    let command = get_str(&manifest.value, &["commands", "verify"])
        .ok_or_else(|| anyhow!("implementation binding does not declare `commands.verify`"))?;
    let root = implementation.parent().unwrap_or_else(|| Path::new("."));

    if dry_run {
        println!("{command}");
        return Ok(());
    }

    let status = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(root)
        .status()
        .with_context(|| format!("failed to run verify command `{command}`"))?;

    if !status.success() {
        bail!("verify command failed with status {status}");
    }

    Ok(())
}

fn run_package(module: &Path, output: Option<&Path>, force: bool) -> Result<()> {
    let package = package_module(module, output, force)?;
    println!("packaged RMS module at {}", package.output.display());
    Ok(())
}

#[derive(Clone, Debug)]
struct PackageBuildResult {
    output: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
struct PackageManifest {
    spec: &'static str,
    module: String,
    version: String,
    source: PackageSource,
    validator: PackageValidator,
    files: Vec<PackageFile>,
}

#[derive(Clone, Debug, Serialize)]
struct PackageSource {
    revision: String,
}

#[derive(Clone, Debug, Serialize)]
struct PackageValidator {
    name: &'static str,
    version: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PackageFile {
    path: String,
    bytes: u64,
    sha256: String,
}

#[derive(Clone, Debug, Serialize)]
struct VerifyPackageReport {
    result: VerifyPackageResult,
    package: String,
    findings: Vec<VerifyPackageFinding>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum VerifyPackageResult {
    Pass,
    Fail,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum VerifyPackageStatus {
    Pass,
    Fail,
}

#[derive(Clone, Debug, Serialize)]
struct VerifyPackageFinding {
    status: VerifyPackageStatus,
    check: String,
    path: Option<String>,
    message: String,
}

fn package_module(
    module_path: &Path,
    output: Option<&Path>,
    force: bool,
) -> Result<PackageBuildResult> {
    let module = load_manifest(module_path)?;
    if get_str(&module.value, &["spec"]) != Some("rms/module/v0.1") {
        bail!("`{}` is not an RMS module manifest", module_path.display());
    }

    let mut diagnostics = Vec::new();
    validate_loaded_manifest(&module, &mut diagnostics);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        bail!(
            "module has validation errors; run `rms validate --module {}`",
            module_path.display()
        );
    }

    let module_base = module.path.parent().unwrap_or_else(|| Path::new("."));
    let module_name = get_str(&module.value, &["module", "name"]).unwrap_or("module");
    let module_version = get_str(&module.value, &["module", "version"]).unwrap_or("0.0.0");
    let output = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| default_package_output(module_name, module_version));

    prepare_package_output(&output, force)?;

    let mut sources = BTreeSet::new();
    sources.insert(module.path.clone());
    for reference in package_referenced_paths(&module.value) {
        sources.insert(module_base.join(reference));
    }

    let implementation = sibling_implementation_manifest(module_base, module_name)?;
    if let Some(implementation) = &implementation {
        validate_loaded_manifest(implementation, &mut diagnostics);
        if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
        {
            bail!(
                "implementation has validation errors; run `rms validate --implementation {}`",
                implementation.path.display()
            );
        }
        sources.insert(implementation.path.clone());
    }

    for source in &sources {
        copy_package_source(module_base, source, &output)?;
    }

    let conformance = build_conformance_report(
        &module.path,
        implementation
            .as_ref()
            .map(|manifest| manifest.path.as_path()),
    )?;
    let conformance_path = output.join("conformance-report.json");
    fs::write(
        &conformance_path,
        serde_json::to_string_pretty(&conformance)?,
    )
    .with_context(|| format!("failed to write `{}`", conformance_path.display()))?;

    let file_entries = package_file_entries(&output)?;
    let manifest = PackageManifest {
        spec: "rms/package/v0.1",
        module: module_name.to_string(),
        version: module_version.to_string(),
        source: PackageSource {
            revision: source_revision(module_base).unwrap_or_else(|| "unknown".to_string()),
        },
        validator: PackageValidator {
            name: VALIDATOR_NAME,
            version: VALIDATOR_VERSION,
        },
        files: file_entries.clone(),
    };

    let package_manifest_path = output.join("PACKAGE.json");
    fs::write(
        &package_manifest_path,
        serde_json::to_string_pretty(&manifest)?,
    )
    .with_context(|| format!("failed to write `{}`", package_manifest_path.display()))?;

    Ok(PackageBuildResult { output })
}

fn default_package_output(module_name: &str, module_version: &str) -> PathBuf {
    PathBuf::from("dist").join(format!(
        "{}-{}.rms",
        sanitize_package_component(module_name),
        sanitize_package_component(module_version)
    ))
}

fn sanitize_package_component(value: &str) -> String {
    let mut output = String::new();
    let mut previous_dash = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() || character == '_' || character == '.' {
            output.push(character);
            previous_dash = false;
        } else if !previous_dash {
            output.push('-');
            previous_dash = true;
        }
    }
    let output = output.trim_matches('-').to_string();
    if output.is_empty() {
        "module".to_string()
    } else {
        output
    }
}

fn prepare_package_output(output: &Path, force: bool) -> Result<()> {
    if output.exists() {
        if !force {
            bail!(
                "output package directory `{}` already exists; use --force to replace it",
                output.display()
            );
        }
        fs::remove_dir_all(output)
            .with_context(|| format!("failed to remove `{}`", output.display()))?;
    }
    fs::create_dir_all(output)
        .with_context(|| format!("failed to create package directory `{}`", output.display()))
}

fn sibling_implementation_manifest(
    module_base: &Path,
    module_name: &str,
) -> Result<Option<LoadedManifest>> {
    let path = module_base.join("implementation.yaml");
    if !path.exists() {
        return Ok(None);
    }
    let manifest = load_manifest(&path)?;
    if get_str(&manifest.value, &["module"]) == Some(module_name) {
        Ok(Some(manifest))
    } else {
        Ok(None)
    }
}

fn package_referenced_paths(value: &YamlValue) -> BTreeSet<String> {
    let mut paths = referenced_paths(value);

    for path in [
        get_str(value, &["state", "model"]),
        get_str(value, &["state", "migration_policy"]),
    ]
    .into_iter()
    .flatten()
    {
        if looks_like_path(path) {
            paths.insert(path.to_string());
        }
    }

    for path in package_string_array_refs(value, &["operations", "runtime_checks"]) {
        paths.insert(path);
    }
    for path in package_string_array_refs(value, &["operations", "reconciliation"]) {
        paths.insert(path);
    }
    for path in package_string_array_refs(value, &["operations", "migrations"]) {
        paths.insert(path);
    }
    for path in package_string_array_refs(value, &["operations", "runbooks"]) {
        paths.insert(path);
    }

    paths
}

fn package_string_array_refs(value: &YamlValue, path: &[&str]) -> Vec<String> {
    get_string_array(value, path)
        .into_iter()
        .filter(|value| looks_like_path(value))
        .collect()
}

fn looks_like_path(value: &str) -> bool {
    value.contains('/')
        || value.ends_with(".md")
        || value.ends_with(".yaml")
        || value.ends_with(".json")
}

fn copy_package_source(module_base: &Path, source: &Path, output: &Path) -> Result<()> {
    let source = source
        .canonicalize()
        .with_context(|| format!("package source does not exist: `{}`", source.display()))?;
    let module_base = module_base
        .canonicalize()
        .with_context(|| format!("failed to canonicalize `{}`", module_base.display()))?;
    if !source.starts_with(&module_base) {
        bail!(
            "refusing to package file outside module directory: `{}`",
            source.display()
        );
    }
    if source.is_dir() {
        for entry in WalkDir::new(&source)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            copy_package_file(&module_base, entry.path(), output)?;
        }
        return Ok(());
    }
    copy_package_file(&module_base, &source, output)
}

fn copy_package_file(module_base: &Path, source: &Path, output: &Path) -> Result<()> {
    let relative = source
        .strip_prefix(module_base)
        .with_context(|| format!("failed to relativize `{}`", source.display()))?;
    let destination = output.join(relative);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, &destination).with_context(|| {
        format!(
            "failed to copy `{}` to `{}`",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn package_file_entries(output: &Path) -> Result<Vec<PackageFile>> {
    let mut entries = Vec::new();
    for entry in WalkDir::new(output)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some("PACKAGE.json") {
            continue;
        }
        let relative = path
            .strip_prefix(output)
            .unwrap_or(path)
            .components()
            .filter(|component| !matches!(component, Component::CurDir))
            .collect::<PathBuf>()
            .display()
            .to_string();
        entries.push(PackageFile {
            path: relative,
            bytes: fs::metadata(path)?.len(),
            sha256: sha256_file(path)?,
        });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("failed to read `{}`", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn run_verify_package(package: &Path, json_output: bool) -> Result<()> {
    let report = verify_package(package)?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_verify_package_report(&report);
    }

    if report.result == VerifyPackageResult::Fail {
        bail!("RMS package verification failed");
    }

    Ok(())
}

fn verify_package(package: &Path) -> Result<VerifyPackageReport> {
    let mut findings = Vec::new();

    if !package.exists() {
        findings.push(verify_package_finding(
            VerifyPackageStatus::Fail,
            "package.directory",
            Some(package),
            "package directory does not exist",
        ));
        return Ok(build_verify_package_report(package, findings));
    }

    if !package.is_dir() {
        findings.push(verify_package_finding(
            VerifyPackageStatus::Fail,
            "package.directory",
            Some(package),
            "package path is not a directory",
        ));
        return Ok(build_verify_package_report(package, findings));
    }

    findings.push(verify_package_finding(
        VerifyPackageStatus::Pass,
        "package.directory",
        Some(package),
        "package directory exists",
    ));

    let package_manifest_path = package.join("PACKAGE.json");
    let package_manifest_source = match fs::read_to_string(&package_manifest_path) {
        Ok(source) => source,
        Err(error) => {
            findings.push(verify_package_finding(
                VerifyPackageStatus::Fail,
                "package.manifest.read",
                Some(&package_manifest_path),
                format!("failed to read PACKAGE.json: {error}"),
            ));
            return Ok(build_verify_package_report(package, findings));
        }
    };
    let package_manifest: JsonValue = match serde_json::from_str(&package_manifest_source) {
        Ok(value) => value,
        Err(error) => {
            findings.push(verify_package_finding(
                VerifyPackageStatus::Fail,
                "package.manifest.parse",
                Some(&package_manifest_path),
                format!("failed to parse PACKAGE.json: {error}"),
            ));
            return Ok(build_verify_package_report(package, findings));
        }
    };

    verify_package_metadata(&package_manifest, &package_manifest_path, &mut findings);

    let declared_files = verify_package_declared_files(
        package,
        &package_manifest,
        &package_manifest_path,
        &mut findings,
    )?;
    verify_package_actual_files(package, &declared_files, &mut findings)?;
    verify_package_manifests(package, &package_manifest, &declared_files, &mut findings)?;

    Ok(build_verify_package_report(package, findings))
}

fn verify_package_metadata(
    manifest: &JsonValue,
    manifest_path: &Path,
    findings: &mut Vec<VerifyPackageFinding>,
) {
    verify_package_string_field(
        manifest,
        manifest_path,
        findings,
        "package.spec",
        "spec",
        Some("rms/package/v0.1"),
    );
    verify_package_string_field(
        manifest,
        manifest_path,
        findings,
        "package.module",
        "module",
        None,
    );
    verify_package_string_field(
        manifest,
        manifest_path,
        findings,
        "package.version",
        "version",
        None,
    );
    verify_package_string_field(
        manifest,
        manifest_path,
        findings,
        "package.source.revision",
        "source.revision",
        None,
    );
    verify_package_string_field(
        manifest,
        manifest_path,
        findings,
        "package.validator.name",
        "validator.name",
        Some(VALIDATOR_NAME),
    );
    verify_package_string_field(
        manifest,
        manifest_path,
        findings,
        "package.validator.version",
        "validator.version",
        None,
    );
}

fn verify_package_string_field(
    manifest: &JsonValue,
    manifest_path: &Path,
    findings: &mut Vec<VerifyPackageFinding>,
    check: &str,
    field: &str,
    expected: Option<&str>,
) {
    let value = package_json_string(manifest, field);
    match (value, expected) {
        (Some(actual), Some(expected)) if actual == expected => {
            findings.push(verify_package_finding(
                VerifyPackageStatus::Pass,
                check,
                Some(manifest_path),
                format!("`{field}` is `{expected}`"),
            ))
        }
        (Some(actual), Some(expected)) => findings.push(verify_package_finding(
            VerifyPackageStatus::Fail,
            check,
            Some(manifest_path),
            format!("`{field}` must be `{expected}`, got `{actual}`"),
        )),
        (Some(_), None) => findings.push(verify_package_finding(
            VerifyPackageStatus::Pass,
            check,
            Some(manifest_path),
            format!("`{field}` is present"),
        )),
        (None, _) => findings.push(verify_package_finding(
            VerifyPackageStatus::Fail,
            check,
            Some(manifest_path),
            format!("missing required string `{field}`"),
        )),
    }
}

fn verify_package_declared_files(
    package: &Path,
    manifest: &JsonValue,
    manifest_path: &Path,
    findings: &mut Vec<VerifyPackageFinding>,
) -> Result<BTreeMap<String, PackageFile>> {
    let mut declared_files = BTreeMap::new();
    let Some(files) = manifest.get("files").and_then(JsonValue::as_array) else {
        findings.push(verify_package_finding(
            VerifyPackageStatus::Fail,
            "package.files",
            Some(manifest_path),
            "`files` must be an array",
        ));
        return Ok(declared_files);
    };

    if files.is_empty() {
        findings.push(verify_package_finding(
            VerifyPackageStatus::Fail,
            "package.files",
            Some(manifest_path),
            "`files` must name at least one payload file",
        ));
        return Ok(declared_files);
    }

    let mut integrity_failed = false;
    for (index, file) in files.iter().enumerate() {
        let check_path = format!("files[{index}]");
        let Some(path) = file.get("path").and_then(JsonValue::as_str) else {
            integrity_failed = true;
            findings.push(verify_package_finding(
                VerifyPackageStatus::Fail,
                "package.file.path",
                Some(manifest_path),
                format!("`{check_path}.path` must be a relative path string"),
            ));
            continue;
        };
        let Some(relative_path) = package_relative_path(path) else {
            integrity_failed = true;
            findings.push(verify_package_finding(
                VerifyPackageStatus::Fail,
                "package.file.path",
                Some(manifest_path),
                format!("`{path}` must not be absolute, empty, or contain `.`/`..` components"),
            ));
            continue;
        };
        let normalized_path = package_path_string(&relative_path);
        if normalized_path != path {
            integrity_failed = true;
            findings.push(verify_package_finding(
                VerifyPackageStatus::Fail,
                "package.file.path",
                Some(manifest_path),
                format!("`{path}` must be stored in normalized package-relative form"),
            ));
            continue;
        }
        if declared_files.contains_key(&normalized_path) {
            integrity_failed = true;
            findings.push(verify_package_finding(
                VerifyPackageStatus::Fail,
                "package.file.duplicate",
                Some(manifest_path),
                format!("duplicate package file entry `{normalized_path}`"),
            ));
            continue;
        }

        let Some(bytes) = file.get("bytes").and_then(JsonValue::as_u64) else {
            integrity_failed = true;
            findings.push(verify_package_finding(
                VerifyPackageStatus::Fail,
                "package.file.bytes",
                Some(manifest_path),
                format!("`{check_path}.bytes` must be an unsigned integer"),
            ));
            continue;
        };
        let Some(sha256) = file.get("sha256").and_then(JsonValue::as_str) else {
            integrity_failed = true;
            findings.push(verify_package_finding(
                VerifyPackageStatus::Fail,
                "package.file.sha256",
                Some(manifest_path),
                format!("`{check_path}.sha256` must be a string"),
            ));
            continue;
        };
        if sha256.len() != 64
            || !sha256
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            integrity_failed = true;
            findings.push(verify_package_finding(
                VerifyPackageStatus::Fail,
                "package.file.sha256",
                Some(manifest_path),
                format!("`{check_path}.sha256` must be a 64-character hexadecimal digest"),
            ));
            continue;
        }

        let payload_path = package.join(&relative_path);
        match fs::symlink_metadata(&payload_path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                integrity_failed = true;
                findings.push(verify_package_finding(
                    VerifyPackageStatus::Fail,
                    "package.file.symlink",
                    Some(&payload_path),
                    "package payload files must not be symlinks",
                ));
            }
            Ok(metadata) if !metadata.file_type().is_file() => {
                integrity_failed = true;
                findings.push(verify_package_finding(
                    VerifyPackageStatus::Fail,
                    "package.file.kind",
                    Some(&payload_path),
                    "declared package payload path is not a file",
                ));
            }
            Ok(metadata) => {
                if metadata.len() != bytes {
                    integrity_failed = true;
                    findings.push(verify_package_finding(
                        VerifyPackageStatus::Fail,
                        "package.file.bytes",
                        Some(&payload_path),
                        format!(
                            "declared size {bytes} does not match actual size {}",
                            metadata.len()
                        ),
                    ));
                }
                let actual_sha256 = sha256_file(&payload_path)?;
                if actual_sha256 != sha256 {
                    integrity_failed = true;
                    findings.push(verify_package_finding(
                        VerifyPackageStatus::Fail,
                        "package.file.sha256",
                        Some(&payload_path),
                        format!("declared SHA-256 {sha256} does not match actual {actual_sha256}"),
                    ));
                }
            }
            Err(error) => {
                integrity_failed = true;
                findings.push(verify_package_finding(
                    VerifyPackageStatus::Fail,
                    "package.file.exists",
                    Some(&payload_path),
                    format!("declared package payload file is missing or unreadable: {error}"),
                ));
            }
        }

        declared_files.insert(
            normalized_path.clone(),
            PackageFile {
                path: normalized_path,
                bytes,
                sha256: sha256.to_string(),
            },
        );
    }

    if !integrity_failed {
        findings.push(verify_package_finding(
            VerifyPackageStatus::Pass,
            "package.files.integrity",
            Some(manifest_path),
            format!(
                "{} declared payload files match size and SHA-256 metadata",
                declared_files.len()
            ),
        ));
    }

    Ok(declared_files)
}

fn verify_package_actual_files(
    package: &Path,
    declared_files: &BTreeMap<String, PackageFile>,
    findings: &mut Vec<VerifyPackageFinding>,
) -> Result<()> {
    let mut failed = false;
    let mut actual_files = BTreeSet::new();

    for entry in WalkDir::new(package).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                failed = true;
                findings.push(verify_package_finding(
                    VerifyPackageStatus::Fail,
                    "package.files.walk",
                    Some(package),
                    format!("failed to inspect package directory: {error}"),
                ));
                continue;
            }
        };
        if entry.path() == package {
            continue;
        }
        if entry.file_type().is_symlink() {
            failed = true;
            findings.push(verify_package_finding(
                VerifyPackageStatus::Fail,
                "package.file.symlink",
                Some(entry.path()),
                "package contents must not contain symlinks",
            ));
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.file_name().to_str() == Some("PACKAGE.json") {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(package)
            .with_context(|| format!("failed to relativize `{}`", entry.path().display()))?;
        let path = package_path_string(relative);
        actual_files.insert(path.clone());
        if !declared_files.contains_key(&path) {
            failed = true;
            findings.push(verify_package_finding(
                VerifyPackageStatus::Fail,
                "package.file.undeclared",
                Some(entry.path()),
                format!("payload file `{path}` is not declared in PACKAGE.json"),
            ));
        }
    }

    for path in declared_files.keys() {
        if !actual_files.contains(path) {
            failed = true;
            findings.push(verify_package_finding(
                VerifyPackageStatus::Fail,
                "package.file.missing",
                Some(package.join(path)),
                format!("declared payload file `{path}` is missing"),
            ));
        }
    }

    if !failed {
        findings.push(verify_package_finding(
            VerifyPackageStatus::Pass,
            "package.files.closed",
            Some(package),
            "all payload files are declared and no undeclared payload files are present",
        ));
    }

    Ok(())
}

fn verify_package_manifests(
    package: &Path,
    package_manifest: &JsonValue,
    declared_files: &BTreeMap<String, PackageFile>,
    findings: &mut Vec<VerifyPackageFinding>,
) -> Result<()> {
    if declared_files.contains_key("PACKAGE.json") {
        findings.push(verify_package_finding(
            VerifyPackageStatus::Fail,
            "package.manifest.payload",
            Some(package.join("PACKAGE.json")),
            "PACKAGE.json must not be listed as a payload file",
        ));
    }

    if !declared_files.contains_key("conformance-report.json") {
        findings.push(verify_package_finding(
            VerifyPackageStatus::Fail,
            "package.conformance.present",
            Some(package.join("conformance-report.json")),
            "package must declare conformance-report.json",
        ));
    }

    let targets = discover_targets(package, vec![], vec![], vec![], vec![], vec![], vec![])?;
    let mut diagnostics = Vec::new();
    let mut module_manifests = Vec::new();
    let mut conformance_found = false;

    for target in targets {
        match load_manifest(&target) {
            Ok(manifest) => {
                match get_str(&manifest.value, &["spec"]) {
                    Some("rms/module/v0.1") => module_manifests.push(manifest.path.clone()),
                    Some("rms/conformance/v0.1") => conformance_found = true,
                    _ => {}
                }
                validate_package_embedded_manifest(&manifest, &mut diagnostics);
            }
            Err(error) => diagnostics.push(Diagnostic {
                severity: Severity::Error,
                check: "manifest.parse".to_string(),
                path: target.display().to_string(),
                message: error.to_string(),
            }),
        }
    }

    if module_manifests.is_empty() {
        findings.push(verify_package_finding(
            VerifyPackageStatus::Fail,
            "package.module-manifest.present",
            Some(package),
            "package must contain one RMS module manifest",
        ));
    } else if module_manifests.len() > 1 {
        findings.push(verify_package_finding(
            VerifyPackageStatus::Fail,
            "package.module-manifest.count",
            Some(package),
            format!(
                "package must contain exactly one RMS module manifest, found {}",
                module_manifests.len()
            ),
        ));
    } else {
        let module_manifest = load_manifest(&module_manifests[0])?;
        verify_package_module_identity(&module_manifest, package_manifest, findings);
    }

    if conformance_found {
        findings.push(verify_package_finding(
            VerifyPackageStatus::Pass,
            "package.conformance.present",
            Some(package.join("conformance-report.json")),
            "package contains a conformance report",
        ));
    } else {
        findings.push(verify_package_finding(
            VerifyPackageStatus::Fail,
            "package.conformance.present",
            Some(package.join("conformance-report.json")),
            "package must contain conformance-report.json with spec rms/conformance/v0.1",
        ));
    }

    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        for diagnostic in diagnostics
            .into_iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
        {
            findings.push(VerifyPackageFinding {
                status: VerifyPackageStatus::Fail,
                check: diagnostic.check,
                path: Some(diagnostic.path),
                message: diagnostic.message,
            });
        }
    } else {
        findings.push(verify_package_finding(
            VerifyPackageStatus::Pass,
            "package.manifests.validate",
            Some(package),
            "included RMS manifests validate",
        ));
    }

    Ok(())
}

fn validate_package_embedded_manifest(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if get_str(&manifest.value, &["spec"]) == Some("rms/implementation/v0.1") {
        validate_against_embedded_schema(manifest, diagnostics);
        scan_for_secret_like_keys(&manifest.value, &manifest.path, diagnostics);
    } else {
        validate_loaded_manifest(manifest, diagnostics);
    }
}

fn verify_package_module_identity(
    module_manifest: &LoadedManifest,
    package_manifest: &JsonValue,
    findings: &mut Vec<VerifyPackageFinding>,
) {
    let package_module = package_json_string(package_manifest, "module");
    let package_version = package_json_string(package_manifest, "version");
    let module_name = get_str(&module_manifest.value, &["module", "name"]);
    let module_version = get_str(&module_manifest.value, &["module", "version"]);

    match (package_module, module_name) {
        (Some(package_module), Some(module_name)) if package_module == module_name => {
            findings.push(verify_package_finding(
                VerifyPackageStatus::Pass,
                "package.module.identity",
                Some(&module_manifest.path),
                format!("package module identity matches `{module_name}`"),
            ));
        }
        (Some(package_module), Some(module_name)) => {
            findings.push(verify_package_finding(
                VerifyPackageStatus::Fail,
                "package.module.identity",
                Some(&module_manifest.path),
                format!("PACKAGE.json declares module `{package_module}` but module manifest declares `{module_name}`"),
            ));
        }
        _ => {}
    }

    match (package_version, module_version) {
        (Some(package_version), Some(module_version)) if package_version == module_version => {
            findings.push(verify_package_finding(
                VerifyPackageStatus::Pass,
                "package.module.version",
                Some(&module_manifest.path),
                format!("package version matches `{module_version}`"),
            ));
        }
        (Some(package_version), Some(module_version)) => {
            findings.push(verify_package_finding(
                VerifyPackageStatus::Fail,
                "package.module.version",
                Some(&module_manifest.path),
                format!("PACKAGE.json declares version `{package_version}` but module manifest declares `{module_version}`"),
            ));
        }
        _ => {}
    }
}

fn package_json_string<'a>(manifest: &'a JsonValue, field: &str) -> Option<&'a str> {
    let mut current = manifest;
    for segment in field.split('.') {
        current = current.get(segment)?;
    }
    current.as_str()
}

fn package_relative_path(path: &str) -> Option<PathBuf> {
    if path.is_empty() {
        return None;
    }
    let path = Path::new(path);
    if path.is_absolute() {
        return None;
    }
    let mut output = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => output.push(segment),
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return None;
            }
        }
    }
    if output.as_os_str().is_empty() {
        None
    } else {
        Some(output)
    }
}

fn package_path_string(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .components()
        .filter_map(|component| match component {
            Component::Normal(segment) => Some(segment.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn verify_package_finding(
    status: VerifyPackageStatus,
    check: impl Into<String>,
    path: Option<impl AsRef<Path>>,
    message: impl Into<String>,
) -> VerifyPackageFinding {
    VerifyPackageFinding {
        status,
        check: check.into(),
        path: path.map(|path| path.as_ref().display().to_string()),
        message: message.into(),
    }
}

fn build_verify_package_report(
    package: &Path,
    findings: Vec<VerifyPackageFinding>,
) -> VerifyPackageReport {
    let result = if findings
        .iter()
        .any(|finding| finding.status == VerifyPackageStatus::Fail)
    {
        VerifyPackageResult::Fail
    } else {
        VerifyPackageResult::Pass
    };
    VerifyPackageReport {
        result,
        package: package.display().to_string(),
        findings,
    }
}

fn print_verify_package_report(report: &VerifyPackageReport) {
    match report.result {
        VerifyPackageResult::Pass => println!("pass: RMS package verified {}", report.package),
        VerifyPackageResult::Fail => {
            println!("fail: RMS package verification failed {}", report.package)
        }
    }

    for finding in &report.findings {
        if report.result == VerifyPackageResult::Pass
            && finding.status != VerifyPackageStatus::Fail
            && !matches!(
                finding.check.as_str(),
                "package.files.integrity"
                    | "package.files.closed"
                    | "package.module.identity"
                    | "package.module.version"
                    | "package.manifests.validate"
            )
        {
            continue;
        }
        let label = match finding.status {
            VerifyPackageStatus::Pass => "pass",
            VerifyPackageStatus::Fail => "fail",
        };
        if let Some(path) = &finding.path {
            println!("{label} [{}] {path}: {}", finding.check, finding.message);
        } else {
            println!("{label} [{}]: {}", finding.check, finding.message);
        }
    }
}

fn run_check_compat(old: &Path, new: &Path, json_output: bool) -> Result<()> {
    let old_manifest = load_manifest(old)?;
    let new_manifest = load_manifest(new)?;
    let report = check_module_compat(&old_manifest, &new_manifest)?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_compat_report(&report);
    }

    if report.result == CompatResult::Breaking {
        bail!("RMS compatibility check failed");
    }

    Ok(())
}

fn run_compose(root: &Path, json_output: bool) -> Result<()> {
    let report = compose_system(root)?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_compose_report(&report);
    }

    if report.result == ComposeResult::Fail {
        bail!("RMS composition check failed");
    }

    Ok(())
}

#[derive(Clone, Debug, Serialize)]
struct ComposeReport {
    result: ComposeResult,
    root: String,
    modules: Vec<String>,
    findings: Vec<ComposeFinding>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ComposeResult {
    Pass,
    ReviewRequired,
    Fail,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ComposeStatus {
    Satisfied,
    NotApplicable,
    ReviewRequired,
    Unresolved,
    Incompatible,
}

#[derive(Clone, Debug, Serialize)]
struct ComposeFinding {
    status: ComposeStatus,
    check: String,
    consumer: Option<String>,
    provider: Option<String>,
    requirement: Option<String>,
    message: String,
}

#[derive(Clone, Debug)]
struct ModuleIndexEntry {
    name: String,
    value: YamlValue,
}

#[derive(Clone, Debug)]
struct ProvidedRequirement {
    module: String,
    group: String,
    contract: Option<String>,
}

fn compose_system(root: &Path) -> Result<ComposeReport> {
    let targets = discover_targets(root, vec![], vec![], vec![], vec![], vec![], vec![])?;
    let mut modules = BTreeMap::new();
    let mut systems = Vec::new();
    let mut context_maps = Vec::new();
    let mut findings = Vec::new();

    for target in targets {
        let manifest = match load_manifest(&target) {
            Ok(manifest) => manifest,
            Err(load_error) => {
                findings.push(compose_finding(
                    ComposeStatus::Incompatible,
                    "manifest.parse",
                    None,
                    None,
                    None,
                    format!("failed to load `{}`: {load_error}", target.display()),
                ));
                continue;
            }
        };

        match get_str(&manifest.value, &["spec"]) {
            Some("rms/module/v0.1") => {
                let name = get_str(&manifest.value, &["module", "name"])
                    .unwrap_or("<unknown>")
                    .to_string();
                if modules.contains_key(&name) {
                    findings.push(compose_finding(
                        ComposeStatus::Incompatible,
                        "module.name.unique",
                        Some(name.clone()),
                        None,
                        None,
                        format!("duplicate module name `{name}`"),
                    ));
                }
                modules.insert(
                    name.clone(),
                    ModuleIndexEntry {
                        name,
                        value: manifest.value,
                    },
                );
            }
            Some("rms/system/v0.1") => systems.push(manifest),
            Some("rms/context-map/v0.1") => context_maps.push(manifest),
            _ => {}
        }
    }

    let context_map = context_maps.first();
    let external_dependencies = systems
        .iter()
        .flat_map(|system| get_string_array(&system.value, &["external_dependencies"]))
        .collect::<BTreeSet<_>>();
    let provided_requirements = provided_requirement_index(&modules);

    if modules.is_empty() {
        findings.push(compose_finding(
            ComposeStatus::Unresolved,
            "modules.discovered",
            None,
            None,
            None,
            "no RMS module manifests were discovered",
        ));
    }

    for module in modules.values() {
        compose_required_modules(module, &modules, context_map, &mut findings);
        compose_required_capabilities(
            module,
            &provided_requirements,
            &external_dependencies,
            &mut findings,
        );
    }
    compose_module_cycles(&modules, &mut findings);

    let result = compose_result(&findings);
    Ok(ComposeReport {
        result,
        root: root.display().to_string(),
        modules: modules.keys().cloned().collect(),
        findings,
    })
}

fn provided_requirement_index(
    modules: &BTreeMap<String, ModuleIndexEntry>,
) -> BTreeMap<String, Vec<ProvidedRequirement>> {
    let mut index: BTreeMap<String, Vec<ProvidedRequirement>> = BTreeMap::new();
    for module in modules.values() {
        let Some(provides) = get_path(&module.value, &["provides"]).and_then(YamlValue::as_mapping)
        else {
            continue;
        };
        for (group, entries) in provides {
            let Some(group) = group.as_str() else {
                continue;
            };
            let Some(entries) = entries.as_sequence() else {
                continue;
            };
            for entry in entries {
                if let Some((name, contract)) = named_reference(entry) {
                    index.entry(name).or_default().push(ProvidedRequirement {
                        module: module.name.clone(),
                        group: group.to_string(),
                        contract,
                    });
                }
            }
        }
    }
    index
}

fn compose_required_modules(
    module: &ModuleIndexEntry,
    modules: &BTreeMap<String, ModuleIndexEntry>,
    context_map: Option<&LoadedManifest>,
    findings: &mut Vec<ComposeFinding>,
) {
    let required_modules = named_contract_map(get_path(&module.value, &["requires", "modules"]));
    if required_modules.is_empty() {
        findings.push(compose_finding(
            ComposeStatus::NotApplicable,
            "requires.modules",
            Some(module.name.clone()),
            None,
            None,
            "module declares no required modules",
        ));
        return;
    }

    for (required_name, required_contract) in required_modules {
        let Some(provider) = modules.get(&required_name) else {
            findings.push(compose_finding(
                ComposeStatus::Unresolved,
                "requires.modules.provider",
                Some(module.name.clone()),
                Some(required_name.clone()),
                Some(required_name.clone()),
                format!("required module `{required_name}` was not found"),
            ));
            continue;
        };

        if let Some(contract) = required_contract.as_deref() {
            let provider_contracts = public_contract_refs(&provider.value);
            if !provider_contracts.contains(contract) {
                findings.push(compose_finding(
                    ComposeStatus::Incompatible,
                    "requires.modules.contract",
                    Some(module.name.clone()),
                    Some(required_name.clone()),
                    Some(contract.to_string()),
                    format!(
                        "required module `{required_name}` does not publish contract `{contract}`"
                    ),
                ));
                continue;
            }
        }

        check_context_relationship(&module.name, &required_name, context_map, findings);
        findings.push(compose_finding(
            ComposeStatus::Satisfied,
            "requires.modules.provider",
            Some(module.name.clone()),
            Some(required_name.clone()),
            Some(required_name),
            "required module is present",
        ));
    }
}

fn compose_required_capabilities(
    module: &ModuleIndexEntry,
    provided_requirements: &BTreeMap<String, Vec<ProvidedRequirement>>,
    external_dependencies: &BTreeSet<String>,
    findings: &mut Vec<ComposeFinding>,
) {
    let required_capabilities =
        named_contract_map(get_path(&module.value, &["requires", "capabilities"]));
    if required_capabilities.is_empty() {
        findings.push(compose_finding(
            ComposeStatus::NotApplicable,
            "requires.capabilities",
            Some(module.name.clone()),
            None,
            None,
            "module declares no required capabilities",
        ));
        return;
    }

    for (required_name, required_contract) in required_capabilities {
        if let Some(providers) = provided_requirements.get(&required_name) {
            let compatible = providers.iter().find(|provider| {
                required_contract.is_none() || provider.contract == required_contract
            });
            if let Some(provider) = compatible {
                findings.push(compose_finding(
                    ComposeStatus::Satisfied,
                    "requires.capabilities.provider",
                    Some(module.name.clone()),
                    Some(provider.module.clone()),
                    Some(required_name.clone()),
                    format!(
                        "required capability `{required_name}` is satisfied by module `{}` public {}",
                        provider.module, provider.group
                    ),
                ));
            } else {
                findings.push(compose_finding(
                    ComposeStatus::Incompatible,
                    "requires.capabilities.contract",
                    Some(module.name.clone()),
                    None,
                    Some(required_name.clone()),
                    format!(
                        "required capability `{required_name}` exists but no provider publishes the requested contract `{}`",
                        required_contract.as_deref().unwrap_or("<none>")
                    ),
                ));
            }
            continue;
        }

        if external_dependencies.contains(&required_name) {
            findings.push(compose_finding(
                ComposeStatus::Satisfied,
                "requires.capabilities.external",
                Some(module.name.clone()),
                Some(required_name.clone()),
                Some(required_name.clone()),
                format!("required capability `{required_name}` is listed as a system external dependency"),
            ));
            check_external_capability_effect(module, &required_name, findings);
        } else {
            findings.push(compose_finding(
                ComposeStatus::Unresolved,
                "requires.capabilities.provider",
                Some(module.name.clone()),
                None,
                Some(required_name.clone()),
                format!(
                    "required capability `{required_name}` is not provided by a module or listed in system external dependencies"
                ),
            ));
        }
    }
}

fn check_external_capability_effect(
    module: &ModuleIndexEntry,
    required_name: &str,
    findings: &mut Vec<ComposeFinding>,
) {
    let has_effect = get_path(&module.value, &["effects"])
        .and_then(YamlValue::as_sequence)
        .into_iter()
        .flatten()
        .any(|effect| {
            get_str(effect, &["name"]) == Some(required_name)
                || get_str(effect, &["capability"]) == Some(required_name)
        });
    if !has_effect {
        findings.push(compose_finding(
            ComposeStatus::ReviewRequired,
            "effects.external-capability",
            Some(module.name.clone()),
            Some(required_name.to_string()),
            Some(required_name.to_string()),
            format!(
                "external capability `{required_name}` is required but no matching effect is declared"
            ),
        ));
    }
}

fn check_context_relationship(
    consumer: &str,
    provider: &str,
    context_map: Option<&LoadedManifest>,
    findings: &mut Vec<ComposeFinding>,
) {
    let Some(context_map) = context_map else {
        return;
    };
    if get_path(&context_map.value, &["contexts", consumer]).is_none()
        || get_path(&context_map.value, &["contexts", provider]).is_none()
    {
        return;
    }
    if context_relationship_exists(&context_map.value, consumer, provider) {
        findings.push(compose_finding(
            ComposeStatus::Satisfied,
            "context-map.relationship",
            Some(consumer.to_string()),
            Some(provider.to_string()),
            None,
            "context map declares a relationship between consumer and provider",
        ));
    } else {
        findings.push(compose_finding(
            ComposeStatus::ReviewRequired,
            "context-map.relationship",
            Some(consumer.to_string()),
            Some(provider.to_string()),
            None,
            "both modules are named contexts, but the context map does not declare their relationship",
        ));
    }
}

fn context_relationship_exists(value: &YamlValue, left: &str, right: &str) -> bool {
    let Some(relationships) = get_path(value, &["relationships"]).and_then(YamlValue::as_sequence)
    else {
        return false;
    };
    relationships.iter().any(|relationship| {
        let upstream = get_str(relationship, &["upstream"]);
        let downstream = get_str(relationship, &["downstream"]);
        matches!(
            (upstream, downstream),
            (Some(up), Some(down)) if (up == left && down == right) || (up == right && down == left)
        )
    })
}

fn compose_module_cycles(
    modules: &BTreeMap<String, ModuleIndexEntry>,
    findings: &mut Vec<ComposeFinding>,
) {
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for module in modules.values() {
        let required_modules =
            named_contract_map(get_path(&module.value, &["requires", "modules"]));
        edges.insert(
            module.name.clone(),
            required_modules
                .keys()
                .filter(|name| modules.contains_key(*name))
                .cloned()
                .collect(),
        );
    }

    for module in modules.keys() {
        let mut path = Vec::new();
        let mut visiting = BTreeSet::new();
        if let Some(cycle) = find_module_cycle(module, module, &edges, &mut visiting, &mut path) {
            findings.push(compose_finding(
                ComposeStatus::Incompatible,
                "requires.modules.cycle",
                Some(module.clone()),
                None,
                None,
                format!("module dependency cycle detected: {}", cycle.join(" -> ")),
            ));
            return;
        }
    }
}

fn find_module_cycle(
    start: &str,
    current: &str,
    edges: &BTreeMap<String, BTreeSet<String>>,
    visiting: &mut BTreeSet<String>,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    if !visiting.insert(current.to_string()) {
        return None;
    }
    path.push(current.to_string());

    for next in edges.get(current).into_iter().flatten() {
        if next == start {
            let mut cycle = path.clone();
            cycle.push(start.to_string());
            return Some(cycle);
        }
        if !path.iter().any(|item| item == next) {
            if let Some(cycle) = find_module_cycle(start, next, edges, visiting, path) {
                return Some(cycle);
            }
        }
    }

    path.pop();
    visiting.remove(current);
    None
}

fn public_contract_refs(value: &YamlValue) -> BTreeSet<String> {
    let mut contracts = BTreeSet::new();
    let Some(provides) = get_path(value, &["provides"]).and_then(YamlValue::as_mapping) else {
        return contracts;
    };
    for entries in provides.values().filter_map(YamlValue::as_sequence) {
        for entry in entries {
            if let Some((_, Some(contract))) = named_reference(entry) {
                contracts.insert(contract);
            }
        }
    }
    contracts
}

fn compose_result(findings: &[ComposeFinding]) -> ComposeResult {
    if findings.iter().any(|finding| {
        matches!(
            finding.status,
            ComposeStatus::Unresolved | ComposeStatus::Incompatible
        )
    }) {
        ComposeResult::Fail
    } else if findings
        .iter()
        .any(|finding| finding.status == ComposeStatus::ReviewRequired)
    {
        ComposeResult::ReviewRequired
    } else {
        ComposeResult::Pass
    }
}

fn compose_finding(
    status: ComposeStatus,
    check: impl Into<String>,
    consumer: Option<String>,
    provider: Option<String>,
    requirement: Option<String>,
    message: impl Into<String>,
) -> ComposeFinding {
    ComposeFinding {
        status,
        check: check.into(),
        consumer,
        provider,
        requirement,
        message: message.into(),
    }
}

fn compose_status_label(status: ComposeStatus) -> &'static str {
    match status {
        ComposeStatus::Satisfied => "satisfied",
        ComposeStatus::NotApplicable => "not-applicable",
        ComposeStatus::ReviewRequired => "review-required",
        ComposeStatus::Unresolved => "unresolved",
        ComposeStatus::Incompatible => "incompatible",
    }
}

fn compose_result_label(result: ComposeResult) -> &'static str {
    match result {
        ComposeResult::Pass => "pass",
        ComposeResult::ReviewRequired => "review-required",
        ComposeResult::Fail => "fail",
    }
}

fn print_compose_report(report: &ComposeReport) {
    println!("RMS composition: {}", compose_result_label(report.result));
    println!("Root: {}", report.root);
    print_string_list("Modules", &report.modules);
    if report.findings.is_empty() {
        println!("No composition findings.");
        return;
    }
    for finding in &report.findings {
        let consumer = finding.consumer.as_deref().unwrap_or("<none>");
        let provider = finding.provider.as_deref().unwrap_or("<none>");
        let requirement = finding.requirement.as_deref().unwrap_or("<none>");
        println!(
            "- {} [{}] consumer={} provider={} requirement={}: {}",
            compose_status_label(finding.status),
            finding.check,
            consumer,
            provider,
            requirement,
            finding.message
        );
    }
}

#[derive(Clone, Debug, Serialize)]
struct CompatReport {
    result: CompatResult,
    old: String,
    new: String,
    findings: Vec<CompatFinding>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
enum CompatResult {
    Compatible,
    CompatibleAdditive,
    OperationalReviewRequired,
    Breaking,
}

#[derive(Clone, Debug, Serialize)]
struct CompatFinding {
    severity: CompatResult,
    check: String,
    message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PublicSurfaceEntry {
    group: String,
    name: String,
    contract: Option<String>,
}

fn check_module_compat(old: &LoadedManifest, new: &LoadedManifest) -> Result<CompatReport> {
    if get_str(&old.value, &["spec"]) != Some("rms/module/v0.1") {
        bail!("old manifest is not an RMS module manifest");
    }
    if get_str(&new.value, &["spec"]) != Some("rms/module/v0.1") {
        bail!("new manifest is not an RMS module manifest");
    }

    let old_label = module_label(old);
    let new_label = module_label(new);
    let mut findings = Vec::new();

    compare_module_identity(old, new, &mut findings);
    compare_module_version(old, new, &mut findings);
    compare_public_surface(old, new, &mut findings);
    compare_string_set(
        old,
        new,
        &mut findings,
        &["profiles"],
        "profiles",
        "declared profile",
        true,
    );
    compare_effects(old, new, &mut findings);
    compare_required_capabilities(old, new, &mut findings);
    compare_compatibility_policy(old, new, &mut findings);

    let result = findings
        .iter()
        .map(|finding| finding.severity)
        .max()
        .unwrap_or(CompatResult::Compatible);

    Ok(CompatReport {
        result,
        old: old_label,
        new: new_label,
        findings,
    })
}

fn compare_module_identity(
    old: &LoadedManifest,
    new: &LoadedManifest,
    findings: &mut Vec<CompatFinding>,
) {
    let old_name = get_str(&old.value, &["module", "name"]).unwrap_or("<missing>");
    let new_name = get_str(&new.value, &["module", "name"]).unwrap_or("<missing>");
    if old_name != new_name {
        findings.push(compat_finding(
            CompatResult::Breaking,
            "module.name",
            format!("module name changed from `{old_name}` to `{new_name}`"),
        ));
    }
}

fn compare_module_version(
    old: &LoadedManifest,
    new: &LoadedManifest,
    findings: &mut Vec<CompatFinding>,
) {
    let old_version = get_str(&old.value, &["module", "version"]);
    let new_version = get_str(&new.value, &["module", "version"]);
    if let (Some(old_version), Some(new_version)) = (old_version, new_version) {
        if version_cmp(new_version, old_version).is_some_and(|ordering| ordering.is_lt()) {
            findings.push(compat_finding(
                CompatResult::Breaking,
                "module.version",
                format!("module version moved backward from `{old_version}` to `{new_version}`"),
            ));
        }
    }
}

fn compare_public_surface(
    old: &LoadedManifest,
    new: &LoadedManifest,
    findings: &mut Vec<CompatFinding>,
) {
    let old_surface = public_surface_map(&old.value);
    let new_surface = public_surface_map(&new.value);

    for (key, old_entry) in &old_surface {
        let Some(new_entry) = new_surface.get(key) else {
            findings.push(compat_finding(
                CompatResult::Breaking,
                "provides.removed",
                format!("removed public {} `{}`", old_entry.group, old_entry.name),
            ));
            continue;
        };

        if old_entry.contract != new_entry.contract {
            findings.push(compat_finding(
                CompatResult::Breaking,
                "provides.contract",
                format!(
                    "changed contract for public {} `{}` from `{}` to `{}`",
                    old_entry.group,
                    old_entry.name,
                    old_entry.contract.as_deref().unwrap_or("<none>"),
                    new_entry.contract.as_deref().unwrap_or("<none>")
                ),
            ));
        }
    }

    for (key, new_entry) in &new_surface {
        if !old_surface.contains_key(key) {
            findings.push(compat_finding(
                CompatResult::CompatibleAdditive,
                "provides.added",
                format!("added public {} `{}`", new_entry.group, new_entry.name),
            ));
        }
    }
}

fn compare_string_set(
    old: &LoadedManifest,
    new: &LoadedManifest,
    findings: &mut Vec<CompatFinding>,
    path: &[&str],
    check_prefix: &str,
    label: &str,
    removal_requires_review: bool,
) {
    let old_set: BTreeSet<_> = get_string_array(&old.value, path).into_iter().collect();
    let new_set: BTreeSet<_> = get_string_array(&new.value, path).into_iter().collect();

    for removed in old_set.difference(&new_set) {
        findings.push(compat_finding(
            if removal_requires_review {
                CompatResult::OperationalReviewRequired
            } else {
                CompatResult::CompatibleAdditive
            },
            format!("{check_prefix}.removed"),
            format!("removed {label} `{removed}`"),
        ));
    }

    for added in new_set.difference(&old_set) {
        findings.push(compat_finding(
            CompatResult::OperationalReviewRequired,
            format!("{check_prefix}.added"),
            format!("added {label} `{added}`"),
        ));
    }
}

fn compare_effects(old: &LoadedManifest, new: &LoadedManifest, findings: &mut Vec<CompatFinding>) {
    let old_effects = named_kind_map(get_path(&old.value, &["effects"]));
    let new_effects = named_kind_map(get_path(&new.value, &["effects"]));

    for (name, kind) in &old_effects {
        match new_effects.get(name) {
            None => findings.push(compat_finding(
                CompatResult::OperationalReviewRequired,
                "effects.removed",
                format!("removed declared effect `{name}`"),
            )),
            Some(new_kind) if new_kind != kind => findings.push(compat_finding(
                CompatResult::OperationalReviewRequired,
                "effects.kind",
                format!("changed effect `{name}` kind from `{kind}` to `{new_kind}`"),
            )),
            _ => {}
        }
    }

    for (name, kind) in &new_effects {
        if !old_effects.contains_key(name) {
            findings.push(compat_finding(
                CompatResult::OperationalReviewRequired,
                "effects.added",
                format!("added declared effect `{name}` of kind `{kind}`"),
            ));
        }
    }
}

fn compare_required_capabilities(
    old: &LoadedManifest,
    new: &LoadedManifest,
    findings: &mut Vec<CompatFinding>,
) {
    let old_capabilities = named_contract_map(get_path(&old.value, &["requires", "capabilities"]));
    let new_capabilities = named_contract_map(get_path(&new.value, &["requires", "capabilities"]));

    for (name, old_contract) in &old_capabilities {
        match new_capabilities.get(name) {
            None => findings.push(compat_finding(
                CompatResult::OperationalReviewRequired,
                "requires.capabilities.removed",
                format!("removed required capability `{name}`"),
            )),
            Some(new_contract) if new_contract != old_contract => findings.push(compat_finding(
                CompatResult::OperationalReviewRequired,
                "requires.capabilities.contract",
                format!(
                    "changed required capability `{name}` contract from `{}` to `{}`",
                    old_contract.as_deref().unwrap_or("<none>"),
                    new_contract.as_deref().unwrap_or("<none>")
                ),
            )),
            _ => {}
        }
    }

    for (name, contract) in &new_capabilities {
        if !old_capabilities.contains_key(name) {
            findings.push(compat_finding(
                CompatResult::OperationalReviewRequired,
                "requires.capabilities.added",
                format!(
                    "added required capability `{name}` with contract `{}`",
                    contract.as_deref().unwrap_or("<none>")
                ),
            ));
        }
    }
}

fn compare_compatibility_policy(
    old: &LoadedManifest,
    new: &LoadedManifest,
    findings: &mut Vec<CompatFinding>,
) {
    let old_policy = get_str(&old.value, &["compatibility", "policy"]);
    let new_policy = get_str(&new.value, &["compatibility", "policy"]);
    if old_policy != new_policy {
        findings.push(compat_finding(
            CompatResult::OperationalReviewRequired,
            "compatibility.policy",
            format!(
                "changed compatibility policy from `{}` to `{}`",
                old_policy.unwrap_or("<missing>"),
                new_policy.unwrap_or("<missing>")
            ),
        ));
    }
}

fn public_surface_map(value: &YamlValue) -> BTreeMap<String, PublicSurfaceEntry> {
    let mut map = BTreeMap::new();
    let Some(provides) = get_path(value, &["provides"]).and_then(YamlValue::as_mapping) else {
        return map;
    };

    for (group, entries) in provides {
        let Some(group) = group.as_str() else {
            continue;
        };
        let Some(entries) = entries.as_sequence() else {
            continue;
        };
        for entry in entries {
            let Some((name, contract)) = named_reference(entry) else {
                continue;
            };
            let key = format!("{group}:{name}");
            map.insert(
                key,
                PublicSurfaceEntry {
                    group: group.to_string(),
                    name,
                    contract,
                },
            );
        }
    }

    map
}

fn named_contract_map(value: Option<&YamlValue>) -> BTreeMap<String, Option<String>> {
    let mut map = BTreeMap::new();
    let Some(entries) = value.and_then(YamlValue::as_sequence) else {
        return map;
    };
    for entry in entries {
        if let Some((name, contract)) = named_reference(entry) {
            map.insert(name, contract);
        }
    }
    map
}

fn named_kind_map(value: Option<&YamlValue>) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Some(entries) = value.and_then(YamlValue::as_sequence) else {
        return map;
    };
    for entry in entries {
        let Some(mapping) = entry.as_mapping() else {
            continue;
        };
        let Some(name) = mapping
            .get(YamlValue::String("name".to_string()))
            .and_then(YamlValue::as_str)
        else {
            continue;
        };
        let kind = mapping
            .get(YamlValue::String("kind".to_string()))
            .and_then(YamlValue::as_str)
            .unwrap_or("<missing>");
        map.insert(name.to_string(), kind.to_string());
    }
    map
}

fn named_reference(value: &YamlValue) -> Option<(String, Option<String>)> {
    match value {
        YamlValue::String(name) => Some((name.clone(), None)),
        YamlValue::Mapping(mapping) => {
            let name = mapping
                .get(YamlValue::String("name".to_string()))
                .and_then(YamlValue::as_str)?;
            let contract = mapping
                .get(YamlValue::String("contract".to_string()))
                .and_then(YamlValue::as_str)
                .map(ToString::to_string);
            Some((name.to_string(), contract))
        }
        _ => None,
    }
}

fn version_cmp(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let left = parse_version_parts(left)?;
    let right = parse_version_parts(right)?;
    Some(left.cmp(&right))
}

fn parse_version_parts(version: &str) -> Option<Vec<u64>> {
    let core = version.split(['-', '+']).next()?;
    core.split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect()
}

fn module_label(manifest: &LoadedManifest) -> String {
    format!(
        "{}@{}",
        get_str(&manifest.value, &["module", "name"]).unwrap_or("<unknown>"),
        get_str(&manifest.value, &["module", "version"]).unwrap_or("<unknown>")
    )
}

fn compat_finding(
    severity: CompatResult,
    check: impl Into<String>,
    message: impl Into<String>,
) -> CompatFinding {
    CompatFinding {
        severity,
        check: check.into(),
        message: message.into(),
    }
}

fn compat_result_label(result: CompatResult) -> &'static str {
    match result {
        CompatResult::Compatible => "compatible",
        CompatResult::CompatibleAdditive => "compatible-additive",
        CompatResult::OperationalReviewRequired => "operational-review-required",
        CompatResult::Breaking => "breaking",
    }
}

fn print_compat_report(report: &CompatReport) {
    println!("RMS compatibility: {}", compat_result_label(report.result));
    println!("Old: {}", report.old);
    println!("New: {}", report.new);
    if report.findings.is_empty() {
        println!("No compatibility findings.");
        return;
    }
    for finding in &report.findings {
        println!(
            "- {} [{}]: {}",
            compat_result_label(finding.severity),
            finding.check,
            finding.message
        );
    }
}

fn run_init(
    path: &Path,
    name: &str,
    purpose: &str,
    version: &str,
    contexts: &[String],
) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create init directory `{}`", path.display()))?;
    let contexts = if contexts.is_empty() {
        vec![name.to_string()]
    } else {
        contexts.to_vec()
    };

    write_new_file(
        &path.join("system.yaml"),
        &render_system_yaml(name, purpose, version, &contexts),
    )?;
    write_new_file(
        &path.join("context-map.yaml"),
        &render_context_map_yaml(&contexts),
    )?;
    write_new_file(&path.join("GLOSSARY.md"), &render_glossary_md(name))?;
    write_new_file(&path.join("AGENTS.md"), INIT_AGENTS_MD)?;
    write_new_file(&path.join(".gitignore"), INIT_GITIGNORE)?;
    write_new_file(
        &path.join(WORKBENCH_CONFIG_PATH),
        &render_workbench_config(Provider::Codex, None, Path::new(DEFAULT_RUN_ROOT)),
    )?;
    scaffold_agent_skills(path)?;

    println!("initialized RMS system at {}", path.display());
    Ok(())
}

fn scaffold_agent_skills(path: &Path) -> Result<()> {
    for (relative_path, contents) in INIT_AGENT_SKILLS {
        write_new_file(
            &path.join(".agents").join("skills").join(relative_path),
            contents,
        )?;
    }
    Ok(())
}

fn run_add_module(
    path: &Path,
    name: &str,
    purpose: &str,
    kind: &str,
    profiles: &[String],
    binding: Option<&str>,
) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create module directory `{}`", path.display()))?;
    fs::create_dir_all(path.join("contracts"))?;
    let profiles = normalized_profiles(profiles);
    for category in ["laws", "contracts", "scenarios", "boundaries"] {
        let verification_dir = path.join("verification").join(category);
        fs::create_dir_all(&verification_dir)?;
        write_new_file(
            &verification_dir.join("README.md"),
            &render_verification_readme(category),
        )?;
    }

    write_new_file(
        &path.join("module.yaml"),
        &render_module_yaml(name, purpose, kind, &profiles),
    )?;
    write_new_file(
        &path.join("README.md"),
        &render_module_readme(name, purpose, kind, &profiles, binding),
    )?;
    write_new_file(
        &path.join("contracts").join("README.md"),
        &render_contracts_readme(),
    )?;

    if let Some(binding) = binding {
        match binding {
            "rust" => scaffold_rust_module(path, name)?,
            "swift" => scaffold_swift_module(path, name)?,
            "executable" => scaffold_executable_module(path, name)?,
            other => bail!("unsupported scaffold binding `{other}`"),
        }
    }

    println!("added RMS module at {}", path.display());
    Ok(())
}

fn scaffold_rust_module(path: &Path, name: &str) -> Result<()> {
    let package_name = sanitize_rust_package_name(name);
    fs::create_dir_all(path.join("src"))?;
    write_new_file(
        &path.join("implementation.yaml"),
        &render_rust_implementation_yaml(name, &package_name),
    )?;
    write_new_file(
        &path.join("Cargo.toml"),
        &render_rust_cargo_toml(&package_name),
    )?;
    write_new_file(
        &path.join("src").join("lib.rs"),
        "pub fn module_name() -> &'static str {\n    env!(\"CARGO_PKG_NAME\")\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn exposes_module_name() {\n        assert_eq!(super::module_name(), env!(\"CARGO_PKG_NAME\"));\n    }\n}\n",
    )?;
    Ok(())
}

fn scaffold_swift_module(path: &Path, name: &str) -> Result<()> {
    let package_name = sanitize_swift_package_name(name);
    let target_name = sanitize_swift_target_name(name);
    fs::create_dir_all(path.join("Sources").join(&target_name))?;
    fs::create_dir_all(path.join("Tests").join(format!("{target_name}Tests")))?;
    write_new_file(
        &path.join("implementation.yaml"),
        &render_swift_implementation_yaml(name, &package_name, &target_name),
    )?;
    write_new_file(
        &path.join("Package.swift"),
        &render_swift_package_swift(&package_name, &target_name),
    )?;
    write_new_file(
        &path
            .join("Sources")
            .join(&target_name)
            .join(format!("{target_name}.swift")),
        &render_swift_source(&target_name),
    )?;
    write_new_file(
        &path
            .join("Tests")
            .join(format!("{target_name}Tests"))
            .join(format!("{target_name}Tests.swift")),
        &render_swift_tests(&target_name),
    )?;
    Ok(())
}

fn scaffold_executable_module(path: &Path, name: &str) -> Result<()> {
    fs::create_dir_all(path.join("scripts"))?;
    write_new_file(
        &path.join("implementation.yaml"),
        &render_executable_implementation_yaml(name),
    )?;
    write_new_file(&path.join("scripts").join("build.sh"), EXECUTABLE_BUILD_SH)?;
    write_new_file(&path.join("scripts").join("smoke.sh"), EXECUTABLE_SMOKE_SH)?;
    write_new_file(
        &path
            .join("verification")
            .join("boundaries")
            .join("executable_smoke.md"),
        &render_executable_smoke_evidence(),
    )?;
    Ok(())
}

fn normalized_profiles(profiles: &[String]) -> Vec<String> {
    let mut normalized = BTreeSet::from(["core".to_string()]);
    normalized.extend(profiles.iter().cloned());
    normalized.into_iter().collect()
}

fn render_system_yaml(name: &str, purpose: &str, version: &str, contexts: &[String]) -> String {
    format!(
        "spec: rms/system/v0.1\n\nsystem:\n  name: {}\n  version: {}\n  purpose: {}\n\ncontexts:\n{}\n\npublic_interfaces: []\nexternal_dependencies: []\nworkflows: []\n\ninvariants: []\n\ncompatibility:\n  policy: backward-compatible-within-major\n\nglossary: GLOSSARY.md\ncontext_map: context-map.yaml\n",
        yaml_quote(name),
        yaml_quote(version),
        yaml_quote(purpose),
        yaml_string_list(contexts, 2)
    )
}

fn render_context_map_yaml(contexts: &[String]) -> String {
    let mut rendered = "spec: rms/context-map/v0.1\n\ncontexts:\n".to_string();
    for context in contexts {
        rendered.push_str(&format!(
            "  {}:\n    publishes: []\n    consumes: []\n",
            yaml_quote(context)
        ));
    }
    rendered.push_str("\nrelationships: []\n");
    rendered
}

fn render_glossary_md(name: &str) -> String {
    format!("# {name} Glossary\n\nAdd context-owned terms here.\n")
}

fn render_module_yaml(name: &str, purpose: &str, kind: &str, profiles: &[String]) -> String {
    format!(
        "spec: rms/module/v0.1\n\nmodule:\n  name: {}\n  version: 0.1.0\n  kind: {}\n  purpose: {}\n\nprofiles:\n{}\n\nowns:\n  concepts: []\n  data: []\n  decisions: []\n\nprovides:\n  commands: []\n  queries: []\n  events: []\n  capabilities: []\n\nrequires:\n  modules: []\n  capabilities: []\n\ninvariants: []\n\neffects: []\n{}compatibility:\n  policy: backward-compatible-within-major\n\nverification:\n  laws:\n    - verification/laws\n  contracts:\n    - verification/contracts\n  scenarios:\n    - verification/scenarios\n  boundaries:\n    - verification/boundaries\n",
        yaml_quote(name),
        yaml_quote(kind),
        yaml_quote(purpose),
        yaml_string_list(profiles, 2),
        render_profile_sections(profiles)
    )
}

fn render_profile_sections(profiles: &[String]) -> String {
    let mut sections = String::new();
    if profiles.iter().any(|profile| profile == "stateful") {
        sections.push_str("\nstate: {}\n");
    }
    if profiles.iter().any(|profile| profile == "distributed") {
        sections.push_str("\noperations:\n  reconciliation: []\n");
    }
    if profiles.iter().any(|profile| profile == "workflow") {
        sections.push_str("\nworkflow: {}\n");
    }
    if profiles.iter().any(|profile| profile == "boundary") {
        sections.push_str("\nboundary: {}\n");
    }
    sections
}

fn render_module_readme(
    name: &str,
    purpose: &str,
    kind: &str,
    profiles: &[String],
    binding: Option<&str>,
) -> String {
    let profile_lines = profiles
        .iter()
        .map(|profile| format!("- `{}`", markdown_inline(profile)))
        .collect::<Vec<_>>()
        .join("\n");
    let binding_line = match binding {
        Some(binding) => format!(
            "Implementation binding: `{}` via `implementation.yaml`.",
            markdown_inline(binding)
        ),
        None => "Implementation binding: none generated yet.".to_string(),
    };

    format!(
        "# {}\n\nPurpose: {}\nKind: `{}`\n{}\n\n## Profiles\n\n{}\n\n## Representation Decisions\n\n- Public domain values with validity rules should use private fields, validated constructors, explicit failure types, semantic-function bindings, and evidence.\n- Public read models or result structs produced only by queries/projectors may keep private fields without public constructors only when `implementation.yaml` declares them in `architecture.allowed_missing_constructors` and evidence names the producing query/projector.\n- Do not add a fake public constructor only to satisfy a binding check; either expose a real contract-backed constructor or document the query-produced exception.\n\n## Canonical Artifacts\n\n- `module.yaml` is the source of module ownership, public surface, dependencies, effects, invariants, profiles, and compatibility.\n- `contracts/` contains public RMS contracts only: commands, queries, events, APIs, capabilities, schemas, and externally consumed failure semantics.\n- `implementation.yaml`, when present, binds code symbols to contracts, invariants, assumptions, and evidence.\n- `verification/` contains evidence for declared promises. Evidence should name the source revision and command or tool used.\n\n## Before Changing Behavior\n\n1. Fill `module.yaml` with owned concepts, data, decisions, public surface, dependencies, effects, invariants, and verification references that are true for this module.\n2. Add or update public contracts before implementing externally consumed behavior.\n3. Keep private implementation details out of `contracts/` unless consumers depend on them.\n4. Add the smallest evidence that proves the declared promise, including negative cases for invalid inputs or illegal transitions when applicable.\n5. Run `rms validate --root <system-root>` and `rms compose --root <system-root>`; run `rms verify implementation.yaml` when an implementation binding exists.\n\n## Agent Workflow\n\nUse `rms explain module.yaml` and `rms context module.yaml --task \"<task>\"` before implementation work. Use `rms evolve-contract module.yaml --task \"<task>\"` when public meaning changes, and `rms evidence module.yaml --task \"<task>\"` when proof design is unclear.\n",
        markdown_inline(name),
        markdown_inline(purpose),
        markdown_inline(kind),
        binding_line,
        profile_lines
    )
}

fn render_contracts_readme() -> String {
    "# Contracts\n\nPlace public RMS contract files here.\n\nA contract belongs here when consumers outside this module can call, observe, depend on, or substitute against the behavior. Private helpers stay in implementation docs and tests.\n\nWhen adding or changing a contract:\n\n1. Declare it from `module.yaml`.\n2. Specify preconditions, postconditions, failure categories, and compatibility policy.\n3. Bind implemented symbols from `implementation.yaml` when code provides the behavior.\n4. Add matching evidence under `verification/contracts/`.\n".to_string()
}

fn render_verification_readme(category: &str) -> String {
    match category {
        "laws" => "# Law Evidence\n\nRecord evidence for invariants and algebraic or domain laws declared in `module.yaml`.\n\nEach evidence file should identify:\n\n- the invariant, law, or manifest promise under test;\n- positive and negative cases;\n- the command or tool used;\n- the source revision when applicable.\n\nDo not add law evidence for behavior that is not declared by the module.\n".to_string(),
        "contracts" => "# Contract Evidence\n\nRecord evidence that public contracts in `contracts/` are satisfied.\n\nEach evidence file should identify:\n\n- the contract path and version;\n- success behavior and expected failures;\n- boundary validation for untrusted or versioned input;\n- the command or tool used;\n- the source revision when applicable.\n".to_string(),
        "scenarios" => "# Scenario Evidence\n\nRecord end-to-end behavior that matters to this module's declared purpose.\n\nEach evidence file should identify:\n\n- the manifest promise or public contract exercised;\n- setup, action, expected result, and failure path;\n- recovery or reconciliation behavior when declared;\n- the command or tool used;\n- the source revision when applicable.\n".to_string(),
        "boundaries" => "# Boundary Evidence\n\nRecord evidence for trust boundaries, external effects, dependency contracts, schemas, limits, and compatibility promises.\n\nEach evidence file should identify:\n\n- the boundary input, dependency, or effect under test;\n- validation, rejection, timeout, retry, or compatibility behavior;\n- the command or tool used;\n- the source revision when applicable.\n".to_string(),
        other => format!("# {other}\n\nAdd RMS {other} evidence here.\n"),
    }
}

fn render_rust_implementation_yaml(module_name: &str, package_name: &str) -> String {
    format!(
        "spec: rms/implementation/v0.1\n\nmodule: {}\nbinding: rust\n\nsource:\n  root: .\n  public_entrypoint: src/lib.rs\n\ncommands:\n  build: cargo build --manifest-path Cargo.toml\n  verify: cargo test --manifest-path Cargo.toml\n  format: cargo fmt --manifest-path Cargo.toml --check\n\ntoolchain:\n  cargo_manifest: Cargo.toml\n  package: {}\n\ndependencies:\n  allowed_external_crates: []\n\narchitecture:\n  public_modules: []\n",
        yaml_quote(module_name),
        yaml_quote(package_name),
    )
}

fn render_rust_cargo_toml(package_name: &str) -> String {
    format!(
        "[package]\nname = \"{package_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[workspace]\n\n[lib]\npath = \"src/lib.rs\"\n\n[dependencies]\n"
    )
}

fn render_swift_implementation_yaml(
    module_name: &str,
    package_name: &str,
    target_name: &str,
) -> String {
    let source_root = format!("Sources/{target_name}");
    let public_entrypoint = format!("Sources/{target_name}/{target_name}.swift");
    format!(
        "spec: rms/implementation/v0.1\n\nmodule: {}\nbinding: swift\n\nsource:\n  root: {}\n  public_entrypoint: {}\n\ncommands:\n  build: swift build --package-path .\n  verify: swift test --package-path .\n\ntoolchain:\n  package_manifest: Package.swift\n  package: {}\n  target: {}\n\ndependencies:\n  allowed_external_modules: []\n\narchitecture:\n  public_modules: []\n",
        yaml_quote(module_name),
        yaml_quote(&source_root),
        yaml_quote(&public_entrypoint),
        yaml_quote(package_name),
        yaml_quote(target_name),
    )
}

fn render_swift_package_swift(package_name: &str, target_name: &str) -> String {
    format!(
        "// swift-tools-version: 5.9\nimport PackageDescription\n\nlet package = Package(\n    name: \"{package_name}\",\n    products: [\n        .library(name: \"{target_name}\", targets: [\"{target_name}\"])\n    ],\n    targets: [\n        .target(name: \"{target_name}\"),\n        .testTarget(name: \"{target_name}Tests\", dependencies: [\"{target_name}\"])\n    ]\n)\n"
    )
}

fn render_swift_source(target_name: &str) -> String {
    format!(
        "import Foundation\n\npublic struct {target_name}Value: Equatable {{\n    private let rawValue: String\n\n    public init?(_ rawValue: String) {{\n        let trimmed = rawValue.trimmingCharacters(in: .whitespacesAndNewlines)\n        guard !trimmed.isEmpty else {{ return nil }}\n        self.rawValue = trimmed\n    }}\n\n    public var value: String {{ rawValue }}\n}}\n"
    )
}

fn render_swift_tests(target_name: &str) -> String {
    format!(
        "import XCTest\n@testable import {target_name}\n\nfinal class {target_name}Tests: XCTestCase {{\n    func testRejectsEmptyValue() {{\n        XCTAssertNil({target_name}Value(\"\"))\n    }}\n\n    func testAcceptsNonEmptyValue() {{\n        XCTAssertEqual({target_name}Value(\"example\")?.value, \"example\")\n    }}\n}}\n"
    )
}

fn render_executable_implementation_yaml(module_name: &str) -> String {
    format!(
        "spec: rms/implementation/v0.1\n\nmodule: {}\nbinding: executable\n\nsource:\n  root: .\n  public_entrypoint: scripts/smoke.sh\n\ncommands:\n  build: sh scripts/build.sh\n  verify: sh scripts/smoke.sh\n\ntoolchain:\n  runner: shell\n\ndependencies:\n  allowed_processes:\n    - sh\n\narchitecture:\n  verification_mode: executable-command\n  static_inspection: opaque\n  public_entrypoints:\n    - scripts/smoke.sh\n  boundary_inputs: []\n  observable_outputs: []\n  declared_assets: []\n\nsemantic_functions:\n  - id: executable-smoke\n    symbol: scripts/smoke.sh\n    kind: adapter\n    purity: boundary\n    assumptions:\n      ensures:\n        - command-backed implementation can be invoked through the declared verify command\n        - RMS does not infer internal domain semantics from opaque executable assets\n    evidence:\n      boundaries:\n        - verification/boundaries/executable_smoke.md\n",
        yaml_quote(module_name),
    )
}

fn render_executable_smoke_evidence() -> String {
    "# Boundary Evidence: executable smoke\n\nPromise:\n\n- `implementation.yaml` declares `binding: executable`.\n- RMS treats the implementation as opaque and verifies it through declared commands rather than static source inspection.\n\nCommand:\n\n- `rms verify implementation.yaml` runs `sh scripts/smoke.sh` from the module directory.\n\nCurrent scaffold:\n\n- `scripts/smoke.sh` verifies that `module.yaml` and `implementation.yaml` exist.\n- Replace or extend this script with module-specific checks before using the binding as release evidence.\n\nSource revision: not recorded by the generated scaffold.\n".to_string()
}

const EXECUTABLE_BUILD_SH: &str =
    "#!/usr/bin/env sh\nset -eu\nprintf '%s\\n' 'executable binding build placeholder'\n";

const EXECUTABLE_SMOKE_SH: &str = "#!/usr/bin/env sh\nset -eu\ntest -f module.yaml\ntest -f implementation.yaml\nprintf '%s\\n' 'executable binding smoke passed'\n";

fn yaml_string_list(values: &[String], indent: usize) -> String {
    let prefix = " ".repeat(indent);
    values
        .iter()
        .map(|value| format!("{prefix}- {}", yaml_quote(value)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn yaml_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn markdown_inline(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn sanitize_rust_package_name(name: &str) -> String {
    let mut output = String::new();
    let mut previous_dash = false;
    for character in name.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() || character == '_' {
            output.push(character);
            previous_dash = false;
        } else if !previous_dash {
            output.push('-');
            previous_dash = true;
        }
    }
    let trimmed = output.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "rms-module".to_string()
    } else {
        trimmed
    }
}

fn sanitize_swift_package_name(name: &str) -> String {
    sanitize_rust_package_name(name)
}

fn sanitize_swift_target_name(name: &str) -> String {
    let mut output = String::new();
    let mut capitalize_next = true;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if output.is_empty() && character.is_ascii_digit() {
                output.push_str("Rms");
            }
            if capitalize_next {
                output.extend(character.to_uppercase());
                capitalize_next = false;
            } else {
                output.push(character);
            }
        } else {
            capitalize_next = true;
        }
    }
    if output.is_empty() {
        "RmsModule".to_string()
    } else {
        output
    }
}

fn write_new_file(path: &Path, contents: &str) -> Result<()> {
    if path.exists() {
        bail!("refusing to overwrite existing file `{}`", path.display());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents).with_context(|| format!("failed to write `{}`", path.display()))
}

const INIT_GITIGNORE: &str = ".DS_Store\ntarget/\ndist/\n.rms/runs/\n";

const INIT_AGENTS_MD: &str = r#"# Agent Instructions

This repository follows Reliable Modular Systems.

RMS artifacts are the architectural source of truth. Do not infer ownership, effects, dependencies, compatibility, or recovery behavior from incidental code shape when manifests or contracts say otherwise.

## Before Changing Behavior

1. Run `rms diagnose`.
2. Identify the owning module for the requested behavior.
3. Run `rms explain <module.yaml>` to understand ownership, public surface, effects, invariants, compatibility, and verification evidence.
4. Run `rms context <module.yaml> --task "<task>"` before implementation work.
5. Read the target `module.yaml`, public contracts, direct dependency contracts, applicable glossary entries, and implementation binding.

Use these advisory workbench commands when they match the task:

- `rms plan <module.yaml> --task "<task>"`
- `rms implement <module.yaml> --task "<task>"`
- `rms evolve-contract <module.yaml> --task "<task>"`
- `rms evidence <module.yaml> --task "<task>"`
- `rms refactor <module.yaml> --task "<task>"`
- `rms review <module.yaml> --impact`

Provider-backed prompts are opt-in. Use `--provider codex` or `--ai` only when an external Codex run is intended.

## While Implementing

- Keep changes inside the owning module boundary.
- Change public contracts or manifests before code when public meaning changes.
- Declare new effects, dependencies, profiles, state, migration, compatibility impact, and recovery paths before relying on them.
- Prefer explicit domain types, validated constructors, explicit result types, schemas at untrusted boundaries, and focused tests.
- Do not edit another module's private implementation to bypass its public contract.
- Treat generated reports, diffs, and provider output as evidence, not architecture.

## Before Completion

Run the smallest checks that prove the changed promise:

- `rms validate --root .`
- `rms compose --root .`
- `rms verify <implementation.yaml>` when the module has an implementation binding.
- `rms gate --root .` when reviewing a working-tree change.

Report remaining manual obligations explicitly, especially compatibility review, missing evidence, undeclared effects, or partial conformance.
"#;

fn referenced_paths(value: &YamlValue) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    collect_contract_paths(value, &mut |path| {
        paths.insert(path.to_string());
    });

    if let Some(invariants) = get_path(value, &["invariants"]).and_then(YamlValue::as_sequence) {
        for invariant in invariants {
            if let Some(path) = get_str(invariant, &["verified_by"]) {
                paths.insert(path.to_string());
            }
        }
    }

    if let Some(verification) = get_path(value, &["verification"]) {
        for category in [
            "laws",
            "contracts",
            "scenarios",
            "boundaries",
            "runtime",
            "reconciliation",
        ] {
            if let Some(items) =
                get_path(verification, &[category]).and_then(YamlValue::as_sequence)
            {
                for item in items.iter().filter_map(YamlValue::as_str) {
                    paths.insert(item.to_string());
                }
            }
        }
    }

    if let Some(protocols) = change_protocol_items(value) {
        for protocol in protocols {
            collect_change_protocol_reference_paths(protocol, &mut |path| {
                paths.insert(path.to_string());
            });
        }
    }

    paths
}

fn has_empty_verification_category(value: &YamlValue) -> bool {
    ["laws", "contracts", "scenarios", "boundaries"]
        .iter()
        .any(|category| {
            get_path(value, &["verification", category])
                .and_then(YamlValue::as_sequence)
                .is_none_or(Vec::is_empty)
        })
}

fn load_manifest(path: &Path) -> Result<LoadedManifest> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read manifest `{}`", path.display()))?;
    let value = serde_yaml::from_str(&contents)
        .with_context(|| format!("failed to parse YAML `{}`", path.display()))?;
    Ok(LoadedManifest {
        path: path.to_path_buf(),
        value,
    })
}

fn get_path<'a>(value: &'a YamlValue, path: &[&str]) -> Option<&'a YamlValue> {
    let mut current = value;
    for segment in path {
        current = current
            .as_mapping()?
            .get(YamlValue::String((*segment).to_string()))?;
    }
    Some(current)
}

fn get_str<'a>(value: &'a YamlValue, path: &[&str]) -> Option<&'a str> {
    get_path(value, path)?.as_str()
}

fn get_bool(value: &YamlValue, path: &[&str]) -> Option<bool> {
    get_path(value, path)?.as_bool()
}

fn get_string_array(value: &YamlValue, path: &[&str]) -> Vec<String> {
    get_path(value, path)
        .and_then(YamlValue::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(YamlValue::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn require_str(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    check: impl Into<String>,
    path: &[&str],
) {
    if get_str(&manifest.value, path).is_none() {
        diagnostics.push(error(
            check,
            &manifest.path,
            format!("missing required string `{}`", path.join(".")),
        ));
    }
}

fn require_array(
    manifest: &LoadedManifest,
    diagnostics: &mut Vec<Diagnostic>,
    check: impl Into<String>,
    path: &[&str],
) {
    if get_path(&manifest.value, path)
        .and_then(YamlValue::as_sequence)
        .is_none()
    {
        diagnostics.push(error(
            check,
            &manifest.path,
            format!("missing required array `{}`", path.join(".")),
        ));
    }
}

fn error(check: impl Into<String>, path: &Path, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        check: check.into(),
        path: path.display().to_string(),
        message: message.into(),
    }
}

fn warning(check: impl Into<String>, path: &Path, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        severity: Severity::Warning,
        check: check.into(),
        path: path.display().to_string(),
        message: message.into(),
    }
}

fn error_diagnostic(
    check: impl Into<String>,
    path: &Path,
    message: impl Into<String>,
) -> Diagnostic {
    error(check, path, message)
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn command_readiness(command: &str, args: &[&str]) -> CommandReadiness {
    match Command::new(command).args(args).output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let version = stdout
                .lines()
                .next()
                .filter(|line| !line.trim().is_empty())
                .or_else(|| stderr.lines().next())
                .unwrap_or("available");
            CommandReadiness {
                command: command.to_string(),
                status: "available".to_string(),
                detail: Some(version.to_string()),
            }
        }
        Ok(output) => CommandReadiness {
            command: command.to_string(),
            status: "found-not-ready".to_string(),
            detail: Some(format!(
                "exit {}",
                output
                    .status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "signal".to_string())
            )),
        },
        Err(_) => CommandReadiness {
            command: command.to_string(),
            status: "not-configured".to_string(),
            detail: None,
        },
    }
}

fn print_command_readiness(readiness: &CommandReadiness) {
    match readiness.detail.as_deref() {
        Some(detail) => println!("{}: {} ({detail})", readiness.command, readiness.status),
        None => println!("{}: {}", readiness.command, readiness.status),
    }
}

fn print_string_list(label: &str, items: &[String]) {
    println!(
        "{label}: {}",
        if items.is_empty() {
            "<none>".to_string()
        } else {
            items.join(", ")
        }
    );
}

fn print_owned_terms(value: &YamlValue) {
    println!();
    println!("## Ownership");
    if let Some(owns) = get_path(value, &["owns"]).and_then(YamlValue::as_mapping) {
        for (key, values) in owns {
            let label = key.as_str().unwrap_or("<unknown>");
            let items = values
                .as_sequence()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(YamlValue::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            println!(
                "- {label}: {}",
                if items.is_empty() { "<none>" } else { &items }
            );
        }
    } else {
        println!("- <missing>");
    }
}

fn print_contract_groups(label: &str, value: Option<&YamlValue>) {
    println!();
    println!("## {label}");
    let Some(groups) = value.and_then(YamlValue::as_mapping) else {
        println!("- <missing>");
        return;
    };
    for (group, items) in groups {
        let group = group.as_str().unwrap_or("<unknown>");
        println!("- {group}:");
        if let Some(items) = items.as_sequence() {
            for item in items {
                match item {
                    YamlValue::String(name) => println!("  - {name}"),
                    YamlValue::Mapping(mapping) => {
                        let name = mapping
                            .get(YamlValue::String("name".to_string()))
                            .and_then(YamlValue::as_str)
                            .unwrap_or("<unnamed>");
                        let contract = mapping
                            .get(YamlValue::String("contract".to_string()))
                            .and_then(YamlValue::as_str)
                            .unwrap_or("<no contract>");
                        println!("  - {name} ({contract})");
                    }
                    _ => println!("  - <unsupported reference>"),
                }
            }
        }
    }
}

fn print_invariants(value: &YamlValue) {
    println!();
    println!("## Invariants");
    let Some(invariants) = get_path(value, &["invariants"]).and_then(YamlValue::as_sequence) else {
        println!("- <missing>");
        return;
    };
    if invariants.is_empty() {
        println!("- <none declared>");
        return;
    }
    for invariant in invariants {
        println!(
            "- {}: {}",
            get_str(invariant, &["id"]).unwrap_or("<missing-id>"),
            get_str(invariant, &["statement"]).unwrap_or("<missing statement>")
        );
    }
}

fn print_effects(value: &YamlValue) {
    println!();
    println!("## Effects");
    let Some(effects) = get_path(value, &["effects"]).and_then(YamlValue::as_sequence) else {
        println!("- <missing>");
        return;
    };
    if effects.is_empty() {
        println!("- <none declared>");
        return;
    }
    for effect in effects {
        println!(
            "- {} ({})",
            get_str(effect, &["name"]).unwrap_or("<unnamed>"),
            get_str(effect, &["kind"]).unwrap_or("<unknown-kind>")
        );
    }
}

fn print_verification(value: &YamlValue) {
    println!();
    println!("## Verification");
    for category in ["laws", "contracts", "scenarios", "boundaries"] {
        let items = get_string_array(value, &["verification", category]);
        println!(
            "- {category}: {}",
            if items.is_empty() {
                "<none>".to_string()
            } else {
                items.join(", ")
            }
        );
    }
}

fn print_change_protocols(value: &YamlValue) {
    let Some(protocols) = change_protocol_items(value) else {
        return;
    };
    if protocols.is_empty() {
        return;
    }

    println!();
    println!("## Change Protocols");
    for protocol in protocols {
        let id = get_str(protocol, &["id"]).unwrap_or("<missing-id>");
        let applies_when = get_str(protocol, &["applies_when"]).unwrap_or("<missing applies_when>");
        println!("- {id}: {applies_when}");
        if let Some(classification) = get_str(protocol, &["classification"]) {
            println!("  classification: {classification}");
        }

        let required_updates = get_string_array(protocol, &["required_updates"]);
        if !required_updates.is_empty() {
            println!("  required updates:");
            for update in required_updates {
                println!("    - {update}");
            }
        }

        let verify = get_string_array(protocol, &["verify"]);
        if !verify.is_empty() {
            println!("  verify:");
            for command in verify {
                println!("    - {command}");
            }
        }
    }
}

fn print_question_focus(value: &YamlValue, question: &str) {
    let normalized = question.to_ascii_lowercase();
    let mut matched = false;

    if normalized.contains("own")
        || normalized.contains("state")
        || normalized.contains("data")
        || normalized.contains("decision")
        || normalized.contains("how")
        || normalized.contains("work")
    {
        println!("Ownership is the first place to look:");
        print_owned_terms(value);
        matched = true;
    }

    if normalized.contains("how") || normalized.contains("work") {
        println!("The public shape and reliability rules show how callers can use it and what it must preserve:");
        print_contract_groups("Provides", get_path(value, &["provides"]));
        print_invariants(value);
        print_effects(value);
        matched = true;
    }

    if normalized.contains("contract")
        || normalized.contains("public")
        || normalized.contains("api")
        || normalized.contains("command")
        || normalized.contains("query")
        || normalized.contains("event")
    {
        println!("Public surface is declared here:");
        print_contract_groups("Provides", get_path(value, &["provides"]));
        print_contract_groups("Requires", get_path(value, &["requires"]));
        matched = true;
    }

    if normalized.contains("effect")
        || normalized.contains("io")
        || normalized.contains("network")
        || normalized.contains("storage")
        || normalized.contains("time")
        || normalized.contains("external")
    {
        println!("Declared effects are:");
        print_effects(value);
        matched = true;
    }

    if normalized.contains("verify")
        || normalized.contains("test")
        || normalized.contains("evidence")
        || normalized.contains("prove")
    {
        println!("Verification evidence is:");
        print_verification(value);
        matched = true;
    }

    if normalized.contains("change")
        || normalized.contains("patch")
        || normalized.contains("modify")
        || normalized.contains("protocol")
    {
        println!("Declared change protocols are:");
        print_change_protocols(value);
        matched = true;
    }

    if normalized.contains("break")
        || normalized.contains("compat")
        || normalized.contains("version")
        || normalized.contains("migration")
    {
        println!(
            "Compatibility policy: {}",
            get_str(value, &["compatibility", "policy"]).unwrap_or("<missing>")
        );
        println!("Check public contract shape, operational semantics, stored state, and active consumers before changing this area.");
        matched = true;
    }

    if !matched {
        println!("No specialized deterministic answer matched this question. Use the sections above as the bounded module explanation, or run `rms context <module> --task \"{question}\"` to prepare an agent packet.");
    }
}

fn source_revision(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()?;
    if output.status.success() {
        Some(format!(
            "git:{}",
            String::from_utf8_lossy(&output.stdout).trim()
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_version_flag_uses_package_version() {
        let version = Cli::command().render_version().to_string();

        assert!(version.contains(VALIDATOR_NAME));
        assert!(version.contains(VALIDATOR_VERSION));
    }

    #[test]
    fn collects_contract_and_verification_references() {
        let value: YamlValue = serde_yaml::from_str(
            r#"
provides:
  commands:
    - name: do-work
      contract: contracts/do-work.yaml
invariants:
  - id: law
    statement: Law holds.
    verified_by: verification/laws/law
verification:
  laws:
    - verification/laws
  contracts: []
  scenarios:
    - verification/scenarios
  boundaries: []
"#,
        )
        .unwrap();

        let references = referenced_paths(&value);

        assert!(references.contains("contracts/do-work.yaml"));
        assert!(references.contains("verification/laws/law"));
        assert!(references.contains("verification/laws"));
        assert!(references.contains("verification/scenarios"));
    }

    #[test]
    fn collects_change_protocol_references() {
        let value: YamlValue = serde_yaml::from_str(
            r#"
x-change-protocols:
  - id: public-status-change
    applies_when: A public status changes meaning.
    references:
      contracts:
        - contracts/status.yaml
      docs:
        - docs/status-lifecycle.md
      evidence:
        - verification/laws/status
"#,
        )
        .unwrap();

        let references = referenced_paths(&value);

        assert!(references.contains("contracts/status.yaml"));
        assert!(references.contains("docs/status-lifecycle.md"));
        assert!(references.contains("verification/laws/status"));
    }

    #[test]
    fn validates_change_protocol_references() {
        let value: YamlValue = serde_yaml::from_str(
            r#"
spec: rms/module/v0.1

module:
  name: example
  version: 0.1.0
  kind: library
  purpose: Test change protocols

profiles:
  - core

owns:
  concepts: []
  data: []
  decisions: []

provides:
  commands: []
  queries: []
  events: []
  capabilities: []

requires:
  modules: []
  capabilities: []

invariants: []
effects: []

compatibility:
  policy: backward-compatible-within-major

verification:
  laws: []
  contracts: []
  scenarios: []
  boundaries: []

x-change-protocols:
  - id: public-status-change
    applies_when: A public status changes meaning.
    references:
      docs:
        - docs/missing.md
"#,
        )
        .unwrap();
        let manifest = LoadedManifest {
            path: PathBuf::from("module.yaml"),
            value,
        };
        let mut diagnostics = Vec::new();

        validate_loaded_manifest(&manifest, &mut diagnostics);

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.check == "references.change-protocol"));
    }

    #[test]
    fn empty_verification_category_makes_conformance_partial() {
        let value: YamlValue = serde_yaml::from_str(
            r#"
verification:
  laws:
    - verification/laws
  contracts: []
  scenarios:
    - verification/scenarios
  boundaries: []
"#,
        )
        .unwrap();

        assert!(has_empty_verification_category(&value));
    }

    #[test]
    fn schema_validation_reports_shape_errors() {
        let value: YamlValue = serde_yaml::from_str(
            r#"
spec: rms/module/v0.1
module:
  name: example
profiles:
  - core
owns: {}
provides: {}
requires: {}
invariants: []
effects: []
compatibility: {}
verification: {}
"#,
        )
        .unwrap();
        let manifest = LoadedManifest {
            path: PathBuf::from("module.yaml"),
            value,
        };
        let mut diagnostics = Vec::new();

        validate_against_embedded_schema(&manifest, &mut diagnostics);

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.check == "schema.validate"));
    }

    #[test]
    fn diagnostic_categories_are_conformance_schema_compatible() {
        assert_eq!(diagnostic_category("schema.validate"), "manifest");
        assert_eq!(diagnostic_category("references.contract"), "contracts");
        assert_eq!(
            diagnostic_category("profile.distributed.reconciliation"),
            "profiles"
        );
        assert_eq!(diagnostic_category("security.secret-key"), "security");
    }

    #[test]
    fn collects_rust_dependencies_from_cargo_manifest() {
        let cargo: TomlValue = r#"
[package]
name = "example"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1"

[dev-dependencies]
proptest = "1"

[target.'cfg(unix)'.dependencies]
libc = "0.2"
"#
        .parse()
        .unwrap();

        let dependencies = collect_rust_dependencies(&cargo);

        assert!(dependencies.contains("serde"));
        assert!(dependencies.contains("proptest"));
        assert!(dependencies.contains("libc"));
    }

    #[test]
    fn extracts_public_modules_from_rust_entrypoint() {
        let modules = public_modules_declared_in_source(
            r#"
pub mod widget;
pub(crate) mod private;
pub mod nested {
}
"#,
        );

        assert!(modules.contains("widget"));
        assert!(modules.contains("nested"));
        assert!(!modules.contains("private"));
    }

    #[test]
    fn parses_public_and_private_rust_import_roots() {
        let file = syn::parse_file(
            r#"
use serde::{Deserialize, Serialize};
pub use widget::Widget;
pub use external_crate::Thing;
"#,
        )
        .unwrap();

        let imports = collect_rust_imports(&file);

        assert!(imports
            .iter()
            .any(|import| import.root == "serde" && !import.is_public));
        assert!(imports
            .iter()
            .any(|import| import.root == "widget" && import.is_public));
        assert!(imports
            .iter()
            .any(|import| import.root == "external_crate" && import.is_public));
    }

    #[test]
    fn classifies_local_and_external_rust_imports() {
        let local_modules = BTreeSet::from(["widget".to_string()]);

        assert_eq!(
            rust_import_root_kind("widget", &local_modules),
            RustImportRootKind::LocalModule
        );
        assert_eq!(
            rust_import_root_kind("serde", &local_modules),
            RustImportRootKind::External
        );
        assert_eq!(
            rust_import_root_kind("std", &local_modules),
            RustImportRootKind::Standard
        );
        assert_eq!(
            rust_import_root_kind("crate", &local_modules),
            RustImportRootKind::SelfQualified
        );
    }

    #[test]
    fn parses_swift_imports_and_public_reexports() {
        let imports = collect_swift_imports(
            r#"
import Foundation
import struct ExternalKit.Widget
@_exported import PublicKit
"#,
        );

        assert!(imports
            .iter()
            .any(|import| import.module == "Foundation" && !import.is_public_reexport));
        assert!(imports
            .iter()
            .any(|import| import.module == "ExternalKit" && !import.is_public_reexport));
        assert!(imports
            .iter()
            .any(|import| import.module == "PublicKit" && import.is_public_reexport));
    }

    #[test]
    fn detects_path_components_for_source_exclusions() {
        assert!(path_has_component(
            Path::new("crate/target/debug/out.rs"),
            "target"
        ));
        assert!(!path_has_component(Path::new("crate/src/lib.rs"), "target"));
    }

    #[test]
    fn init_scaffold_generates_valid_system_artifacts() {
        let root = unique_test_dir("init");

        run_init(
            &root,
            "example-system",
            "Demonstrate RMS initialization.",
            "0.1.0",
            &[String::from("example")],
        )
        .unwrap();

        let mut diagnostics = Vec::new();
        for file in ["system.yaml", "context-map.yaml"] {
            let manifest = load_manifest(&root.join(file)).unwrap();
            validate_loaded_manifest(&manifest, &mut diagnostics);
        }

        let agents = fs::read_to_string(root.join("AGENTS.md")).unwrap();
        let gitignore = fs::read_to_string(root.join(".gitignore")).unwrap();
        let config = load_workbench_config(&root).unwrap().unwrap();
        for (relative_path, contents) in INIT_AGENT_SKILLS {
            let generated =
                fs::read_to_string(root.join(".agents").join("skills").join(relative_path))
                    .unwrap();
            assert_eq!(generated, *contents);
        }

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        assert!(agents.contains("RMS artifacts are the architectural source of truth"));
        assert!(agents.contains("rms context <module.yaml> --task"));
        assert!(gitignore.contains(".rms/runs/"));
        assert_eq!(config.value.ai.default_provider.as_deref(), Some("codex"));
        assert_eq!(config.value.ai.codex.sandbox.as_deref(), Some("read-only"));
        assert_eq!(
            config.value.runs.directory.as_deref(),
            Some(Path::new(".rms/runs"))
        );
    }

    #[test]
    fn embedded_init_agent_skills_match_canonical_source_when_available() {
        let canonical_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../skills");
        if !canonical_root.exists() {
            return;
        }

        for (relative_path, contents) in INIT_AGENT_SKILLS {
            let canonical = fs::read_to_string(canonical_root.join(relative_path)).unwrap();
            assert_eq!(
                canonical, *contents,
                "embedded skill drift: {relative_path}"
            );
        }
    }

    #[test]
    fn rust_module_scaffold_generates_valid_binding_artifacts() {
        let root = unique_test_dir("rust-module");

        run_add_module(
            &root,
            "example-rust",
            "Demonstrate Rust module scaffolding.",
            "library",
            &[],
            Some("rust"),
        )
        .unwrap();

        let mut diagnostics = Vec::new();
        for file in ["module.yaml", "implementation.yaml"] {
            let manifest = load_manifest(&root.join(file)).unwrap();
            validate_loaded_manifest(&manifest, &mut diagnostics);
        }
        assert_module_scaffold_guidance(
            &root,
            "example-rust",
            "Demonstrate Rust module scaffolding.",
            "rust",
        );

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    #[test]
    fn swift_module_scaffold_generates_valid_binding_artifacts() {
        let root = unique_test_dir("swift-module");

        run_add_module(
            &root,
            "example-swift",
            "Demonstrate Swift module scaffolding.",
            "library",
            &[],
            Some("swift"),
        )
        .unwrap();

        let mut diagnostics = Vec::new();
        for file in ["module.yaml", "implementation.yaml"] {
            let manifest = load_manifest(&root.join(file)).unwrap();
            validate_loaded_manifest(&manifest, &mut diagnostics);
        }
        assert_module_scaffold_guidance(
            &root,
            "example-swift",
            "Demonstrate Swift module scaffolding.",
            "swift",
        );

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    #[test]
    fn executable_module_scaffold_generates_valid_binding_artifacts() {
        let root = unique_test_dir("executable-module");

        run_add_module(
            &root,
            "example-executable",
            "Demonstrate executable module scaffolding.",
            "adapter",
            &["boundary".to_string()],
            Some("executable"),
        )
        .unwrap();

        let mut diagnostics = Vec::new();
        for file in ["module.yaml", "implementation.yaml"] {
            let manifest = load_manifest(&root.join(file)).unwrap();
            validate_loaded_manifest(&manifest, &mut diagnostics);
        }
        assert_module_scaffold_guidance(
            &root,
            "example-executable",
            "Demonstrate executable module scaffolding.",
            "executable",
        );

        let implementation = fs::read_to_string(root.join("implementation.yaml")).unwrap();
        let build_script = fs::read_to_string(root.join("scripts/build.sh")).unwrap();
        let smoke_script = fs::read_to_string(root.join("scripts/smoke.sh")).unwrap();
        let smoke_evidence =
            fs::read_to_string(root.join("verification/boundaries/executable_smoke.md")).unwrap();

        assert!(implementation.contains("binding: executable"));
        assert!(implementation.contains("verification_mode: executable-command"));
        assert!(implementation.contains("static_inspection: opaque"));
        assert!(implementation.contains("RMS does not infer internal domain semantics"));
        assert!(build_script.contains("executable binding build placeholder"));
        assert!(smoke_script.contains("test -f module.yaml"));
        assert!(smoke_evidence.contains("RMS treats the implementation as opaque"));
        run_verify(&root.join("implementation.yaml"), false).unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    #[test]
    fn module_scaffold_generates_required_profile_sections() {
        let root = unique_test_dir("profile-module");

        run_add_module(
            &root,
            "profiled-module",
            "Demonstrate profile section scaffolding.",
            "module",
            &[
                "stateful".to_string(),
                "distributed".to_string(),
                "workflow".to_string(),
                "boundary".to_string(),
            ],
            None,
        )
        .unwrap();

        let mut diagnostics = Vec::new();
        let manifest = load_manifest(&root.join("module.yaml")).unwrap();
        validate_loaded_manifest(&manifest, &mut diagnostics);
        let module_yaml = fs::read_to_string(root.join("module.yaml")).unwrap();

        assert!(module_yaml.contains("state: {}"));
        assert!(module_yaml.contains("operations:\n  reconciliation: []"));
        assert!(module_yaml.contains("workflow: {}"));
        assert!(module_yaml.contains("boundary: {}"));

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    fn assert_module_scaffold_guidance(root: &Path, name: &str, purpose: &str, binding: &str) {
        let readme = fs::read_to_string(root.join("README.md")).unwrap();
        let contracts = fs::read_to_string(root.join("contracts/README.md")).unwrap();
        let laws = fs::read_to_string(root.join("verification/laws/README.md")).unwrap();
        let contract_evidence =
            fs::read_to_string(root.join("verification/contracts/README.md")).unwrap();
        let scenarios = fs::read_to_string(root.join("verification/scenarios/README.md")).unwrap();
        let boundaries =
            fs::read_to_string(root.join("verification/boundaries/README.md")).unwrap();

        assert!(readme.contains(&format!("# {name}")));
        assert!(readme.contains(purpose));
        assert!(readme.contains(&format!("Implementation binding: `{binding}`")));
        assert!(readme.contains("## Representation Decisions"));
        assert!(readme.contains("query/projector"));
        assert!(readme.contains("architecture.allowed_missing_constructors"));
        assert!(readme.contains("`module.yaml` is the source of module ownership"));
        assert!(readme.contains("Use `rms explain module.yaml`"));
        assert!(contracts.contains("Place public RMS contract files here"));
        assert!(contracts.contains("Private helpers stay in implementation docs and tests"));
        assert!(laws.contains("Record evidence for invariants"));
        assert!(contract_evidence.contains("success behavior and expected failures"));
        assert!(scenarios.contains("end-to-end behavior"));
        assert!(boundaries.contains("trust boundaries"));
    }

    #[test]
    fn rust_typing_rejects_public_primitive_aliases() {
        let root = rust_typing_fixture(
            "primitive-alias",
            &["core"],
            "",
            "pub type WidgetId = String;\n",
        );

        let diagnostics = validate_fixture_implementation(&root);

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.check == "implementation.rust.typing.primitive-alias"));
    }

    #[test]
    fn rust_typing_rejects_public_domain_fields() {
        let root = rust_typing_fixture(
            "public-fields",
            &["core"],
            "",
            "pub struct Widget {\n    pub name: String,\n}\n",
        );

        let diagnostics = validate_fixture_implementation(&root);

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.check == "implementation.rust.typing.public-fields"));
    }

    #[test]
    fn rust_typing_rejects_panic_and_unwrap_in_domain_code() {
        let root = rust_typing_fixture(
            "failure-discipline",
            &["core"],
            "",
            "pub fn parse_widget(value: Option<&str>) -> &str {\n    value.unwrap_or_else(|| panic!(\"missing\"))\n}\n",
        );

        let diagnostics = validate_fixture_implementation(&root);

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.check == "implementation.rust.typing.failure-discipline"
        }));
    }

    #[test]
    fn rust_typing_requires_stateful_representation_declaration() {
        let root = rust_typing_fixture(
            "stateful-representation",
            &["core", "stateful"],
            "",
            "pub enum WidgetState {\n    Draft,\n    Active,\n}\n",
        );

        let diagnostics = validate_fixture_implementation(&root);

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.check == "implementation.rust.typing.stateful-representation"
        }));
    }

    #[test]
    fn rust_typing_accepts_declared_stateful_representation() {
        let root = rust_typing_fixture(
            "stateful-ok",
            &["core", "stateful"],
            "  state_type: WidgetState\n  transition_function: transition_widget\n",
            "pub enum WidgetState {\n    Draft,\n    Active,\n}\n\npub fn transition_widget(state: WidgetState) -> WidgetState {\n    state\n}\n",
        );

        let diagnostics = validate_fixture_implementation(&root);

        fs::remove_dir_all(&root).unwrap();
        assert!(
            !diagnostics.iter().any(|diagnostic| diagnostic
                .check
                .starts_with("implementation.rust.typing.state")),
            "{diagnostics:#?}"
        );
    }

    #[test]
    fn rust_semantic_functions_reject_missing_symbols() {
        let root = rust_typing_fixture(
            "semantic-missing-symbol",
            &["core"],
            "\nsemantic_functions:\n  - id: missing-decision\n    symbol: missing_decision\n    kind: decision\n    purity: pure\n",
            "pub fn existing_decision() {}\n",
        );

        let diagnostics = validate_fixture_implementation(&root);

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.check == "implementation.rust.semantic-functions.symbol"
        }));
    }

    #[test]
    fn semantic_functions_reject_unknown_invariants() {
        let root = rust_typing_fixture(
            "semantic-unknown-invariant",
            &["core"],
            "\nsemantic_functions:\n  - id: existing-decision\n    symbol: existing_decision\n    kind: decision\n    purity: pure\n    discharges:\n      invariants:\n        - missing-invariant\n",
            "pub fn existing_decision() {}\n",
        );

        let diagnostics = validate_fixture_implementation(&root);

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.check == "implementation.semantic-functions.invariant"
        }));
    }

    #[test]
    fn rust_semantic_functions_accept_method_symbols() {
        let mut summary = RustTypingSummary::default();
        summary
            .impl_methods
            .insert("Widget".to_string(), BTreeSet::from(["new".to_string()]));

        assert!(rust_symbol_exists(&summary, "Widget::new"));
        assert!(rust_symbol_exists(&summary, "crate::widget::Widget::new"));
        assert!(!rust_symbol_exists(&summary, "Widget::missing"));
    }

    #[test]
    fn swift_typing_rejects_public_primitive_aliases() {
        let root = swift_typing_fixture(
            "swift-primitive-alias",
            &["core"],
            "",
            "public typealias WidgetId = String\n",
        );

        let diagnostics = validate_fixture_implementation(&root);

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.check == "implementation.swift.typing.primitive-alias"));
    }

    #[test]
    fn swift_typing_rejects_public_domain_fields() {
        let root = swift_typing_fixture(
            "swift-public-fields",
            &["core"],
            "",
            "public struct Widget {\n    public let name: String\n}\n",
        );

        let diagnostics = validate_fixture_implementation(&root);

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.check == "implementation.swift.typing.public-fields"));
    }

    #[test]
    fn swift_typing_rejects_traps_in_domain_code() {
        let root = swift_typing_fixture(
            "swift-traps",
            &["core"],
            "",
            "public func parseWidget(_ value: String?) -> String {\n    guard let value else { fatalError(\"missing\") }\n    return value\n}\n",
        );

        let diagnostics = validate_fixture_implementation(&root);

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.check == "implementation.swift.typing.failure-discipline"
        }));
    }

    #[test]
    fn swift_typing_requires_stateful_representation_declaration() {
        let root = swift_typing_fixture(
            "swift-stateful-representation",
            &["core", "stateful"],
            "",
            "public enum WidgetState {\n    case draft\n    case active\n}\n",
        );

        let diagnostics = validate_fixture_implementation(&root);

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.check == "implementation.swift.typing.stateful-representation"
        }));
    }

    #[test]
    fn swift_typing_accepts_declared_stateful_representation() {
        let root = swift_typing_fixture(
            "swift-stateful-ok",
            &["core", "stateful"],
            "  state_type: WidgetState\n  transition_function: transitionWidget\n",
            "public enum WidgetState {\n    case draft\n    case active\n}\n\npublic func transitionWidget(_ state: WidgetState) -> WidgetState {\n    state\n}\n",
        );

        let diagnostics = validate_fixture_implementation(&root);

        fs::remove_dir_all(&root).unwrap();
        assert!(
            !diagnostics.iter().any(|diagnostic| diagnostic
                .check
                .starts_with("implementation.swift.typing.state")),
            "{diagnostics:#?}"
        );
    }

    #[test]
    fn compat_reports_additive_public_surface() {
        let old = loaded_module_manifest(
            "old.yaml",
            r#"
spec: rms/module/v0.1
module:
  name: example
  version: 1.0.0
  kind: library
  purpose: Example
profiles: [core]
owns: {}
provides:
  commands: []
  queries: []
  events: []
  capabilities: []
requires:
  modules: []
  capabilities: []
invariants: []
effects: []
compatibility:
  policy: backward-compatible-within-major
verification:
  laws: []
  contracts: []
  scenarios: []
  boundaries: []
"#,
        );
        let new = loaded_module_manifest(
            "new.yaml",
            r#"
spec: rms/module/v0.1
module:
  name: example
  version: 1.1.0
  kind: library
  purpose: Example
profiles: [core]
owns: {}
provides:
  commands:
    - name: do-work
      contract: contracts/do-work.yaml
  queries: []
  events: []
  capabilities: []
requires:
  modules: []
  capabilities: []
invariants: []
effects: []
compatibility:
  policy: backward-compatible-within-major
verification:
  laws: []
  contracts: []
  scenarios: []
  boundaries: []
"#,
        );

        let report = check_module_compat(&old, &new).unwrap();

        assert_eq!(report.result, CompatResult::CompatibleAdditive);
    }

    #[test]
    fn compat_reports_removed_public_surface_as_breaking() {
        let old = loaded_module_manifest(
            "old.yaml",
            r#"
spec: rms/module/v0.1
module:
  name: example
  version: 1.0.0
  kind: library
  purpose: Example
profiles: [core]
owns: {}
provides:
  commands:
    - name: do-work
      contract: contracts/do-work.yaml
  queries: []
  events: []
  capabilities: []
requires:
  modules: []
  capabilities: []
invariants: []
effects: []
compatibility:
  policy: backward-compatible-within-major
verification:
  laws: []
  contracts: []
  scenarios: []
  boundaries: []
"#,
        );
        let new = loaded_module_manifest(
            "new.yaml",
            r#"
spec: rms/module/v0.1
module:
  name: example
  version: 1.1.0
  kind: library
  purpose: Example
profiles: [core]
owns: {}
provides:
  commands: []
  queries: []
  events: []
  capabilities: []
requires:
  modules: []
  capabilities: []
invariants: []
effects: []
compatibility:
  policy: backward-compatible-within-major
verification:
  laws: []
  contracts: []
  scenarios: []
  boundaries: []
"#,
        );

        let report = check_module_compat(&old, &new).unwrap();

        assert_eq!(report.result, CompatResult::Breaking);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.check == "provides.removed"));
    }

    #[test]
    fn compat_reports_effect_changes_for_operational_review() {
        let old = loaded_module_manifest(
            "old.yaml",
            r#"
spec: rms/module/v0.1
module:
  name: example
  version: 1.0.0
  kind: library
  purpose: Example
profiles: [core]
owns: {}
provides:
  commands: []
  queries: []
  events: []
  capabilities: []
requires:
  modules: []
  capabilities: []
invariants: []
effects: []
compatibility:
  policy: backward-compatible-within-major
verification:
  laws: []
  contracts: []
  scenarios: []
  boundaries: []
"#,
        );
        let new = loaded_module_manifest(
            "new.yaml",
            r#"
spec: rms/module/v0.1
module:
  name: example
  version: 1.1.0
  kind: library
  purpose: Example
profiles: [core]
owns: {}
provides:
  commands: []
  queries: []
  events: []
  capabilities: []
requires:
  modules: []
  capabilities: []
invariants: []
effects:
  - name: network
    kind: external-network
compatibility:
  policy: backward-compatible-within-major
verification:
  laws: []
  contracts: []
  scenarios: []
  boundaries: []
"#,
        );

        let report = check_module_compat(&old, &new).unwrap();

        assert_eq!(report.result, CompatResult::OperationalReviewRequired);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.check == "effects.added"));
    }

    #[test]
    fn compose_satisfies_module_provided_capability() {
        let root = unique_test_dir("compose-satisfied");
        fs::create_dir_all(&root).unwrap();
        write_compose_module(
            &root.join("provider.module.yaml"),
            "provider",
            "  capabilities:\n    - name: send-email\n      contract: contracts/send-email.yaml\n",
            "  modules: []\n",
            "  capabilities: []\n",
        );
        write_compose_module(
            &root.join("consumer.module.yaml"),
            "consumer",
            "  capabilities: []\n",
            "  modules: []\n",
            "  capabilities:\n    - name: send-email\n      contract: contracts/send-email.yaml\n",
        );

        let report = compose_system(&root).unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(report.result, ComposeResult::Pass);
        assert!(report.findings.iter().any(|finding| {
            finding.status == ComposeStatus::Satisfied
                && finding.check == "requires.capabilities.provider"
        }));
    }

    #[test]
    fn compose_reports_unresolved_capability() {
        let root = unique_test_dir("compose-unresolved");
        fs::create_dir_all(&root).unwrap();
        write_compose_module(
            &root.join("consumer.module.yaml"),
            "consumer",
            "  capabilities: []\n",
            "  modules: []\n",
            "  capabilities:\n    - name: send-email\n      contract: contracts/send-email.yaml\n",
        );

        let report = compose_system(&root).unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(report.result, ComposeResult::Fail);
        assert!(report.findings.iter().any(|finding| {
            finding.status == ComposeStatus::Unresolved
                && finding.check == "requires.capabilities.provider"
        }));
    }

    #[test]
    fn compose_unions_external_dependencies_from_discovered_systems() {
        let root = unique_test_dir("compose-external-union");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("alpha.system.yaml"),
            "spec: rms/system/v0.1\n\nsystem:\n  name: alpha\n  version: 0.1.0\n  purpose: First system\n\ncontexts: []\npublic_interfaces: []\nexternal_dependencies: []\nworkflows: []\ninvariants: []\ncompatibility:\n  policy: backward-compatible-within-major\n",
        )
        .unwrap();
        fs::write(
            root.join("beta.system.yaml"),
            "spec: rms/system/v0.1\n\nsystem:\n  name: beta\n  version: 0.1.0\n  purpose: Second system\n\ncontexts: []\npublic_interfaces: []\nexternal_dependencies:\n  - send-email\nworkflows: []\ninvariants: []\ncompatibility:\n  policy: backward-compatible-within-major\n",
        )
        .unwrap();
        fs::write(
            root.join("consumer.module.yaml"),
            "spec: rms/module/v0.1\n\nmodule:\n  name: consumer\n  version: 0.1.0\n  kind: library\n  purpose: Test composition\n\nprofiles:\n  - core\n\nowns:\n  concepts: []\n  data: []\n  decisions: []\n\nprovides:\n  commands: []\n  queries: []\n  events: []\n  capabilities: []\nrequires:\n  modules: []\n  capabilities:\n    - name: send-email\n      contract: contracts/send-email.yaml\ninvariants: []\n\neffects:\n  - name: send-email\n    kind: external-message\n    capability: send-email\n\ncompatibility:\n  policy: backward-compatible-within-major\n\nverification:\n  laws: []\n  contracts: []\n  scenarios: []\n  boundaries: []\n",
        )
        .unwrap();

        let report = compose_system(&root).unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(report.result, ComposeResult::Pass);
        assert!(report.findings.iter().any(|finding| {
            finding.status == ComposeStatus::Satisfied
                && finding.check == "requires.capabilities.external"
                && finding.requirement.as_deref() == Some("send-email")
        }));
    }

    #[test]
    fn compose_reports_module_dependency_cycles() {
        let root = unique_test_dir("compose-cycle");
        fs::create_dir_all(&root).unwrap();
        write_compose_module(
            &root.join("alpha.module.yaml"),
            "alpha",
            "  capabilities: []\n",
            "  modules:\n    - beta\n",
            "  capabilities: []\n",
        );
        write_compose_module(
            &root.join("beta.module.yaml"),
            "beta",
            "  capabilities: []\n",
            "  modules:\n    - alpha\n",
            "  capabilities: []\n",
        );

        let report = compose_system(&root).unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(report.result, ComposeResult::Fail);
        assert!(report.findings.iter().any(|finding| {
            finding.status == ComposeStatus::Incompatible
                && finding.check == "requires.modules.cycle"
        }));
    }

    #[test]
    fn package_includes_manifest_references_and_metadata() {
        let root = unique_test_dir("package");
        write_package_fixture(&root);
        let output = root.join("out.rms");

        package_module(&root.join("module.yaml"), Some(&output), false).unwrap();

        assert!(output.join("module.yaml").exists());
        assert!(output.join("contracts/do-work.yaml").exists());
        assert!(output.join("verification/laws/law").exists());
        assert!(output.join("conformance-report.json").exists());
        let package_manifest = fs::read_to_string(output.join("PACKAGE.json")).unwrap();
        assert!(package_manifest.contains("\"spec\": \"rms/package/v0.1\""));
        assert!(package_manifest.contains("\"module\": \"package-fixture\""));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn verify_package_accepts_clean_package_and_rejects_tampering() {
        let root = unique_test_dir("verify-package");
        write_package_fixture(&root);
        let output = root.join("out.rms");
        package_module(&root.join("module.yaml"), Some(&output), false).unwrap();

        let report = verify_package(&output).unwrap();
        assert_eq!(report.result, VerifyPackageResult::Pass);

        fs::write(output.join("contracts/do-work.yaml"), "tampered\n").unwrap();
        let report = verify_package(&output).unwrap();
        assert_eq!(report.result, VerifyPackageResult::Fail);
        assert!(report.findings.iter().any(|finding| {
            finding.status == VerifyPackageStatus::Fail && finding.check == "package.file.sha256"
        }));

        fs::remove_dir_all(&root).unwrap();
    }

    fn write_package_fixture(root: &Path) {
        fs::create_dir_all(root.join("contracts")).unwrap();
        fs::create_dir_all(root.join("verification/laws")).unwrap();
        fs::write(
            root.join("contracts/do-work.yaml"),
            "spec: rms/contract/v0.1\nname: do-work\nkind: command\nmeaning: Do work.\n",
        )
        .unwrap();
        fs::write(root.join("verification/laws/law"), "law evidence\n").unwrap();
        fs::write(
            root.join("module.yaml"),
            r#"spec: rms/module/v0.1

module:
  name: package-fixture
  version: 0.1.0
  kind: library
  purpose: Test packaging

profiles:
  - core

owns:
  concepts: []
  data: []
  decisions: []

provides:
  commands:
    - name: do-work
      contract: contracts/do-work.yaml
  queries: []
  events: []
  capabilities: []

requires:
  modules: []
  capabilities: []

invariants:
  - id: law
    statement: Law holds.
    verified_by: verification/laws/law

effects: []

boundary:
  validation: reject-before-domain-entry

compatibility:
  policy: backward-compatible-within-major

verification:
  laws:
    - verification/laws
  contracts: []
  scenarios: []
  boundaries: []
"#,
        )
        .unwrap();
    }

    #[test]
    fn module_atlas_derives_stable_nodes_from_manifest() {
        let root = atlas_fixture("atlas-graph");
        let manifest = load_manifest(&root.join("module.yaml")).unwrap();

        let atlas = build_module_atlas(&manifest, &root).unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(atlas.spec, "rms/atlas/v0.1");
        assert_eq!(atlas.module.id, "module:atlas-fixture");
        assert!(atlas.interaction.supports_live_reconciliation);
        assert!(atlas.nodes.iter().any(|node| {
            node.id == "provides-commands:do-work"
                && node.layer == "public-surface"
                && node
                    .source_refs
                    .iter()
                    .any(|reference| reference.path == "contracts/do-work.v1.yaml")
        }));
        assert!(atlas
            .nodes
            .iter()
            .any(|node| node.id == "invariant:work-is-safe"));
        assert!(atlas.edges.iter().any(|edge| {
            edge.kind == "verifies"
                && edge.from == "verification:verification-laws-work-is-safe"
                && edge.to == "invariant:work-is-safe"
        }));
        let trace = atlas
            .traces
            .iter()
            .find(|trace| trace.id == "trace:do-work")
            .expect("do-work trace");
        assert_eq!(trace.entry_node_id, "provides-commands:do-work");
        let rule_step = trace
            .steps
            .iter()
            .find(|step| {
                step.role == "Rule"
                    && step.confidence == "inferred"
                    && step
                        .node_ids
                        .contains(&"invariant:work-is-safe".to_string())
            })
            .expect("rule trace step");
        assert_eq!(
            rule_step.reading.impact,
            "Changing this stage can weaken or move domain authority."
        );
        assert!(trace
            .gaps
            .iter()
            .any(|gap| gap.id == "effect" && gap.suggested_artifact.is_some()));
        assert!(atlas
            .tours
            .first()
            .is_some_and(|tour| !tour.steps.is_empty()));
    }

    #[test]
    fn atlas_command_writes_json_and_html_artifacts() {
        let root = atlas_fixture("atlas-write");
        let output = root.join("atlas-out");

        run_atlas(&root.join("module.yaml"), &root, Some(&output), false).unwrap();

        let atlas_json = fs::read_to_string(output.join("atlas.json")).unwrap();
        let html = fs::read_to_string(output.join("index.html")).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert!(atlas_json.contains("\"spec\": \"rms/atlas/v0.1\""));
        assert!(atlas_json.contains("\"traces\""));
        assert!(atlas_json.contains("\"trace:do-work\""));
        assert!(atlas_json.contains("\"confidence\": \"inferred\""));
        assert!(atlas_json.contains("\"clauses\""));
        assert!(atlas_json.contains("\"reading\""));
        assert!(atlas_json.contains("\"promise\""));
        assert!(atlas_json.contains("\"justification\""));
        assert!(atlas_json.contains("Not declared by this command contract."));
        assert!(atlas_json.contains("\"supports_live_reconciliation\": true"));
        assert!(html.contains("RMS Atlas"));
        assert!(html.contains("Human story"));
        assert!(html.contains("In plain words"));
        assert!(html.contains("Meaning"));
        assert!(html.contains("Show evidence"));
        assert!(!html.contains("three@0.164.1"));
        assert!(html.contains("atlas-data"));
    }

    #[test]
    fn workbench_prompt_renders_bounded_context() {
        let root = prompt_fixture("prompt-render");
        let manifest = load_manifest(&root.join("module.yaml")).unwrap();

        let rendered = render_workbench_prompt(
            &manifest,
            &root,
            PromptKind::Plan,
            Some("add a command"),
            None,
            false,
        )
        .unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert!(rendered.contains("Prompt: rms.plan@v1"));
        assert!(rendered.contains("Mode: advisory"));
        assert!(rendered.contains("## Bounded RMS Context"));
        assert!(rendered.contains("rms validate --root <root>"));
    }

    #[test]
    fn refactor_prompt_preserves_public_meaning() {
        let root = prompt_fixture("refactor-render");
        let manifest = load_manifest(&root.join("module.yaml")).unwrap();

        let rendered = render_workbench_prompt(
            &manifest,
            &root,
            PromptKind::Refactor,
            Some("separate decisions from effects"),
            None,
            false,
        )
        .unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert!(rendered.contains("Prompt: rms.refactor@v1"));
        assert!(rendered.contains("Preserve public contracts"));
        assert!(rendered.contains("Escalate to implement-change or evolve-contract"));
    }

    #[test]
    fn implement_prompt_classifies_change_before_steps() {
        let root = prompt_fixture("implement-render");
        let manifest = load_manifest(&root.join("module.yaml")).unwrap();

        let rendered = render_workbench_prompt(
            &manifest,
            &root,
            PromptKind::Implement,
            Some("add a public command"),
            None,
            false,
        )
        .unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert!(rendered.contains("Prompt: rms.implement@v1"));
        assert!(rendered.contains("Classify the change as private implementation"));
        assert!(rendered.contains("Contract/manifest updates required before code changes"));
        assert!(rendered.contains("do not claim edits were made"));
    }

    #[test]
    fn evolve_contract_prompt_classifies_compatibility() {
        let root = prompt_fixture("evolve-render");
        let manifest = load_manifest(&root.join("module.yaml")).unwrap();

        let rendered = render_workbench_prompt(
            &manifest,
            &root,
            PromptKind::EvolveContract,
            Some("change command failure semantics"),
            None,
            false,
        )
        .unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert!(rendered.contains("Prompt: rms.evolve-contract@v1"));
        assert!(rendered.contains("Classify compatibility impact"));
        assert!(rendered.contains("Migration, coexistence, translation, and deprecation plan"));
        assert!(rendered.contains("rms check-compat <old-module.yaml> <new-module.yaml>"));
    }

    #[test]
    fn evidence_prompt_names_smallest_proof() {
        let root = prompt_fixture("evidence-render");
        let manifest = load_manifest(&root.join("module.yaml")).unwrap();

        let rendered = render_workbench_prompt(
            &manifest,
            &root,
            PromptKind::Evidence,
            Some("prove malformed input is rejected"),
            None,
            false,
        )
        .unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert!(rendered.contains("Prompt: rms.evidence@v1"));
        assert!(rendered.contains("Prefer the smallest evidence"));
        assert!(rendered.contains("Manifest or implementation binding references to update"));
    }

    #[test]
    fn explain_subject_infers_module_and_question() {
        let root = prompt_fixture("explain-infer");
        let subject = vec!["how does this module work?".to_string()];

        let (module, question) = resolve_explain_subject(&subject, None, &root).unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            module.file_name().and_then(|name| name.to_str()),
            Some("module.yaml")
        );
        assert_eq!(question.as_deref(), Some("how does this module work?"));
    }

    #[test]
    fn explain_prompt_answers_from_bounded_context() {
        let root = prompt_fixture("explain-prompt");
        let manifest = load_manifest(&root.join("module.yaml")).unwrap();

        let rendered = render_workbench_prompt(
            &manifest,
            &root,
            PromptKind::Explain,
            Some("how does this module work?"),
            None,
            false,
        )
        .unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert!(rendered.contains("Prompt: rms.explain@v1"));
        assert!(rendered.contains("Intelligible plain-language explanation"));
        assert!(rendered.contains("Do not invent architecture"));
    }

    #[test]
    fn review_prompt_can_include_impact_report() {
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }

        let root = prompt_fixture("review-impact");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn example() {}\n").unwrap();
        fs::write(
            root.join("implementation.yaml"),
            "spec: rms/implementation/v0.1\n\nmodule: prompt-fixture\nbinding: rust\n\nsource:\n  root: src\n  public_entrypoint: src/lib.rs\n\ncommands:\n  verify: cargo test --manifest-path Cargo.toml\n\ntoolchain:\n  cargo_manifest: Cargo.toml\n  package: prompt-fixture\n\ndependencies:\n  allowed_external_crates: []\n\narchitecture:\n  public_modules: []\n",
        )
        .unwrap();

        let init = Command::new("git")
            .arg("init")
            .current_dir(&root)
            .output()
            .unwrap();
        if !init.status.success() {
            fs::remove_dir_all(&root).unwrap();
            return;
        }
        Command::new("git")
            .args(["config", "user.email", "rms@example.test"])
            .current_dir(&root)
            .status()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "RMS Test"])
            .current_dir(&root)
            .status()
            .unwrap();
        let add = Command::new("git")
            .args(["add", "."])
            .current_dir(&root)
            .status()
            .unwrap();
        if !add.success() {
            fs::remove_dir_all(&root).unwrap();
            return;
        }
        let commit = Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(&root)
            .output()
            .unwrap();
        if !commit.status.success() {
            fs::remove_dir_all(&root).unwrap();
            return;
        }

        fs::write(
            root.join("src/lib.rs"),
            "pub fn example() -> bool { true }\n",
        )
        .unwrap();
        let manifest = load_manifest(&root.join("module.yaml")).unwrap();
        let rendered =
            render_workbench_prompt(&manifest, &root, PromptKind::Review, None, None, true)
                .unwrap();
        let options = PromptRunOptions {
            provider: Provider::None,
            record: true,
            run_root: PathBuf::from("runs"),
            model: None,
            sandbox: CodexSandbox::ReadOnly,
            write_scope: ProviderWriteScope::Root,
        };
        let run_dir = write_prompt_run_record(
            &manifest,
            &root,
            PromptKind::Review,
            None,
            None,
            true,
            &rendered,
            &options,
        )
        .unwrap();
        let request = fs::read_to_string(run_dir.join("request.yaml")).unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert!(rendered.contains("Prompt: rms.review@v1"));
        assert!(rendered.contains("## Impact"));
        assert!(rendered.contains("Derived from git changed paths"));
        assert!(rendered.contains("- Result: implementation-only"));
        assert!(rendered.contains("src/lib.rs"));
        assert!(rendered.contains("## Diff"));
        assert!(request.contains("impact: true"));
        assert!(request.contains("write_scope: \"root\""));
    }

    #[test]
    fn impact_prelude_is_review_only() {
        let root = prompt_fixture("review-impact-only");
        let manifest = load_manifest(&root.join("module.yaml")).unwrap();

        let error = render_workbench_prompt(
            &manifest,
            &root,
            PromptKind::Plan,
            Some("inspect impact"),
            None,
            true,
        )
        .unwrap_err()
        .to_string();

        fs::remove_dir_all(&root).unwrap();
        assert!(error.contains("`--impact` is only supported for review prompts"));
    }

    #[test]
    fn workbench_run_record_writes_prompt_request_and_checks() {
        let root = prompt_fixture("run-record");
        let manifest = load_manifest(&root.join("module.yaml")).unwrap();
        let options = PromptRunOptions {
            provider: Provider::None,
            record: true,
            run_root: PathBuf::from("runs"),
            model: None,
            sandbox: CodexSandbox::ReadOnly,
            write_scope: ProviderWriteScope::Root,
        };

        let prompt = render_workbench_prompt(
            &manifest,
            &root,
            PromptKind::Evidence,
            Some("prove prompt rendering"),
            None,
            false,
        )
        .unwrap();
        let run_dir = write_prompt_run_record(
            &manifest,
            &root,
            PromptKind::Evidence,
            Some("prove prompt rendering"),
            None,
            false,
            &prompt,
            &options,
        )
        .unwrap();

        assert!(run_dir.join("request.yaml").exists());
        assert!(run_dir.join("prompt.md").exists());
        assert!(run_dir.join("checks.json").exists());
        let request = fs::read_to_string(run_dir.join("request.yaml")).unwrap();
        let checks = fs::read_to_string(run_dir.join("checks.json")).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert!(request.contains("prompt: \"rms.evidence@v1\""));
        assert!(request.contains("provider: \"none\""));
        assert!(request.contains("write_scope: \"root\""));
        assert!(request.contains("execution_root:"));
        assert!(checks.contains("\"validation\""));
    }

    #[test]
    fn provider_module_write_scope_uses_module_execution_root() {
        let root = unique_test_dir("provider-module-scope");
        let module_dir = root.join("modules/widget");
        fs::create_dir_all(&module_dir).unwrap();
        fs::write(
            module_dir.join("module.yaml"),
            render_module_yaml(
                "widget",
                "Own widget behavior",
                "module",
                &["core".to_string()],
            ),
        )
        .unwrap();
        let manifest = load_manifest(&module_dir.join("module.yaml")).unwrap();
        let options = PromptRunOptions {
            provider: Provider::Codex,
            record: true,
            run_root: PathBuf::from("runs"),
            model: None,
            sandbox: CodexSandbox::WorkspaceWrite,
            write_scope: ProviderWriteScope::Module,
        };

        let execution_root = provider_execution_root(&root, &manifest, &options);
        let scope = render_provider_execution_scope(&manifest, &root, &options);

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(execution_root, module_dir);
        assert!(scope.contains("- Sandbox: workspace-write"));
        assert!(scope.contains("- Write scope: module"));
        assert!(scope.contains("Edit only files under the owning module directory"));
    }

    #[test]
    fn provider_response_path_is_absolute_for_module_cd() {
        let response = provider_response_path(Path::new(".rms/runs/test-run")).unwrap();

        assert!(response.is_absolute());
        assert!(response.ends_with(".rms/runs/test-run/response.md"));
    }

    #[test]
    fn source_revision_uses_requested_root() {
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }

        let root = unique_test_dir("git-root");
        fs::create_dir_all(&root).unwrap();
        let init = Command::new("git")
            .arg("init")
            .current_dir(&root)
            .output()
            .unwrap();
        if !init.status.success() {
            fs::remove_dir_all(&root).unwrap();
            return;
        }
        Command::new("git")
            .args(["config", "user.email", "rms@example.test"])
            .current_dir(&root)
            .status()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "RMS Test"])
            .current_dir(&root)
            .status()
            .unwrap();
        fs::write(root.join("marker.txt"), "marker\n").unwrap();
        Command::new("git")
            .args(["add", "marker.txt"])
            .current_dir(&root)
            .status()
            .unwrap();
        let commit = Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(&root)
            .output()
            .unwrap();
        if !commit.status.success() {
            fs::remove_dir_all(&root).unwrap();
            return;
        }
        let expected = Command::new("git")
            .args(["rev-parse", "--short=12", "HEAD"])
            .current_dir(&root)
            .output()
            .unwrap();
        let expected = format!("git:{}", String::from_utf8_lossy(&expected.stdout).trim());

        let actual = source_revision(&root);

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(actual.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn impact_classifies_contract_and_source_paths() {
        let root = prompt_fixture("impact-owned");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("contracts")).unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn example() {}\n").unwrap();
        fs::write(
            root.join("implementation.yaml"),
            "spec: rms/implementation/v0.1\n\nmodule: prompt-fixture\nbinding: rust\n\nsource:\n  root: src\n  public_entrypoint: src/lib.rs\n\ncommands:\n  verify: cargo test --manifest-path Cargo.toml\n\ntoolchain:\n  cargo_manifest: Cargo.toml\n  package: prompt-fixture\n\ndependencies:\n  allowed_external_crates: []\n\narchitecture:\n  public_modules: []\n",
        )
        .unwrap();
        let changed = vec![
            ChangedPath {
                status: "M".to_string(),
                path: "contracts/do-work.v1.yaml".to_string(),
            },
            ChangedPath {
                status: "M".to_string(),
                path: "src/lib.rs".to_string(),
            },
            ChangedPath {
                status: "M".to_string(),
                path: "verification/contracts/do_work.md".to_string(),
            },
        ];

        let report = build_impact_report(&root, None, &changed).unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(report.result, ImpactResult::ReviewRequired);
        assert_eq!(report.affected_modules.len(), 1);
        assert_eq!(report.affected_modules[0].name, "prompt-fixture");
        assert!(report.changed_paths.iter().any(|path| {
            path.path == "contracts/do-work.v1.yaml"
                && path.category == ImpactCategory::Contract
                && path.module.as_deref() == Some("prompt-fixture")
        }));
        assert!(report.changed_paths.iter().any(|path| {
            path.path == "src/lib.rs"
                && path.category == ImpactCategory::Source
                && path.module.as_deref() == Some("prompt-fixture")
        }));
        assert!(report.changed_paths.iter().any(|path| {
            path.path == "verification/contracts/do_work.md"
                && path.category == ImpactCategory::VerificationEvidence
                && path.module.as_deref() == Some("prompt-fixture")
        }));
        assert!(report
            .recommended_checks
            .iter()
            .any(|check| check == "rms verify implementation.yaml"));
    }

    #[test]
    fn impact_reports_unowned_paths_without_semantic_authority() {
        let root = unique_test_dir("impact-unowned");
        fs::create_dir_all(&root).unwrap();
        let changed = vec![ChangedPath {
            status: "M".to_string(),
            path: "notes/idea.md".to_string(),
        }];

        let report = build_impact_report(&root, None, &changed).unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(report.result, ImpactResult::NoRmsImpact);
        assert!(report.affected_modules.is_empty());
        assert!(report.findings.is_empty());
        assert_eq!(report.changed_paths[0].category, ImpactCategory::Other);
        assert_eq!(report.changed_paths[0].module, None);
    }

    #[test]
    fn impact_git_paths_are_normalized_against_requested_root() {
        let mut paths = BTreeMap::new();

        insert_changed_path(
            Path::new("modules/widget"),
            &mut paths,
            "M",
            "modules/widget/src/lib.rs",
        );

        assert_eq!(
            paths.get("src/lib.rs").map(|path| path.status.as_str()),
            Some("M")
        );
    }

    #[test]
    fn gate_plan_skips_unrelated_paths() {
        let root = unique_test_dir("gate-unrelated");
        fs::create_dir_all(&root).unwrap();
        let changed = vec![ChangedPath {
            status: "M".to_string(),
            path: "notes/idea.md".to_string(),
        }];
        let report = build_impact_report(&root, None, &changed).unwrap();

        let plan = build_gate_plan(&root, None, &report);

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(plan.report.impact_result, ImpactResult::NoRmsImpact);
        assert_eq!(plan.report.result, GateResult::Pass);
        assert!(plan.report.executable_checks.is_empty());
        assert!(plan.report.manual_checks.is_empty());
    }

    #[test]
    fn gate_plan_runs_verify_for_source_changes() {
        let root = prompt_fixture("gate-source");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn example() {}\n").unwrap();
        fs::write(
            root.join("implementation.yaml"),
            "spec: rms/implementation/v0.1\n\nmodule: prompt-fixture\nbinding: rust\n\nsource:\n  root: src\n  public_entrypoint: src/lib.rs\n\ncommands:\n  verify: cargo test --manifest-path Cargo.toml\n\ntoolchain:\n  cargo_manifest: Cargo.toml\n  package: prompt-fixture\n\ndependencies:\n  allowed_external_crates: []\n\narchitecture:\n  public_modules: []\n",
        )
        .unwrap();
        let changed = vec![ChangedPath {
            status: "M".to_string(),
            path: "src/lib.rs".to_string(),
        }];
        let report = build_impact_report(&root, None, &changed).unwrap();

        let plan = build_gate_plan(&root, None, &report);
        let commands = plan
            .report
            .executable_checks
            .iter()
            .map(|check| check.command.as_str())
            .collect::<Vec<_>>();

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(plan.report.impact_result, ImpactResult::ImplementationOnly);
        assert_eq!(plan.report.result, GateResult::Pending);
        assert!(commands
            .iter()
            .any(|command| command.starts_with("rms validate --root ")));
        assert!(commands.contains(&"rms verify implementation.yaml"));
        assert!(plan.report.manual_checks.is_empty());
    }

    #[test]
    fn gate_plan_marks_contract_changes_for_review_and_compatibility() {
        let root = prompt_fixture("gate-contract");
        fs::create_dir_all(root.join("contracts")).unwrap();
        fs::write(
            root.join("contracts/do-work.v1.yaml"),
            "spec: rms/contract/v0.1\nname: do-work\nversion: 1\nkind: command\nmeaning: Do work.\n",
        )
        .unwrap();
        let changed = vec![ChangedPath {
            status: "M".to_string(),
            path: "contracts/do-work.v1.yaml".to_string(),
        }];
        let report = build_impact_report(&root, Some("HEAD~1..HEAD"), &changed).unwrap();

        let plan = build_gate_plan(&root, Some("HEAD~1..HEAD"), &report);
        let commands = plan
            .report
            .executable_checks
            .iter()
            .map(|check| check.command.as_str())
            .collect::<Vec<_>>();

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(plan.report.impact_result, ImpactResult::ReviewRequired);
        assert!(commands
            .iter()
            .any(|command| command.starts_with("rms validate --root ")));
        assert!(commands
            .iter()
            .any(|command| command.starts_with("rms compose --root ")));
        assert!(plan
            .report
            .manual_checks
            .iter()
            .any(|check| check == "rms review module.yaml --impact --diff HEAD~1..HEAD"));
        assert!(plan
            .report
            .manual_checks
            .iter()
            .any(|check| { check == "rms check-compat <previous module.yaml> module.yaml" }));
    }

    #[test]
    fn run_list_and_inspect_read_generated_records() {
        let root = prompt_fixture("run-read");
        let manifest = load_manifest(&root.join("module.yaml")).unwrap();
        let options = PromptRunOptions {
            provider: Provider::None,
            record: true,
            run_root: PathBuf::from("runs"),
            model: None,
            sandbox: CodexSandbox::ReadOnly,
            write_scope: ProviderWriteScope::Root,
        };
        let prompt = render_workbench_prompt(
            &manifest,
            &root,
            PromptKind::Plan,
            Some("inspect saved run"),
            None,
            false,
        )
        .unwrap();
        let run_dir = write_prompt_run_record(
            &manifest,
            &root,
            PromptKind::Plan,
            Some("inspect saved run"),
            None,
            false,
            &prompt,
            &options,
        )
        .unwrap();
        fs::write(run_dir.join("response.md"), "response body\n").unwrap();
        let run_id = run_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap()
            .to_string();

        run_list_runs(&root, Path::new("runs")).unwrap();
        run_inspect_run(Path::new(&run_id), &root, Path::new("runs")).unwrap();

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn prompt_options_use_configured_ai_defaults() {
        let root = prompt_fixture("configured-ai");
        fs::create_dir_all(root.join(".rms")).unwrap();
        fs::write(
            root.join(".rms/config.yaml"),
            r#"ai:
  default_provider: codex
  codex:
    model: gpt-test
    sandbox: read-only
runs:
  directory: .rms/test-runs
"#,
        )
        .unwrap();

        let options = resolve_prompt_run_options(
            &root,
            RawPromptRunOptions {
                ai: true,
                provider: None,
                record: false,
                run_root: None,
                model: None,
                sandbox: None,
                write_scope: None,
            },
        )
        .unwrap();
        let run_root = resolve_run_root(&root, None).unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(options.provider, Provider::Codex);
        assert_eq!(options.model.as_deref(), Some("gpt-test"));
        assert!(matches!(options.sandbox, CodexSandbox::ReadOnly));
        assert_eq!(options.write_scope, ProviderWriteScope::Root);
        assert_eq!(options.run_root, PathBuf::from(".rms/test-runs"));
        assert_eq!(run_root, PathBuf::from(".rms/test-runs"));
    }

    #[test]
    fn prompt_options_default_workspace_write_to_module_scope() {
        let root = prompt_fixture("workspace-write-default");
        fs::create_dir_all(root.join(".rms")).unwrap();
        fs::write(
            root.join(".rms/config.yaml"),
            r#"ai:
  default_provider: codex
  codex:
    sandbox: workspace-write
runs:
  directory: .rms/test-runs
"#,
        )
        .unwrap();

        let options = resolve_prompt_run_options(
            &root,
            RawPromptRunOptions {
                ai: true,
                provider: None,
                record: false,
                run_root: None,
                model: None,
                sandbox: None,
                write_scope: None,
            },
        )
        .unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert!(matches!(options.sandbox, CodexSandbox::WorkspaceWrite));
        assert_eq!(options.write_scope, ProviderWriteScope::Module);
    }

    #[test]
    fn prompt_options_allow_configured_root_write_scope() {
        let root = prompt_fixture("workspace-write-root");
        fs::create_dir_all(root.join(".rms")).unwrap();
        fs::write(
            root.join(".rms/config.yaml"),
            r#"ai:
  default_provider: codex
  codex:
    sandbox: workspace-write
    write_scope: root
"#,
        )
        .unwrap();

        let options = resolve_prompt_run_options(
            &root,
            RawPromptRunOptions {
                ai: true,
                provider: None,
                record: false,
                run_root: None,
                model: None,
                sandbox: None,
                write_scope: None,
            },
        )
        .unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert!(matches!(options.sandbox, CodexSandbox::WorkspaceWrite));
        assert_eq!(options.write_scope, ProviderWriteScope::Root);
    }

    #[test]
    fn prompt_options_require_configured_ai_provider() {
        let root = prompt_fixture("missing-ai-config");

        let error = resolve_prompt_run_options(
            &root,
            RawPromptRunOptions {
                ai: true,
                provider: None,
                record: false,
                run_root: None,
                model: None,
                sandbox: None,
                write_scope: None,
            },
        )
        .unwrap_err()
        .to_string();

        fs::remove_dir_all(&root).unwrap();
        assert!(error.contains("ai.default_provider"));
    }

    #[test]
    fn gate_reports_friendly_message_outside_git_repository() {
        let root = prompt_fixture("gate-no-git");

        let error = run_gate(&root, None, false, false).unwrap_err().to_string();

        fs::remove_dir_all(&root).unwrap();
        assert!(error.contains("git repository required to read changed paths"));
        assert!(error.contains("git init"));
        assert!(!error.contains("usage: git diff"));
    }

    #[test]
    fn diagnose_report_includes_config_and_serializes_to_json() {
        let root = prompt_fixture("diagnose-config");
        fs::create_dir_all(root.join(".rms")).unwrap();
        fs::write(
            root.join(".rms/config.yaml"),
            r#"ai:
  default_provider: codex
  codex:
    model: gpt-test
runs:
  directory: .rms/test-runs
"#,
        )
        .unwrap();

        let report = build_diagnose_report(&root).unwrap();
        let rendered = serde_json::to_string(&report).unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(report.config.status, "present");
        assert_eq!(report.config.default_provider.as_deref(), Some("codex"));
        assert_eq!(report.config.run_directory, ".rms/test-runs");
        assert!(rendered.contains("\"ai_providers\""));
        assert!(rendered.contains("\"run_records\""));
    }

    #[test]
    fn latest_run_dir_uses_newest_run_id() {
        let root = unique_test_dir("latest-run");
        let runs = root.join("runs");
        fs::create_dir_all(runs.join("100-plan-example")).unwrap();
        fs::create_dir_all(runs.join("200-review-example")).unwrap();
        fs::create_dir_all(runs.join("150-evidence-example")).unwrap();

        let latest = latest_run_dir(&runs).unwrap().unwrap();

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            latest.file_name().and_then(|name| name.to_str()),
            Some("200-review-example")
        );
    }

    #[test]
    fn config_init_writes_defaults_and_refuses_overwrite() {
        let root = unique_test_dir("config-init");
        fs::create_dir_all(&root).unwrap();

        run_config_init(
            &root,
            Provider::Codex,
            Some("gpt-test"),
            Path::new(".rms/test-runs"),
            false,
        )
        .unwrap();
        let loaded = load_workbench_config(&root).unwrap().unwrap();
        let overwrite_error =
            run_config_init(&root, Provider::Codex, None, Path::new(".rms/runs"), false)
                .unwrap_err()
                .to_string();

        fs::remove_dir_all(&root).unwrap();
        assert_eq!(loaded.value.ai.default_provider.as_deref(), Some("codex"));
        assert_eq!(loaded.value.ai.codex.model.as_deref(), Some("gpt-test"));
        assert_eq!(
            loaded.value.runs.directory.as_deref(),
            Some(Path::new(".rms/test-runs"))
        );
        assert!(overwrite_error.contains("already exists"));
    }

    #[test]
    fn codex_plugin_sync_detects_packaged_skill_drift() {
        let root = unique_test_dir("plugin-sync");
        fs::create_dir_all(root.join("integrations/codex/rms/.codex-plugin")).unwrap();
        fs::write(
            root.join("integrations/codex/rms/.codex-plugin/plugin.json"),
            r#"{
  "name": "rms",
  "version": "0.1.0",
  "description": "test",
  "skills": "./skills/"
}
"#,
        )
        .unwrap();

        for skill in CANONICAL_SKILLS {
            let canonical = root.join("skills").join(skill);
            let packaged = root.join("integrations/codex/rms/skills").join(skill);
            fs::create_dir_all(&canonical).unwrap();
            fs::create_dir_all(&packaged).unwrap();
            let contents = format!("---\nname: {skill}\n---\n\n# {skill}\n");
            fs::write(canonical.join("SKILL.md"), &contents).unwrap();
            fs::write(packaged.join("SKILL.md"), contents).unwrap();
        }

        validate_codex_plugin_sync(&root).unwrap();
        fs::write(
            root.join("integrations/codex/rms/skills/implement-change/SKILL.md"),
            "drift\n",
        )
        .unwrap();
        let error = validate_codex_plugin_sync(&root).unwrap_err().to_string();

        fs::remove_dir_all(&root).unwrap();
        assert!(error.contains("out of sync"));
    }

    #[test]
    fn release_metadata_detects_version_drift() {
        let root = unique_test_dir("release-metadata");
        fs::create_dir_all(root.join("tooling/rust/rms")).unwrap();
        fs::create_dir_all(root.join("integrations/codex/rms/.codex-plugin")).unwrap();
        fs::write(
            root.join("tooling/rust/rms/Cargo.toml"),
            "[package]\nname = \"rms\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        fs::write(
            root.join("tooling/rust/rms/module.yaml"),
            "spec: rms/module/v0.1\nmodule:\n  name: rms-cli\n  version: 0.1.0\n  kind: tool\n  purpose: Test release metadata\n",
        )
        .unwrap();
        fs::write(
            root.join("integrations/codex/rms/.codex-plugin/plugin.json"),
            r#"{
  "name": "rms",
  "version": "0.1.0",
  "description": "test",
  "skills": "./skills/"
}
"#,
        )
        .unwrap();

        validate_release_metadata(&root).unwrap();
        fs::write(
            root.join("integrations/codex/rms/.codex-plugin/plugin.json"),
            r#"{
  "name": "rms",
  "version": "0.2.0",
  "description": "test",
  "skills": "./skills/"
}
"#,
        )
        .unwrap();
        let error = validate_release_metadata(&root).unwrap_err().to_string();

        fs::remove_dir_all(&root).unwrap();
        assert!(error.contains("version drift"));
    }

    fn unique_test_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rms-{label}-{}-{nanos}", std::process::id()))
    }

    fn prompt_fixture(label: &str) -> PathBuf {
        let root = unique_test_dir(label);
        fs::create_dir_all(root.join("verification/laws")).unwrap();
        fs::create_dir_all(root.join("verification/contracts")).unwrap();
        fs::create_dir_all(root.join("verification/scenarios")).unwrap();
        fs::create_dir_all(root.join("verification/boundaries")).unwrap();
        fs::write(
            root.join("module.yaml"),
            render_module_yaml(
                "prompt-fixture",
                "Exercise workbench prompt rendering",
                "tool",
                &[String::from("core")],
            ),
        )
        .unwrap();
        root
    }

    fn atlas_fixture(label: &str) -> PathBuf {
        let root = unique_test_dir(label);
        fs::create_dir_all(root.join("contracts")).unwrap();
        fs::create_dir_all(root.join("verification/laws")).unwrap();
        fs::write(
            root.join("contracts/do-work.v1.yaml"),
            "spec: rms/contract/v0.1\nname: do-work\nversion: 1\nkind: command\nmeaning: Accept valid work and reject malformed work.\n",
        )
        .unwrap();
        fs::write(
            root.join("verification/laws/work_is_safe"),
            "law evidence\n",
        )
        .unwrap();
        fs::write(
            root.join("module.yaml"),
            r#"spec: rms/module/v0.1

module:
  name: atlas-fixture
  version: 0.1.0
  kind: bounded-context
  purpose: Exercise atlas generation

profiles:
  - core
  - boundary

owns:
  concepts:
    - Work
  data:
    - work-log
  decisions:
    - work-acceptance

provides:
  commands:
    - name: do-work
      contract: contracts/do-work.v1.yaml
  queries: []
  events: []
  capabilities: []

requires:
  modules: []
  capabilities: []

invariants:
  - id: work-is-safe
    statement: Accepted work satisfies the module rules.
    verified_by: verification/laws/work_is_safe

effects: []

boundary:
  validation: reject-before-domain-entry

compatibility:
  policy: backward-compatible-within-major

verification:
  laws:
    - verification/laws
  contracts: []
  scenarios: []
  boundaries: []
"#,
        )
        .unwrap();
        root
    }

    fn rust_typing_fixture(
        label: &str,
        profiles: &[&str],
        architecture_extra: &str,
        source: &str,
    ) -> PathBuf {
        let root = unique_test_dir(label);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"typing-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), source).unwrap();
        fs::write(
            root.join("implementation.yaml"),
            format!(
                "spec: rms/implementation/v0.1\n\nmodule: typing-fixture\nbinding: rust\n\nsource:\n  root: .\n  public_entrypoint: src/lib.rs\n\ncommands:\n  build: cargo build --manifest-path Cargo.toml\n  verify: cargo test --manifest-path Cargo.toml\n\ntoolchain:\n  cargo_manifest: Cargo.toml\n  package: typing-fixture\n\ndependencies:\n  allowed_external_crates: []\n\narchitecture:\n  public_modules: []\n{architecture_extra}"
            ),
        )
        .unwrap();
        fs::write(
            root.join("module.yaml"),
            format!(
                "spec: rms/module/v0.1\n\nmodule:\n  name: typing-fixture\n  version: 0.1.0\n  kind: library\n  purpose: Test typing fixture\n\nprofiles:\n{}\n\nowns:\n  concepts: []\n  data: []\n  decisions: []\n\nprovides:\n  commands: []\n  queries: []\n  events: []\n  capabilities: []\n\nrequires:\n  modules: []\n  capabilities: []\n\ninvariants: []\n\neffects: []\n\nstate:\n  model: docs/state.md\n  consistency_boundary: fixture\n  concurrency: single-threaded\n  migration_policy: none\n\ncompatibility:\n  policy: backward-compatible-within-major\n\nverification:\n  laws: []\n  contracts: []\n  scenarios: []\n  boundaries: []\n",
                profiles
                    .iter()
                    .map(|profile| format!("  - {profile}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        )
        .unwrap();
        root
    }

    fn write_compose_module(
        path: &Path,
        name: &str,
        provides_extra: &str,
        requires_modules: &str,
        requires_capabilities: &str,
    ) {
        fs::write(
            path,
            format!(
                "spec: rms/module/v0.1\n\nmodule:\n  name: {name}\n  version: 0.1.0\n  kind: library\n  purpose: Test composition\n\nprofiles:\n  - core\n\nowns:\n  concepts: []\n  data: []\n  decisions: []\n\nprovides:\n  commands: []\n  queries: []\n  events: []\n{provides_extra}\nrequires:\n{requires_modules}{requires_capabilities}\ninvariants: []\n\neffects: []\n\ncompatibility:\n  policy: backward-compatible-within-major\n\nverification:\n  laws: []\n  contracts: []\n  scenarios: []\n  boundaries: []\n"
            ),
        )
        .unwrap();
    }

    fn swift_typing_fixture(
        label: &str,
        profiles: &[&str],
        architecture_extra: &str,
        source: &str,
    ) -> PathBuf {
        let root = unique_test_dir(label);
        fs::create_dir_all(root.join("Sources/TypingFixture")).unwrap();
        fs::write(
            root.join("Package.swift"),
            "// swift-tools-version: 5.9\nimport PackageDescription\n\nlet package = Package(\n    name: \"typing-fixture\",\n    targets: [.target(name: \"TypingFixture\")]\n)\n",
        )
        .unwrap();
        fs::write(
            root.join("Sources/TypingFixture/TypingFixture.swift"),
            source,
        )
        .unwrap();
        fs::write(
            root.join("implementation.yaml"),
            format!(
                "spec: rms/implementation/v0.1\n\nmodule: typing-fixture\nbinding: swift\n\nsource:\n  root: Sources/TypingFixture\n  public_entrypoint: Sources/TypingFixture/TypingFixture.swift\n\ncommands:\n  build: swift build --package-path .\n  verify: swift test --package-path .\n\ntoolchain:\n  package_manifest: Package.swift\n  package: typing-fixture\n  target: TypingFixture\n\ndependencies:\n  allowed_external_modules: []\n\narchitecture:\n  public_modules: []\n{architecture_extra}"
            ),
        )
        .unwrap();
        fs::write(
            root.join("module.yaml"),
            format!(
                "spec: rms/module/v0.1\n\nmodule:\n  name: typing-fixture\n  version: 0.1.0\n  kind: library\n  purpose: Test typing fixture\n\nprofiles:\n{}\n\nowns:\n  concepts: []\n  data: []\n  decisions: []\n\nprovides:\n  commands: []\n  queries: []\n  events: []\n  capabilities: []\n\nrequires:\n  modules: []\n  capabilities: []\n\ninvariants: []\n\neffects: []\n\nstate:\n  model: docs/state.md\n  consistency_boundary: fixture\n  concurrency: single-threaded\n  migration_policy: none\n\ncompatibility:\n  policy: backward-compatible-within-major\n\nverification:\n  laws: []\n  contracts: []\n  scenarios: []\n  boundaries: []\n",
                profiles
                    .iter()
                    .map(|profile| format!("  - {profile}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        )
        .unwrap();
        root
    }

    fn validate_fixture_implementation(root: &Path) -> Vec<Diagnostic> {
        let manifest = load_manifest(&root.join("implementation.yaml")).unwrap();
        let mut diagnostics = Vec::new();
        validate_loaded_manifest(&manifest, &mut diagnostics);
        diagnostics
    }

    fn loaded_module_manifest(path: &str, source: &str) -> LoadedManifest {
        LoadedManifest {
            path: PathBuf::from(path),
            value: serde_yaml::from_str(source).unwrap(),
        }
    }
}
