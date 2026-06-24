# Contracts

Place public RMS contract files here.

A contract belongs here when consumers outside this module can call, observe, depend on, or substitute against the behavior. Private helpers stay in implementation docs and tests.

When adding or changing a contract:

1. Declare it from `module.yaml`.
2. Specify preconditions, postconditions, failure categories, and compatibility policy.
3. Bind implemented symbols from `implementation.yaml` when code provides the behavior.
4. Add matching evidence under `verification/contracts/`.
