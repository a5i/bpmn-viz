---
id: RISK-004
type: risk
title: Third-party fixture rights and provenance
status: mitigated
relations:
  addresses: [SPEC-001]
  mitigated_by: [ADR-0005]
---
# Third-party fixture rights and provenance

## Description

BPMN MIWG files are CC-BY-3.0; vendoring them into an MIT repository would tangle licenses and lose upstream traceability.

## Likelihood

High without a policy (copying test files is the path of least resistance).

## Impact

Medium: license contamination of the MIT codebase and unreproducible conformance claims.

## Mitigation

ADR-0005: external datasets are never committed; `scripts/download-fixtures.sh` reproduces `external/` from `tests/manifests/external-fixtures.lock.json`, which pins the upstream revision and SHA-256 per file and carries the CC-BY-3.0 attribution. CI stays self-contained; MIWG conformance is a local/scheduled activity. Release attribution requirements are in `docs/guides/release.md`.

## Outcome

Mitigated: provenance is exact (revision hash + hashes) and license boundaries are enforced by the gitignore-plus-manifest structure.
