---
id: TASK-001
type: task
title: Repository bootstrap
status: done
relations:
  parent: EPIC-003
  implements: [SPEC-001]

---
# Repository bootstrap

## Goal

Create the Cargo workspace, toolchain pin and repository policy files from the scaffold so every later task lands in a reproducible tree.

## Implementation

Extracted the prototype scaffold, wrote the workspace manifest with workspace dependency inheritance, pinned `rust-toolchain.toml` to 1.99.1, added `rustfmt.toml`, `deny.toml`, merged `.gitignore` per SPEC-001 section 6, and committed the tree (commit `d9d5e77`).

## Verification

`cargo fetch` resolves the pinned dependency graph; `cargo build --workspace` succeeds; repository structure matches SPEC-001 section 5.

## Done

- Bootstrap commit pushed to `main`.
- Structure reviewed against SPEC-001 section 5.
