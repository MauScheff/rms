# Law Evidence: boundary parse

The CLI adapter is a boundary machine: raw text is never delegated directly to the rules module.

Covered cases:

- malformed text returns an explicit `Rejected` boundary result;
- out-of-board coordinates are rejected before rules delegation;
- valid text is converted into a finite board-cell command before the rules port runs;
- delegated work crosses the declared rules port rather than importing private rules internals.

Evidence commands:

- `sh scripts/smoke.sh`
- `rms trace check verification/traces/boundary_parse.yaml`

Source revision: repository example fixture.
