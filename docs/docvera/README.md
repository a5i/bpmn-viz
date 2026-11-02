# DocVera integration contract

**Verified:** `@a5i/docvera` is described as a schema-validated, queryable Markdown engine.

**Unverified:** CLI flags, schema dialect, lifecycle hooks, relation constraints, executable interface, runtime configuration. Do not treat the frontmatter examples as validated configuration.

## Required integration

1. Discover exact installed CLI API and docs. Record versions and observations in `EXP-003`.
2. Define types `specification`, `adr`, `epic`, `task`, `experiment`, `test_plan`, `report`, `risk`.
3. Validate required sections, unique IDs, allowed statuses, links and transitions.
4. Add a version-checked `scripts/docvera-check.sh` only after command verification.
5. Wire this check into CI after successful local execution.

Sample [frontmatter and relationships](../specifications/SPEC-001-bpmn-renderer.md#12-docvera-documentation-as-code-and-work-management).
