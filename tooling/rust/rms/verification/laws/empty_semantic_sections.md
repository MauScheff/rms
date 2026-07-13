# Law Evidence: inert semantic template sections

Promise:

- `empty-semantic-change-sections-are-inert`: empty machine, role, and surface operation blocks do not request implementation work.

Scenario:

- Load a semantic-only module and validate a semantic change containing empty machine, role, and surface operations plus one real evidence addition.

Command/tool:

- `cargo test -p rms empty_machine_and_surface_sections_are_inert_without_binding -- --nocapture`

Expected result:

- RMS produces no machine change and emits neither `surface.implementation-missing` nor `semantic.machine-change-empty` for the inert sections.

Source revision: recorded by git commit or strict audit provenance before production use.
