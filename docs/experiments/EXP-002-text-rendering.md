---
id: EXP-002
type: experiment
title: Text rendering without font metrics
status: concluded
relations:
  informs: [SPEC-001]
  decides: [ADR-0004]
---
# Text rendering without font metrics

## Hypothesis

Deterministic label layout is achievable without measuring real glyphs, by wrapping with a fixed average character-width ratio.

## Setup

Implemented `bpmn-scene::text` with a fixed 0.6 × font-size average width, greedy word wrap, whole-word preservation and DI label-bounds precedence. Verified with unit tests and visual checks of rendered fixtures (including MIWG A/C cases with multi-word labels).

## Results

- Output is byte-stable across runs and platforms: no font loader participates in scene construction.
- Wrapping quality is good for typical BPMN labels (1–4 words) and degrades for narrow shapes with long words, which stay unsplit by design.
- Vertical labels (pools/lanes) avoid the width estimation problem entirely by rotating single-line text.
- Rasterization (resvg + fontdb, DejaVu Sans as `sans-serif` fallback) renders the wrapped lines consistently on this toolchain; cross-platform pixel equality is explicitly not asserted (ADR-0004).

## Conclusion

Adopted for the prototype. Structural SVG assertions do not depend on glyph widths; golden tests compare SVG structure and PNG dimensions/content. If pixel-fidelity complaints appear, the follow-up is an optional `font-kit`-based measurement behind a feature flag — recorded as a next action in REPORT-001, not built now.
