# Changelog

## Unreleased

- Resolve Swift declaration bodies by exact type identifier so a longer helper such as `TransitionWitness` cannot shadow the declared `Transition` output during rejection-channel verification.

- Treat the stable transition `case` as the exact canonical branch discriminator within `input-only` and `state-and-input` signatures, reject duplicate dispatch cases during machine apply, avoid prefix-matching one replay branch to another, and make compact check output disclose omitted failure counts while `--details` shows every reason.
- Keep generated Rust boundary fuzz runners visibly connected to their declared constructor operation so strict semantic-operation proof accepts clean-room scaffolds without an exemption.

- Added language-neutral RMS behavioral contracts with `rms/contract/v0.2`, stable executable clauses, exhaustive accepted/rejected cases, explicit frames, query purity, caller/provider/evidence blame, and canonical invocation records for stateless calls.
- Generalized the property evaluator from temporal expressions to shared step and temporal expressions; added `rms/property-analysis/v0.2`, deterministic SMT-LIB v2 emission, optional cvc5 evidence with RMS model revalidation, behavioral compatibility refinement, and fail-open production monitoring.
- Added non-overwriting `rms spec migrate-contract`, migrated every tracked public contract without renaming files or changing public versions, and made legacy v0.1 contracts migration-only.
- Added one record protocol and conformance corpus across Rust, Swift, JavaScript, Python, and executable bindings. RMS generates no host-language contract wrappers or predicates.
- Added finding-first guided hunts over declared probe assemblies: seeded semantic-novelty scheduling, continued search after failures, up to eight distinct targeted-replay counterexamples, stable deduplicated `rms/hunt-report/v0.2` findings, and bounded-evidence-only proof semantics.
- Added optional `rms/probe-assembly/v0.2` public-example workloads with per-action budgets and exact recorded input replay; v0.1 assemblies and hunt reports remain readable.
- Added direct `rms hunt --assembly` dogfood, root/repository-relative target resolution, declaration-closure digests, configuration-faithful and finalized-report-immutable resume, finalized checkpoints, extension-aware report output, build-revision version provenance, progress lines, clarified frontier metrics, and finding-first human replay summaries.
- Added deterministic bounded multi-module probing with canonical protocol/dependency routing, virtual-time scheduling, explicit transport faults and substitutes, batched v0.2 adapters with v0.1 fallback, minimized replayable counterexamples, canonical regression execution, and plain Lawbook causal timelines.
- Added maintainer understandability laws, state-space review, comprehension evidence, and staged self-hosting guidance; made public workflow actions and ownership resolution construction-safe internally while preserving `rms.surface/v2`.
- Added first-class inspectable Python implementation bindings with `pyproject.toml` packaging, role-separated source, `unittest` proof runners, probes, traces, properties, native tree-sitter inspection, dependency allowlists, capability composition, examples, CI, and release-gate coverage.
- Published `probe-machine.v2` with Python support while retaining v1 unchanged.
- Made composite export mutations preservation-safe: null, omitted, and inert collection changes preserve existing exports, while an explicit `composition_exports.set: []` remains the intentional delete-all operation.

## 0.1.0-rc.8

- Added first-class `rms probe` support for pure, ephemeral command/event/effect-result sequences through Rust, Swift, JavaScript, and Python transition-record adapters, including discovery, protocol/schema validation, expectations, trace output, scaffold generation, verification handshakes, and maintained dogfood examples.
- Replaced the former recorded-timeline replay subcommand directly with `rms trace show`; no compatibility alias is recognized.
- Made probe bindings and initial machine state mandatory for inspectable Rust, Swift, JavaScript, and Python implementations.
- Narrowed managed agent routing so `rms next` begins software-change work, while read-only investigation, explanation, review, status/history inspection, ordinary Git or tool operations, and discussion remain native until a concrete change is proposed.
- Give Rust test threads enough stack for the complete Clap command tree, and run the CLI version assertion through the built executable so Linux CI verifies the real `rms --version` boundary.

## 0.1.0-rc.7

- Made the generated intent JSON Schema compatible with strict structured-output providers by explicitly typing every constant and enum node.
- Added regression coverage that rejects any future untyped `const` or `enum` in the provider schema.

## 0.1.0-rc.6

- Made structured intent provider failures terminal and auditable: failed runs now preserve complete artifacts, issue a non-ready empty-action receipt, select no owner, and prescribe explicit provider recovery without automatic typed-intent fallback.
- Made every semantically non-ready route explicitly ownerless, removing tentative owner context and owner-scoped implementation steps while retaining candidates only as non-authoritative evidence.
- Updated managed Codex/Claude guidance and packaged RMS skills to prohibit inferring an owner from candidates, neighboring modules, inspection context, or implementation language.

## 0.1.0-rc.5

Compatibility impact: intentionally breaking presentation and agent-JSON revision. The primary CLI is now the five-command `init`, `next`, `explain`, `check`, and `view` surface; specialist commands remain available through `rms help --all`.

- Required ready, current, target-compatible route receipts for canonical semantic and topology mutations, including dry-runs.
- Made explicit provider intent extraction schema-constrained, single-repair, root-local cached, concurrency-safe, and fully auditable through per-invocation run artifacts.
- Added exact coverage and proof projections that distinguish observed execution from declared-but-unobserved evidence and name every selected RMS closure.
- Added receipt-gated `rms release prepare` so RMS can update its own canonical and package version metadata without hand-editing manifests.
- Changed `rms next` to positional intent and added repository-operation classification with `no-rms-change` results.
- Made `rms explain` deterministic and answer-first, with complete canonical evidence behind `--details`.
- Added `rms check` modes for environment, canonical system, working candidate, and committed-candidate proof.
- Added the versioned `rms.surface/v2` JSON envelope with typed command and manual actions.
- Compressed generated agent guidance and consolidated public documentation around the narrow-waist workflow.
- Replaced raw-language architecture heuristics with `rms/intent-model/v0.1`: agents extract typed facts, RMS validates them, and deterministic policy selects topology.
- Renamed recursive scaffolding to `rms add-capability-tree` and removed `rms add-capability`; standalone capability publication now uses typed `contracts.* kind: capability` changes plus behavior bindings.
- Added progressive and complete workspace coverage, `rms adoption status/set`, and module-scoped changes/committed checks.
- Made `rms init --adopt` preserve project documents while setting progressive coverage, including upgrades of existing managed configuration.

## 0.1.0-rc.4 - 2026-06-30

Compatibility impact: additive within RMS 0.1. Existing manifests, packages, examples, `rms init` invocations, `rms add-module` invocations, and `rms add-capability` invocations remain compatible. Strict production audit is intentionally stronger and may now fail projects that lack git source provenance or source-pinned evidence.

Known limitations:

- RMS 0.1 remains a pilot draft, not a 1.0 compatibility promise.
- Production pilot use requires project CI, source-pinned evidence, and human review for domain, security, operational, and product risk.
- Optional agent plugins remain convenience packaging; project-local RMS guidance and the `rms` CLI remain sufficient.

- Added `PRODUCTION.md` with the production-pilot RMS operating gate, evidence provenance rules, agent bootstrap flow, and release decision table.
- Added `templates/ci/github-actions-rms-project.yml` as a downstream RMS project CI template.
- Added `rms audit --root <path> --strict` as the production-readiness gate for RMS projects.
- Added strict audit source-provenance checks so production audit fails when the audit root has no resolvable source revision.
- Added local trace bundle first-bad-transition metadata support in `rms trace diagnose`.
- Added `rms add-capability` recursive capability scaffolding for composite parent plus domain and boundary children.
- Added semantic inner-structure scaffolding for domain-named machines, ADTs, message envelopes, transition outputs, transition records, replay bundles, and first-bad-transition evidence.
- Added Codex and Claude project-local agent bootstrap and sync support.
- Added `rms --version` for quick installed CLI version checks.
- Added `rms add-module --binding executable` for opaque command-backed modules, with generated build/smoke scripts, boundary evidence, validation, and release scaffold coverage.
- Added empty profile sections to `rms add-module` output when Stateful, Distributed, Workflow, or Boundary profiles are requested, so generated manifests validate without inventing module-specific semantics.
- Added Codex provider `workspace-write` execution with module/root write scope, module-scoped execution roots, bounded provider timeout, and run-record metadata for sandbox, write scope, timeout, and execution root.
- Improved `rms gate` and `rms impact` failures outside git repositories with actionable RMS guidance instead of raw git usage text.
- Clarified generated module and skill guidance for query-produced read models that intentionally use `architecture.allowed_missing_constructors`.

## 0.1.0-rc.3 - 2026-06-22

Compatibility impact: additive within RMS 0.1. Existing manifests, packages, examples, `rms init` invocations, and `rms add-module` invocations remain compatible. The `rms-cli` module now publishes the `add-rms-module` command contract to make module scaffolding semantics explicit.

Known limitations:

- RMS 0.1 remains a pilot draft, not a 1.0 compatibility promise.
- Generated module guidance is an operational adapter; canonical manifests and contracts remain the source of module semantics.
- The atlas remains derived evidence and is still being tested through maintainer journeys.

- Added the `add-rms-module` public contract, implementation binding entry, and verification evidence.
- Improved `rms add-module` scaffolding with a module README, public contract guidance, and stronger verification evidence guidance for Codex-ready first modules.

## 0.1.0-rc.2 - 2026-06-22

Compatibility impact: additive within RMS 0.1. Existing manifests, packages, examples, and `rms init` invocations remain compatible. The `rms-cli` module now publishes the `init-rms-system` command contract to make initialization semantics explicit.

Known limitations:

- RMS 0.1 remains a pilot draft, not a 1.0 compatibility promise.
- The generated agent and workbench files are operational adapters; canonical manifests and contracts remain the source of module semantics.
- The atlas remains derived evidence and is still being tested through maintainer journeys.

- Added `rms init` agent/workbench bootstrap output: `AGENTS.md`, `.rms/config.yaml`, `.agents/skills/`, and `.gitignore` are now generated with the canonical system files.
- Embedded RMS workflow skills in the CLI so release-installed binaries can scaffold Codex-ready projects without requiring a source checkout.
- Added the `init-rms-system` public contract, implementation binding entry, and verification evidence.
- Documented the new `rms init` output in `README.md`, `QUICKSTART.md`, `TOOLING.md`, the CLI README, and Codex integration guidance.
- Added atlas journey probe evidence and linked it from the `build-module-atlas` implementation evidence.
- Improved the atlas toward a human-centered maintainer workflow, including guided traces and explicit gaps.
- Added git impact/gate workflow support for selecting validation, composition, verification, review, and compatibility obligations from changed RMS artifacts.

## 0.1.0-rc.1 - 2026-06-22

Compatibility impact: additive within RMS 0.1. Existing manifests and examples remain compatible. New CLI commands and release checks expand the workbench surface without changing the RMS semantic core.

Known limitations:

- RMS 0.1 is still a pilot draft, not a 1.0 compatibility promise.
- The Rust and Swift bindings are intentionally shallow static checks; deeper language analysis remains binding work.
- Provider-backed workbench execution currently supports Codex as the first adapter. Claude and local-model adapters are planned without changing RMS semantics.
- GitHub release archives are runner-native artifacts, not a full cross-compilation matrix.

- Added `RELEASE.md` with release authority, version rules, artifact expectations, and done criteria.
- Added `QUICKSTART.md` for first-use proof and `DOGFOOD.md` for using RMS on the `rms-cli` module itself.
- Added a tag-driven GitHub release workflow for runner-native CLI archives, source crate packaging, checksums, and GitHub release publication.
- Added release metadata drift checks across the Cargo package, `rms-cli` module manifest, and packaged Codex plugin manifest.
- Added release-binary and clean-room PATH install smoke to `rms release check`.
- Added `rms atlas` for derived local module atlas JSON and HTML artifacts.
- Added `rms verify-package` package metadata, payload integrity, and included artifact validation.
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
