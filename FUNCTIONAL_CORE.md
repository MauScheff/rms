# RMS Functional Core and Composition

This guide explains the strict `0.1.0-rc.9` analysis and generation tools. Canonical manifests and contracts remain semantic authority. Effect analyses, generated workloads, composition models, probe assemblies, and proof certificates are derived evidence.

## Choose the Tool

| Need | Command | Result | Use the result for |
| --- | --- | --- | --- |
| Upgrade a legacy binding | `rms binding migrate IMPLEMENTATION --to v0.2 --route-receipt RECEIPT --dry-run` | A deterministic candidate, or an explicit ambiguity | Review before the write-mode migration. |
| Prove declared purity and authority rows | `rms structure IMPLEMENTATION` | Transitive call/effect diagnostics | Fix unresolved calls, undeclared authority, or false purity claims. |
| Check property declarations | `rms property check TARGET` | Declaration and realization diagnostics | Confirm that inputs, operations, oracles, runners, and evidence close. |
| Derive machine inputs from probe schemas | `rms property generate IMPLEMENTATION --out ASSEMBLY` | `rms/probe-assembly/v0.3` | Run deterministic machine exploration without maintaining a duplicate generator. |
| Execute binding-native property runners | `rms property run IMPLEMENTATION --profile smoke` | Observed runner results | Prove the declared smoke realization. |
| Inspect one transition path | `rms probe IMPLEMENTATION --describe` and `rms probe IMPLEMENTATION --input '<JSON>'` | Ephemeral probe descriptions and transition records | Debug a machine without creating evidence. |
| Check dependency and protocol closure | `rms compose --root ROOT` | A read-only composition verdict | Review providers, mappings, effects, authorities, cycles, and lifecycle ownership. |
| Generate an executable composed model | `rms compose --root ROOT --output DIR --dry-run` | A validated write plan | Review the symbolic composition before artifact creation. |
| Write composed evidence inputs | `rms compose --root ROOT --output DIR` | `composition.json` and `probe-assembly.yaml` | Explore the exact declared cross-module wiring. |
| Search a finite model | `rms property search TARGET --assembly ASSEMBLY --goal violate --out ANALYSIS` | Analysis and, only after exhaustion, a sibling proof certificate | Find a counterexample or prove the declared finite model. |
| Run risk-derived long verification | `rms hunt --root ROOT --dry-run` | Planned generated, fuzz, schedule, fault, analyzer, sanitizer, and mutation lanes | Inspect expensive work before a clean-commit campaign. |

Use `property generate` for one implementation. Use `compose --output` for a system of implementations. Use `probe` for a focused diagnostic. Use `property search` or `property analyze` for a declared finite proof obligation. Use `hunt` for budgeted discovery. None of these commands replaces raw-parser coverage fuzzing.

## Implementation Binding v0.2

Production checks require `spec: rms/implementation/v0.2`. The v0.1 schema remains readable for migration and compatibility.

Each semantic function declares:

- `purity: pure|effectful`;
- `trust: internal|boundary`;
- `authorities: []` or the exact inferred authority row.

Each declared authority resolves through one exact `path#symbol` safe facade. `rms structure`, `rms verify`, audit, change checks, and committed checks run transitive effect analysis for Rust, Swift, Python, JavaScript, and shell.

The analyzer resolves local calls, imports, aliases, closed Python dispatch tables, recursion, and strongly connected call groups. It ignores comments and string literals. A pure closure fails when it reaches ambient authority, an unresolved call, or unresolved dynamic dispatch. An effectful function fails when its inferred authority row differs from its declaration. Creating an effect value is pure; executing the effect requires authority.

An executable binding can keep its command runner while its semantic functions use inspectable `path#symbol` references. RMS selects the analyzer from each symbol path. A successful migration changes `architecture.static_inspection` from `opaque` to `transitive-effects`. It records a route-receipt-bound candidate seal under `x-rms.binding_migration`. The seal preserves the prior authorized semantic revision for this schema-only change. A later metadata change invalidates the candidate seal.

Shell analysis resolves exact local functions and a small closed set of shell built-ins. An unknown command, a dynamic command name, an unresolved local call, or multiple authority facades produces no migration candidate.

Migration is receipt-gated because it changes canonical implementation metadata:

```bash
rms binding migrate path/to/implementation.yaml \
  --to v0.2 \
  --route-receipt .rms/runs/<run-id>/route-receipt.json \
  --dry-run

rms binding migrate path/to/implementation.yaml \
  --to v0.2 \
  --route-receipt .rms/runs/<run-id>/route-receipt.json
```

Migration infers trust only from unambiguous parser or boundary roles. It infers authorities only from exact authority bindings and static analysis. One safe facade can bind the ambient effects in its statically resolved closure. Multiple matching facades are ambiguous. Ambiguity produces no write. Repeating a successful migration is idempotent.

## Schema-Derived Properties

`property generate` executes the declared machine-probe v0.2 `describe` operation. The response supplies concrete starting states and JSON Schemas for public commands, observed events, and effect results.

The seeded generator supports `const`, `enum`, booleans, bounded numbers, bounded integers, strings, objects, arrays, `oneOf`, and `anyOf`. It exercises minimum, maximum, required-field, length, and recursion-depth boundaries. Unsupported keywords produce an explicit unsupported result; use a manual generator for that schema or for a custom domain law.

Defaults are 64 cases per input and a seed derived from the implementation digest. Equal source, schema, generator version, seed, and case count produce equal workloads. An explicit `--seed` supports exact replay.

Generated workloads:

- start only from described concrete states;
- expand only states reached through the real transition-record path;
- repeat identical state/input evaluations for determinism;
- check declared variants, cases, frames, contracts, and invariants;
- preserve a concrete minimized input, failure identity, seed, generator version, schema digest, and replay command after shrinking.

Shrinking removes fields or schedule suffixes only when the candidate stays valid and preserves the same failure identity. Numbers, strings, arrays, variants, schedules, and faults shrink in deterministic order.

## Executable Composition

`rms compose --root ROOT` is read-only. It does not execute probes and does not write artifacts.

Supplying `--output DIR` executes only declared probe `describe` operations. RMS then builds a symbolic composed machine whose state is the ordered vector of participant states. It routes outputs only through exact dependency and protocol mappings. The permitted effects and authorities are the union of participant requirements.

Generation fails before writing when RMS finds:

- no provider or multiple providers;
- an incompatible contract;
- non-dual protocol endpoints;
- an unauthorized effect;
- an unresolved mapping;
- a forbidden dependency cycle;
- a lifecycle result that bypasses the transition that owns progression.

`--dry-run` validates the plan without writing. Normal output writes `composition.json` and `probe-assembly.yaml` atomically. Existing output requires `--force`. RMS validates the generated assembly before replacement. RMS does not generate production runtime wiring.

Run the assembly with the proof lane that matches the claim:

```bash
rms probe --file generated/composition/probe-assembly.yaml --explore
rms property search module.yaml \
  --assembly generated/composition/probe-assembly.yaml \
  --goal violate \
  --property <property-id> \
  --out verification/properties/<analysis>.json
```

If a generated model becomes canonical evidence, strict checks must regenerate it and compare its digest.

## Algebraic Laws and Proof Certificates

Executable algebraic property kinds are `idempotent`, `commutative`, `associative`, and `monotonic`. Each law names its public behavior or semantic function, equality projection or order relation, input domain, and exact proof property.

RMS uses proved laws only for their declared reduction:

- idempotence removes duplicate-delivery schedules;
- commutativity enables partial-order schedule reduction;
- associativity permits regrouping across identical contract types;
- monotonicity prunes states only under the declared order.

An exhaustive violation search that finds no violation writes `<ANALYSIS>.proof-certificate.json` when `--out` is present. Composition discovers these sibling certificates under the selected root. Reuse requires exact subject, contract, implementation, source, tool, strategy, assumption, and evidence digests. Any drift rejects reuse.

Fuzzing, generated samples, bounded non-exhaustive search, sampled traces, and unresolved solver results do not produce universal certificates.

## Functional-Core Rule

Each new subsystem follows one shape:

```text
parse and normalize boundary input
→ pure decision
→ closed outputs, diagnostics, and requested effects
→ thin adapter performs each request once
```

Effect analysis, schema generation, composition planning, migration candidate construction, and proof aggregation belong in pure modules. Filesystem discovery, process execution, provider calls, Git inspection, and artifact writes remain in adapters. Tests should fuzz the pure decisions directly and verify the adapter boundary separately.

## Completion

Run focused proof first. Then run:

```bash
rms check --changes --root .
# authorized candidate commit
rms check --committed --root .
```

Generated artifacts remain evidence. They never become semantic authority.
