# Scenario Evidence: context-packet

`rms context <module> --task "<task>"` produces a bounded packet with the module brief, canonical file references, public references, and working rules for humans or agents.

When the target is a composite parent or recursive module tree, the packet includes route evidence before implementation work: recommended owner when clear, candidate modules when ambiguous, and follow-up `rms context` commands for the owning child.
