---
id: DOC-002
type: note
title: DocVera integration contract
status: active
---
# DocVera integration contract

**Verified** (EXP-003): `docvera 0.1.2` installed locally; upstream repository `a5i/docvera`, CLI on crates.io as `docvera-cli`, npm addon as `@a5i/docvera`. This workspace uses `docvera.toml` (config `version = 1`) with nine document schemas under `docs/docvera/schemas/` and validates with `docvera check` (exit 0 = clean, 1 = findings, 2 = operational error).

## What is enforced

- Front matter: JSON Schema per document type — id patterns (`SPEC-NNN`, `ADR-NNNN`, `EPIC-NNN`, `TASK-NNN`, `EXP-NNN`, `TEST-NNN`, `REPORT-NNN`, `RISK-NNN`, `DOC-NNN`), type/status enums, `additionalProperties: false`.
- Identity: every type has `identity.field: frontMatter.id` with `uniqueWithin: workspace`.
- Typed relations: `SPEC.links.decisions → adr`, `TASK.parent → epic`, `ADR.implements → specification`, `RISK.mitigated_by → adr`, and so on, with `mustExist: true`.
- Body structure: mandatory sections per SPEC-001 section 12 (ADR: Context/Decision/Alternatives Considered/Consequences/Validation/References; TASK: Goal/Implementation/Verification/Done; and so on), with H1 equality against `frontMatter.title`.

## Known gaps (documented, not guessed)

- Lifecycle **transitions** (proposed → accepted, backlog → done) are plain enum values in the engine; transition discipline is review-enforced, with SHACL policies (`[validation.shacl]`) as a future upgrade path (REPORT-001 next actions).
- `nonEmpty` checks direct section content only; the specification schema therefore relies on presence counts for numbered sections.
- The engine has no Git hooks integration; the gate runs locally via `scripts/docvera-check.sh` (also in `scripts/check.sh`). The `docvera` CI job was removed from `.github/workflows/ci.yml` (TASK-015).

## Sample front matter (validated shape)

```yaml
---
id: ADR-0002
type: adr
title: Separate semantic, DI and scene models
status: accepted
created: 2026-11-02
relations:
  implements: [SPEC-001]
  informed_by: [EXP-001]
  supersedes: []
---
```
