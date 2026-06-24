#!/usr/bin/env sh
set -eu
node --check src/representation.mjs
if [ -f src/transition.mjs ]; then node --check src/transition.mjs; fi
if [ -f src/parser.mjs ]; then node --check src/parser.mjs; fi
if [ -f src/ports.mjs ]; then node --check src/ports.mjs; fi
if [ -f src/adapter.mjs ]; then node --check src/adapter.mjs; fi
cargo build --manifest-path rules-bridge/Cargo.toml
