# Law Evidence: semantic shape

Shape: `domain-engine` (pure decisions, closed variants, validated values, transitions, laws, and trace replay)

Representation obligations:

- Closed alternatives should be represented as ADTs, sealed variants, enums, or tagged constructors.
- Values with validity rules should be created through validated constructors.
- Expected failures should be explicit accepted/rejected outcomes.
- Lifecycle or order-dependent behavior should be replayable through transition traces.
- Boundary input should be parsed before it reaches pure decisions.

Generated roles:
- `representation`
- `commands`
- `transitions`
- `trace-replay`
- `law-evidence`

Command:

- Replace this placeholder with module-specific law, trace, property, fuzz, contract, or boundary evidence.

Source revision: not recorded by the generated scaffold.
