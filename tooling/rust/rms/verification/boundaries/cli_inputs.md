# Boundary Evidence: CLI Inputs

The CLI treats command-line paths, YAML manifests, optional `.rms/config.yaml`, JSON conformance reports, TOML package manifests, and language source files as inputs. Parsing and validation errors are reported as diagnostics or command failures instead of becoming implicit architecture.

Workbench config is operational input only. It may select provider, model, sandbox, write scope, provider timeout, and run-record defaults, but it cannot define module ownership, contracts, invariants, effects, or compatibility.
