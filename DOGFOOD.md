# Dogfood Walkthrough

The `rms` CLI is itself an RMS module. This walkthrough uses the same narrow surface expected in downstream projects.

## Target

| Role | Path |
| --- | --- |
| Module | `tooling/rust/rms/module.yaml` |
| Implementation | `tooling/rust/rms/implementation.yaml` |
| Contracts | `tooling/rust/rms/contracts/` |
| Evidence | `tooling/rust/rms/verification/` |
| Source | `tooling/rust/rms/src/main.rs` |

The `rms-cli` module owns the command surface, deterministic reports, agent integration, packaging, and release checks.

## Start

```bash
rms check --environment --root .
rms next "explain how the release gate protects packaged skills and binaries" \
  --root . \
  --module tooling/rust/rms/module.yaml
rms explain "How are packaged skills and binaries protected?" \
  --module tooling/rust/rms/module.yaml
```

Use the compact answer first. Add `--details` for complete context, roles, diagnostics, and evidence. If the explanation and canonical artifacts disagree, treat that as drift.

## Explore

```bash
rms view --root . --watch
```

The viewer is read-only derived evidence. It does not replace the module, contracts, implementation binding, or evidence files.

## Change

Ask `next` with the actual intent and follow its owner, declaration, implementation, and proof steps:

```bash
rms next "add release artifact smoke coverage" --root .
```

Load the selected skill. Use `rms help --all` only when it prescribes a specialist semantic, surface, trace, property, review, or package command. Provider output remains explicit and advisory.

## Verify

Run the focused native and RMS checks selected by the implementation binding and skill, then:

```bash
rms check --changes --root .
# Authorized manual candidate commit, when host policy allows it.
rms check --committed --root .
```

Repository release and distribution changes additionally require the maintainer publication gate:

```bash
rms release check --root .
```

## Done

A dogfood change is complete when:

- canonical contracts still describe the public surface;
- declared implementation symbols exist;
- generated artifacts remain derived and semantically reachable;
- provider-backed work remains opt-in;
- the change and committed checks pass in the required order;
- the maintainer release gate passes when release distribution changed;
- the summary names compatibility impact and remaining obligations.
