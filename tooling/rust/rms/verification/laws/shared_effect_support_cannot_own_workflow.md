# Law Evidence: effect-support ownership

Promise: invariant `shared-effect-support-cannot-own-workflow` keeps shared effect support private IO mechanism code outside machine progression.

Command/tool: `cargo test --manifest-path Cargo.toml effect_support_is_private_mechanism_not_machine_progression`

Expected and observed result: a private mechanism helper passes; constructing the declared machine state reports `structure.effect-support-owns-workflow`. The focused fixture passes.

Source revision: resolved from the committed candidate by strict audit.
