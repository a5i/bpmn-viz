---
id: ADR-0007
type: adr
title: Generated DI and Preserve/Generate/Reflow layout modes
status: proposed
created: 2026-11-03
relations:
  implements: [SPEC-001]
  supersedes: []
  informed_by: [EXP-004]
---
# Generated DI and Preserve/Generate/Reflow layout modes

## Context

The render pipeline (SPEC-001 section 4) passes authored DI to the renderer verbatim; ADR-0003 makes missing DI a loud error. Once `bpmn-layout` (ADR-0006) can synthesize DI, the workspace needs explicit rules for when geometry is taken from the document, when it is computed, and how the three concerns stay separated — in particular, who is allowed to replace geometry, and what the layout API may and may not touch.

## Decision

Define three explicit, separate modes at the workspace level:

- **Preserve (default, unchanged):** the renderer consumes authored DI verbatim and errors on incomplete required DI (`di.missing_interchange`, `di.missing_shape`, `di.missing_waypoints`), exactly as ADR-0003 specifies. No layout code runs in this mode.
- **Generate:** a caller selects a semantic scope (`ScopeId::Process(id)`) and receives freshly computed geometry as an in-memory `bpmn_model::di::Diagram` plus warnings. Authored DI in the document is ignored for placement and is never mutated; if expansion state of subprocesses matters it is an explicit option (M2). Generate is the M1 contract of `bpmn-layout`.
- **Reflow:** reserved for a future BPMN XML writer workstream: run the same layout computation as Generate and *replace* the requested diagram's geometry in a serialized document. Until a writer exists, Reflow is specified but not implemented; no partial or lossy XML rewriting is smuggled into `bpmn-layout`.

Supporting rules:

- The layout API never mutates the semantic model: `LayoutEngine::layout(&self, model: &BpmnDocument, scope: &ScopeId, options: &LayoutOptions)` borrows the document.
- Scope selection is explicit and typed; multiple diagrams and multiple processes are never silently merged, and flows referencing nodes outside the selected scope are structured errors, not cross-scope edges.
- Generated DI uses the existing `bpmn-model` DI types (`Diagram`/`Plane`/`Shape`/`Edge`/`Bounds`/`Point`), so `bpmn-render` and `bpmn-svg` consume it through the unchanged `render_diagram(&document, &diagram, &options)` entry point. The renderer must not repair geometry; a shape or waypoint missing from generated DI is a layout-engine defect caught by the layout verifier, not a render-time fallback.
- Every generated shape carries the DI shape id `<bpmnElement>_di` and the referenced semantic element id; every edge carries ordered waypoints with both endpoints docking on their shapes' boundaries.

## Alternatives Considered

- **Layout inside the renderer as a hidden fallback:** directly violates ADR-0003 (conspicuous failure, AC-09) and hides authoring defects. Rejected.
- **Layout output as renderer-specific primitives (skip DI):** couples layout to the scene model, prevents XML export (Reflow), and breaks the semantic/DI separation of ADR-0002. Rejected.
- **Reflow by string-editing the original XML:** lossy and unsafe on arbitrary vendor extensions; requires the typed writer workstream first. Deferred.
- **Global auto-layout for any document missing DI (implicit mode):** mixes Preserve and Generate semantics in one entry point and surprises callers. Rejected in favor of explicit mode selection.

## Consequences

- `bpmn-layout` depends only on `bpmn-model` (plus `thiserror`); it stays independent of `bpmn-svg`, `resvg`, and CLI concerns.
- CLI integration (for example `bpmn render --layout generate` for no-DI documents) is a separate follow-up task; it must surface layout warnings in diagnostics output and keep Preserve as the default for documents that carry DI.
- Tests must cover all three boundaries: Preserve regressions (renderer unchanged), Generate invariants (TEST-005), and the explicit non-implementation of Reflow (documented, not stubbed).
- A generated `Diagram` is self-contained: rendering it requires only the semantic document for labels/glyph decisions — never the original authored DI.

## Validation

- Consumability test: for every accepted layout fixture, `bpmn_render::render_diagram(document, &generated_diagram, ...)` produces a valid SVG scene, and `bpmn-svg` serialization is deterministic (SVG assertion suite, TEST-005).
- Purity test: `layout()` called with `&BpmnDocument` compiles without mutability; a regression test asserts the authored `definitions.diagrams` are byte-identical before and after a layout call.
- Mode-separation test: rendering a no-DI document through the unchanged renderer still returns `di.missing_interchange` (ADR-0003) even though the layout crate is linked into the workspace.
- Scope tests: `layout.unknown_scope`, `layout.cross_scope_flow`, and `layout.missing_endpoint` return stable error codes without panics.

## References

- [SPEC-001](../specifications/SPEC-001-bpmn-renderer.md)
- [ADR-0002](ADR-0002-separate-semantic-di-and-scene-models.md) — semantic/DI/scene separation
- [ADR-0003](ADR-0003-no-autolayout-in-poc.md) — DI-required rendering (Preserve)
- [ADR-0006](ADR-0006-native-bpmn-aware-layered-layout.md) — native layout engine
- [EXP-004](../experiments/EXP-004-reference-layout.md) — reference audit and pin
