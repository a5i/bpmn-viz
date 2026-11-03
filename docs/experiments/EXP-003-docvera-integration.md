---
id: EXP-003
type: experiment
title: Verify DocVera CLI and schema compatibility
status: concluded
relations:
  informs: [SPEC-001]
---
# Verify DocVera CLI and schema compatibility

## Hypothesis

Installed DocVera can validate typed Markdown artifacts and relations needed for this repository.

## Setup

- Installed CLI: `docvera 0.1.2` (crates.io binary at `~/.cargo/bin/docvera`; upstream: `@a5i/docvera`, repository `https://github.com/a5i/docvera`).
- Inspected the embedded offline manual: `docvera guide`, `docvera guide --format json`, `docvera --help`, `docvera schema config`, `docvera schema document`.
- Implemented `docvera.toml` (`version = 1`, `include = ["docs/**/*.md"]`, `[schemas]` path-glob mappings) and nine document schemas under `docs/docvera/schemas/` covering `specification`, `adr`, `epic`, `task`, `experiment`, `test_plan`, `report`, `risk`, `note` and `guide` document types.
- Schema envelope: `frontMatter` is plain JSON Schema (`pattern`, `enum`, `minLength`, `additionalProperties: false` all confirmed working); `identity.field`/`uniqueWithin: workspace` enforces unique ids; typed `relations` entries (`from`/`targetType`/`targetTypes`/`mustExist`) validate cross-document references; the body section DSL (`title`, `titleEquals`, `pattern`, `min`/`max`, `nonEmpty`, `additionalSections`) validates section structure.
- Verified behavior by iterating `docvera check` until 15 documents pass with 0 diagnostics, and by observing real failures (missing H1 title equality, missing link targets, sections with only child headings) reported with source spans.

## Results

- `docvera check` exit codes confirmed: 0 success, 1 validation findings, 2 operational errors.
- Exit code of the final gate run: `0` (`Checked 15 documents; 0 diagnostics`).
- Typed relations resolve ids across directories (`SPEC-001` links, `TASK.parent` → `EPIC`, `ADR.implements` → `SPEC`).
- One schema-language limitation found: `nonEmpty` checks *direct* section content only — SPEC-001 sections 3 and 10 legitimately contain only child headings, so the specification schema uses presence (`min`/`max`) instead of `nonEmpty` for numbered top-level sections.
- Lifecycle *transitions* (proposed → accepted, backlog → done) are **not** enforced by the engine; statuses are plain enum values. Transition discipline is enforced by review, and candidate rules for a future SHACL policy (`[validation.shacl]`).

## Conclusion

Confirmed: DocVera 0.1.2 supports this repository's documentation-as-code workflow without invented syntax. `docvera check` is wired into CI through `scripts/docvera-check.sh` (SPEC-001 section 12; RISK-001 mitigated). Known gaps (transition enforcement, SHACL policies) are tracked in REPORT-001 next actions.
