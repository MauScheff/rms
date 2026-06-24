# Contract Evidence: plan-module-change

Covered by CLI smoke execution of `rms plan <module> --task "<task>"` and unit prompt rendering. The prompt asks for owner and surface classification, scope expansion or module-boundary decisions, semantic shape before file layout, compatibility impact, affected invariants/effects/profiles, implementation outline, and focused verification.

`plan_prompt_routes_composite_parent_to_boundary_child` verifies that a composite parent prompt includes route evidence and recommends the boundary child for CLI/parser/display work.

Optional provider execution uses the same rendered prompt and stores provider output under the generated run record.
