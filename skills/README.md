# RMS Agent Skills

These skills express architecture workflows without assuming a programming language or coding-agent vendor.

Canonical skills:

- `inspect-module`
- `implement-change`
- `refactor-module`
- `prune-module`
- `add-module`
- `evolve-contract`
- `compose-modules`
- `verify-module`

Each skill uses the portable `SKILL.md` form with only common `name` and `description` frontmatter. Agent-specific packaging belongs under `integrations/` or generated installation directories.

The semantic workflow in these skills is normative only where it restates `SPEC.md`. The skills themselves are operational guidance.

## Safety

The canonical skills are instruction-only. Agent-specific packages may add scripts, but executable additions should be version-pinned, reviewed, and granted least privilege. Skills should call project-native RMS validation rather than embedding a second set of architectural rules.
