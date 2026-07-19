# Prepare RMS release

`rms release prepare` validates the route receipt before constructing a candidate, derives every RMS release metadata update from one validated semantic version, validates the complete candidate, and writes no partial result on rejection. Its contract test exercises dry-run, exact multi-file preparation, invalid-version rejection, receipt rejection before candidate construction, and release-metadata consistency. The command never commits, publishes, or installs the prepared release.
