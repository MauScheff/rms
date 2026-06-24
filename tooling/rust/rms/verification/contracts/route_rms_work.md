# Contract Evidence: route-rms-work

Promise:

- `rms route <module.yaml> --task "<task>"` derives a route recommendation from canonical module manifests, composition children, parent exports, semantic shapes, public surfaces, and task language.
- The route report is advisory evidence. It does not create or change module ownership.

Evidence:

- `route_recommends_domain_child_for_rule_task` verifies that a composite capability routes rule/invariant work to the `domain-engine` child.
- `route_recommends_boundary_child_for_cli_task` verifies that a composite capability routes CLI/boundary work to the `boundary-adapter` child.
- `route_non_composite_targets_current_module` verifies that a module without declared children remains the owning target.

Source revision: recorded by release or conformance tooling.
