# Hooks

The RMS Codex plugin does not install hooks yet.

Future hooks should call the shared `rms` CLI, for example:

```bash
rms validate --root .
```

Hooks must remain advisory before CI exists and must not redefine RMS semantic rules.

