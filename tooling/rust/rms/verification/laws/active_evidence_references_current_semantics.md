# Law Evidence: active evidence references current semantics

Promise:

- Law `active-evidence-references-current-semantics` rejects qualified references to removed machine variants in active evidence.

Scenario:

- Add `StaleEvidenceVariantCommand.Removed` to an active evidence file while `Removed` is absent from the canonical command variants.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml active_evidence_rejects_removed_qualified_variant`

Expected result:

- RMS reports `evidence.semantic-reference-stale` with the removed qualified variant.

Observed result: the focused regression passed. Source revision is resolved from the committed candidate by strict audit.
