# RMS Manifest Reference

RMS manifests make module meaning available to people, agents, and deterministic tooling without requiring them to reverse-engineer implementation code.

The manifest is a semantic index, not a duplicate of every detail in the codebase.

## 1. Canonical artifact set

The canonical semantic set is:

```text
System and module manifests
Published contracts and invariants
Context language and glossary
Compatibility declarations
Linked decision records
```

These artifacts must agree. A contradiction is architectural drift and should fail validation; tools and agents must not hide it behind an undocumented precedence rule.

Implementation must conform to the set. Agent instructions and generated summaries may adapt it, but they may not introduce unique architectural truth.

## 2. Files

A typical project uses:

```text
system.yaml          System boundary and composition
context-map.yaml     Relationships between bounded contexts
module.yaml          Semantic module contract
implementation.yaml  Language- and toolchain-specific binding
conformance-report.json  Reproducible result for one evaluated implementation
```

YAML is the canonical exchange format for RMS 0.1 manifests. JSON is used for conformance reports and is equivalent for manifests when it preserves the same model.

### Portable module package

A replaceable module can travel as a directory, archive, registry artifact, library, container, or remote-service descriptor. A conventional package layout is:

```text
module-package/
├── module.yaml
├── contracts/
├── conformance/
│   ├── required/
│   └── reports/
├── implementations/
│   └── <binding>.yaml
├── docs/
└── migrations/          # when state replacement requires it
```

Only the semantic contents are standardized. The transport and presence of source code are not.

## 3. `system.yaml`

Minimal example:

```yaml
spec: rms/system/v0.1

system:
  name: commerce
  version: 1.0.0
  purpose: Sell and fulfill physical products

contexts:
  - ordering
  - payments
  - inventory
  - fulfillment

public_interfaces:
  - name: commerce-api
    contract: contracts/commerce-api.yaml

invariants:
  - id: shipped-orders-are-payable
    statement: Every shipped order has an accepted payment outcome.

compatibility:
  policy: backward-compatible-within-major
```

Recommended fields:

| Field | Meaning |
|---|---|
| `spec` | Manifest schema/version identifier. |
| `system.name` | Stable system identifier. |
| `system.version` | System release or contract version. |
| `system.purpose` | One-sentence reason the system exists. |
| `contexts` | Contained bounded contexts or major modules. |
| `public_interfaces` | APIs, event streams, libraries, CLIs, or other external surfaces. |
| `external_dependencies` | Vendors, platforms, or systems outside the repository boundary. |
| `invariants` | Important system-wide properties. |
| `workflows` | Cross-context workflows owned at system level. |
| `compatibility` | Public compatibility policy. |
| `glossary` | System glossary location. |
| `context_map` | Context-map location. |

## 4. `module.yaml`

Minimal example:

```yaml
spec: rms/module/v0.1

module:
  name: payments
  version: 2.1.0
  kind: bounded-context
  purpose: Authorize, capture, and refund payments

profiles:
  - core
  - stateful
  - distributed
  - boundary

owns:
  concepts:
    - Payment
    - Authorization
    - Capture
  data:
    - payment-records
  decisions:
    - capture-eligibility
    - refund-eligibility

provides:
  commands:
    - name: authorize-payment
      contract: contracts/authorize-payment.yaml
    - name: capture-payment
      contract: contracts/capture-payment.yaml
  queries:
    - name: get-payment-status
      contract: contracts/get-payment-status.yaml
  events:
    - name: payment-captured
      contract: contracts/payment-captured.v1.yaml

requires:
  capabilities:
    - name: payment-gateway
      contract: contracts/payment-gateway.yaml
    - name: event-store
      contract: contracts/event-store.yaml

invariants:
  - id: capture-requires-authorization
    statement: A payment can be captured only after authorization.
    enforced_by: payment-aggregate
    verified_by: verification/laws/capture_requires_authorization

  - id: refund-within-capture
    statement: Total refunded amount never exceeds captured amount.
    enforced_by: payment-aggregate
    verified_by: verification/laws/refund_within_capture

effects:
  - name: payment-gateway
    kind: external-financial-operation
    semantics:
      idempotency: command-id
      ordering: per-payment
      timeout: unknown-outcome
      retry: same-idempotency-key-only
      compensation: refund-payment
      reconciliation: required

state:
  model: docs/payment-lifecycle.md
  consistency_boundary: one-payment
  concurrency: optimistic-version
  migration_policy: versioned-upcasters

compatibility:
  policy: backward-compatible-within-major
  events: additive-fields-only-within-version

verification:
  laws:
    - verification/laws
  contracts:
    - verification/contracts
  scenarios:
    - verification/scenarios
  boundaries:
    - verification/boundaries

operations:
  observability:
    correlation: payment-id
    causation: command-or-event-id
  runtime_checks:
    - ops/checks/payment-invariants
  reconciliation:
    - ops/reconcile/payment-provider
  runbooks:
    - ops/runbooks/unknown-payment-outcome.md
```

### Required semantic sections

An RMS module should provide these sections, even when some lists are empty:

```text
module
profiles
owns
provides
requires
invariants
effects
compatibility
verification
operations       # when required by profiles or effects
```

### `module`

| Field | Meaning |
|---|---|
| `name` | Stable machine-readable identifier. |
| `version` | Public semantic version or project-defined compatible equivalent. |
| `kind` | `bounded-context`, `module`, `workflow`, `adapter`, `library`, or extension. |
| `purpose` | One clear sentence describing responsibility. |
| `owner` | Optional team, role, or governance owner. |
| `status` | Optional lifecycle status such as `experimental`, `active`, or `deprecated`. |

### `profiles`

Allowed core profiles:

```text
core
stateful
distributed
workflow
boundary
```

`core` is always required. Profiles activate additional semantic requirements from `SPEC.md`.

### `owns`

Ownership may include:

```text
concepts     Domain terms and models
data         Logical data sets or state
identities   Identifier namespaces
decisions    Business decisions and policies
workflows    Coordination state owned by this module
```

The manifest describes logical ownership, not necessarily physical storage location.

### `provides`

A module may provide:

```text
commands
queries
events
capabilities
apis
libraries
```

Each public item should have a stable name and a contract location. Public items may also declare version, deprecation, authorization, and service-constraint metadata when consumers rely on it.

### `contracts/*.yaml`

A contract names the shape and meaning of one public command, query, event, capability, API, or data exchange. Preconditions and postconditions belong here when consumers must know them.

```yaml
spec: rms/contract/v0.1
name: authorize-payment
version: 1
kind: command
meaning: Request authorization for a payment amount.

preconditions:
  - id: positive-payment-amount
    statement: The requested payment amount is greater than zero.

postconditions:
  - id: authorization-outcome-recorded
    statement: The result is authorized, declined, or requires review.

failure_categories:
  - id: unusable-payment-method
    statement: The payment method cannot be used for this buyer or amount.
```

Use domain language in contract assumptions. Implementation-specific expressions belong in `implementation.yaml` semantic function declarations or native code annotations.

### `requires`

A module may require:

```text
modules
capabilities
contracts
platform_services
```

Dependencies should be the smallest public surfaces needed. Requiring an entire module when one capability is sufficient weakens substitutability.

### `composition`

A composite module may declare contained submodules and explicit exports:

```yaml
composition:
  contains:
    - name: rules-engine
      visibility: internal
      path: modules/rules-engine/module.yaml
  exports:
    - group: capabilities
      name: resolve-move
      from: rules-engine
      contract: contracts/resolve-move.yaml
```

`visibility: internal` means consumers outside the parent boundary must depend on the parent export rather than the child directly. `visibility: public` permits direct external dependency on the child while still allowing the parent to export a product-level surface.

### `invariants`

Recommended invariant fields:

```yaml
- id: stable-identifier
  statement: Plain-language property that must remain true.
  scope: Optional state or aggregate scope.
  enforced_by: Code or boundary responsible for enforcement.
  verified_by: Evidence path or verification identifier.
  severity: Optional criticality classification.
```

### `effects`

Recommended effect fields:

```yaml
- name: inventory-store
  kind: persistent-storage
  capability: contracts/inventory-store.yaml
  semantics:
    idempotency: reservation-id
    ordering: per-sku
    consistency: serializable-per-reservation
    timeout: definite-failure
    retry: bounded-exponential
    compensation: release-reservation
    reconciliation: daily-and-on-demand
```

Only declare semantics that matter. Use explicit values such as `not-applicable` rather than silently omitting a critical question.

### `artifacts` and `transformations`

Artifacts are versioned semantic inputs or outputs, not incidental files. Transformations name how one declared artifact becomes another:

```yaml
artifacts:
  - name: source-unit
    version: v1
    direction: provided
    contract: contracts/source-unit.v1.yaml
    invariants: [source-unit-is-valid]
  - name: object-unit
    version: v1
    direction: provided
    contract: contracts/object-unit.v1.yaml
    invariants: [object-unit-is-valid]

transformations:
  - name: lower-source
    input: source-unit
    output: object-unit
    semantic_function: lower-source
    rejections: [InvalidSource]
    properties: [lowering-preserves-validity]
```

`direction` is `provided`, `required`, or `internal`. Artifact and transformation identifiers use the same language-neutral stable-ID grammar as other RMS semantics: begin with a letter or underscore, then use letters, digits, hyphens, or underscores. Composition matches required artifacts to one provider by semantic name, version, and contract identity.

### `authorities`

Elevated operations must be explicit:

```yaml
authorities:
  - id: native-backend
    kind: foreign
    capabilities: [emit-native-code]
    rationale: Native emission crosses a foreign ABI.
```

`kind` is `privileged`, `unsafe`, or `foreign`. `implementation.yaml` binds the authority to declared roles, one exact safe-facade `path#symbol`, and evidence. Authority is a containment boundary, not permission for arbitrary code elsewhere.

### Contract protocols

A public contract may define an ordered conversation shared by several modules:

```yaml
semantics:
  protocol:
    participants: [client, compiler]
    messages:
      - {id: compile-requested, kind: command, from: client, to: compiler}
    states: [Ready, Requested]
    initial_state: Ready
    terminal_states: [Requested]
    transitions:
      - {from: Ready, on: compile-requested, to: Requested}
```

Message kinds are `command`, `event`, `reply`, or `rejection`. Composition requires one implementation owner per participant and one sender and receiver mapping per message.

### `state`

Required only for the Stateful profile. It should identify:

```text
State model or lifecycle
Consistency boundary
Concurrency policy
Persistence policy
Migration policy
```

The manifest may link to a diagram or state specification instead of embedding every transition.

### `workflow`

Required for the Workflow profile. Typical fields are:

```yaml
workflow:
  trigger: start-checkout
  completion:
    - checkout-completed
    - checkout-requires-review
  deadlines:
    payment: PT3M
  terminal_states:
    - completed
    - rejected
    - manual-review
  compensations:
    payment-rejected: release-inventory
  resumption: replay-from-durable-state
```

### `boundary`

Required for the Boundary profile. Typical fields are:

```yaml
boundary:
  accepted_contracts:
    - public-api.v1
  validation: reject-before-domain-entry
  authorization: declared-scope
  resource_limits: documented
  malformed_input: stable-rejection
  deprecation: versioned-contract-policy
```

### `compatibility`

A module should distinguish:

```text
Public contract compatibility
Event/message compatibility
Stored-state compatibility
Implementation compatibility
Deprecation policy
```

### `verification`

Verification lists evidence locations, not testing-framework names.

```yaml
verification:
  laws:
    - verification/laws
  contracts:
    - verification/contracts
  scenarios:
    - verification/scenarios
  boundaries:
    - verification/boundaries
```

### `operations`

Operational declarations are required when the module's profiles or effects need them. Typical fields are:

```yaml
operations:
  observability:
    correlation: order-id
    causation: command-or-event-id
  runtime_checks:
    - ops/checks/order-invariants
  reconciliation:
    - ops/reconcile/external-orders
  migrations:
    - ops/migrations
  runbooks:
    - ops/runbooks/unknown-outcome.md
```

Keep operational evidence separate from test evidence: verification demonstrates behavior before release; operations detects and repairs divergence in the running system.

## 5. `implementation.yaml`

Language and toolchain details belong in a separate binding.

```yaml
spec: rms/implementation/v0.1

module: payments
binding: typescript

source:
  root: src
  public_entrypoint: src/index.ts

commands:
  build: project build payments
  verify: project verify payments
  format: project format payments

architecture:
  dependency_checker: tools/check-module-boundaries
  contract_generator: tools/generate-payment-contracts
  machine:
    name: PaymentsMachine
    mode: workflow-effect-machine
    transition_signature: state-and-input
    types:
      state: PaymentsState
      input: PaymentsInput
      command: PaymentsCommand
      event: PaymentsEvent
      effect: PaymentsEffect
      effect_result: PaymentsEffectResult
      reply: PaymentsReply
      rejection: PaymentsRejection
      transition: PaymentsTransition
      transition_record: PaymentsTransitionRecord
    states:
      - NotStarted
      - WaitingForEffect
      - Completed
      - Failed
    commands:
      - AuthorizePayment
    observed_events: []
    events:
      - PaymentAuthorized
      - PaymentDeclined
    effects:
      - CallPaymentProvider
    effect_results:
      - PaymentProviderAuthorized
      - PaymentProviderUnknown
    replies:
      - AuthorizationAccepted
      - AuthorizationRejected
    rejections:
      - InvalidPaymentRequest
    effect_protocols:
      - effect: CallPaymentProvider
        results:
          - PaymentProviderAuthorized
          - PaymentProviderUnknown
        executor_role: effect_executor
        atomicity: one-request-one-result
    transition_function: transition
    transitions:
      - from: NotStarted
        on: AuthorizePayment
        to: WaitingForEffect
        case: RequestPaymentAuthorization
        effects:
          - CallPaymentProvider
        no_reply_justification: Payment provider outcome is pending.
      - from: WaitingForEffect
        on: PaymentProviderAuthorized
        to: Completed
        case: CompleteAuthorizedPayment
        events:
          - PaymentAuthorized
        reply: AuthorizationAccepted
  roles:
    representation:
      - src/representation.ts
    transition:
      - src/transition.ts
    effect_executor:
      - src/effects.ts
  public_behavior_bindings:
    - id: authorize-payment-public
      public_kind: command
      public_name: authorize-payment
      contract: contracts/authorize-payment.yaml
      semantic_function: authorize-payment-decision
      machine_inputs: [AuthorizePayment]
      machine_outputs: [AuthorizationAccepted, AuthorizationRejected, InvalidPaymentRequest]
  dependency_behavior_bindings:
    - id: payment-gateway-provider
      capability: payment-gateway
      contract: contracts/payment-gateway.yaml
      consumer: src/effects.ts#executePaymentProvider
      resolution: external

semantic_functions:
  - id: authorize-payment-decision
    symbol: payments::authorize_payment
    kind: decision
    purity: pure
    discharges:
      contracts:
        - contracts/authorize-payment.yaml
      invariants:
        - capture-requires-authorization
    assumptions:
      requires:
        - amount.is_positive()
      ensures:
        - output.is_authorized_or_declined()
    evidence:
      laws:
        - verification/laws/authorize_payment
      scenarios:
        - verification/scenarios/authorize_payment
```

A Rust, Go, Python, Java, or remote-service implementation can satisfy the same semantic `module.yaml` with a different implementation binding.

The binding may define:

```text
Source locations
Public export discovery
Build and verification commands
Toolchain and lockfile identity
Dependency-analysis configuration
Generated and private paths
Required filesystem, network, and credential permissions
Schema/code-generation commands
Runtime adapter registration
Semantic function declarations
Public contract-to-function-to-machine bindings
Required capability consumer-to-provider bindings
```

It must not redefine domain meaning or compatibility promises.

### Semantic-First Gate

Product meaning changes move through `rms spec apply` before code. The canonical semantic-change fields are language-neutral:

| Field | Meaning |
|---|---|
| `supersedes` | Historical change records replaced for active reflection checks. `rms spec apply` automatically adds every currently active semantic revision; explicit entries are only for additional non-local branches. Applied records are append-only. |
| `intent.summary` | Human-readable reason for the semantic delta. |
| `laws.add` | Invariants, laws, or product promises that must hold. |
| `contracts.add/set/remove` | Contract references with required `kind: command|query|event|capability` and direction. Provided entries update matching `provides.*`; only capabilities may be required. Implemented capabilities include their public/dependency behavior binding in the same final change. |
| `artifacts.add/set/remove` | Versioned provided, required, or internal artifact contracts. |
| `transformations.add/set/remove` | Artifact input/output mappings with exact semantic owner, rejection cases, and preservation properties. |
| `authorities.add/set/remove` | Privileged, unsafe, or foreign capabilities and their rationale. |
| `properties.*.temporal` | Optional executable `always`, `eventually`, `precedence`, `exclusion`, `at_most_once`, or `bounded_response` semantics over typed observations and assumptions. |
| `properties.*.explorations` | Canonical probe assembly, satisfy/violate goal, and finite step, schedule, and state bounds discovered by `rms hunt`. |
| `hunt_exceptions.set/add/remove` | Closed risk-derived verification obligations that are genuinely inapplicable, each with a focused reason. |
| `protocol_bindings.add/set/remove` | Implementation ownership and send/receive mapping for public contract protocol messages. |
| `authority_bindings.add/set/remove` | Implementation roles, exact safe facade, and evidence for declared elevated authority. |
| `public_behavior_bindings.add/set/remove` | One exact path from each implemented public command, query, or capability contract through a discharging semantic function into classified machine inputs and outputs. |
| `dependency_behavior_bindings.add/set/remove` | One exact path from each implemented required capability through a local `path#symbol` consumer into a matching RMS module provider contract or explicit external resolution. |
| `machine` | Optional machine section reused from `rms/machine-change/v0.1` for states, inputs, outputs, transitions, and inner roles. |
| `surfaces.add` | Runnable surface declarations for app, UI, CLI, browser, HTTP, batch, mobile, desktop, or executable entrypoints that adapt outside input into declared RMS commands. Browser-style surfaces may include a controller `entrypoint`, a host `launch_entrypoint`, and checked local `launch_scripts`. |
| `evidence.add` | Required proof lanes for laws, contracts, transitions, effects, scenarios, traces, or boundary behavior. |

Use `rms spec apply` to add or change laws, contracts, machine structure, runnable surfaces, effects, semantic roles, public entrypoints, and evidence obligations together. Machine and semantic changes support `set`, `add`, and `remove`. Spec apply automatically closes every active semantic revision and stores `change_record_digest`; use explicit `supersedes` only for additional non-local branches, never to replace history by deleting or rewriting old records. Use `rms surface apply/check` for focused runnable entrypoint changes. Browser launch files and local launch scripts are part of the surface and must route through the declared controller, adapter, parser, or boundary machine rather than duplicating domain decisions. Agents may edit bodies inside declared role files after the semantic delta is applied. Focused inner-machine edits may use `rms machine apply` when laws, public contracts, and evidence obligations are already correct.

Capability publication and module topology are independent. `rms spec apply` can add the first provided or required capability to an existing standalone module. `rms add-capability-tree` only scaffolds a composite with children after typed design explicitly selects recursive topology.

### Typed Intent and Workspace Coverage

`rms/intent-model/v0.1` is advisory input to deterministic policy, not a canonical module artifact. It contains operation, change scope, semantic subjects, facts with `required|absent|unknown`, responsibilities, binding preferences, and open questions. Explicit facts preserve exact user quotes; inferred facts carry rationale. Architecture, topology, shape, module names, and scaffold recommendations are forbidden.

`.rms/config.yaml` records `workspace.coverage: progressive|complete`. Adopted repositories default to progressive coverage, where checks certify discovered RMS module closures. Complete coverage is accepted only when production paths are RMS-owned.

### Semantic Machine Structure

Every implemented module should declare a domain-named machine. The canonical fields are language-neutral:

| Field | Meaning |
|---|---|
| `architecture.machine.name` | Domain-named machine role, such as `PaymentsMachine` or `NutritionAssistantMachine`. |
| `architecture.machine.mode` | One of `stateless-decision-machine`, `stateful-transition-machine`, `workflow-effect-machine`, `boundary-machine`, `storage-machine`, `integration-machine`, or `projection-machine`. |
| `architecture.machine.transition_signature` | `input-only` for a justified stateless decision machine; `state-and-input` for every stateful, boundary, workflow, storage, integration, or projection machine. |
| `architecture.machine.initial_state` | Initial semantic state variant. Required for inspectable Rust, Swift, JavaScript, and Python bindings and must name one declared state. |
| `architecture.machine.driver_function` | Exact callable that drives an effectful stateful machine through transition, emitted effects, typed results, and follow-up transitions. |
| `architecture.machine.transition_record_function` | Exact pure callable used by the live machine driver to construct each complete transition record. |
| `architecture.machine.types` | Binding-native names for state, input, command, event, effect, effect result, reply, rejection, transition, transition record, and declared message-envelope containers. These are not semantic cases. |
| `architecture.machine.states` | Closed state variants. Stateful variants must be reachable from `initial_state` through canonical transitions. Stateless machines usually contain `Ready` and must justify why lifecycle state is not meaningful. |
| `architecture.machine.commands/observed_events/events/effects/effect_results/replies/rejections` | Semantic cases that define what the machine accepts, observes, emits, asks the world to do, receives back, returns, and rejects. A case belongs to exactly one input category. |
| `architecture.machine.effect_protocols` | Effect-to-result mapping, exact executor role and symbol, and atomicity. One-request-one-result is the default when an individual outcome can affect later decisions. |
| `architecture.machine.resource_protocols` | Ownership plus closed acquire/use/release/transfer automata for lifetime-sensitive resources. Product and resource states are validated together so terminal machine paths cannot leak resources. |
| `architecture.machine.transitions` | Accepted and rejected state/input/output transitions. Every branch has a stable `case` represented in declared transition source; source-only branches are drift. Trace provenance names that source file and exact case. |
| `architecture.roles.*` | Binding files or artifacts that realize representation, transition, parser, adapter, machine driver, effect executor, private effect support, journal, replay, trace evidence, and related roles. |
| `architecture.probe` | `rms/machine-probe/v0.1` or batched v0.2 protocol, command key, exact runner, optional pure payload mappers, and—on state-and-input machines—the binding-native complete initial-state constructor. |
| `architecture.protocol_bindings` | Participant ownership and exact machine-case mapping for public protocol messages. |
| `architecture.authority_bindings` | Declared authority roles, exact safe facade, and containment evidence. |
| `dependencies.local_modules` | Language-neutral RMS module identities consumed by this implementation. Change them through `rms spec apply` `binding_dependencies`; the binding adapter realizes native allowlists and local package metadata. |

`architecture.dependency_behavior_bindings[].probe_bridge` may bind one emitted consumer command/effect to a provider public input and provider replies, events, or rejections back to consumer effect results or observed events. Protocol message mappings and bridge legs may name an exact pure probe mapper when payload schemas differ. Assemblies select instances for canonical roles but cannot redefine those transformations or inject private machine inputs.

Use `rms machine apply` to add or change these architecture fields only when the semantic layer is already correct. RMS validates the complete final candidate and records the focused change, but it does not synthesize active trace evidence from transition declarations. Implemented transition paths must populate and replay the declared evidence roles. If laws, public contracts, effects, or evidence obligations change, use `rms spec apply` instead.

This is the only accepted implementation-machine model. Collapsed declarations such as `architecture.machine.state`, top-level `architecture.state_type`, or semantic lists containing container type names are invalid rather than compatibility aliases.

Invariant entries declare `authority` as `representation`, `constructor`, `parser`, `transition`, `effect-executor`, or `composition`. An effect protocol's exact executor symbol is represented by an effectful `effect-executor` semantic function. Atomicity applies to that exact protocol: aggregate iteration does not make an unrelated atomic executor aggregate. Shared IO mechanism code may be declared under `architecture.roles.effect_support`, but it remains private and cannot own state progression or transition outputs. State progression, sequencing, retry, compensation, and stop/continue laws belong to `transition`; effect executors may enforce only the mechanics of the external request. An effect-emitting runnable surface must delegate to an exact callable that reaches the declared machine driver and declare `usage_document` plus a `smoke_command` key. The driver calls `transition_record_function`, retains complete records, advances from `state_after`, executes `output.effects`, and owns the complete repeated cycle rather than leaving lifecycle work in a surface or adapter. Declared message-envelope types must exist in inspectable bindings, and arithmetic over represented transition inputs must be checked or bounded.

Composite modules may declare `verification.delegations[]` with `proves`, `provider_module`, `provider_law`, `provider_property`, `through_export`, and `evidence`. The record discharges a parent property obligation only when `rms compose` resolves every link.

Ordering, safety, bounded, normalization, parser, and numeric laws also declare semantic properties with input spaces and oracles. A fixed example corpus does not satisfy an open-ended generated-property or coverage-fuzzer claim. Every executable realization binds an exact relative `path#symbol` runner; generated-property and deterministic-exhaustive strategies also bind a generator. The runner calls the generator when required, executes a declared semantic operation, and applies an oracle. `mutation-tester` is the closed strategy for project-declared oracle-strength campaigns. Generated property evidence remains an obligation until that exact realization executes.

Risk-derived hunt posture requires generated or exhaustive checks for pure and numeric decisions, finite exploration for stateful machines and workflows, coverage fuzzing for untrusted boundaries, schedule/fault exploration for distributed behavior, analyzer or sanitizer lanes for unsafe authority, mutation testing for reusable semantic oracles, and real-trace evaluation plus violation search for temporal promises. A focused `verification.hunt_exceptions` item may discharge only a genuinely inapplicable obligation. Fast checks validate this posture; `rms hunt` executes expensive nightly realizations separately.

Production trace bundles declare `architecture.trace.producers[]` with a profile, bundle, command, and exact runner. Inspectable runners call `architecture.machine.transition_record_function` and serialize returned records. `rms trace run --record` validates before replacing committed bundles; normal runs regenerate into temporary paths and compare canonical values. Strict audit reruns smoke producers and property realizations from committed code, rebuilds reusable packages, and rejects proof commands that mutate production files.

Inspectable Rust, Swift, JavaScript, and Python implementations also declare `commands.probe`, `architecture.probe`, and `architecture.roles.probe_adapter`. The adapter receives temporary request/output paths through `RMS_PROBE_REQUEST` and `RMS_PROBE_OUTPUT`, is selected by `RMS_PROBE_RUNNER`, calls the exact transition-record function, and chains returned `state_after`. It must not call the driver or any effect executor. Fixture and opaque executable bindings are exempt.

Binding references use an exact relative `path#symbol`. Swift symbols may name a declared type-qualified static `let`, `var`, or function member, such as `Sources/SecureMediaSession/Representation.swift#SecureMediaSessionWorkflowState.initial`; a redundant free-function alias is not required.

### Applied Semantic Revision

`rms spec apply`, `rms machine apply`, and `rms surface apply` record the exact accepted change and write an `x-rms.semantic_revision` seal into the owning module and implementation manifests. The seal covers canonical module semantics, local referenced contracts, and implementation semantics while excluding the seal metadata itself.

Strict audit recomputes this digest. A clean Git commit is necessary provenance, but it is not proof that semantic changes passed through RMS: direct edits after apply produce `semantic.revision-drift`. Apply commands should be run with `--dry-run` first, and agents should inspect the final semantic state before editing source.

Runnable surface declarations resolve delegation to an existing semantic role or to a concrete implementation symbol. Each surface declares its boundary effects or a precise `no_effects_justification`; a surface cannot use an undeclared role name as architectural evidence.

Public domain values with validity rules use private fields and validated constructors. `architecture.allowed_public_field_structs` is reserved for structural envelopes, transition outputs, transition records, and source-provenance records; it cannot waive validity for arbitrary domain values.

### `semantic_functions`

Semantic functions connect the portable RMS semantic set to implementation source symbols.

Recommended fields:

| Field | Meaning |
|---|---|
| `id` | Stable identifier for the semantic function declaration. |
| `symbol` | Language-binding source symbol such as `Widget::new` or `payments::authorize_payment`. |
| `kind` | `constructor`, `parser`, `decision`, `transition`, `projector`, `adapter`, `interpreter`, `transformation`, or `effect-executor`. |
| `purity` | `pure`, `effectful`, or `boundary`. |
| `discharges.contracts` | Published contracts this function implements or helps satisfy. |
| `discharges.invariants` | Module invariant identifiers this function enforces or preserves. |
| `assumptions.requires` | Function-local preconditions that are not already represented by types. |
| `assumptions.maintains` | Invariants preserved before and after execution. |
| `assumptions.ensures` | Function-local postconditions. |
| `evidence` | Law, contract, scenario, or boundary evidence paths for this function. |
| `authorities` | Declared privileged, unsafe, or foreign authority ids used by this function. |

Prefer typed representations over repeated preconditions. A constructor or parser should discharge raw-value assumptions once, so later decision and transition functions can accept validated values.

Add, replace, or remove these bindings through `rms spec apply` with `semantic_functions.add/set/remove`. RMS validates the final authority graph, prints it during dry-run, records the exact operation, and rejects removal of the last owner of an active non-composition invariant. Direct edits are semantic revision drift.

Concrete evidence attached to the exact semantic function that discharges or enforces an invariant satisfies that invariant's evidence closure. The evidence must still exist and prove the declared behavior; a symbol binding or semantic-shape assertion alone is not concrete proof.

## 6. `conformance-report.json`

A conformance report records one reproducible evaluation:

```json
{
  "spec": "rms/conformance/v0.1",
  "subject": {
    "module": "payments",
    "version": "2.1.0",
    "implementation": "typescript"
  },
  "source": {
    "revision": "git:0123456789abcdef"
  },
  "profiles": ["core", "stateful", "distributed", "boundary"],
  "validator": {
    "name": "rms",
    "version": "0.1.0"
  },
  "result": "pass",
  "checks": [
    {
      "id": "contracts.compatibility",
      "category": "contracts",
      "result": "pass",
      "evidence": "verification/contracts/report.json"
    }
  ]
}
```

A report should make skipped and not-applicable checks explicit. It is evidence, not a permanent guarantee: it applies only to the identified source or artifact and tool versions.

## 7. `context-map.yaml`

Example:

```yaml
spec: rms/context-map/v0.1

contexts:
  ordering:
    publishes:
      - order-submitted.v1
    consumes:
      - payment-outcome.v1

  payments:
    publishes:
      - payment-outcome.v1
    external_integrations:
      - name: payment-provider
        relationship: anti-corruption-layer

relationships:
  - upstream: ordering
    downstream: fulfillment
    contract: contracts/order-ready-for-fulfillment.v1.yaml
```

The map should reveal semantic direction, not merely package imports.

## 8. Extensions

Custom fields should use an `x-` prefix:

```yaml
x-risk-tier: critical
x-regulatory-domain: payments
```

Extensions must not weaken core requirements or silently change standard semantics.

## 9. Keep manifests concise

A manifest is useful when it can be loaded quickly by a person or agent. Put large schemas, transition tables, examples, and runbooks in linked files.

Prefer a small accurate manifest over a comprehensive stale one.
