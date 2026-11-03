---
id: DOC-004
type: note
title: Data model
status: active
---
# Data model

## Semantic model (`bpmn-model::element`)

- `Definitions` — id, targetNamespace, ordered root elements (`Process` | `Collaboration`), ordered diagrams, and `unsupported` elements (data shapes/associations kept for id and DI resolution).
- `Process` — flow elements in source order (nodes and `SequenceFlow`), lane sets, artifacts (text annotations, unsupported data shapes).
- `FlowNode` — id, name, incoming/outgoing refs, default-flow pointer, and a `NodeKind`: start/end/intermediate events with `EventDefinition` markers, boundary events (`attachedToRef`, `cancelActivity`), tasks with `TaskType`, gateways with `GatewayType`, subprocesses with nested bodies, and `Other` for modeled-but-unsupported elements.
- `Collaboration` — participants (pools) and message flows.

## Diagram Interchange (`bpmn-model::di`)

- `Diagram` → `Plane` → `Shape` (bounds, label bounds, `isHorizontal`, `isExpanded`) and `Edge` (ordered waypoints, label bounds), all in source order. Diagram selection by index follows document order; selection by id uses the DI id.
- Numbers are `f64`, validated by `Bounds::is_valid` / `Point::is_finite`; the renderer never recalculates provided geometry (ADR-0003).

## Diagnostics (`bpmn-model::diagnostic`)

`Diagnostic { code, severity, message, element_id, location }` with a closed set of stable codes (see `DiagnosticCode::as_str`): `xml.*` for import, `reference.*` for structure, `geometry.*`/`di.*` for DI, `symbol.unsupported` and `import.ignored_element` for tolerance, `diagram.not_found` for selection.

## Scene (`bpmn-scene`)

Ordered `Node` tree (group/path/rect/ellipse/text) plus marker defs and a viewport. Paint order is insertion order; the render crate constructs scenes in a fixed z-order: containers → flows → nodes → node markers → labels.
