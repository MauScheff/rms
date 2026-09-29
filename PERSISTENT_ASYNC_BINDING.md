# Persistent Async Rust Binding

Status: candidate binding extension. The native RMS maintainer workflow owns this change.

## Scope and compatibility

An effectful Rust machine may retain its state, complete records, and pending effect futures in a persistent runtime. The driver may poll external input while an effect remains pending. The binding does not change machine variants, contracts, effects, or authority.

An omitted execution binding retains the existing synchronous signature checks. An explicit `kind: synchronous` restores those checks. The new variant requires implementation v0.2 and Rust. Unsupported source forms fail closed.

Existing declarations require no migration. Older RMS binaries do not support the new declaration. Apply a new declaration only with a candidate or release that supports it. Rollback to synchronous requires a real synchronous realization; removing the declaration is not an exemption for async code.

The structural analyzer checks declared storage and callable types. It does not prove fairness, cancellation, correlation equality, or drop safety from field names. Executable properties must prove these behaviors. Effect analysis remains independent. Unknown calls and undeclared adapter authority remain blockers.

## Candidate declaration

Use receipt-gated `rms machine apply` with dry-run first. Do not edit a canonical manifest directly.

```yaml
spec: rms/machine-change/v0.1
module: implementation.yaml
machine:
  mode: workflow-effect-machine
  execution_binding:
    kind: persistent-async
    runtime: src/machine_driver.rs#AdmissionDriver
    state_field: state
    records_field: records
    pending_field: pending
    input_poll: src/machine_driver.rs#AdmissionAdapters::poll_input
    executors:
      - symbol: src/start_media_executor.rs#execute_start_media
        request_parameter: 1
      - symbol: src/stop_media_executor.rs#execute_stop_media
        request_parameter: 1
      - symbol: src/release_grant_executor.rs#execute_release_grant
        request_parameter: 1
```

Each executor entry must match one declared effect protocol. The zero-based request parameter must have the existing `effect_envelope` type. Its future output must have the existing `effect_result_envelope` type. The `effect` and `effect_result` enum bindings remain separate.

The runtime fields must contain the existing state type, `Vec<TransitionRecord>`, and `Vec<Future<Output = EffectResultEnvelope>>`. The driver must accept a mutable reference to that runtime. The input poll method must return `Poll<Option<Input>>`. Type aliases must resolve without ambiguity or cycles. The analyzer must not infer that an arbitrary generic type is a future or a record collection.

## Proof boundary

Native regressions cover explicit opt-in, legacy signatures, wrong storage types, wrong envelope output, alias cycles, ambiguous symbols, and preserved authority failures. Consumer lifecycle tests must cover cancellation during pending start, late completion, repeated operations, exact correlation, and retained work after driving-future suspension or drop.

The state-space delta is one closed binding alternative. No runtime state or authority is added. A candidate binary may inspect this binding. It does not certify the consumer until all structural, authority, and lifecycle proof passes.
