# Contract Evidence: add-rms-binding

Promise:

- `add-rms-binding` attaches one supported binding to a semantic-only module through the canonical adapter.

Scenario:

- Attach JS to a semantic-only domain module.
- Attempt a duplicate attachment and a conflict against an existing generated path.

Command/tool:

- `cargo test -p rms add_binding -- --nocapture`
- `cargo test -p rms --no-run`

Expected result:

- Supported attachment creates a complete, compilable binding using the existing modular adapter.
- Existing implementations, unsupported bindings, and destination conflicts are explicit failures.
- Failure before installation leaves the semantic-only module coherent.

Source revision: recorded by git commit or strict audit provenance before production use.
