---
id: TASK-007
type: task
title: Node glyphs
status: done
relations:
  parent: EPIC-002
  implements: [SPEC-001]

---
# Node glyphs

## Goal

Draw the SPEC-001 P0 symbol set plus boundary events at DI-provided coordinates (AC-03, AC-04, AC-05).

## Implementation

Implemented `bpmn-render::glyphs`: thin/thick/double event circles with marker glyphs (message, timer, error, escalation, terminate, compensation, conditional, link, signal, cancel, multiple), rounded task rects with kind icons (user, service, script, manual, send, receive, business rule), gateways with X/plus/circle markers, expanded/collapsed subprocesses with marker boxes, pool bands and lanes, text annotation brackets, and the amber dashed fallback for unsupported symbols with `symbol.unsupported` warnings.

## Verification

Unit tests cover task glyph composition, gateway marker variants and event-based-gateway warnings; integration tests assert glyphs for fixture and MIWG files.

## Done

- `cargo test -p bpmn-render` green.
- MIWG A/C files render with `symbol.unsupported` warnings only for P1 symbols (data shapes), recorded in REPORT-001.
