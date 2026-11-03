---
id: SPEC-001
type: specification
title: BPMN 2.0 Rust Parser and SVG/PNG Renderer Prototype
status: proposed
version: 0.1.0
links:
  decisions: [ADR-0001, ADR-0002, ADR-0003, ADR-0004, ADR-0005, ADR-0006, ADR-0007]
  epics: [EPIC-001, EPIC-002, EPIC-003, EPIC-004, EPIC-005]
---

# BPMN 2.0 Rust Parser and SVG/PNG Renderer — Prototype Specification

## 1. Purpose and desired outcomes

Build an MIT-licensed Rust workspace that reads BPMN 2.0 XML (`.bpmn`, `.bpmn20.xml`), validates minimum required structure, resolves BPMN Diagram Interchange (DI) geometry, and renders selected diagrams as deterministic SVG and PNG. The product exposes reusable Rust library APIs and a cross-platform CLI. The document set and task graph are kept in Git and managed through DocVera with schema validation and typed links.

**Success for prototype:** a user can run `bpmn render input.bpmn --format svg --output diagram.svg` or `--format png`, select one diagram from a multi-diagram model, obtain intelligible output for a documented subset of BPMN symbols, and inspect diagnostics for invalid models. The tool is reproducible in CI and usable without Node.js or a browser.

## 2. Normative context and assumptions

- BPMN 2.0 semantics and BPMN DI are different concerns; semantic element IDs are resolved from `bpmndi:BPMNShape/@bpmnElement` and `bpmndi:BPMNEdge/@bpmnElement`.
- DI supplies shape bounds and edge waypoints; renderer must preserve provided geometry rather than recalculating it.
- Files may contain multiple `bpmndi:BPMNDiagram` elements. Users must be able to select diagram by index or ID.
- Unknown vendor extensions must not crash parsing. Read-only prototype does not promise lossless export of original XML.
- No default auto-layout for models without DI. Return a structured error/diagnostic explaining missing geometry.
- Initial implementation may render unsupported semantic elements using a conspicuous but valid fallback glyph, while emitting warnings; never silently claim full BPMN conformance.
- Font metrics vary across environments. Visual tests use pinned font assets only when licenses allow redistribution, and pin resvg/toolchain versions.

## 3. Scope and non-goals

### In scope, P0

1. File/byte loading, safe XML parsing, namespace-aware element identification, root `definitions`, IDs and references.
2. Semantic subset: start/end/intermediate events; task and common task kinds; exclusive/parallel/inclusive gateways; sequence flows; pools/participants and lanes; expanded/collapsed subprocess boundaries; labels and annotations.
3. BPMN DI: `BPMNDiagram`, `BPMNPlane`, `BPMNShape`, `BPMNEdge`, `dc:Bounds`, `di:waypoint`, label bounds, orientation and IDs.
4. SVG renderer: stable IDs, viewBox, markers, strokes, text layout, arrowheads, default themes, escaped labels, deterministic output.
5. PNG export through `resvg` using explicit scale and background configuration.
6. CLI: `render`, `inspect`, `validate`; diagnostics support text and JSON; documented exit codes.
7. Unit, integration, fixture, visual regression, malformed-input and fuzz testing; MIT-compatible dependency/license checks.

### P1 extension after prototype

Boundary event variants; message/association flows; event markers; data objects; horizontal/vertical pools and lanes; subprocess markers; additional BPMN symbols, CSS-like theme options; streaming input; WASM integration, benchmark suite; automatic layout of no-DI models (EPIC-005; ADR-0006/ADR-0007, `bpmn-layout` crate).

### Out of scope

BPMN process execution engine; BPMN model editor; BPMN XML writer/roundtrip; BPMN XSD validation in full; interactive canvas/DOM; guaranteed pixel-perfect match with Camunda/bpmn-js; PDF output; arbitrary embedded external resources.

## 4. Target architecture

Pipeline:

`BPMN XML bytes -> XML AST/index -> Semantic BPMN Model + DI Model -> Resolver/Validator -> Render Scene -> SVG serializer -> (optional resvg -> PNG)`

Each stage is independent and returns structured diagnostics. The semantic model is free of SVG-specific data; DI uses floating-point geometry; scene graph is agnostic of BPMN-specific element IDs except optional metadata.

### Workspace crates

| Crate | Purpose | Key dependencies |
| --- | --- | --- |
| `bpmn-model` | semantic elements, DI types, identifiers, diagnostics | `serde`, `thiserror` (optional) |
| `bpmn-xml` | namespace-aware import and index building | `quick-xml`, `bpmn-model` |
| `bpmn-validate` | missing references, duplicate IDs, invalid DI | `bpmn-model` |
| `bpmn-scene` | renderer-agnostic primitives: group, path, rectangle, text, transform, styles | `kurbo` |
| `bpmn-render` | BPMN glyphs and scene construction | `bpmn-model`, `bpmn-scene`, `kurbo` |
| `bpmn-svg` | deterministic scene-to-SVG serialization | `bpmn-scene`, `svg` (optional; may use owned XML writer) |
| `bpmn-raster` | SVG-to-PNG rendering | `resvg` |
| `bpmn-cli` | arguments, filesystem, output formats and errors | `clap`, `serde_json`, above crates |

`bpmn-layout` was kept out of the initial workspace and is created only once automatic layout is explicitly approved — approved 2026-11-03 via ADR-0006/ADR-0007 under EPIC-005 (M1 scope, TEST-005). `petgraph` is optional for later graph analysis and is not needed in first parsing/rendering path. `ebi_bpmn` should be evaluated in a spike but not hardwired: its focus is behavioral BPMN models, not full DI rendering.

### Data model requirements

- Preserve QName namespace URI and local name (do not rely on literal prefixes `bpmn:`, `bpmndi:`, `dc:`, `di:`).
- Keep stable IDs and typed, resolvable references; detect duplicate IDs.
- Store document-level definitions and an ordered collection of diagrams.
- Model `Bounds {x, y, width, height}` and edge waypoints with finite-number validation, plus optional `BPMNLabel` bounds.
- Separate parsing errors, conformance warnings, missing-geometry failures and unsupported-symbol warnings.
- Render plans must define z-order: lane/pool backgrounds, flows, nodes, markers, labels, overlays.
- Unsupported nonessential extensions may be ignored with diagnostic; missing critical DI or dangling mandatory references must yield actionable errors.
- Avoid XML entity expansion, external resource fetching and arbitrary script execution.

## 5. Repository structure

```text
bpmn-rs/
├── .cargo/config.toml
├── .github/
│   ├── CODEOWNERS
│   ├── ISSUE_TEMPLATE/{bug_report.md,feature_request.md}
│   ├── PULL_REQUEST_TEMPLATE.md
│   └── workflows/{ci.yml,security.yml,release.yml}
├── .gitignore
├── .gitattributes
├── .editorconfig
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── rustfmt.toml
├── clippy.toml                       # only if supported settings are needed
├── deny.toml
├── about.toml                       # optional license report config
├── LICENSE
├── README.md
├── CONTRIBUTING.md
├── SECURITY.md
├── CHANGELOG.md
├── crates/
│   ├── bpmn-model/src/{lib.rs,element.rs,di.rs,diagnostic.rs}
│   ├── bpmn-xml/src/{lib.rs,parser.rs,namespaces.rs,resolve.rs}
│   ├── bpmn-validate/src/{lib.rs,reference.rs,geometry.rs}
│   ├── bpmn-scene/src/{lib.rs,node.rs,path.rs,style.rs,text.rs}
│   ├── bpmn-render/src/{lib.rs,renderer.rs,glyphs.rs,labels.rs,flows.rs}
│   ├── bpmn-svg/src/{lib.rs,serialize.rs}
│   ├── bpmn-raster/src/{lib.rs,png.rs}
│   └── bpmn-cli/src/{main.rs,commands/{render.rs,inspect.rs,validate.rs}}
├── tests/
│   ├── fixtures/
│   │   ├── minimal/{start-task-end.bpmn,start-task-end.svg}
│   │   ├── invalid/{duplicate-id.bpmn,dangling-reference.bpmn,missing-di.bpmn}
│   │   └── edge-cases/{multi-diagram.bpmn,unicode.bpmn}
│   ├── golden/                       # small reviewed, redistributable PNG fixtures
│   ├── integration/{cli.rs,parse.rs,render.rs}
│   └── manifests/{external-fixtures.lock.json,coverage.json}
├── fuzz/fuzz_targets/{xml_parse.rs,render_document.rs}
├── benches/render.rs
├── scripts/{check.sh,download-fixtures.sh,compare-images.sh}
├── docs/
│   ├── index.md
│   ├── specifications/SPEC-001-bpmn-renderer.md
│   ├── architecture/{overview.md,data-model.md,render-pipeline.md}
│   ├── adr/
│   │   ├── ADR-0001-rust-only-rendering-pipeline.md
│   │   ├── ADR-0002-separate-semantic-di-and-scene-models.md
│   │   ├── ADR-0003-no-autolayout-in-poc.md
│   │   ├── ADR-0004-golden-visual-tests.md
│   │   └── ADR-0005-third-party-fixture-policy.md
│   ├── tasks/{EPIC-001-parser.md,EPIC-002-renderer.md,EPIC-003-testing.md,EPIC-004-release.md}
│   ├── experiments/{EXP-001-ebi-bpmn-evaluation.md,EXP-002-text-rendering.md}
│   ├── reports/{REPORT-001-conformance-baseline.md}
│   ├── guides/{development.md,testing.md,release.md}
│   └── docvera/{README.md,schema-placeholder.md}
└── external/                         # gitignored downloaded MIWG datasets
```

`SPEC-001` is the canonical specification; this downloadable standalone file must be copied to `docs/specifications/SPEC-001-bpmn-renderer.md` on initialization. Avoid duplicating divergent specification text in multiple locations.

## 6. Git policy

**Commit:** handwritten `.rs`, `.md`, `.toml`, `.yml`, `.bpmn` fixtures created by this project, reviewed small golden images with verified redistribution rights, licenses/attributions, stable manifests, CI configs, toolchain pin, `Cargo.lock` for reproducible workspace/CLI builds, script sources, releases metadata.

**Do not commit:** `target/`, IDE local settings, `.env` or secrets, local caches, logs, temp files, large downloads, generated bulk PNG/SVG reports, fuzz corpora generated during runs, compiled executables, coverage outputs, generated third-party notices unless explicitly part of release bundle, datasets under incompatible or unclear terms.

**External datasets:** pin repository URL, upstream revision/hash, path and license in `tests/manifests/external-fixtures.lock.json`. Download to gitignored `external/`, verify SHA256 where possible. CI must not require externally network-fetched fixtures for every PR; keep a self-contained small committed test suite, and schedule separate upstream conformance jobs. Never strip source attribution.

Example `.gitignore`:

```gitignore
/target/
/external/
/coverage/
/artifacts/
/benchmark-results/
/fuzz/artifacts/
/fuzz/corpus/
*.profraw
*.profdata
*.log
.env
.env.*
!.env.example
.DS_Store
.idea/
.vscode/
**/*.tmp
```

If choosing to version curated fuzz corpus cases, relocate them under `tests/fixtures/` with provenance instead of broadly tracking generated fuzz directories.

## 7. Dependency, licensing and supply-chain policy

Prefer MIT or MIT/Apache-2.0 dependencies; verify exact chosen versions and full transitive graph prior to publication. Start with `quick-xml`, `kurbo`, `resvg`, `clap`, `serde`, `serde_json`, `thiserror`, optionally `svg`; use `roxmltree` only if tree-based XML traversal demonstrably simplifies implementation. Use `cargo-deny` to enforce allowed licenses and registries, and `cargo-about` or equivalent for notices. Treat `bpmn-js` as a non-runtime reference tool only, never embed its viewer by default; bpmn.io licensing adds a watermark condition. BPMN MIWG data is CC BY 3.0, so treat it as attributed fixtures independent of MIT code.

Security baseline: `cargo audit` or RustSec-aware equivalent, `cargo deny check`, pinned GitHub Actions SHA for hardening, minimal permissions, rustfmt/clippy, untrusted-input tests. Confirm the current exact CLI and config syntax of third-party tools before enabling CI.

## 8. CLI contract

```text
bpmn render INPUT --format svg|png --output FILE [--diagram ID|INDEX] [--scale FLOAT]
bpmn inspect INPUT [--format text|json]
bpmn validate INPUT [--format text|json] [--strict]
bpmn --version
```

- Default to SVG when extension/format is omitted; refuse mismatched explicit format vs extension or document conversion clearly.
- `--scale` positive finite, only applies to rasterized output; provide max output pixels to prevent resource exhaustion.
- If multiple diagrams and none selected, explicitly document default first diagram and emit a warning; stable order follows source XML.
- All output writes atomic (temporary file + rename where supported); errors go to stderr; stdout allowed when output is `-` for SVG only initially.
- Exit codes: 0 success, 2 invalid CLI, 3 invalid BPMN/DI, 4 IO failure, 5 render failure; JSON diagnostics include stable codes, severity, element ID, source location when available.

## 9. Public Rust API target

```rust
let document = BpmnDocument::from_xml(xml_bytes)?;
let report = document.validate();
let diagram = document.diagram_by_index(0)?;
let svg = diagram.render_svg(&RenderOptions::default())?;
let png = diagram.render_png(&PngOptions { scale: 2.0, ..Default::default() })?;
```

This is **illustrative interface design**, not an existing API or compilation-tested code. Keep byte buffer handling and errors owned and explicit; use `Result` and typed diagnostics rather than panic on malformed input.

## 10. Acceptance criteria and quality gates

### Functional

- AC-01: parse a minimal valid BPMN sample with arbitrary namespace prefixes and read element IDs and names.
- AC-02: detect duplicate IDs, missing referenced elements and invalid/NaN/infinite bounds.
- AC-03: resolve DI shape IDs to semantic IDs and render at provided coordinates.
- AC-04: render start/end events, tasks, exclusive and parallel gateways, sequence flow arrows and text labels to valid SVG.
- AC-05: support at least one collaboration example with pool and lanes and one expanded subprocess example; document exact supported variations.
- AC-06: enumerate 2+ diagrams and select the requested diagram consistently.
- AC-07: PNG exports at scale 1x and 2x have expected dimensions and nonempty pixel content.
- AC-08: Unicode labels, XML escaping, negative/large coordinates and broken inputs do not crash.
- AC-09: no hidden layout fallback if DI missing; return diagnostic with location/element ID when possible.
- AC-10: CLI works on Windows, macOS and Linux with pinned toolchain; no JS runtime required.

### Quality

- All crates build with `cargo test --workspace --all-features` and pass `cargo fmt --check` and clippy with warnings denied for CI-selected targets.
- Unit tests include parser, DI resolver, geometry validation, SVG serialization and CLI argument handling.
- Golden tests are reviewed; diff reports with rendered actual/expected/diff images are attached as CI artifacts only, not committed by default.
- CI validates docs graph/schema, licenses and vulnerable dependencies. No silent test skips.
- Initial performance budget is measured and recorded as a baseline (do not invent speed guarantees); benchmark a 100-, 1,000-, and 10,000-element synthetic document with bounded input limits.

### Definition of Done

For each task: implementation merged; public API documented; meaningful tests pass; regression fixture supplied for fixed bugs; docs links valid; ADR updated if architecture changed; DocVera schema/reference validation passes; third-party attributions recorded; CI green.

## 11. Test datasets and provenance

Primary: [BPMN MIWG reference suite](https://github.com/bpmn-miwg/bpmn-miwg-test-suite), especially A.1.0, A.2.0, A.3.0, A.4.0, A.4.1, C.4.0, C.8.0. MIWG reference files are CC BY 3.0; do not relicense as MIT. Keep external pulls in ignored `external/` and use a manifest. Supplement with synthetic project-owned fixtures that target namespace changes, invalid XML, dangling refs, extreme geometry and text escaping.

Comparison dimensions: structural counts and resolved references; DI coordinate preservation; SVG structure snapshot; PNG dimension and non-empty image assertions; controlled visual metrics including pixel mismatch and perceptual similarity. Pixel-perfect comparisons with unrelated renderers are not pass/fail conformance tests because fonts and drawing conventions vary.

## 12. DocVera: documentation-as-code and work management

DocVera should manage Markdown documents as typed, versioned artifacts in Git. Known public package description: `@a5i/docvera` is a schema-validated, queryable Markdown knowledge engine; exact CLI syntax, schema file syntax, hooks and query language were **not independently verified**. Do not put guessed `docvera` commands in CI. First conduct `EXP-003` to inspect the installed CLI/manual and create a tested adapter in `scripts/docvera-check.sh`.

### Desired artifact types

| Type | ID format | Mandatory sections | Typical links |
| --- | --- | --- | --- |
| Specification | `SPEC-NNN` | Context, Goals, Scope, Requirements, Acceptance Criteria | ADR, EPIC, TEST |
| ADR | `ADR-NNNN` | Context, Decision, Alternatives, Consequences, Validation | SPEC, ADR, EXP |
| Epic | `EPIC-NNN` | Outcome, Scope, Acceptance Criteria, Tasks | SPEC, TASK, REPORT |
| Task | `TASK-NNN` | Goal, Implementation, Verification, Done | EPIC, ADR, TEST |
| Experiment | `EXP-NNN` | Hypothesis, Setup, Results, Conclusion | ADR, SPEC |
| Test Plan | `TEST-NNN` | Fixtures, Assertions, Coverage, Exit Criteria | SPEC, TASK, REPORT |
| Report | `REPORT-NNN` | Method, Results, Gaps, Next Actions | TEST, EPIC |
| Risk | `RISK-NNN` | Description, Likelihood, Impact, Mitigation | SPEC, ADR |

### Common minimal YAML frontmatter (design contract, not verified DocVera syntax)

```yaml
---
id: ADR-0002
type: adr
title: Separate semantic, DI and scene models
status: accepted
created: 2026-11-02
relations:
  implements: [SPEC-001]
  informed_by: [EXP-001]
  supersedes: []
---
```

### Required validation rules

- All IDs unique and match type prefix; only declared types/statuses allowed.
- Required sections exist and are nonempty (not only frontmatter).
- Every relationship target exists; links respect allowed types (e.g., task belongs to epic; ADR impacts specification).
- `accepted` ADR must contain decision, consequences and supporting evidence; `superseded` ADR must point to successor.
- Task cannot become `done` without verification evidence and linked acceptance criterion.
- Epic cannot become `done` until all required child tasks are done and acceptance tests passed.
- DocVera issues should be visible as CI annotations/report; schema migration is versioned with its own ADR.
- No generated state database or search index committed unless intentionally approved as reproducible source.

### Suggested document lifecycle

- SPEC: proposed -> approved -> implemented -> deprecated
- ADR: proposed -> accepted -> superseded (or rejected)
- EPIC/TASK: backlog -> ready -> in_progress -> review -> done (or cancelled/blocked)
- EXP: planned -> running -> concluded
- REPORT: draft -> final

Only use lifecycle transitions and cross-document expressions actually supported by the installed DocVera version; otherwise enforce missing logic in a small Rust or CI script while tracking the gap in an experiment document.

### Agent workflow

AI agents start by reading `AGENTS.md` (create this as a committed top-level file), `docs/index.md`, relevant SPEC/ADR and active TASK. Each change references a TASK ID in PR description. Decisions affecting public API, error semantics or dependencies require ADR review. Agent output must link modified code and tests to the task; never mark tasks done solely on generated prose.

## 13. ADR policy and initial decisions

ADR filename `ADR-NNNN-short-kebab-title.md`, stable ID, never renumber. Decisions are immutable historically; changed decisions create a new ADR and `supersedes` relation. Use one topic per ADR; store trade-offs and rejection reasons. Initial ADRs:

1. **ADR-0001:** Pure Rust runtime and no embedded bpmn-js (chosen for MIT-compatible binary distribution and no JS runtime).
2. **ADR-0002:** Semantic, DI and scene models separated (chosen for correctness, testing, alternate outputs).
3. **ADR-0003:** DI-required rendering; no automatic layout in prototype (chosen for bounded scope).
4. **ADR-0004:** Structural plus visual regression testing (chosen because pixel comparisons alone are insufficient).
5. **ADR-0005:** External third-party fixtures pinned by manifest and not blindly vendored (attribution and supply-chain reasons).

Any ADR must specify `Status`, `Context`, `Decision`, `Alternatives Considered`, `Consequences`, `Validation`, `References`.

## 14. Delivery backlog

| Epic | Deliverables | Exit criteria |
| --- | --- | --- |
| EPIC-001 Parser/DI | crates `bpmn-model`, `bpmn-xml`, `bpmn-validate`; fixtures and errors | AC-01, AC-02, AC-03, AC-06 |
| EPIC-002 Renderer | scene, glyphs, SVG, raster and CLI | AC-04, AC-05, AC-07, AC-09 |
| EPIC-003 Quality | MIWG harness, goldens, fuzz, cross-platform CI | AC-08, AC-10, quality gates |
| EPIC-004 Documentation/Release | docs schemas, ADRs, DocVera adapter, license notices, release guide | DoD, documented compatibility, reproducible build |

Initial task order: TASK-001 repository bootstrap; TASK-002 DocVera CLI/schema spike; TASK-003 BPMN XML namespaces; TASK-004 DI indexing; TASK-005 semantic resolver and diagnostics; TASK-006 scene primitives; TASK-007 node glyphs; TASK-008 flows/labels; TASK-009 SVG serializer; TASK-010 PNG export; TASK-011 CLI; TASK-012 MIWG fixture manifest; TASK-013 golden tests; TASK-014 fuzz/security; TASK-015 cross-platform CI; TASK-016 published API and docs.

## 15. Risks and open questions

- `RISK-001`: exact DocVera schema, query and hook capabilities unverified; resolve with dedicated spike before committing configuration.
- `RISK-002`: text metrics/font fallback may cause unstable visual output; resolve by font policy and structural tests.
- `RISK-003`: BPMN symbol coverage larger than expected; resolve by published support matrix and warnings.
- `RISK-004`: third-party fixture rights/provenance; resolve through manifests and license review.
- `RISK-005`: malformed XML and large geometry might exhaust memory; resolve with size limits, checked arithmetic and fuzzing.
- Open: exact crate publication names on crates.io; verify namespace availability before publishing.
- Open: whether Rust SVG serializer should use `svg` crate or custom narrow writer; benchmark and ADR if choice affects output stability.
- Open: DocVera schema dialect, validation engine, CLI command names, relationship support, and Git hooks.

## 16. Verified external references

- [BPMN MIWG Test Suite](https://github.com/bpmn-miwg/bpmn-miwg-test-suite) — reference models, images, and CC BY 3.0 terms.
- [bpmn.io license](https://bpmn.io/license/) — special watermark condition for embedded bpmn-js.
- [ebi_bpmn docs](https://docs.rs/ebi_bpmn/latest/ebi_bpmn/) — Rust behavioral BPMN parser/writer; DI support not assumed.
- [resvg repository](https://github.com/linebender/resvg) — Rust SVG rasterization.
- [quick-xml docs](https://docs.rs/quick-xml/) — Rust XML parsing.
- [DocVera package listing](https://socket.dev/npm/package/%40a5i/docvera) — public description, not a CLI specification.
