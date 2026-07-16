# Contract Evidence: manage-rms-adoption

Promise: `rms adoption status/set` reports exact coverage, supports dry-run updates, validates module closures, and blocks complete coverage with unowned production paths.

Command/tool:

- `cargo test -p rms adoption_complete_rejects_unowned_production_paths`
- `cargo test -p rms init_adopt_`

Expected result: progressive adoption is preserved across init and config upgrade, JSON includes deterministic adoption diagnostics, and invalid complete promotion does not mutate configuration.

Source revision: strict audit binds this evidence to the committed candidate.
