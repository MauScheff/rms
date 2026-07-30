# Law Evidence: RMS self-application authority remains independent

RMS development is governed by the repository maintainer workflow, not by an
RMS-generated route or by the candidate implementation certifying itself.

The `repository maintainer seal` semantic-revision authority is accepted only
when all of the following are true:

- the owning module explicitly declares `scope: rms-self-development`;
- the declaration names the same closed authority variant and module-local
  evidence;
- the module publicly owns semantic application, production audit, and release
  preparation;
- the immutable semantic-change record digest and the canonical module,
  implementation, and contract projection digest both match;
- strict audit runs against committed Git provenance.

The native regression first applies the maintainer authority to an ordinary
module and requires `semantic.revision-authority-invalid`. It then adds the
complete self-application declaration and required public ownership, reseals
the same artifacts, and requires semantic revision integrity to pass. Direct
record or manifest mutation must still fail.

This exception identifies an independent repository-maintainer authority. It
does not let downstream RMS modules bypass route receipts or ordinary
`rms spec apply`, `rms machine apply`, and `rms surface apply` authority.
