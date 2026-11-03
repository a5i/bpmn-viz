# bpmn-viz

Read BPMN 2.0 XML, validate its structure and Diagram Interchange (DI), and render diagrams as deterministic SVG or PNG — a pure-Rust workspace and CLI with no Node.js, browser or C dependencies.

**Status: prototype (SPEC-001 implemented; crates not yet published).**

## Install

```sh
cargo install --path crates/bpmn-cli    # provides the `bpmn` binary
```

Requires Rust 1.99.1 (pinned in `rust-toolchain.toml`).

## Usage

```sh
bpmn render INPUT.bpmn --format svg --output diagram.svg
bpmn render INPUT.bpmn --format png --scale 2 --output diagram.png
bpmn render INPUT.bpmn --output -                       # SVG to stdout
bpmn render INPUT.bpmn --diagram 1 --output second.svg  # diagram by index or id
bpmn inspect INPUT.bpmn [--format json]
bpmn validate INPUT.bpmn [--format json] [--strict]
```

Exit codes: `0` success, `2` invalid usage, `3` invalid BPMN/DI, `4` IO failure, `5` render failure. Diagnostics carry stable codes (`reference.dangling`, `di.missing_shape`, …) with severity, element id and source location.

## What it renders

Events (start/end/intermediate/boundary, all common markers), tasks and the common task kinds, exclusive/parallel/inclusive gateways, sequence flows (default tick, conditional diamond), pools/lanes, expanded/collapsed subprocesses, message flows, associations and text annotations — at the exact coordinates provided by BPMN DI. No auto-layout: models without DI fail with actionable diagnostics (AC-09). Unknown vendor extensions are ignored with warnings; unsupported symbols render as conspicuous fallback glyphs. The full compatibility matrix is in [REPORT-001](docs/reports/REPORT-001-conformance-baseline.md).

All seven reference cases of the [BPMN MIWG test suite](https://github.com/bpmn-miwg/bpmn-miwg-test-suite) subset selected by the specification parse, validate and render (pinned by revision in `tests/manifests/external-fixtures.lock.json`).

## Workspace

| Crate | Purpose |
| --- | --- |
| `bpmn-model` | semantic elements, DI types, stable diagnostics |
| `bpmn-xml` | namespace-aware import (quick-xml), bounded and entity-safe |
| `bpmn-validate` | references, geometry, DI coverage |
| `bpmn-scene` | renderer-agnostic scene primitives |
| `bpmn-render` | BPMN glyphs and scene construction |
| `bpmn-svg` | deterministic scene → SVG (owned writer) |
| `bpmn-raster` | SVG → PNG (resvg) with pixel budgets |
| `bpmn-cli` | `render` / `inspect` / `validate` |

## Documentation

Documentation is managed as schema-validated Markdown ([DocVera](docs/docvera/README.md)); start at the [documentation index](docs/index.md):

- [SPEC-001 — prototype specification](docs/specifications/SPEC-001-bpmn-renderer.md)
- [ADRs](docs/adr/ADR-0001-rust-only-rendering-pipeline.md) — accepted architecture decisions
- [Architecture notes](docs/architecture/overview.md) and guides ([development](docs/guides/development.md), [testing](docs/guides/testing.md), [release](docs/guides/release.md))
- [TEST-001 test plan](docs/testing/TEST-001-test-plan.md), [REPORT-001 conformance & performance baseline](docs/reports/REPORT-001-conformance-baseline.md)

## Development

```sh
scripts/check.sh        # fmt + clippy + tests + docvera + cargo-deny + cargo-audit
scripts/download-fixtures.sh   # pinned MIWG fixtures into gitignored external/
cargo bench -p bpmn-cli --bench render   # performance baseline
```

Every change references a task id; see [AGENTS.md](AGENTS.md) for the agent workflow.

## License

MIT — see [LICENSE](LICENSE). BPMN MIWG data is CC-BY-3.0 and is not redistributed (ADR-0005).
