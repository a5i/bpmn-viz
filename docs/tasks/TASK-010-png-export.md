---
id: TASK-010
type: task
title: PNG export
status: done
relations:
  parent: EPIC-002
  implements: [SPEC-001]

---
# PNG export

## Goal

Rasterize SVG to PNG via resvg with explicit scale and resource limits (AC-07).

## Implementation

Implemented `bpmn-raster`: `usvg` tree parsing with system fonts (DejaVu Sans as sans-serif fallback), scale validation, pixel budget (max width/height/total pixels), background fill, PNG encoding, plus an IHDR-only `png_dimensions` helper for tests.

## Verification

5 unit tests cover 1x/2x dimensions, non-background pixel content, invalid scale, pixel budget rejection and invalid SVG.

## Done

- `cargo test -p bpmn-raster` green.
- CLI renders PNG at `--scale 2` with expected 2x dimensions (integration test).
