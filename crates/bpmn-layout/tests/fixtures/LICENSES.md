# Fixture provenance and licenses

All `.bpmn` fixtures under `core/`, `complex/` and `failures/` are
**project-owned**, written by hand for this repository and licensed under the
repository MIT license (root `LICENSE`). The `core/` and `failures/` fixtures
are intentionally small and exercise one behavior each; the `complex/` stress
fixtures (TEST-005 §Fixtures) are larger realistic process topologies authored
for engine stress coverage. No third-party BPMN files are redistributed here.

Reference parity data under `reference/` is **derived output** of the pinned
upstream engine, committed as small canonical JSON for differential tests:

- Upstream project: <https://github.com/bpmn-io/bpmn-auto-layout>
- Pinned revision: tag `v2.0.0-alpha.2`, commit
  `75a7f30958c7050474d48db09afd4cc7d352ccad` (2026-07-24)
- Upstream license: MIT, © 2016-present Camunda Services GmbH
  (`LICENSE` at the pinned commit; full text reproducible from the tag)
- Derivation: `tools/reference-layout/run.mjs <fixture>` ran the pinned engine
  on the project-owned `core/*.bpmn` fixtures above (the upstream repository,
  its fixtures, and `node_modules` are **not** committed) and normalized the
  generated DI into `{shapes, edges, warnings}` JSON.
- Modifications: canonical re-serialization only (sorted by semantic id,
  integer coordinates preserved); fixture inputs are not upstream material.

Regenerating the JSON requires network access (npm install) and Node ≥ 22 and
is a development-only step; `cargo test` only reads the committed JSON.
