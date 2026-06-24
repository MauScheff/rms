#!/usr/bin/env sh
set -eu
for test_file in tests/*.mjs; do
  node "$test_file"
done
printf '%s\n' 'js semantic scaffold smoke passed'
