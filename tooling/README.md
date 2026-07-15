# RMS Tooling

RMS tooling projects canonical semantics and evidence through a small public doorway. Manifests and contracts remain authoritative; validators, bindings, plugins, and reports only enforce or package that truth.

## Public Doorway

The Rust reference CLI exposes five primary commands:

```text
rms init [OPTIONS] [PATH]
rms next "<intent>" [--root PATH] [--module MODULE] [--json] [--details]
rms explain ["<question>"] [--root PATH] [--module MODULE] [--json] [--details]
rms check [--environment | --changes | --committed] [--root PATH] [--json] [--details]
rms view [OPTIONS]
```

`rms --help` shows only this doorway plus `help`. `rms help --all` groups the directly callable specialist commands used by selected skills and detailed reports.

Human answers follow `Outcome/Answer → Why → Next → Done when`. Agent responses use the typed `rms.surface/v2` JSON envelope. Full canonical inventories remain behind `--details`.

Environment checks report skill sources detected on disk. Detection is not proof of runtime activation: activation remains `unknown` and precedence is `host-defined` because the CLI cannot inspect a host's injected task catalog.

## One Workflow

```text
init → authorized bootstrap commit
→ next → explain when needed → follow the prescribed work
→ check --changes → authorized candidate commit → check --committed
```

The expanded bootstrap path still runs deterministic `design` before either the standalone-module or recursive-capability scaffold prescribed by `next`.

Git commits are required evidence, not implied authority. RMS does not authorize Git operations. Without task and host authority, stop at the reported pending state and do not claim completion.

## Tooling Contract

| Command | Compact responsibility |
| --- | --- |
| `rms init` | Initialize or safely adopt a system. |
| `rms next` | Classify prospective intent, resolve ownership without guessing, and prescribe ordered work. |
| `rms explain` | Answer a focused question deterministically from canonical evidence. |
| `rms check` | Delegate project, environment, change, or committed proof to the existing RMS engines. |
| `rms view` | Open the read-only semantic explorer. |

Specialist commands preserve the deeper declaration, composition, verification, packaging, and integration machinery. They are not a second semantic authority.

Language bindings belong beside or underneath `tooling/<language>/`. A binding may discover imports, public exports, effects, and native verification commands, but it must not redefine RMS concepts. The current reference bindings are Rust and Swift.

## Rust Binding

When an implementation binding declares `binding: rust`, the CLI checks:

- `toolchain.cargo_manifest`, defaulting to `source.root/Cargo.toml`;
- Cargo manifest parseability and `[package]` or `[workspace]` shape;
- `toolchain.package` against `package.name` when a package is present;
- `source.public_entrypoint` as a Rust file inside `source.root`;
- Cargo dependencies against `dependencies.allowed_external_crates` when declared;
- `pub mod` declarations in the public entrypoint against `architecture.public_modules` when declared.
- source-level `use` and `extern crate` roots against `dependencies.allowed_external_crates`;
- public external re-exports against `architecture.allowed_public_reexports`;
- public local-module re-exports against `architecture.public_modules`.
- public primitive type aliases unless listed in `architecture.allowed_primitive_type_aliases`;
- public fields on domain structs unless listed in `architecture.allowed_public_field_structs`;
- `panic!`, `todo!`, `unimplemented!`, `.unwrap()`, and `.expect()` in non-test domain code unless `architecture.allow_panics: true`;
- constructor evidence for public structs with private fields, unless listed in `architecture.allowed_missing_constructors`;
- for Stateful modules, `architecture.state_type` or `architecture.transition_function`, with declared symbols present in source.

See `examples/rust`.

## Swift Binding

When an implementation binding declares `binding: swift`, the CLI checks:

- `toolchain.package_manifest`, defaulting to `Package.swift`;
- Swift package manifest shape and package name;
- `toolchain.package` and `toolchain.target` declarations;
- `source.public_entrypoint` as a Swift file inside `source.root`;
- source-level imports against `dependencies.allowed_external_modules`;
- public external re-exports against `architecture.allowed_public_reexports`;
- public primitive type aliases unless listed in `architecture.allowed_primitive_type_aliases`;
- public stored fields on domain structs unless listed in `architecture.allowed_public_field_structs`;
- `fatalError`, `preconditionFailure`, `try!`, and `as!` in domain code unless `architecture.allow_traps: true`;
- constructor evidence for public structs with private fields, unless listed in `architecture.allowed_missing_constructors`;
- for Stateful modules, `architecture.state_type` or `architecture.transition_function`, with declared symbols present in source.

See `examples/swift`.
