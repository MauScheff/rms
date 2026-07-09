# Law Evidence: Reusable Modules Use Public Facades

Promise:

- Invariant `reusable-module-consumers-use-public-facade` holds.
- Reusable modules declare capability semantics in RMS, not only in native package metadata.
- Consumer code must use the provider's declared public facade or contract-shaped entrypoint instead of private representation, transition, parser, adapter, or port role files.

Command/tool:

- `cargo test -p rms reusable -- --nocapture`
- `cargo test -p rms native_package -- --nocapture`
- `rms validate --root .`
- `rms audit --root . --strict`

Expected result:

- `semantic_completeness_flags_reusable_domain_without_capability` reports `semantic.reusable-capability-missing` and `semantic.reusable-package-evidence-missing` when `x-rms.reusable: true` is declared without capability/package evidence.
- `structure_report_flags_native_package_export_bypassing_public_facade` reports `structure.native-package-export-mismatch` when native exports point only at a private role file.
- Strict audit promotes these semantic/structure diagnostics to production blockers.

Source revision: recorded by the git commit that includes this evidence and enforced by strict audit provenance.
