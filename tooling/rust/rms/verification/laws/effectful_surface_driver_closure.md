# Law Evidence: effectful surface driver closure

Promise:

- `runnable-effects-are-transition-driven`: an effectful runnable path cannot keep the transition/effect-result progress loop outside its declared machine driver merely because its public command has a different name from the machine command.

Scenario:

- A Rust boundary fixture declares public command `mini-xargs`, machine command `RunMiniXargs`, and machine effect `ExecuteInvocation`.
- Its surface reaches a one-step driver but loops around that driver while rendering progress.
- RMS classifies the surface as machine-effectful from its declared effects and reports `structure.effectful-control-flow-outside-machine-driver`.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml rust_boundary_rejects_hidden_imperative_effect_loop_bypassing_machine_driver`

Expected result:

- The fixture is rejected even though the public and machine command names differ and the runnable path does reach the driver.
- Rust statement macros such as `println!` and `writeln!` count as observable effects inside loop detection.

Source revision: resolved from the candidate Git commit by strict audit.
