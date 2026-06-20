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

### `requires`

A module may require:

```text
modules
capabilities
contracts
platform_services
```

Dependencies should be the smallest public surfaces needed. Requiring an entire module when one capability is sufficient weakens substitutability.

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
```

It must not redefine domain meaning or compatibility promises.

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
