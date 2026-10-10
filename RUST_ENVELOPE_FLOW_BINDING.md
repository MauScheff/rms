# Closed Rust Envelope Flow

The Rust implementation v0.2 binding can prove one closed envelope projection flow. This opt-in permits an executor to consume an opaque context and matched payload fields instead of the full request envelope. It does not change a public contract, effect protocol, authority, or scheduling rule.

## Authority and compatibility

The native RMS maintainer workflow owns this binding extension. Consumer projects select the binding through receipt-gated machine or semantic apply. A report is evidence, not permission to edit canonical artifacts.

An omitted `envelope_flow` retains the exact-envelope signature rule. Existing bindings need no migration. Older RMS binaries reject the new fields. Every executor in an opted-in binding uses the same proved context. Mixed projected and full-envelope executors are not supported.

The proof assumes a valid Rust build and the ordinary Rust standard library. Native build, protocol, IO, authority, atomicity, lifecycle, and release checks remain required. This source-flow proof does not certify an external operation's business meaning or result accuracy.

## Canonical declaration

Set `machine.execution_binding` in a `rms/machine-change/v0.1` packet or a `rms/semantic-change/v0.1` packet. Preserve the existing binding kind and its runtime fields. Add `envelope_flow` and each executor's exact `status_symbol`.

```yaml
spec: rms/machine-change/v0.1
module: implementation.yaml
machine:
  mode: workflow-effect-machine
  execution_binding:
    kind: persistent-async
    runtime: src/runtime.rs#Runtime
    state_field: state
    records_field: records
    pending_field: pending
    input_poll: src/runtime.rs#Inputs::poll
    envelope_flow:
      context: src/representation.rs#IssuedContext
      split: src/representation.rs#IssuedContext::split
      observe: src/representation.rs#IssuedContext::observed
      dispatcher: src/runtime.rs#dispatch_issued
      request_parameter: 1
    executors:
      - symbol: src/read.rs#execute
        request_parameter: 1
        status_symbol: src/io.rs#Ports::read
      - symbol: src/stop.rs#execute
        request_parameter: 1
        status_symbol: src/io.rs#Ports::stop
```

`request_parameter` is a zero-based source signature index. The flow index selects the dispatcher's owned request envelope. Each executor index selects its owned context. Resource parameters occupy the other indices.

The same `envelope_flow` and `status_symbol` fields are supported by `kind: synchronous-envelopes`. A synchronous dispatcher returns the exact result envelope. Synchronous executors and status operations do not use `await`.

Use the ready route receipt on dry-run and apply:

```sh
rms machine apply implementation.yaml --change-file envelope-flow.yaml --route-receipt <RUN_ID> --dry-run
rms machine apply implementation.yaml --change-file envelope-flow.yaml --route-receipt <RUN_ID>
```

The canonical effect protocols select variant-to-executor mappings. The canonical message declarations select identity fields. The packet does not supply an identity-field subset or an alternative protocol map.

## Adoption when the old signature rule fails

An existing projected executor can fail the default exact-envelope rule before adoption. Obtain a fresh owner-local machine-binding repair route. Select the exact owning `module.yaml`, not `implementation.yaml`.

```sh
rms next "Adopt the closed envelope-flow execution binding declaration for the existing <module> implementation. Preserve all contracts, states, lifecycle, effects, authorities, and dependencies. No source change or new product meaning is requested." --root <root> --module <root>/modules/<module>/module.yaml --ai
```

This recovery route requires an explicit owner and only recognized owner-local binding errors. The exact `path.rs#symbol request parameter must be the exact EffectEnvelope` finding qualifies. Other persistent-binding findings, closed-flow proof failures, authority errors, canonical drift, and invalid schemas in the selected owner still block the route. Repository-root errors and unresolved intent also block it. Unrelated module debt remains visible under the existing owner-scoped routing rules; recovery does not certify that debt or waive final proof.

A ready recovery receipt permits focused machine apply, not spec apply or source edits. Apply only the reviewed execution-binding declaration. Use the new receipt for dry-run and write mode. Do not substitute an older ready receipt for a blocked result. Source repair needs its own ready private-implementation route. Final source conformance, native proof, candidate checks, and committed checks remain required. Recovery readiness does not certify the current source or the proposed projection.

## Supported source shape

The context is one source-owned, non-generic struct. Its fields are private. The context has no derives and no trait implementation. Its one inherent implementation contains exactly the registered split and observe methods. Documentation attributes are permitted. The context is not publicly exported outside the crate.

The context fields are exactly the result envelope fields other than its unique typed result payload. Each identity field has the same resolved type in the request, context, and result. The request may retain canonical request-only fields, such as an idempotency key.

The splitter consumes one owned request envelope. Its only expression is a tuple. The first tuple member constructs `Self` from unchanged, same-name request field projections. The second tuple member moves the request's exact effect field.

```rust
fn split(request: RequestEnvelope) -> (Self, Effect) {
    (
        Self {
            effect_id: request.effect_id,
            requester: request.requester,
            correlation_id: request.correlation_id,
            causation_id: request.causation_id,
        },
        request.effect,
    )
}
```

The observer consumes `self` and one exact result payload. Its only expression constructs the declared result envelope. Each identity is an unchanged, same-name `self` field projection. The status field is the passed status value. Struct-update syntax, extra fields, missing fields, and computed projections are rejected.

A dispatcher accepts an owned request and explicit source-owned resource parameters. It is a free function. Its body splits once and immediately matches the returned effect. An async function can use this body directly. A non-async persistent dispatcher can return `Box::pin(async move { ... })` with exactly this body. An existing exact future alias is permitted.

```rust
fn dispatch_issued(ports: Arc<Ports>, request: RequestEnvelope) -> Pending {
    Box::pin(async move {
        let (context, effect) = IssuedContext::split(request);
        match effect {
            Effect::Read { key } => crate::read::execute(ports, context, key).await,
            Effect::Stop => crate::stop::execute(ports, context).await,
        }
    })
}
```

Each arm names one exact variant. Unit, tuple, and named-field variants are supported. An arm binds every payload field once. The arm calls only its canonical executor. The call moves the same context and every matched field once. Each payload argument has the exact declared field type. Other arguments are explicit source-owned resources. Scalar substitutions, wildcard arms, guards, rest patterns, aliases, cloning, shadowing, and duplicate calls are rejected.

The registered dispatcher does not contain admission policy, resource preparation, prefix statements, or another lifecycle. A native wrapper may perform existing host work before it passes its owned request and prepared resources to this dispatcher. The binding does not prove that wrapper's issuance or scheduling policy.

Each executor is crate-private. Its body calls the registered status operation with its exact operation parameters. It passes that operation's actual returned status to the registered observer. Immutable status binding and direct nesting are supported.

```rust
async fn execute(ports: Arc<Ports>, context: IssuedContext, key: Payload)
    -> ResultEnvelope
{
    let status = crate::io::Ports::read(&ports, key).await;
    context.observed(status)
}
```

The status operation has an exact source symbol and returns the declared result enum. The executor does not manufacture, transform, discard, or replace its returned status. Extra statements, callbacks, loops, branches, mutable bindings, unrelated arguments, and context arguments to the status operation are rejected. The operation's IO and scheduling remain subject to the existing independent checks.

## Source closure and refusal

The proof checks the complete inspected Rust source corpus. Each registered source file must be wired through one ordinary `src/lib.rs` or `src/main.rs` root. Missing modules, ambiguous module files, path remapping, conditional out-of-line module declarations, and module-import shadowing are rejected.

Flow-participating files use explicit, unaliased imports. Context, payload, identity, and result type aliases are unsupported. Proof types must have unique declarations, including nested scopes. Additional context signatures, fields, constructors, implementations, callable aliases, and unregistered executor or split/observe references are rejected. Conservative same-name collisions also require source clarification; the proof does not guess an owner.

Explicit imports and re-exports may retain unqualified built-in `doc` attributes, including documentation comments, `#[doc = "text"]`, `#[doc(hidden)]`, and `#[doc(alias = "name")]`. These attributes do not grant identity or provenance. All other import attributes remain unsupported, including `cfg`, `cfg_attr`, `path`, custom attributes, and procedural attributes. Aliases, globs, and ambiguous import names remain rejected. An import refusal identifies the source file, its top-level import scope, exact import spelling, and actual cause. Derive and macro checks retain this cause instead of replacing it with a generic identity failure.

Source-generating macros and unsupported attributes are rejected. This includes arbitrary procedural derives, `cfg_attr`, `include!`, and custom test attributes. The checker permits a closed set of ordinary standard macros only when their identity is unshadowed and their tokens contain no flow type or registered callable. Unsupported syntax remains a blocker.

## Proof boundary and evidence

The proof covers header and payload provenance within one dispatcher invocation. It proves that each matched branch uses the original context once and preserves exact result identity. It does not prove globally unique request issuance, fairness, cancellation, drop safety, stale-result policy, or the external operation's semantics.

The source flow does not clone or copy the context or payload. A native regression uses non-Clone types, executes the dispatch-to-result path, checks four identities, and checks that a `Vec<u8>` retains the same allocation. This evidence does not certify buffer-copy behavior inside arbitrary consumer IO operations.

Native regressions cover synchronous and persistent forms, unit/named/tuple variants, canonical mutation and schema round-trip, missing canonical evidence, default rejection, swapped or dropped identities, forged status, extra constructors, aliases, shadowing, duplicate dispatch, unrelated payloads, unregistered calls, and hidden source generation. Full native RMS unit and CLI suites remain the completion gates for RMS maintenance.

The state-space delta is one explicit binding choice: full envelope or closed projection. A projection has one context, one splitter, one observer, one dispatcher, and a complete executor/status-operation map. A failed proof never grants partial projected acceptance. No machine state, effect, protocol, dependency, or authority is added.
