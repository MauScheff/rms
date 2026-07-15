#!/usr/bin/env sh
set -eu

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"
canonical_skills="$repo_root/skills"
guidance_assets="$repo_root/tooling/rust/rms/assets/guidance"

sync_skills() {
  destination="$1"
  mkdir -p "$destination"
  cp -R "$canonical_skills/." "$destination/"
}

sync_skills "$repo_root/tooling/rust/rms/assets/skills"
sync_skills "$repo_root/integrations/codex/rms/skills"
sync_skills "$repo_root/.agents/skills"
sync_skills "$repo_root/.claude/skills"

cp "$guidance_assets/agents-full.md" "$repo_root/AGENTS.md"
cp "$guidance_assets/claude.md" "$repo_root/CLAUDE.md"

echo "Synced RMS skills and generated guidance distributions."
