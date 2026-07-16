# Law Evidence: package source discovery ignores shell syntax

Law: `package-command-discovery-ignores-shell-syntax`

Promise: package collection extracts only safe relative project paths from build and verification commands; shell redirections and substitutions never become package sources.

Scenario: a Swift architecture probe contains `2>/dev/null` and command substitution alongside the real script `scripts/verify.sh`.

Command/tool: `cargo test -p rms package_command_paths_ignore_shell_redirections`

Expected result: only `scripts/verify.sh` is collected; `/dev/null` and `2>/dev/null` are excluded.

Source revision: strict audit binds this evidence to the committed candidate.
