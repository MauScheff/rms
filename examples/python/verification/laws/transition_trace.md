# Law Evidence: transition trace

Promise:

- `ExampleMachine` decisions are replayable from explicit transition inputs.
- Each transition record captures state before, state after, input variant, outputs, and source provenance.
- Active trace records are emitted by the declared producer through the real transition-record function.

Command/tool:

- `rms trace run implementation.yaml --profile smoke --record` regenerates and validates the committed bundle.
- `rms trace run implementation.yaml --profile smoke` compares fresh execution with committed evidence.

Expected result:

- Generated records name current `ExampleState`, `ExampleCommand`, `ExampleEvent`, and `ExampleReply` variants and preserve state/output consistency.
- Any behavior change produces trace drift until the implementation and canonical semantics agree and evidence is deliberately re-recorded.

Source revision: resolved from the committed candidate by strict audit.
