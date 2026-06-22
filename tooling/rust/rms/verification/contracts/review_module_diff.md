# Contract Evidence: review-module-diff

Covered by CLI smoke execution of `rms review <module>`. The prompt includes bounded context, review workflow instructions, deterministic checks, and the current git diff when available.

`cargo test --manifest-path Cargo.toml` includes `review_prompt_can_include_impact_report`, which verifies `--impact` review prompts include a derived RMS impact prelude before diff context.

Optional provider execution uses the same rendered prompt and stores provider output under the generated run record.
