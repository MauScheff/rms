use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json::{json, Value as JsonValue};
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use syn::visit::{self, Visit};
use syn::{
    Attribute, ExprMacro, ExprMethodCall, Fields, ImplItem, Item, ItemStruct, Meta, Type, UseTree,
    Visibility,
};
use toml::Value as TomlValue;
use walkdir::WalkDir;

const VALIDATOR_NAME: &str = "rms";
const VALIDATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "rms")]
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

        /// Optional implementation binding to scaffold. Currently supports `rust` and `swift`.
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
            implementation,
            conformance,
            json,
        } => run_validate(
            root,
            module,
            system,
            context_map,
            implementation,
            conformance,
            json,
        ),
        Commands::Inspect { module } => {
            let manifest = load_manifest(&module)?;
            print_module_brief(&manifest);
            Ok(())
        }
        Commands::Context { module, task, root } => {
            let manifest = load_manifest(&module)?;
            print_context_packet(&manifest, &root, task.as_deref())?;
            Ok(())
        }
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

fn run_validate(
    root: PathBuf,
    module: Vec<PathBuf>,
    system: Vec<PathBuf>,
    context_map: Vec<PathBuf>,
    implementation: Vec<PathBuf>,
    conformance: Vec<PathBuf>,
    json_output: bool,
) -> Result<()> {
    let targets = discover_targets(
        &root,
        module,
        system,
        context_map,
        implementation,
        conformance,
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

    if json_output {
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

fn discover_targets(
    root: &Path,
    modules: Vec<PathBuf>,
    systems: Vec<PathBuf>,
    context_maps: Vec<PathBuf>,
    implementations: Vec<PathBuf>,
    conformance_reports: Vec<PathBuf>,
) -> Result<Vec<PathBuf>> {
    let mut explicit = Vec::new();
    explicit.extend(modules);
    explicit.extend(systems);
    explicit.extend(context_maps);
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
    check_profile_obligations(manifest, diagnostics, &profiles);
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

    match get_str(&manifest.value, &["binding"]) {
        Some("rust") => validate_rust_implementation(manifest, diagnostics),
        Some("swift") => validate_swift_implementation(manifest, diagnostics),
        _ => {}
    }
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
                    "public struct `{struct_name}` has private fields but no public constructor evidence; add `new`/`try_new`/`parse` or declare `architecture.allowed_missing_constructors`"
                ),
            ));
            continue;
        };
        if !methods.iter().any(|method| is_constructor_like(method)) {
            diagnostics.push(warning(
                "implementation.rust.typing.constructor",
                &implementation.path,
                format!(
                    "public struct `{struct_name}` has private fields but no constructor-like method (`new`, `try_new`, `parse`, `from_*`); add one or declare `architecture.allowed_missing_constructors`"
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
                finish_swift_struct_scan(
                    implementation,
                    diagnostics,
                    path,
                    &allowed_public_field_structs,
                    summary,
                    current_struct.take().unwrap(),
                );
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
                "public struct `{struct_name}` has private fields but no public initializer/factory evidence; add `init`, `new`, `parse`, or declare `architecture.allowed_missing_constructors`"
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
            "revision": source_revision().unwrap_or_else(|| "unknown".to_string()),
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
    let targets = discover_targets(root, vec![], vec![], vec![], vec![], vec![])?;
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
        .first()
        .map(|system| get_string_array(&system.value, &["external_dependencies"]))
        .unwrap_or_default()
        .into_iter()
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

    println!("initialized RMS system at {}", path.display());
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
    for category in ["laws", "contracts", "scenarios", "boundaries"] {
        let verification_dir = path.join("verification").join(category);
        fs::create_dir_all(&verification_dir)?;
        write_new_file(
            &verification_dir.join("README.md"),
            &format!("# {category}\n\nAdd RMS {category} evidence here.\n"),
        )?;
    }

    let profiles = normalized_profiles(profiles);
    write_new_file(
        &path.join("module.yaml"),
        &render_module_yaml(name, purpose, kind, &profiles),
    )?;
    write_new_file(
        &path.join("contracts").join("README.md"),
        "# Contracts\n\nAdd public RMS contracts here.\n",
    )?;

    if let Some(binding) = binding {
        match binding {
            "rust" => scaffold_rust_module(path, name)?,
            "swift" => scaffold_swift_module(path, name)?,
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
        "spec: rms/module/v0.1\n\nmodule:\n  name: {}\n  version: 0.1.0\n  kind: {}\n  purpose: {}\n\nprofiles:\n{}\n\nowns:\n  concepts: []\n  data: []\n  decisions: []\n\nprovides:\n  commands: []\n  queries: []\n  events: []\n  capabilities: []\n\nrequires:\n  modules: []\n  capabilities: []\n\ninvariants: []\n\neffects: []\n\ncompatibility:\n  policy: backward-compatible-within-major\n\nverification:\n  laws:\n    - verification/laws\n  contracts:\n    - verification/contracts\n  scenarios:\n    - verification/scenarios\n  boundaries:\n    - verification/boundaries\n",
        yaml_quote(name),
        yaml_quote(kind),
        yaml_quote(purpose),
        yaml_string_list(profiles, 2)
    )
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

const INIT_AGENTS_MD: &str = "# Agent Instructions\n\nThis repository follows Reliable Modular Systems.\n\nBefore changing behavior, identify the owning module, read its `module.yaml`, public contracts, declared effects, invariants, compatibility policy, and verification evidence. Keep implementation changes inside the owning boundary and run `rms validate --root .` before completion.\n";

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

fn source_revision() -> Option<String> {
    let output = Command::new("git")
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

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
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

        fs::remove_dir_all(&root).unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
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

    fn unique_test_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rms-{label}-{}-{nanos}", std::process::id()))
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
