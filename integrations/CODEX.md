# Codex Integration

**Status:** Non-normative adapter guidance  
**Checked against official OpenAI documentation:** 2026-06-20

RMS remains agent-neutral. This adapter makes the neutral CLI, manifests, and skills convenient in Codex.

## Repository instructions

Keep the portable working agreement in the repository root as `AGENTS.md`. Codex discovers `AGENTS.md` from the project root down toward the working directory, allowing more local instructions for nested modules.

For new projects, use this order:

```text
init → authorized bootstrap commit → design → recommended scaffold
```

`rms init` writes `AGENTS.md`, `.rms/config.yaml`, `.agents/skills/`, and `.gitignore` with the standard RMS/Codex operating surface. Before choosing the first module tree, use `rms next --task "<intent>" --root .` for a prospective prescription and run `rms design --root . --task "<intent>"`. Follow the deterministic design recommendation: `rms add-module` and `rms add-capability` are alternatives, not sequential defaults. Omit child-name overrides for `rms add-capability` unless the product already has meaningful child names.

Use `rms surface apply/check` before adding or changing runnable app, UI, CLI, browser, HTTP, batch, or executable entrypoints. Use `--binding executable` for opaque command-backed surfaces such as web, mobile, CLI, native UI, generated assets, or integration scripts. Keep `AGENTS.md` concise. It should tell Codex to use the `rms` CLI before inferring module boundaries from prompt context. Detailed, task-specific procedures belong in Agent Skills rather than permanent startup context.

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness. Report the corresponding pending state instead of executing or implying a commit.

## Skills

Codex skills build on the open Agent Skills standard. RMS skills use a `SKILL.md` entry point with `name` and `description` frontmatter. To make the canonical skills available to Codex, install or generate them under:

```text
.agents/skills/<skill-name>/SKILL.md
```

Codex can discover repository skills from `.agents/skills` directories between the working directory and repository root. `rms init` installs the canonical RMS skills there for fresh projects.

The canonical source should remain the project-level `skills/` directory. Skills should enter through `rms next --task "<intent>" --root <root>` or `rms --help`, then call the shared CLI rather than embedding a second RMS workflow. Use `rms config init` to create local provider defaults when appropriate. Use `rms diagnose --json` when an agent needs structured readiness and `rms audit --root . --strict` only after the candidate commit when claiming production-ready RMS software.

`rms diagnose` and `rms agent diagnose` report observable skill sources. A detected project copy, direct user installation, marketplace configuration, or plugin cache is not proof that Codex injected that skill into the current task. RMS reports `runtime_activation: unknown` and `precedence: host-defined`; use Codex's current injected skill catalog as the runtime authority.

Use `rms ... --provider codex` for explicit Codex execution, or `rms ... --ai` when `.rms/config.yaml` declares Codex as the intended default provider. For writable Codex execution, prefer `--sandbox workspace-write --write-scope module`; this runs Codex from the target module directory while still supplying the canonical RMS context in the prompt. Provider execution is bounded by `ai.codex.timeout_seconds` or `--provider-timeout-seconds`, defaulting to 900 seconds. Use `--write-scope root` only when the task intentionally changes system, context, glossary, or cross-module artifacts. Review and pin executable skill content before installation. Run `rms release check --root .` before sharing the Codex plugin wrapper.

## Plugins

Package the skills as a Codex plugin only when installable distribution is useful. Plugins can bundle skills and integrations. Plugin packaging is an adapter concern and must not become the source of RMS semantics.

This repository includes a thin Codex plugin wrapper:

```text
integrations/codex/rms
```

The wrapper packages the canonical RMS skills for Codex. Refresh the packaged copy before release:

```bash
./integrations/codex/rms/scripts/sync-skills.sh
```

Install the neutral CLI:

```bash
cargo install --path tooling/rust/rms
```

## Hooks

Codex hooks may invoke shared RMS validation at lifecycle points, such as when a turn stops. Hooks should call the same repository validator used by CI; they should not implement separate architectural rules.

## Recommended layout

```text
AGENTS.md
skills/                         Canonical skill source
.agents/skills/                 Generated or installed Codex skills
integrations/codex/             Optional plugin and hook packaging
```

## Official references

- [Custom instructions with AGENTS.md](https://developers.openai.com/codex/guides/agents-md)
- [Agent Skills](https://developers.openai.com/codex/skills)
- [Plugins](https://developers.openai.com/codex/plugins)
- [Hooks](https://developers.openai.com/codex/hooks)
