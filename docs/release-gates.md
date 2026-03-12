# Release Verification Gates — Cargo/Native First

Status: active (as of 2026-03-12)
Owner: Lane 3 (release gate redesign)

This repository is transitioning to a cargo/native-first product story. The release gate is now provably satisfied using native checks; Node/JS tests are compatibility-only and are not required for product readiness.

## Required (product gate)

- cargo test --workspace
- Optional flags: --all-features when applicable

Acceptance: all tests pass.

## Optional (compatibility quarantine)

Run only when you are touching compatibility surfaces (tmux / notify-hook / JS SDK):

- npm run test:compat:node

Acceptance: failures here do not block the release unless the change targets a compat-only surface directly.

## Additional sanity checks (non-gating)

- omx doctor
- omx team --help (CLI loads and prints usage)

## Proof matrix

| Gate                  | Command                           | Required | Last run (UTC)      | Result |
|-----------------------|------------------------------------|----------|---------------------|--------|
| Native product tests  | cargo test --workspace            | Yes      | 2026-03-12 05:44    | PASS   |
| Node compat tests     | npm run test:compat:node          | No       | not run             | —      |
| Doctor                | omx doctor                        | No       | not run             | —      |

## How to run

```bash
# Required product gate
cargo test --workspace

# Optional compat-only gate
npm run test:compat:node
```

## Notes

- Documentation and prompts should refer to the required native gate only. Any mention of npm/Node applies to the optional compat lane.
- CI should mark the cargo/native job as required and Node compat jobs as informational.

## Evidence snapshot (2026-03-12)

cargo test --workspace summary:

```
106 passed; 0 failed; finished in ~1.64s (core suite)
```
