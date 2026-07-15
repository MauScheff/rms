#!/usr/bin/env sh
set -eu

plugin_root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
repo_root="$(CDPATH= cd -- "$plugin_root/../../.." && pwd)"

exec sh "$repo_root/scripts/sync-rms-agent-distributions.sh"
