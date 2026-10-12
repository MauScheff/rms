//! Exact, source-pinned compatibility for previously mapped Rust records.
use super::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    compatibility: Compatibility,
    prior_revision: String,
    kind: Kind,
    name: String,
    source: String,
    source_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Compatibility {
    PreservePublicShape,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    CommandEnvelope,
    EventEnvelope,
    EffectEnvelope,
    EffectResultEnvelope,
    Transition,
    TransitionRecord,
}

impl Kind {
    fn field(&self) -> &'static str {
        match self {
            Self::CommandEnvelope => "command_envelope",
            Self::EventEnvelope => "event_envelope",
            Self::EffectEnvelope => "effect_envelope",
            Self::EffectResultEnvelope => "effect_result_envelope",
            Self::Transition => "transition",
            Self::TransitionRecord => "transition_record",
        }
    }
}

pub(super) fn bindings(value: &YamlValue) -> Result<Vec<Binding>> {
    get_path(
        value,
        &["architecture", "machine", "retained_structural_bindings"],
    )
    .map(|value| serde_yaml::from_value(value.clone()).map_err(Into::into))
    .unwrap_or_else(|| Ok(Vec::new()))
}

fn git_bytes(root: &Path, revision: &str, path: &Path) -> Result<Vec<u8>> {
    let relative = path.strip_prefix(root)?;
    let output = Command::new("git")
        .current_dir(root)
        .arg("show")
        .arg(format!("{revision}:{}", relative.display()))
        .output()?;
    if !output.status.success() {
        bail!("prior binding or source is absent from the pinned commit");
    }
    Ok(output.stdout)
}

fn role_contains(value: &YamlValue, source: &str) -> bool {
    get_path(value, &["architecture", "roles"])
        .and_then(YamlValue::as_mapping)
        .is_some_and(|roles| {
            roles
                .values()
                .filter_map(YamlValue::as_sequence)
                .flatten()
                .any(|path| path.as_str() == Some(source))
        })
}

pub(super) fn verify(manifest: &LoadedManifest, items: &[Binding]) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    if get_str(&manifest.value, &["spec"]) != Some(IMPLEMENTATION_V2_SPEC)
        || get_str(&manifest.value, &["binding"]) != Some("rust")
    {
        bail!("retained structural bindings require a Rust implementation v0.2");
    }
    let path = fs::canonicalize(&manifest.path)?;
    let base = path
        .parent()
        .ok_or_else(|| anyhow!("missing module directory"))?;
    let root_output = Command::new("git")
        .current_dir(base)
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if !root_output.status.success() {
        bail!("retained structural provenance requires its Git repository");
    }
    let root = fs::canonicalize(String::from_utf8(root_output.stdout)?.trim())?;
    let mut names = BTreeSet::new();
    for item in items {
        if !names.insert(&item.name)
            || !is_stable_identifier(&item.name)
            || item.prior_revision.len() != 40
            || !item.prior_revision.bytes().all(|b| b.is_ascii_hexdigit())
            || item.source_sha256.len() != 64
            || !item.source_sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || !is_safe_relative_artifact_path(&item.source)
        {
            bail!("retained structural binding requires a unique exact name, full commit, safe source path, and SHA-256");
        }
        let ancestor = Command::new("git")
            .current_dir(&root)
            .args(["merge-base", "--is-ancestor", &item.prior_revision, "HEAD"])
            .output()?;
        if !ancestor.status.success() {
            bail!("retained structural prior revision is not an ancestor of HEAD");
        }
        let prior: YamlValue =
            serde_yaml::from_slice(&git_bytes(&root, &item.prior_revision, &path)?)?;
        let mut schema_diagnostics = Vec::new();
        validate_against_embedded_schema(
            &LoadedManifest {
                path: path.clone(),
                value: prior.clone(),
            },
            &mut schema_diagnostics,
        );
        if schema_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
        {
            bail!("retained prior binding is not schema-valid");
        }
        if get_str(&prior, &["spec"]) != Some(IMPLEMENTATION_V2_SPEC)
            || get_str(&prior, &["binding"]) != Some("rust")
            || get_str(&prior, &["module"]) != get_str(&manifest.value, &["module"])
            || get_str(
                &prior,
                &["architecture", "machine", "types", item.kind.field()],
            ) != Some(item.name.as_str())
            || !get_string_array(&prior, &["architecture", "allowed_public_field_structs"])
                .contains(&item.name)
            || !role_contains(&prior, &item.source)
            || !role_contains(&manifest.value, &item.source)
        {
            bail!("retained type `{}` lacks the exact prior structural mapping, allowance, or source role", item.name);
        }
        let source_path = fs::canonicalize(base.join(&item.source))?;
        if !source_path.starts_with(base) || source_path != base.join(&item.source) {
            bail!("retained source must be a normalized module-local path without symlink aliases");
        }
        let old = git_bytes(&root, &item.prior_revision, &source_path)?;
        let current = fs::read(&source_path)?;
        if old != current || sha256_bytes(&old) != item.source_sha256 {
            bail!(
                "retained source `{}` differs from its pinned prior bytes or hash",
                item.source
            );
        }
        let parsed = syn::parse_file(std::str::from_utf8(&old)?)?;
        let matches = parsed
            .items
            .iter()
            .filter(|node| {
                matches!(node, syn::Item::Struct(s)
            if s.ident == item.name && matches!(s.vis, syn::Visibility::Public(_))
                && public_struct_field_count(s) > 0 && !has_cfg_test_attr(&s.attrs))
            })
            .count();
        if matches != 1 {
            bail!("retained type must resolve to one public structural record in its exact source");
        }
    }
    Ok(())
}

pub(super) fn validate(manifest: &LoadedManifest, diagnostics: &mut Vec<Diagnostic>) {
    if let Err(error) = bindings(&manifest.value).and_then(|items| verify(manifest, &items)) {
        diagnostics.push(super::error(
            "structure.retained-binding-provenance",
            &manifest.path,
            error.to_string(),
        ));
    }
}

// Some(false) prevents an ordinary name-based allowance from masking a forged
// retained record or the same name declared in a different file.
pub(super) fn permits(manifest: &LoadedManifest, name: &str, path: &Path) -> Option<bool> {
    let items = bindings(&manifest.value).ok()?;
    let item = items.iter().find(|item| item.name == name)?;
    let expected = manifest.path.parent()?.join(&item.source);
    Some(
        fs::canonicalize(path).ok() == fs::canonicalize(expected).ok()
            && verify(manifest, &items).is_ok(),
    )
}

pub(super) fn apply(value: &mut YamlValue, items: &Option<Vec<Binding>>) {
    if let Some(items) = items {
        let previous = names(value);
        let mut allowed =
            get_string_array(value, &["architecture", "allowed_public_field_structs"]);
        allowed.retain(|name| !previous.contains(name));
        set_yaml_string_sequence_path(
            value,
            &["architecture", "allowed_public_field_structs"],
            &allowed,
        );
        set_yaml_value_path(
            value,
            &["architecture", "machine", "retained_structural_bindings"],
            serde_yaml::to_value(items).expect("serializable bindings"),
        );
    }
}

pub(super) fn names(value: &YamlValue) -> Vec<String> {
    bindings(value)
        .unwrap_or_default()
        .into_iter()
        .map(|item| item.name)
        .collect()
}

pub(super) fn check_intent(task: &str, items: &[Binding]) -> Result<()> {
    let lower = task.to_ascii_lowercase();
    if !items.is_empty()
        && !(lower.contains("retain")
            && lower.contains("structural")
            && lower.contains("compatibility"))
    {
        bail!("retained bindings require explicit retained structural compatibility intent");
    }
    let tokens: BTreeSet<_> = task
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .collect();
    if items.iter().any(|item| {
        !tokens.contains(item.name.as_str()) || !tokens.contains(item.prior_revision.as_str())
    }) {
        bail!("retained compatibility route must name every exact type and full prior revision");
    }
    Ok(())
}

// Read-only preflight for restoring allowances removed by an earlier mapping.
// It cannot repair unrelated source debt or authorize inferred type identities.
pub(super) fn route_repairs(task: &str, owner: &OwnerResolution) -> BTreeMap<String, PathBuf> {
    let lower = task.to_ascii_lowercase();
    if !(lower.contains("retain")
        && lower.contains("structural")
        && lower.contains("compatibility"))
    {
        return BTreeMap::new();
    }
    let attempt = || -> Result<BTreeMap<String, PathBuf>> {
        let selected = owner
            .selected_module()
            .ok_or_else(|| anyhow!("owner required"))?;
        let path = Path::new(&selected.path)
            .parent()
            .ok_or_else(|| anyhow!("module directory required"))?
            .join("implementation.yaml");
        let manifest = load_manifest(&path)?;
        let base = fs::canonicalize(path.parent().unwrap())?;
        let output = Command::new("git")
            .current_dir(&base)
            .args(["rev-parse", "--show-toplevel"])
            .output()?;
        if !output.status.success() {
            bail!("Git required");
        }
        let root = fs::canonicalize(String::from_utf8(output.stdout)?.trim())?;
        let tokens: BTreeSet<_> = task
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .collect();
        let revisions = tokens
            .iter()
            .filter(|token| token.len() == 40 && token.bytes().all(|b| b.is_ascii_hexdigit()))
            .collect::<Vec<_>>();
        if revisions.len() != 1 {
            bail!("one exact prior revision required");
        }
        let revision = *revisions[0];
        let prior: YamlValue =
            serde_yaml::from_slice(&git_bytes(&root, revision, &fs::canonicalize(&path)?)?)?;
        let roles = get_path(&prior, &["architecture", "roles"])
            .and_then(YamlValue::as_mapping)
            .ok_or_else(|| anyhow!("roles required"))?;
        let sources: BTreeSet<_> = roles
            .values()
            .filter_map(YamlValue::as_sequence)
            .flatten()
            .filter_map(YamlValue::as_str)
            .collect();
        let mut items = Vec::new();
        for kind in [
            Kind::CommandEnvelope,
            Kind::EventEnvelope,
            Kind::EffectEnvelope,
            Kind::EffectResultEnvelope,
            Kind::Transition,
            Kind::TransitionRecord,
        ] {
            let Some(name) = get_str(&prior, &["architecture", "machine", "types", kind.field()])
            else {
                continue;
            };
            if !tokens.contains(name) {
                continue;
            }
            let matches = sources
                .iter()
                .filter_map(|source| {
                    if !is_safe_relative_artifact_path(source) {
                        return None;
                    }
                    let bytes = git_bytes(&root, revision, &base.join(source)).ok()?;
                    let parsed = syn::parse_file(std::str::from_utf8(&bytes).ok()?).ok()?;
                    parsed
                        .items
                        .iter()
                        .any(|node| matches!(node, syn::Item::Struct(s) if s.ident == name))
                        .then(|| ((*source).to_string(), sha256_bytes(&bytes)))
                })
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                bail!("ambiguous prior source");
            }
            items.push(Binding {
                compatibility: Compatibility::PreservePublicShape,
                prior_revision: revision.into(),
                kind,
                name: name.into(),
                source: matches[0].0.clone(),
                source_sha256: matches[0].1.clone(),
            });
        }
        if items.is_empty() {
            bail!("no explicit structural identities");
        }
        check_intent(task, &items)?;
        verify(&manifest, &items)?;
        Ok(items
            .into_iter()
            .map(|item| (item.name, base.join(item.source)))
            .collect())
    };
    attempt().unwrap_or_default()
}

pub(super) fn repaired_diagnostic(
    diagnostic: &Diagnostic,
    owner: &OwnerResolution,
    names: &BTreeMap<String, PathBuf>,
) -> bool {
    let Some(selected) = owner.selected_module() else {
        return false;
    };
    let Some(base) = Path::new(&selected.path).parent() else {
        return false;
    };
    Path::new(&diagnostic.path) == base.join("implementation.yaml")
        && matches!(
            diagnostic.check.as_str(),
            "implementation.rust.typing.public-fields" | "structure.public-domain-field-bypass"
        )
        && names.iter().any(|(name, expected)| {
            [
                format!("public struct `{name}` in `"),
                format!("public domain struct `{name}` in `"),
            ]
            .iter()
            .any(|prefix| {
                diagnostic
                    .message
                    .strip_prefix(prefix)
                    .and_then(|tail| tail.split_once('`'))
                    .is_some_and(|(path, _)| fs::canonicalize(path).ok().as_ref() == Some(expected))
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        add_module_request, initialize_test_git_repository, no_provider_options, unique_test_dir,
    };

    #[test]
    fn retained_structural_all_six_kinds_do_not_exempt_new_domain_records() {
        let root = unique_test_dir("retained-six-kinds");
        run_add_module(
            add_module_request(
                &root,
                "versioned",
                "Retain V1 records beside V2.",
                "library",
                &[],
                Some(ScaffoldShape::Workflow),
                Some("rust"),
            ),
            &no_provider_options(),
        )
        .unwrap();
        let root = fs::canonicalize(root).unwrap();
        initialize_test_git_repository(&root);
        let revision = git_head(&root).unwrap();
        let path = root.join("implementation.yaml");
        let mut manifest = load_manifest(&path).unwrap();
        let mut items = Vec::new();
        for kind in [
            Kind::CommandEnvelope,
            Kind::EventEnvelope,
            Kind::EffectEnvelope,
            Kind::EffectResultEnvelope,
            Kind::Transition,
            Kind::TransitionRecord,
        ] {
            let name = get_str(
                &manifest.value,
                &["architecture", "machine", "types", kind.field()],
            )
            .unwrap()
            .to_string();
            let source = if matches!(kind, Kind::Transition | Kind::TransitionRecord) {
                "src/transition.rs"
            } else {
                "src/representation.rs"
            };
            items.push(Binding {
                compatibility: Compatibility::PreservePublicShape,
                prior_revision: revision.clone(),
                kind,
                name,
                source: source.into(),
                source_sha256: sha256_bytes(&fs::read(root.join(source)).unwrap()),
            });
        }
        verify(&manifest, &items).unwrap();
        for item in &items {
            set_yaml_string_path(
                &mut manifest.value,
                &["architecture", "machine", "types", item.kind.field()],
                &format!("V2{}", item.name),
            );
        }
        apply(&mut manifest.value, &Some(items.clone()));
        for item in &items {
            assert_eq!(
                permits(&manifest, &item.name, &root.join(&item.source)),
                Some(true)
            );
        }
        let parsed = syn::parse_file("pub struct NewDomainEnvelope { pub secret: u64 }").unwrap();
        let mut diagnostics = Vec::new();
        inspect_rust_typing_file(
            &manifest,
            &mut diagnostics,
            &root.join("src/new.rs"),
            &parsed,
            &mut RustTypingSummary::default(),
        );
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.check == "implementation.rust.typing.public-fields"));
        let shadow = syn::parse_file(&format!(
            "pub struct {} {{ pub secret: u64 }}",
            items[0].name
        ))
        .unwrap();
        diagnostics.clear();
        inspect_rust_typing_file(
            &manifest,
            &mut diagnostics,
            &root.join("src/new.rs"),
            &shadow,
            &mut RustTypingSummary::default(),
        );
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.check == "structure.retained-binding-provenance"));
        let mut malformed = serde_yaml::to_value(&items).unwrap();
        malformed.as_sequence_mut().unwrap()[0]
            .as_mapping_mut()
            .unwrap()
            .insert(YamlValue::String("forged".into()), YamlValue::Bool(true));
        set_yaml_value_path(
            &mut manifest.value,
            &["architecture", "machine", "retained_structural_bindings"],
            malformed,
        );
        diagnostics.clear();
        validate(&manifest, &mut diagnostics);
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.check == "structure.retained-binding-provenance"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn retained_structural_binding_is_exact_explicit_and_source_pinned() {
        let root = unique_test_dir("retained-structural");
        run_add_module(
            add_module_request(
                &root,
                "versioned",
                "Preserve exact public V1 records.",
                "library",
                &[],
                Some(ScaffoldShape::DomainEngine),
                Some("rust"),
            ),
            &no_provider_options(),
        )
        .unwrap();
        let root = fs::canonicalize(root).unwrap();
        let path = root.join("implementation.yaml");
        let old = load_manifest(&path).unwrap();
        initialize_test_git_repository(&root);
        let revision = git_head(&root).unwrap();
        let types = machine_types_from_value(&old.value);
        let source = structure_role_paths(&old, "transition")[0].clone();
        let name = types.transition.clone().unwrap();
        let bytes = fs::read(root.join(&source)).unwrap();
        let entry = json!({"compatibility":"preserve-public-shape", "prior_revision":revision, "kind":"transition", "name":name, "source":source, "source_sha256":sha256_bytes(&bytes)});
        let items: Vec<Binding> = serde_json::from_value(json!([entry])).unwrap();
        verify(&old, &items).unwrap();
        let task = format!("Change versioned binding to retain structural compatibility for {name} from {revision}");
        check_intent(&task, &items).unwrap();
        assert!(check_intent("change versioned binding", &items).is_err());
        assert!(check_intent("retain structural compatibility", &items).is_err());
        let mode = get_str(&old.value, &["architecture", "machine", "mode"]).unwrap();
        let mut change: MachineChange = serde_json::from_value(json!({"spec":"rms/machine-change/v0.1", "machine":{"mode":mode, "types":{"transition":"V2Transition"}}})).unwrap();
        change.machine.retained_structural_bindings = Some(items.clone());
        let report = build_next_report(&root, Some(&root.join("module.yaml")), &task).unwrap();
        assert_eq!(report.result, NextResult::Ready, "{:?}", report.blockers);
        let receipt = Path::new(&report.receipt_path);
        validate_machine_change_route_receipt(&root, receipt, &path, &change).unwrap();
        assert!(validate_machine_change_route_receipt(
            &root,
            receipt,
            &root.join("other/implementation.yaml"),
            &change
        )
        .is_err());
        let absent = build_next_report(
            &root,
            Some(&root.join("module.yaml")),
            "Change the versioned contract",
        )
        .unwrap();
        assert!(validate_machine_change_route_receipt(
            &root,
            Path::new(&absent.receipt_path),
            &path,
            &change
        )
        .is_err());
        change.machine.retained_structural_bindings = None;
        let mut replaced = old.value.clone();
        apply_machine_change_to_manifest(&mut replaced, &change);
        assert!(
            !get_string_array(&replaced, &["architecture", "allowed_public_field_structs"])
                .contains(&name)
        );
        // This fixture models the post-replacement recovery, without editing source.
        write_yaml_manifest(&LoadedManifest {
            path: path.clone(),
            value: replaced,
        })
        .unwrap();
        // Seal using an independent immutable test record.
        fs::write(root.join("recovery-record.yaml"), "recovery: test\n").unwrap();
        seal_implementation_semantics(
            &mut load_manifest(&path).unwrap(),
            &root.join("recovery-record.yaml"),
            SemanticRevisionAuthority::MachineApply,
        )
        .unwrap();
        change.machine.retained_structural_bindings = Some(items.clone());
        let before = fs::read(&path).unwrap();
        run_machine_apply_change(&path, &change, true).unwrap();
        assert_eq!(fs::read(&path).unwrap(), before);
        run_machine_apply_change(&path, &change, false).unwrap();
        let applied = load_manifest(&path).unwrap();
        assert_eq!(permits(&applied, &name, &root.join(&source)), Some(true));
        assert_eq!(
            permits(&applied, &name, &root.join("src/other.rs")),
            Some(false)
        );
        assert_eq!(
            permits(&applied, "ArbitraryDomain", &root.join(&source)),
            None
        );
        assert_eq!(fs::read(root.join(&source)).unwrap(), bytes);
        let mut diagnostics = Vec::new();
        validate(&applied, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let owner = OwnerResolution::selected(
            "exact owner".into(),
            RouteModuleSummary {
                name: "versioned".into(),
                path: root.join("module.yaml").display().to_string(),
                kind: "library".into(),
                shape: "domain-engine".into(),
                visibility: None,
            },
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let repairs = route_repairs(&task, &owner);
        assert!(repairs.contains_key(&name));
        let diagnostic = error(
            "implementation.rust.typing.public-fields",
            &path,
            format!(
                "public struct `{name}` in `{}` exposes fields",
                root.join(&source).display()
            ),
        );
        assert!(repaired_diagnostic(&diagnostic, &owner, &repairs));
        let unrelated = error(
            "implementation.rust.typing.public-fields",
            &path,
            "public struct `ArbitraryDomain` in `src/domain.rs` exposes fields",
        );
        assert!(!repaired_diagnostic(&unrelated, &owner, &repairs));
        assert!(route_repairs("retain structural compatibility", &owner).is_empty());
        for (field, value) in [
            ("name", json!("ArbitraryDomain")),
            ("kind", json!("event_envelope")),
            ("source", json!("../escape.rs")),
            ("source_sha256", json!("0".repeat(64))),
            ("prior_revision", json!("0".repeat(40))),
        ] {
            let mut invalid = entry.clone();
            invalid[field] = value;
            let invalid: Vec<Binding> = serde_json::from_value(json!([invalid])).unwrap();
            assert!(verify(&applied, &invalid).is_err(), "{field}");
        }
        let mut no_intent = entry.clone();
        no_intent.as_object_mut().unwrap().remove("compatibility");
        assert!(serde_json::from_value::<Binding>(no_intent).is_err());
        fs::write(
            root.join(&source),
            [bytes.as_slice(), b"\n// changed\n"].concat(),
        )
        .unwrap();
        assert!(verify(&applied, &items).is_err());
        assert_eq!(permits(&applied, &name, &root.join(&source)), Some(false));
        assert!(route_repairs(&task, &owner).is_empty());
        fs::write(root.join(&source), &bytes).unwrap();
        let mut removed = applied.value.clone();
        change.machine.retained_structural_bindings = Some(Vec::new());
        apply_machine_change_to_manifest(&mut removed, &change);
        assert!(
            !get_string_array(&removed, &["architecture", "allowed_public_field_structs"])
                .contains(&name)
        );
        let commit = Command::new("git")
            .args(["commit", "--allow-empty", "-m", "advance head"])
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(commit.status.success());
        change.machine.retained_structural_bindings = Some(items);
        assert!(validate_machine_change_route_receipt(&root, receipt, &path, &change).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
