# PR Draft: Add `omx sparkshell` Rust sidecar preview

## Target branch
`experimental/dev`

## Source branch
`feat/omx-sparkshell`

## Summary
This PR adds a preview implementation of `omx sparkshell` with a Rust sidecar and an explicit tmux-pane summary mode.

The feature keeps the current JS CLI surface thin while moving the execution and summarization logic into Rust. It supports direct command execution, adaptive long-output summarization, and an explicit tmux-pane capture mode for larger leader-side worker/context inspection.

## Changes
- add `omx sparkshell <command> [args...]`
- add `omx sparkshell --tmux-pane <pane-id> [--tail-lines <100-1000>]`
- keep the JS layer as a thin launcher/help surface only
- add Rust sidecar at `native/omx-sparkshell/` for:
  - direct argv execution
  - combined stdout/stderr line-threshold branching
  - local `codex exec` summary bridge
  - embedded command-family registry
  - explicit tmux-pane capture mode
- package the staged native binary under `bin/native/linux-x64/omx-sparkshell`
- add helper scripts:
  - `scripts/build-sparkshell.mjs`
  - `scripts/test-sparkshell.mjs`
- document preview behavior in `README.md`
- add/update focused JS bridge tests and native Rust tests

## User-visible behavior
- short output stays raw
- long output is summarized into markdown sections limited to:
  - `summary:`
  - `failures:`
  - `warnings:`
- summary model precedence:
  1. `OMX_SPARKSHELL_MODEL`
  2. `OMX_SPARK_MODEL`
  3. spark default model
- native binary lookup precedence:
  1. `OMX_SPARKSHELL_BIN`
  2. packaged `bin/native/<platform>-<arch>/omx-sparkshell[.exe]`
  3. repo-local `native/omx-sparkshell/target/release/omx-sparkshell[.exe]`
- tmux-pane summarization is explicit opt-in, not always-on

## Why this is good
- introduces a bounded Rust-first execution surface without forcing a full OMX CLI rewrite
- keeps existing worker spark semantics separate from sparkshell model routing
- enables larger tmux-pane context summarization for leader/operator inspection without embedding hidden JS-only heuristics into team status
- creates a stronger foundation for future Rust-native OMX work

## Validation
- [x] `cargo test --manifest-path native/omx-sparkshell/Cargo.toml`
- [x] `node scripts/build-sparkshell.mjs`
- [x] `node scripts/test-sparkshell.mjs`
- [x] `node --test dist/cli/__tests__/sparkshell-cli.test.js`
- [x] `node --test dist/cli/__tests__/sparkshell-packaging.test.js`
- [x] `npm test`
- [x] architect verification approved

## Commits in this branch
- `d3e7fd2` feat(sparkshell): add Rust sidecar preview for omx sparkshell
- `1dd013d` fix(sparkshell): keep missing-binary error path testable
- `d76e761` feat(sparkshell): add opt-in tmux pane summary mode

## Notes / Risks
- preview currently stages a Linux x64 binary in-tree
- direct team-status auto-summary is intentionally not enabled by default; tmux-pane summarization stays explicit
- native tests mutate env vars in a few places, so parallel CI flake risk should still be watched

## Related artifacts
- PRD: `.omx/plans/prd-omx-sparkshell.md`
- Test spec: `.omx/plans/test-spec-omx-sparkshell.md`
- Prior draft: `docs/prs/dev-omx-sparkshell.md`
