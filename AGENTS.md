# Agent instructions

1. Read `docs/index.md`, `docs/specifications/SPEC-001-bpmn-renderer.md`, relevant ADRs and the active TASK before changing code.
2. Associate each PR with an existing TASK and its acceptance criteria.
3. Make no unreviewed changes to public API, licensing, dependency policy or BPMN DI semantics; author an ADR proposal first.
4. Add regression tests for fixed defects; never report completion without execution evidence.
5. Treat BPMN XML and third-party fixtures as untrusted inputs. Do not invent DocVera command syntax.
