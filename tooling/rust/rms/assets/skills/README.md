# RMS Agent Skills

## Authority

Canonical RMS manifests, contracts, applied revisions, and evidence declarations own system semantics. These skills are operational workflows: they select deterministic CLI context and proof lanes without becoming architectural authority.

Product intent is sufficient input. Apply semantic changes before code, edit only declared roles, and treat provider output as advisory.

## Catalog

- `inspect-module`: establish ownership, boundaries, contracts, effects, and evidence before planning.
- `implement-change`: implement a feature or fix inside declared RMS roles, using a probe-first red/green loop for observable RMS-owned behavior and the narrowest native fallback otherwise.
- `refactor-module`: improve internal structure without changing declared public meaning.
- `prune-module`: remove unnecessary implementation residue while preserving obligations.
- `add-module`: design and add a coherent module or recursive capability.
- `evolve-contract`: change commands, queries, events, APIs, schemas, or failure semantics.
- `compose-modules`: verify provider/consumer fit and cross-module behavior.
- `verify-module`: prove declared laws, contracts, boundaries, profiles, and compatibility promises.
- `hunt-bugs`: run resumable proof, fuzz, sanitizer, schedule, and mutation campaigns; minimize and replay every behavioral failure.

## Doorway

For work that requests or may require a software change, start with:

```text
rms next "<exact change task>" --root . --ai
```

Use native project tools for read-only investigation, explanation, review, status or history inspection, ordinary Git/repository/tool operations, and discussion that requests no change. If that work reveals a proposed change, stop before editing and enter this doorway with the exact change task.

Typed intent is reserved for CI, offline, or intentionally pre-structured caller input. Do not synthesize it as an automatic fallback when provider execution fails. A non-ready or ownerless route selects no owner; candidates and context are evidence, not permission to imply one.

RMS records schema-constrained fact extraction without accepting provider topology. CI, offline, and intentionally pre-structured callers use `--intent-json`, `--intent-yaml`, or `--intent-file`. Pass the returned `run_id`, run directory, or receipt file through `--route-receipt` on prescribed canonical semantic and topology mutations, including dry-runs.

Use `rms explain ["<question>"]` when the compact prescription needs clarification and `rms help --all` for specialist commands. Load the catalog skill selected by the task lane; detailed machine, surface, property, trace, and completion rules live in selected skills and rendered RMS context.

## Specialist Tool Selection

| Need | Use | Do not substitute |
| --- | --- | --- |
| Migrate `rms/implementation/v0.1` | `rms binding migrate ... --to v0.2 --route-receipt ... --dry-run`, then write mode | Direct YAML edits or guessed trust/authority rows |
| Prove purity or authority closure | `rms structure <implementation.yaml>` | A lexical search for IO names |
| Generate valid inputs for one machine | `rms property generate <implementation.yaml> --out <assembly>` | Hand-copied probe examples |
| Debug one real transition | `rms probe ... --describe`, then one input | Property proof or production effect execution |
| Check system closure without writes | `rms compose --root <root>` | Generated runtime wiring |
| Generate a symbolic composed probe | `rms compose --root <root> --output <dir> --dry-run`, then write mode | An eager Cartesian state product |
| Search a declared finite model | `rms property search ... --assembly ... --goal satisfy|violate` | Fuzzing presented as universal proof |
| Discover bugs under a budget | `rms hunt --dry-run`, then a clean-commit hunt | The fast change or committed gate |

`property generate` is for one implementation. `compose --output` is for multiple wired implementations. Proof certificates appear only beside an output analysis after an exhausted violation search finds no violation. Composition reuses only an exact digest match.

## Executable Property Loop

Behavioral contracts and temporal promises use typed observations, explicit assumptions, closed expressions, and dimensionally valid quantities. Use `property check` to type-check, `evaluate` for real invocation or transition records, `search` for a witness or counterexample, `analyze` for finite or solver-backed obligations, `monitor` for a fail-open streaming prefix, and `replay` for recorded `rms/property-analysis/v0.2` evidence. A bounded or open run is inconclusive, never proof.

For a transition-count bound, declare the metric observation exactly as `source: {kind: trace-metric, name: transition-count}` with `value: {quantity: transition}`; put `{metric: <observation-id>, value: <decimal>, unit: transition}` in the temporal bound. `rms spec plan` renders a complete bounded-response example for other agents and dimensions.

For finite machine-, protocol-, resource-, artifact-, or composition-scope proof, use `strategy: deterministic-exhaustive` with `exhaustive: true`. Without the explicit flag RMS treats the realization as non-exhaustive.

## Proof-First Bug Hunting

Derive strong lanes from risk when adding or changing software: generated or exhaustive checks for pure and numeric decisions; finite exploration for machines and workflows; coverage fuzzing for untrusted boundaries; schedule and fault exploration for distributed behavior; analyzers or sanitizers for unsafe authority; mutation testing for important reusable oracles; and real-trace evaluation plus violation search for temporal promises. Preserve historical counterexamples as smoke replays.

Fast commit checks require the appropriate lane or a focused `verification.hunt_exceptions` reason, but do not run overnight work. Use `rms hunt --root . --dry-run` to inspect the campaign and `rms hunt --root . --budget 8h` from a clean commit to run it. `clean-under-recorded-bounds` is bounded evidence, not a bug-free guarantee.

## Coverage-Aware Local Cadence

Use `rms check --changes --root .` before an authorized candidate commit and `rms check --committed --root .` after it. These local modes select exact changed RMS owners. They add declared reverse consumers only when a public contract, capability, effect, export, or other consumer-visible projection changed. Their deterministic receipt explains every selected, native, outside-coverage, and skipped path.

The check compares the selected candidate closure with the same baseline closure. New candidate regressions block the delta. Unchanged baseline debt stays visible but does not masquerade as a new failure. A native workflow is a project-owned handoff: run its local proof commands, preserve its release and hardware gates, and never call its paths RMS-certified. An outside-coverage path records a gap and continues through the project-native workflow; RMS adoption remains separate architecture work.

Repositories declare only project-specific native routing under `.rms/config.yaml`:

```yaml
workspace:
  coverage: progressive
  native_workflows:
    - id: native-client
      paths: [clients/native]
      consumes: [service-provider]
      proof:
        local: [native-test]
        release: [native-release]
        hardware: [native-hardware]
```

RMS reports these commands but does not execute them. Keep consumer adoption ledgers, deployment runbooks, release decisions, and hardware procedures project-owned. Use `rms check --all --root .` for exhaustive release or CI certification. `--all` retains strict full-repository provenance, composition, proof regeneration, and coverage requirements.
