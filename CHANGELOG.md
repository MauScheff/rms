# Changelog

## Unreleased

- Added `rms package` portable module package directories with conformance reports and SHA-256 file checksums.
- Added contract schema validation with structured preconditions and postconditions.
- Added implementation `semantic_functions` for mapping source symbols to contracts, invariants, assumptions, and evidence.
- Added Rust validation for semantic function source symbols.
- Added the `prune-module` skill and semantic-residue guidance for continuously removing unneeded artifacts.
- Added `rms compose` manifest-level module composition checks.
- Added Swift binding scaffolding, validation checks, and `examples/swift`.
- Added `rms check-compat` manifest-level compatibility classification.
- Tightened agent guidance for ADTs, validated constructors, explicit result types, boundary schemas, state machines, and negative verification.
- Added the `refactor-module` skill for behavior-preserving RMS module refactors.
- Added `rms init` and `rms add-module` scaffolding commands.
- Added Rust module scaffolding for `rms add-module --binding rust`.
- Added the first Rust language binding checks for Cargo manifests, package identity, public entrypoints, crate dependency allowlists, and public modules.
- Added source-level Rust binding checks for import roots and public re-exports.
- Added Rust typing checks for primitive aliases, public fields, failure discipline, constructor evidence, and Stateful representation declarations.
- Added `examples/rust` as a Rust binding fixture.
- Added embedded JSON Schema validation to the Rust CLI.
- Added conformance-report discovery and explicit `--conformance` validation.
- Set Swift as the next planned language binding.
- Added the first Rust reference CLI with validation, inspection, context packet, conformance, and verification commands.
- Added a thin Codex plugin wrapper that packages canonical RMS skills.
- Added concrete example fixture contracts and verification markers so examples pass reference validation.
- Rewrote the README for public setup and adoption.

## 0.1.0 Canonical Draft — 2026-06-20

- Froze the semantic core for pilot use.
- Clarified canonical artifacts as a coherent set rather than a precedence ladder.
- Added portable module-package and composition requirements.
- Added service constraints to operational substitutability.
- Added reproducible conformance reports tied to source or artifact identity.
- Added agent, plugin, secret, and supply-chain trust guidance.
- Added a composition skill and conformance-report schema.
- Kept language and agent integrations outside the semantic core.

## 0.1 Draft

- Initial guide, specification, manifests, schemas, examples, skills, and Codex/Claude Code adapters.
