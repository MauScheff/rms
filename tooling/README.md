# RMS Tooling

RMS tooling is an adapter layer over the canonical specification. Tools may be written in any language as long as they preserve the same semantic model: manifests and contracts remain the source of truth; validators, language bindings, plugins, and hooks only enforce or package that truth.

## Current Reference Tool

The first reference implementation is the Rust CLI in `tooling/rust/rms`.

```bash
cargo run -p rms -- init /tmp/rms-example --name rms-example --purpose "Try RMS"
cargo run -p rms -- add-module /tmp/rms-example/modules/widget --name widget --purpose "Own widgets" --binding rust
cargo run -p rms -- validate --root examples/minimal
cargo run -p rms -- inspect examples/minimal/module.yaml
cargo run -p rms -- context examples/minimal/module.yaml --task "add a public command"
cargo run -p rms -- conformance examples/minimal/module.yaml
```

The CLI intentionally starts small:

- validates manifests against embedded RMS JSON Schemas;
- checks required semantic fields and RMS version identifiers;
- checks referenced contracts, verification evidence, and implementation paths;
- inspects module ownership, profiles, contracts, effects, and verification;
- emits bounded context packets for agents;
- produces explicit partial/pass/fail conformance reports.
- applies the first language binding when `implementation.yaml` declares `binding: rust`.

## Tooling Contract

Other implementations should preserve these command meanings even if flags and output formats vary:

| Command | Meaning |
|---|---|
| `rms init` | Scaffold a new RMS system. |
| `rms add-module` | Scaffold a valid RMS module directory. |
| `rms validate` | Check canonical artifacts and references. |
| `rms inspect` | Print a concise module brief. |
| `rms context` | Build a bounded packet for a task. |
| `rms conformance` | Emit machine-readable evaluation evidence. |

Language bindings belong beside or underneath `tooling/<language>/`. A binding may discover imports, public exports, effects, and native verification commands for a language, but it must not redefine RMS concepts. The first binding is Rust; Swift is next.

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
