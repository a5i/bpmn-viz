---
id: DOC-001
type: guide
title: Documentation index
status: active
---
# Documentation index

- [Prototype specification](specifications/SPEC-001-bpmn-renderer.md)
- [ADRs](adr/ADR-0001-rust-only-rendering-pipeline.md) — accepted decisions; layout proposals: [ADR-0006](adr/ADR-0006-native-bpmn-aware-layered-layout.md), [ADR-0007](adr/ADR-0007-generated-di-and-layout-modes.md); corpus proposal: [ADR-0008](adr/ADR-0008-external-corpus-manifests.md); bindings proposal: [ADR-0009](adr/ADR-0009-node-wasm-bindings.md)
- [Epics](epics/EPIC-001-parser.md) and [tasks](tasks/TASK-001-repo-bootstrap.md); auto-layout: [EPIC-005](epics/EPIC-005-auto-layout.md) / [TASK-018](tasks/TASK-018-bpmn-auto-layout.md); bindings: [TASK-020](tasks/TASK-020-node-wasm-bindings.md)
- [Experiments](experiments/EXP-001-ebi-bpmn-evaluation.md) — including the DocVera spike ([EXP-003](experiments/EXP-003-docvera-integration.md)) and the layout reference audit ([EXP-004](experiments/EXP-004-reference-layout.md))
- [Risks](risks/RISK-001-docvera-capabilities.md)
- [Test plans](testing/TEST-001-test-plan.md): prototype ([TEST-001](testing/TEST-001-test-plan.md)) and layout engine ([TEST-005](testing/TEST-005-layout-test-plan.md)); [conformance/performance report](reports/REPORT-001-conformance-baseline.md) and [external corpus smoke baseline](reports/REPORT-002-external-corpus-smoke.md)
- [Architecture](architecture/overview.md), [data model](architecture/data-model.md), [render pipeline](architecture/render-pipeline.md)
- [Guides](guides/development.md): [development](guides/development.md), [testing](guides/testing.md), [release](guides/release.md)
- [DocVera integration contract](docvera/README.md) — schemas under [docvera/schemas/](docvera/README.md)
