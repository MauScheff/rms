# Draft Schemas

These JSON Schemas validate the structural shape of RMS 0.1 manifests:

- `system.schema.json`
- `module.schema.json`
- `contract-v0.2.schema.json`
- `contract-v0.3.schema.json`
- `contract.schema.json` (migration reader only)
- `context-map.schema.json`
- `implementation.schema.json`
- `implementation-v0.2.schema.json`
- `effect-analysis.schema.json`
- `composition-model.schema.json`
- `proof-certificate.schema.json`
- `machine-probe.schema.json`
- `machine-probe-v2.schema.json`
- `probe-assembly.schema.json`
- `probe-assembly-v0.2.schema.json`
- `probe-assembly-v0.3.schema.json`
- `probe-system-trace.schema.json`
- `property-analysis.schema.json`
- `property-analysis-v0.2.schema.json`
- `property-observation.schema.json`
- `invocation-record.schema.json`
- `test-execution-receipt.schema.json`
- `compatibility-analysis.schema.json`
- `hunt-lane-result.schema.json`
- `hunt-report.schema.json`
- `hunt-report-v0.2.schema.json`
- `probe-counterexample.schema.json`
- `conformance.schema.json`

Schema validation is necessary but not sufficient. Semantic conformance also requires ownership, dependency, effect, compatibility, verification, and profile checks that cannot be expressed fully in JSON Schema.

## Functional Core Schemas

| Schema | Producer | Meaning |
| --- | --- | --- |
| `implementation-v0.2.schema.json` | Canonical authoring or `rms binding migrate` | Declares `pure|effectful`, `internal|boundary`, exact authority rows, and exact safe facades. Production checks require this version. |
| `effect-analysis.schema.json` | `rms structure`, verification, audit, and checks | Records source/tool digests, direct and resolved calls, inferred direct/transitive authorities, unresolved calls, and function verdicts. It is derived evidence. |
| `composition-model.schema.json` | `rms compose --output` | Records participant digests, symbolic vector state, wiring, protocol routes, effect/authority unions, generators, obligations, algebraic reductions, and reused proofs. It is derived evidence. |
| `proof-certificate.schema.json` | Exhausted `rms property search --goal violate --out` | Binds a satisfied universal finite result to exact subject, implementation, source, tool, strategy, assumptions, and evidence digests. |

`implementation.schema.json` remains the v0.1 migration reader. Do not author new production bindings against it. Generated effect analyses, composition models, and proof certificates never authorize semantic edits.

Hunt report v0.2 configurations retain an optional selected module or direct probe assembly plus the explicit output path so resume can restore the exact campaign configuration.
