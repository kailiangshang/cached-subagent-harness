# Optional Superpowers Integration

Status: current optional integration contract

cached-subagent-harness is standalone. Superpowers integration is
explicitly optional and is never installed, fetched, detected, or copied by the
default installer path.

Enable it explicitly with scripts/install.sh --with-superpowers.

When enabled, compatible planning, TDD, review, or finishing guidance loads only
when that phase begins and its context cost is justified. It cannot replace the
numbered invariant contract, force a fresh session per assignment, or redefine
standalone completion.

Optional methodology absence is not degraded. If explicitly requested setup
fails, the installer reports failure while leaving the standalone core
installed. SUPERPOWERS_REF may pin the optional clone.

See [Current Product State](current-state.md) for the standalone architecture
and current product boundary.
