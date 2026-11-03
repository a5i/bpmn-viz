# Changelog

All notable changes to this project are documented here. Format based on [Keep a Changelog](https://keepachangelog.com/); the project does not yet follow a release cadence.

## [0.1.0] — prototype (unreleased)

### Added

- `bpmn-model`: semantic BPMN 2.0 subset (events incl. boundary events, tasks and common kinds, gateways, sequence flows, pools/lanes, subprocesses, text annotations), Diagram Interchange types, and stable diagnostic codes.
- `bpmn-xml`: namespace-URI-based importer (never prefix-based), `xsi:type` handling, subprocess bodies, collaborations, lane sets, data shapes/associations at id/DI resolution level, bounded input (size/depth), entity-expansion rejection.
- `bpmn-validate`: duplicate ids, dangling/foreign references, invalid geometry, missing Diagram Interchange, boundary attachment checks, collapsed-subprocess exemptions.
- `bpmn-scene`: renderer-agnostic scene primitives with deterministic paint order and approximate label wrapping.
- `bpmn-render`: glyphs for the P0 symbol set plus boundary events, pools/lanes with rotated labels, expanded/collapsed subprocesses, flow decorations (default tick, conditional diamond), dashed message/association edges, text annotations, conspicuous fallback glyphs.
- `bpmn-svg`: owned deterministic scene-to-SVG serializer.
- `bpmn-raster`: resvg-based PNG export with scale validation and pixel budgets.
- `bpmn-cli`: `render`, `inspect`, `validate` with documented exit codes (0/2/3/4/5), text and JSON diagnostics, atomic writes, diagram selection by index or id.
- Test suite: 18 suites (90+ tests) with committed golden SVG, MIWG smoke against pinned external fixtures, fuzz harnesses, and DocVera-validated documentation set (SPEC/ADR/EPIC/TASK/EXP/TEST/REPORT/RISK).
