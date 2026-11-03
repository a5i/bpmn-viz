---
id: ADR-0005
type: adr
title: External third-party fixtures pinned by manifest
status: accepted
created: 2026-11-02
relations:
  implements: [SPEC-001]
  supersedes: []
---
# External third-party fixtures pinned by manifest

## Context

BPMN MIWG test-suite files are CC-BY-3.0 and must not be relicensed as MIT code; CI must not depend on network fetches per PR; and provenance must be auditable.

## Decision

External datasets are downloaded into gitignored `external/` by `scripts/download-fixtures.sh`, pinned by upstream revision and SHA-256 in `tests/manifests/external-fixtures.lock.json`, and used for local conformance runs (REPORT-001). Committed fixtures under `tests/fixtures/` are synthetic and project-owned (MIT). CI runs a self-contained synthetic suite only; MIWG conformance is a local/scheduled job, not a per-PR dependency.

## Alternatives Considered

- **Vendor MIWG files into the repository:** mixes CC-BY-3.0 content into an MIT tree and bloats history. Rejected.
- **Fetch MIWG in CI per PR:** network flakiness and upstream drift would break PRs. Rejected (scheduled/local only).

## Consequences

- `external/` contents can be verified against the manifest (`sha256sum -c`) but are not versioned.
- Conformance claims (REPORT-001) cite an exact upstream revision hash.

## Validation

- Manifest records upstream revision `85240b8689d835d8bd1a96fbea6af46558ca0568` (2026-10-18) with SHA-256 per file; `scripts/download-fixtures.sh` reproduces `external/` from it.
- License attribution kept in the manifest and `docs/guides/release.md`; MIWG CC-BY-3.0 attribution is never stripped.

## References

- [SPEC-001](../specifications/SPEC-001-bpmn-renderer.md)
- [REPORT-001](../reports/REPORT-001-conformance-baseline.md)
