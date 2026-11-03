---
id: DOC-003
type: note
title: Architecture overview
status: active
---
# Architecture overview

The workspace is a pipeline of small crates (SPEC-001 section 4, ADR-0002):

```
BPMN XML bytes
  └─ bpmn-xml      namespace-aware import (quick-xml NsReader, bounded, entity-safe)
       └─ bpmn-model    semantic elements + DI types + diagnostics
            └─ bpmn-validate  references, geometry, DI coverage
                 └─ bpmn-render   DI + semantics → bpmn-scene (glyphs, flows, labels)
                      └─ bpmn-svg      deterministic scene → SVG (owned writer)
                           └─ bpmn-raster   SVG → PNG (resvg)
                                └─ bpmn-cli   render / inspect / validate CLI
```

Layering rules:

- `bpmn-model` depends on nothing workspace-internal; the semantic model contains no geometry and the DI model contains no semantics.
- `bpmn-scene` knows nothing about BPMN; it is a plain 2D scene with deterministic paint order.
- Output crates consume scenes, never BPMN types; a new output format would only add a scene consumer.
- Diagnostics (`bpmn-model::diagnostic`) are the shared currency of every stage, with stable string codes (e.g. `reference.dangling`, `di.missing_shape`).

See [data model](data-model.md) and [render pipeline](render-pipeline.md) for details.
