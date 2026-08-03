# Draft Schemas

These JSON Schemas validate the structural shape of RMS 0.1 manifests:

- `system.schema.json`
- `module.schema.json`
- `contract-v0.2.schema.json`
- `contract-v0.3.schema.json`
- `contract.schema.json` (migration reader only)
- `context-map.schema.json`
- `implementation.schema.json`
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
- `compatibility-analysis.schema.json`
- `hunt-lane-result.schema.json`
- `hunt-report.schema.json`
- `hunt-report-v0.2.schema.json`
- `probe-counterexample.schema.json`
- `conformance.schema.json`

Schema validation is necessary but not sufficient. Semantic conformance also requires ownership, dependency, effect, compatibility, verification, and profile checks that cannot be expressed fully in JSON Schema.

Hunt report v0.2 configurations retain an optional selected module or direct probe assembly plus the explicit output path so resume can restore the exact campaign configuration.
