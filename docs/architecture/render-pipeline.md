---
id: DOC-005
type: note
title: Render pipeline
status: active
---
# Render pipeline

`render_diagram(document, diagram, options)` (in `bpmn-render`) performs one pass per selected diagram:

1. **Index** — semantic nodes/flows/participants/lanes/annotations by id (recursing into subprocess bodies), DI shapes/edges by `bpmnElement`, collapsed-subprocess sets and their skipped descendant ids.
2. **Viewport** — union of all non-skipped shape bounds and edge waypoints, padded by `RenderOptions::padding`, minimum 50x50.
3. **Containers** — pool band (header left for horizontal pools, top otherwise) and lane rectangles.
4. **Flows** — sequence flows: polyline from DI waypoints, arrowhead marker, default-flow slash tick, conditional diamond; message flows and associations: dashed lines without sequence arrowheads.
5. **Nodes** — glyph per `NodeKind` (see `glyphs.rs`); unsupported kinds draw the amber dashed fallback and emit `symbol.unsupported`.
6. **Labels** — DI label bounds take precedence; otherwise tasks/subprocess labels wrap inside the shape, event/gateway labels sit below, pool/lane names rotate in their bands, flow names sit at the polyline midpoint.

Geometry sources (ADR-0007): this pipeline always consumes provided DI verbatim (Preserve). Generated geometry enters only through the explicit `bpmn-layout` Generate API, which returns a `bpmn_model::di::Diagram` consumable by the same `render_diagram` entry point; the renderer never repairs or lays out geometry itself.

Determinism contract: iteration follows DI document order, number formatting trims trailing zeros, text is escaped at serialization, and no stage reads the clock, environment or network. The SVG writer (`bpmn-svg`) and rasterizer (`bpmn-raster`) are pure consumers of the scene; PNG adds a scale factor and enforces pixel budgets.
