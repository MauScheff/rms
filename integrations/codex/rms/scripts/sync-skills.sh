#!/usr/bin/env sh
set -eu

plugin_root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
repo_root="$(CDPATH= cd -- "$plugin_root/../../.." && pwd)"

rm -rf "$plugin_root/skills"
mkdir -p "$plugin_root/skills"

for skill in \
  inspect-module \
  implement-change \
  refactor-module \
  add-module \
  evolve-contract \
  compose-modules \
  verify-module
do
  cp -R "$repo_root/skills/$skill" "$plugin_root/skills/$skill"
done

echo "Synced RMS skills into $plugin_root/skills"
