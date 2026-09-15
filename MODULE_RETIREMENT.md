# Exact Leaf Module Retirement

## Accepted RFC and authority

An unused canonical scaffold can outlive the implementation that it was intended to describe. Repairing that scaffold can create false proof. Deleting its manifests can hide the same gap from active discovery.

`retire-module` archives one explicitly selected, unreferenced leaf directory. The caller supplies the retirement decision. RMS checks the bounded preconditions and seals the exact committed inventory. RMS does not infer that the module is unnecessary.

This workflow does not transfer ownership, remove domain promises, certify native code, or authorize Git operations. Ordinary RMS maintenance uses native maintainer proof, not RMS self-routing.

## Contract

- The target is an exact safe repository-relative `module.yaml` in a non-root directory outside `.rms` and `.git`.
- The Git worktree is clean. Every target file and executable bit matches the selected committed baseline. Untracked or ignored target files, symlinks, hard links, and submodules cause refusal.
- The leaf has no declared ownership, public promises, requirements, effects, invariants, composition, protocols, resources, artifacts, or transformations. A nonempty declaration requires a separate semantic decision before retirement.
- An implementation uses local `source.root: .`. Nested modules and detected external or ambiguous source paths cause refusal.
- Active canonical or native references to the module name or directory cause refusal. Static relative-path screening also rejects detected consumers and escaping paths. Markdown prose and archived history are not active consumers. This conservative screening does not prove the absence of dynamic consumers; the caller must establish non-use.
- Planning records an exact reason and byte inventory. Applying requires the fresh retirement-only receipt from that plan. An ordinary `next` receipt, a non-ready receipt, a stale baseline, a changed plan, or a different target cannot authorize retirement.
- Dry-run changes no file. Apply moves the exact directory without deleting its contents. No file outside that directory changes, except the new retirement record and archive.
- The archive preserves all committed bytes and executable modes, including failed or obsolete historical evidence. That evidence remains history. It does not become current website or native proof.
- Active discovery excludes the archive. Retirement validation reads it explicitly and fails on incomplete records, changed bytes or modes, missing archives, reactivated original directories, or unrecorded historical module deletion.
- Affected and committed selection classify archive-only changes as `retirement-provenance`. They do not invent an active owner or report the removed files as an outside-coverage deletion. Each non-environment check and strict audit retains retirement validation.
- Surviving native paths retain their existing coverage status. The project must run its applicable native acceptance. Candidate and release gates are unchanged.

## Commands and worked example

The example retires an unused scaffold in `modules/obsolete-website/`. The real website remains in `landing/`.

```sh
rms retire-module plan modules/obsolete-website/module.yaml --root . \
  --reason "Retire the unused scaffold. Preserve landing/ and its native acceptance."

# Use the exact route_receipt path returned by plan.
rms retire-module apply modules/obsolete-website/module.yaml --root . \
  --route-receipt <receipt-path> --dry-run
rms retire-module apply modules/obsolete-website/module.yaml --root . \
  --route-receipt <receipt-path>

rms retire-module check --root .
# Run the project-owned native acceptance and inspect the exact Git diff.
rms check --changes --root .
# Create the candidate commit only with caller authorization.
rms check --committed --root .
# Run the unchanged full release gate when release is in scope.
rms check --all --root .
```

These specialist commands emit JSON. Plan writes `.rms/runs/<run>/retirement-plan.json` and the existing route receipt artifacts. Run directories must already be ignored. Apply writes `.rms/retirements/<plan-sha256>/retirement.json` and moves the target to its sibling `archive/`. The retirement directory must be tracked with the candidate commit.

The durable record embeds `rms/module-retirement-plan/v0.1` and its `rms/route-receipt/v0.1`. The record uses `rms/module-retirement/v0.1`. The [exchange schema](schemas/module-retirement.schema.json) defines both new shapes. Hashes provide integrity and provenance, not signatures or independent authorization.

Historical validation checks the embedded receipt seal and exact base Git tree. The base must remain an ancestor of HEAD. The receipt retains its issuing worktree identity; historical validation does not require a clone to have the same local path or current CLI version. Apply still requires the original fresh worktree and version.

## Recovery

Apply writes and flushes `pending.json` before the directory move. It renames that file to `retirement.json` after the move. An interruption leaves a visible validation failure; RMS does not delete evidence or attempt an implicit restore.

Inspect the exact pending record and both paths. If the original directory still exists, preserve it and move the incomplete retirement directory to a separate recovery location. If only `archive/` exists, verify its inventory against the base and move it back to the exact original directory, then move the incomplete retirement directory aside. Never overwrite either destination. Resolve concurrent edits explicitly. Start a fresh plan after the worktree is clean. A completed, committed retirement has no automatic restore operation in v0.1.

## Compatibility and migration

Active manifest formats do not change. The `rms/check-selection/v0.2` projection gains the `retirement-provenance` coverage value. Consumers of that closed enum must accept the new version and value as historical RMS provenance, not native or active implementation certification.

Existing arbitrary module deletions remain visible as baseline debt. Restore the exact original directory and make the retirement decision explicitly; do not fabricate a retrospective archive or remove failed evidence. Non-environment checks require complete Git history for deletion validation. Fetch missing history in shallow CI checkouts before checking. Retirement does not make a shallow or missing baseline valid.

## State-space and proof

The new distinction is active leaf versus archived historical leaf. A fresh plan is a derived observation, not a second owner. The content digest determines the archive identity. The only mutation is a one-way directory move. Pending publication is an interrupted filesystem operation and always fails proof.

The implementation and native tests reside in `tooling/rust/rms/src/retirement.rs`. CLI tests exercise plan, dry-run, apply, historical validation, and affected/committed selection. Refusal tests cover stale authority, escaping paths, links, consumers, domain declarations, archive tampering, incomplete moves, and unrecorded deletion. A native sentinel outside the target must remain byte-identical.

The local explanation boundary is the plan, embedded receipt, exact archive, and base Git tree. No provider inference or product semantics are required to explain an acceptance or refusal.
