# Security and Trust Model

RMS treats reliability and security as related but distinct. The semantic specification constrains meaning and change; the trust model constrains who and what may alter or execute the system.

## 1. Agent output is untrusted until verified

A coding agent produces a proposed change, not proof of correctness. Trust comes from reviewable artifacts and reproducible evidence:

```text
Declared ownership and contracts
Deterministic boundary checks
Compatibility analysis
Focused verification
Source revision or artifact digest
Pinned validator and implementation binding
```

Conformance must not depend on the reputation or identity of the model that wrote the code.

## 2. Separate instructions from authority

`AGENTS.md`, `CLAUDE.md`, skills, and prompts guide behavior. They are not architectural authority and do not replace CI, permissions, contracts, or runtime controls.

Repository prose, issue descriptions, copied web content, test fixtures, generated files, and external documents can contain misleading instructions. Treat them as data unless the project explicitly places them in the canonical artifact set.

## 3. Least privilege

Grant each task only the capabilities it needs. A change to a pure domain module normally does not require production credentials, unrestricted network access, deployment authority, or broad filesystem access.

Use separate identities and approval paths for:

```text
Reading source
Modifying source
Accessing external services
Reading secrets
Publishing packages
Deploying to production
Changing contracts or security policy
```

## 4. Secrets

Never store credentials, tokens, private keys, production data, or secret values in:

```text
Manifests
Public contracts
Agent context packets
Skills or prompts
Conformance reports
Logs and fixtures
Generated documentation
```

Use references to a secret capability or secret manager instead. Redact command output before it becomes agent context or evidence.

## 5. Skills, plugins, hooks, and MCP servers

These can execute code or reach external systems and therefore belong to the software supply chain.

Before enabling one:

1. Pin a reviewed version or digest.
2. Inspect bundled scripts and declared permissions.
3. Prefer instruction-only skills when executable code is unnecessary.
4. Restrict filesystem, network, credentials, and shell access.
5. Make hooks call the same validator used by CI rather than implementing private rules.
6. Record updates like any other dependency change.

## 6. Dependencies and released artifacts

Implementation bindings should identify the toolchain and dependency lock used to build and verify a module. Published deployable artifacts should provide, where practical:

```text
Artifact digest
Source revision
Build provenance
Dependency inventory or SBOM
Conformance report
Signature or trusted publication identity
```

## 7. High-impact changes

Projects should require stronger review or explicit authorization for changes to:

```text
Public contracts and compatibility policy
Authorization and trust boundaries
Payment, identity, permissions, or safety-critical invariants
Migration and deletion behavior
Agent permissions and executable integrations
Release and deployment configuration
```

## 8. Vulnerability reporting

Before public launch, the project must publish a private security-reporting channel and supported-version policy. Do not use a public issue for an unpatched vulnerability that could put users at risk.
