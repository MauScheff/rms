# Contract Evidence: compose-rms-system

Covered by `cargo test --manifest-path Cargo.toml`, including flat and recursive composition cases.

Executable coverage:

- `compose_satisfies_module_provided_capability` verifies a required capability can be satisfied by a discovered module provider.
- `compose_reports_unresolved_capability` verifies missing required capabilities fail composition.
- `compose_reports_module_dependency_cycles` verifies module dependency cycles fail composition.
- `compose_accepts_parent_export_backed_by_internal_child` verifies a parent export backed by an internal child satisfies external consumers through the parent surface.
- `compose_rejects_missing_contained_module` verifies missing declared children fail composition.
- `compose_rejects_export_not_backed_by_child_provides` verifies parent exports must be backed by child `provides`.
- `compose_rejects_external_dependency_on_internal_child` verifies external consumers cannot depend directly on internal children.
- `compose_rejects_child_contained_by_two_parents` verifies a child module cannot have multiple parents.
- `shape_direction_accepts_generated_capability_tree` verifies the generated composite/domain/boundary capability tree does not produce semantic shape warnings.
- `shape_direction_warns_when_domain_requires_boundary_adapter` verifies a domain engine depending on a boundary adapter is reported for review.
- `shape_direction_warns_when_domain_declares_boundary_profile_or_effects` verifies a domain engine with boundary profile or declared effects is reported for review and appears in conformance checks.
- `shape_direction_warns_when_composite_declares_effects` verifies a composite parent with declared effects is reported for review.

Provider execution is not involved; composition is deterministic over discovered canonical artifacts.
