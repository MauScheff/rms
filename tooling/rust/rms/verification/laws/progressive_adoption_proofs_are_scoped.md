# Law Evidence: progressive adoption proofs are scoped

Promise: adoption defaults to progressive coverage, reports the certified closure explicitly, and refuses complete coverage while production paths remain outside RMS ownership.

Scenarios:

- `rms init --adopt` preserves project documents and writes or upgrades progressive workspace coverage.
- Progressive status can report unowned legacy code without claiming it is certified.
- Complete coverage emits `adoption.unowned-production-path` and is blocked.

Command/tool:

- `cargo test -p rms init_adopt_`
- `cargo test -p rms adoption_complete_rejects_unowned_production_paths`

Expected result: adopted workspaces remain usable module by module, and complete-repository claims cannot pass early.

Source revision: strict audit binds this evidence to the committed candidate.
