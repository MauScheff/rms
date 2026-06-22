# Examples

- `minimal/` shows the smallest useful system, module, context map, and implementation binding.
- `commerce/` shows a stateful distributed boundary module (`payments`) and a long-running workflow module (`checkout`).
- `rust/` shows the first language binding fixture: a Cargo library with a Rust implementation binding.
- `swift/` shows the second language binding fixture: a Swift package with a Swift implementation binding.

The examples illustrate semantics rather than prescribing a language, framework, deployment topology, or persistence model.

The minimal example also includes a machine-readable partial-conformance report. Current examples keep referenced contracts and evidence paths present so the reference validator can run cleanly.
