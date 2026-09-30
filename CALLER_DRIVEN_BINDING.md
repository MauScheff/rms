# Caller-driven Rust request providers

Status: bounded binding extension. The native RMS maintainer workflow owns this change.

## Problem and decision

A pure provider can return a request without executing that request. RMS previously
required a local driver and executor for every declared machine effect. That rule
forced a pure reusable provider to claim execution that its caller actually owns.

Rust implementation v0.2 adds one closed `execution_binding` variant:
`caller-driven`. The provider uses the existing `stateful-transition-machine`.
No new machine mode, topology, message kind, authority, or runtime state is added.

```yaml
machine:
  mode: stateful-transition-machine
  execution_binding: {kind: caller-driven}
  transition_record_function: transition_record
  effects:
    set: [RequestSeal]
  effect_results:
    set: [Sealed, SealFailed]
  effect_protocols:
    set:
      - effect: RequestSeal
        results: [Sealed, SealFailed]
        atomicity: one-request-one-result
```

This fragment belongs in a complete receipt-gated semantic change. It is not a
standalone product contract. The product owns payloads, identities, cases, laws,
resource lifecycles, and evidence.

## Ownership and proof

- The provider maps explicit State and Input to Transition. The pure record
  function returns TransitionRecord. One callable reaches the other. A record
  wrapper and a transition wrapper over a record-producing core are both valid.
- Requests remain effects. Results remain effect results. Both retain their typed
  envelopes and ordinary canonical protocol, case, trace, and property checks.
- The caller retains state and records, executes requests, and returns correlated
  result facts. The provider has no local driver, executor, effect-support role,
  runnable surface, or effectful semantic function.
- Source authority analysis remains mandatory. A pure declaration cannot hide IO,
  unresolved calls, or dynamic dispatch. Both transition callables have declared
  pure semantic owners and belong to transition-role source files.
- Provider properties prove its decisions for stale and duplicate results,
  cancellation, replay, and cleanup requests. They do not prove actual cleanup.
  Caller-native integration tests prove execution, correlation, and resource release.
- Composed exploration still requires real request/result bridges or explicit
  substitutes. This binding does not close an execution gap in a composition.

The typed-design `effects` fact describes execution owned by the proposed module.
An explicitly pure request provider can use `effects: absent` and
`lifecycle: required`. Its canonical machine still declares request descriptors
under `effects`. An effect's free-form `kind` does not grant this exemption.

## Migration and compatibility

Omission preserves all existing execution checks. `synchronous`,
`synchronous-envelopes`, and `persistent-async` retain their existing meanings.
Older RMS binaries reject `caller-driven`; install a supporting revision first.
Other language bindings are not included in this extension.

A generic workflow scaffold is a placeholder, not the final architecture.
Through one semantic change, select the new binding, replace the scaffold machine
and protocols, and remove scaffold driver/executor roles and semantic functions.
Omit `driver_function`; apply removes the inherited driver declaration. Explicit
local executor declarations fail validation. Replace scaffold source and proof
with product-owned pure roles before claiming completion. Do not hand-edit manifests.

Returning to local execution requires real drivers, executors, authority, and proof.
Removing the opt-in alone does not exempt a provider from legacy checks.

## State-space review

One binding choice distinguishes caller-owned execution from provider-owned
execution. Mixed local and caller execution is not introduced. Signature and
ownership checks are deterministic. No scheduling choice is added. Existing
replay inputs and transition records retain request identity. Native regressions
cover the opt-in, protocol preservation, invalid ownership, pure call paths, and
legacy refusal. Consumer tests remain the authority for product lifecycle behavior.
