# Property Evidence: package command paths are shell-safe

Promise: every token admitted as a package source is a safe relative path and contains no shell control, redirection, or substitution syntax.

Scenario: the deterministic corpus combines quoted command substitution, a stderr redirect, boolean shell operators, and one valid relative script path.

Command/tool: `cargo test -p rms package_command_paths_ignore_shell_redirections`

Expected result: the path set is exactly `{scripts/verify.sh}`.

Source revision: strict audit binds this evidence to the committed candidate.
