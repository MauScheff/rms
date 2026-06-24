# Contract Evidence: capture-intent-workbench

Covered by `cargo test --manifest-path tooling/rust/rms/Cargo.toml`.

Executable coverage:

- `intent_prompt_gates_implementation_on_accepted_context` verifies the rendered `rms.intent@v1` prompt names the think-before-code gate, asks for normalized stories and accepted interpretation, names canonical artifacts to update before implementation, and requires an implementation gate result.

The prompt is advisory. Accepted intent and rationale become authoritative only when encoded in canonical RMS artifacts such as intent notes, decision records, glossary entries, module manifests, contracts, laws, and verification evidence.
