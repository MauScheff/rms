# Codex Integration

**Status:** Non-normative adapter guidance  
**Checked against official OpenAI documentation:** 2026-06-20

RMS remains agent-neutral. This adapter makes the canonical CLI, guidance, and skills convenient in Codex without defining a second architecture.

## Repository Instructions

Keep the portable agreement in root `AGENTS.md`. Generate or refresh it from the RMS CLI; put task-specific mechanics in repository skills rather than permanent startup context.

The onboarding order is:

```text
init → authorized bootstrap commit → design → recommended scaffold
```

For software-change work, Codex needs only the five-command doorway:

```bash
rms check --environment --root .
rms next "<exact change task>" --root . --ai
rms explain "<question>" --root .
rms check --root .
rms view --root .
```

Codex extracts typed facts without architecture fields; recorded read-only `--ai` extraction is also available. `next` validates those facts, then selects the owner, task lane, context, skill, and proof path. Codex should use its compact response first and request `--details` only when needed. `rms help --all` exposes specialist commands when the selected skill prescribes one. Read-only investigation, explanation, review, status or history inspection, ordinary Git/repository/tool operations, and discussion that requests no change use native tools without `next`; if they reveal a proposed change, Codex stops before editing and routes that exact change task.

Local completion is focused proof → coverage-aware `rms check --changes` → authorized candidate commit → coverage-aware `rms check --committed`. These modes certify only selected RMS closures, separate new regressions from baseline debt, and return project-owned native handoffs without claiming RMS proof for native or outside-coverage paths. Use `rms check --all` for exhaustive release or CI certification and strict full-repository provenance evidence.

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness. Report the applicable pending state instead of executing or implying a commit.

## Skills

RMS skills use the Agent Skills `SKILL.md` format. Project skills live under:

```text
.agents/skills/<skill-name>/SKILL.md
```

The canonical source remains repository `skills/`. Generated project, embedded, and plugin copies must match it. Skills call the shared CLI and carry focused declaration and proof mechanics; they do not redefine manifests or contracts.

Keep generic cadence in the RMS-distributed plugin and skills. Consumer repositories retain only project-specific path ownership, native proof commands, and release or hardware gates. Plugin installation must not overwrite consumer `AGENTS.md`, adoption ledgers, or deployment runbooks.

`rms check --environment --json` reports observed skill sources. A detected project copy, user installation, marketplace entry, or plugin cache is not proof that Codex injected it into the current task. RMS reports `runtime_activation: unknown` and `precedence: host-defined`; the current injected Codex skill catalog is the runtime authority.

For agent automation, use `--json`. The `rms.surface/v2` envelope exposes typed `program` plus `args` for executable actions. A `kind: manual` action with `authorization: host-required` is never executable by inference.

Provider-backed prompts remain explicit specialist workflows. They are advisory until canonical apply and deterministic checks succeed. Review and pin executable skill and plugin content before installation.

Executable temporal properties use one specialist loop: `property check` type-checks observations, assumptions, expressions, and units; `evaluate` reads a real trace; `search` finds a finite witness or counterexample; `analyze` relates properties; `monitor` consumes trace or observation prefixes; and `replay` rechecks recorded analysis evidence. Codex must not translate temporal prose into an implicit oracle or call a bounded search proof.

Quantity observation dimensions are scalar declarations such as `value: {quantity: transition}`. Bounds carry their decimal and unit separately. Use the complete executable example rendered by `rms spec plan`; do not infer nested quantity shapes.

For “find bugs,” “fuzz,” “harden,” “soak,” “overnight,” or reliability-audit requests, select `hunt-bugs`. Inspect the campaign with `rms hunt --dry-run`; run it only from a clean commit, preserve its seed and replay recipes, and report bounded proof scope honestly.

## Plugin Wrapper

The optional plugin wrapper lives at:

```text
integrations/codex/rms
```

Refresh all managed RMS skill distributions through its compatibility entrypoint:

```bash
./integrations/codex/rms/scripts/sync-skills.sh
```

The wrapper calls the canonical repository sync script. It must preserve unrelated local skills and remain packaging rather than semantic authority.

After `rms agent plugin sync --target codex`, RMS removes obsolete RMS-owned cache versions and verifies packaged skill equivalence. Start a new Codex task so the host loads the refreshed catalog.

## Hooks

Hooks may call shared RMS checks at lifecycle boundaries. They should use the same `rms check` mode as CI and must not duplicate semantic rules, invoke providers implicitly, or grant Git authority.

## Recommended Layout

```text
AGENTS.md                     Concise portable guidance
skills/                       Canonical skill source
.agents/skills/               Managed project copies
integrations/codex/rms/       Optional plugin packaging
```

## Official References

- [Custom instructions with AGENTS.md](https://developers.openai.com/codex/guides/agents-md)
- [Agent Skills](https://developers.openai.com/codex/skills)
- [Plugins](https://developers.openai.com/codex/plugins)
- [Hooks](https://developers.openai.com/codex/hooks)
