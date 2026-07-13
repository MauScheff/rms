# Law Evidence: canonical binding attachment

Promise:

- `semantic-only-modules-have-canonical-binding-attachment`: a module created without a binding can gain one through RMS without changing its semantic manifest or copying a separate scaffold.

Scenario:

- Create a semantic-only domain module, preserve the exact `module.yaml` bytes, attach the JS binding, and compare the manifest before and after.
- Repeat attachment and verify RMS rejects the existing binding.
- Place a conflicting generated source path in another semantic-only module and verify attachment leaves no partial binding.

Command/tool:

- `cargo test -p rms add_binding -- --nocapture`

Expected result:

- The first module gains `implementation.yaml` and domain-named JS roles while `module.yaml` remains byte-identical.
- A second attachment is rejected explicitly.
- A destination conflict leaves neither `implementation.yaml` nor unrelated generated source or script files.

Source revision: recorded by git commit or strict audit provenance before production use.
