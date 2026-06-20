# `rms`

`rms` is the first reference CLI for Reliable Modular Systems. It validates canonical artifacts, inspects modules, builds agent context packets, and emits conformance reports.

Install from this repository:

```bash
cargo install --path tooling/rust/rms
```

Run without installing:

```bash
cargo run -p rms -- validate --root examples/minimal
```

Common commands:

```bash
rms validate --root examples/minimal
rms inspect examples/minimal/module.yaml
rms context examples/minimal/module.yaml --task "change payment capture behavior"
rms conformance examples/minimal/module.yaml --implementation examples/minimal/implementation.yaml
```

The CLI is intentionally conservative. It reports missing evidence explicitly instead of claiming more conformance than the artifacts prove.

