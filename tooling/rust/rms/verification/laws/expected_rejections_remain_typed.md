# Law Evidence: expected rejections remain typed

Promise:

- A machine that declares rejection variants exposes them through its transition output rather than hiding them in replies or provenance strings.

Scenario:

- Rust, Swift, and JavaScript fixtures remove the transition rejection channel while retaining declared rejection variants.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml rejection_channel`

Expected result:

- All three inspectable bindings report `structure.transition-rejection-channel-missing`.

Source revision: recorded by git commit or strict audit provenance before production use.
