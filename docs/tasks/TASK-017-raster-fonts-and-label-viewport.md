---
id: TASK-017
type: task
title: Raster font fallback and label-aware viewport
status: done
relations:
  parent: EPIC-003
  implements: [SPEC-001]

---
# Raster font fallback and label-aware viewport

## Goal

Two defects found while re-rendering the pinned MIWG subset on a machine without DejaVu fonts:

1. `bpmn-raster` pinned `sans-serif` to DejaVu Sans; on systems without it, usvg dropped every text node silently, so PNG output lost all labels (no error, no warning).
2. The diagram viewport unioned only shape bounds and waypoints; DI `BPMNLabel` bounds placed outside their shape were clipped by the viewBox (visible as a cut-off "Start Event" in MIWG A.1.0).

## Implementation

- `bpmn-model`: new diagnostic codes `raster.missing_fonts` (no usable fonts found) and `raster.font_substituted` (preferred family not installed).
- `bpmn-raster`: font resolution over installed families with the preferred chain DejaVu → Liberation → Noto → Arial (sans; analogous serif chain), deterministic smallest-family last resort, emoji faces excluded. Additive API `render_png_with_diagnostics`/`render_pixmap_with_diagnostics` returning `PngOutput { bytes, diagnostics }`; `render_png`/`render_pixmap` keep their signatures. `bpmn-cli render` forwards raster warnings into its warning report. Per ADR-0004 no font assets are committed.
- `bpmn-render`: `union_viewport` unions valid `BPMNLabel` bounds of shapes and edges; invalid label bounds are ignored. Trade-off: label bounds of DI elements that are not rendered still extend the canvas (the padding absorbs the rare small offset).

## Verification

- New unit tests: font picker (preferred order, smallest-installed fallback, empty set), empty font database emits `raster.missing_fonts` warning, `render_png_with_diagnostics` returns a valid PNG with warning-only diagnostics, viewport covers out-of-bounds DI labels and ignores invalid label bounds.
- `scripts/check.sh` green: fmt, clippy `-D warnings`, full workspace tests, docvera, deny, audit.
- MIWG smoke re-render with DejaVu absent: A.1.0 PNG shows node labels (Liberation Sans substitution warning printed) and "Start Event" is no longer clipped (viewport extends to the label bound).

## Done

- `cargo test -p bpmn-raster` 10 tests green, `cargo test -p bpmn-render` 14 tests green.
- MIWG A.1.0/C.8.0 re-rendered with visible labels and expanded viewport; evidence in the TASK review notes.
