# Native Launch Legacy-Parity Review

> Status update (2026-03-12): Detached-session tmux path is implemented behind opt-in (env/flag). Inside-tmux HUD split+cleanup present. Help text + fixture aligned. Remaining work: broaden failure-path rollbacks and ensure docs across surfaces reflect native-first authority.

## Scope

This review captures the current launch-behavior gap between the TypeScript `launchWithHud()` path and the native Rust launcher so the parity port can land with a clear, reviewable contract.

Authoritative planning inputs:
- `.omx/context/native-launch-legacy-parity-20260312T034208Z.md`
- `.omx/plans/prd-native-launch-legacy-parity.md`
- `.omx/plans/test-spec-native-launch-legacy-parity.md`

## Current gap snapshot

### Native Rust today
- `crates/omx-cli/src/launch.rs` launches `codex` directly and has no tmux/HUD bootstrap path.
- `crates/omx-cli/src/lib.rs` help text currently promises `HUD auto-attaches only when already inside tmux`.
- `src/compat/fixtures/help.stdout.txt` mirrors the same help contract.

### Legacy JS/TS authority
`src/cli/index.ts` remains the shipped behavioral reference for launch parity:
- inside tmux: splits a HUD pane, runs Codex in the current pane, and kills HUD panes on exit
- outside tmux with tmux available: creates a detached session, splits a HUD pane, registers resize/reconcile hooks, then attaches
- without tmux: falls back cleanly to direct Codex launch

## Legacy behavior inventory to preserve

### 1. Inside tmux
Reference implementation: `src/cli/index.ts` `runCodex()` inside the `launchPolicy === 'inside-tmux'` branch.

Required behavior:
1. Detect current pane from `TMUX_PANE`.
2. Kill stale HUD watch panes in the current window before creating a new one.
3. Split a new bottom HUD pane with `omx hud --watch`.
4. Run Codex in the current pane.
5. Always clean up created/stale HUD panes on exit.
6. Best-effort only for tmux conveniences (mouse enabling, HUD creation failures must not block Codex).

### 2. Outside tmux, tmux available
Reference implementation: `buildDetachedSessionBootstrapSteps()`, `buildDetachedSessionFinalizeSteps()`, `buildDetachedSessionRollbackSteps()`, and the detached-session branch in `runCodex()`.

Required behavior:
1. Create a detached tmux session rooted at the launch cwd.
2. Start Codex in the main pane for that session.
3. Split a HUD pane and capture its pane id.
4. Register resize/client-attached reconcile hooks when supported.
5. Reconcile/schedule HUD resizing.
6. Attach to the new session.
7. If bootstrap/finalize fails after creating the session, roll back hooks/session best-effort, then fall back to direct Codex.

### 3. No tmux available
Required behavior:
1. Do not emit noisy tmux ENOENT failures.
2. Launch Codex directly.
3. Preserve current arg normalization and Codex spawn error handling.

## Review notes for the Rust port

### Good substrate already available
The Rust worktree already has reusable process/tmux helpers in `crates/omx-process/**`:
- `process_bridge.rs`
- `process_plan.rs`
- `tmux_commands.rs`
- `tmux_shell.rs`

This means the parity work should prefer reusing those helpers rather than open-coding shell strings in `crates/omx-cli/src/launch.rs`.

### Documentation contract that must move with implementation
When parity lands, these surfaces must change together:
- `crates/omx-cli/src/lib.rs` help text
- `src/compat/fixtures/help.stdout.txt`
- any release/handoff docs that still describe native launch as inside-tmux-only HUD behavior

#

## Partial implementation status in the current worktree

A native launch implementation is now in flight in `crates/omx-cli/src/launch.rs`.

Current state from direct code review:
- **Landed:** native inside-tmux branch now probes tmux, creates a HUD watch pane, runs Codex, and kills the created HUD pane on exit.
- **Still missing for full parity:** the out-of-tmux detached-session bootstrap/attach path is not yet present in Rust; non-tmux launches still fall through to direct Codex.
- **Still drifting:** help text in `crates/omx-cli/src/lib.rs` and `src/compat/fixtures/help.stdout.txt` still describes only the old inside-tmux auto-attach contract.

Review implication: the branch appears to be moving from "no parity" to **inside-tmux-only partial parity**. Do not mark the native launch task complete until detached-session parity, truthful help, and full launch-matrix tests land together.

## Review risks to watch for
1. **Help drift:** do not leave Rust help/fixture text promising less or more than the implementation.
2. **Cleanup regressions:** inside-tmux HUD panes must be removed on exit; detached-session rollback must clean up best-effort.
3. **Fallback regressions:** tmux failure must still reach direct Codex without noisy hard failure when fallback is expected.
4. **Scope creep:** keep this feature focused on legacy parity; do not mix in a larger redesign of tmux/HUD policy.

## Reviewer checklist

Before calling the parity port complete, verify:
- [ ] inside-tmux native launch creates a HUD pane and still runs Codex in the current pane
- [ ] inside-tmux native launch kills HUD pane(s) on exit
- [ ] outside-tmux native launch creates a detached tmux session with HUD and attach behavior
- [ ] detached-session failure path rolls back best-effort and still falls back to direct Codex when appropriate
- [ ] tmux-missing path launches Codex directly without noisy tmux failure output
- [ ] `omx`/`omxshell --help` text matches shipped behavior
- [ ] `src/compat/fixtures/help.stdout.txt` matches Rust help output
- [ ] parity tests cover all three launch branches

## Immediate follow-up for this branch slice

The current Rust launcher is still in the pre-parity state, so this review should be used as the acceptance checklist while worker implementation/testing lands. Once the code changes are present, this document can serve as the reviewer-facing proof checklist for final sign-off.


## Review notes on the in-flight native parity test file

Current worktree also contains an in-flight test candidate at `crates/omx-cli/tests/native_launch_legacy_parity.rs`.

Notable review findings:
1. **Fixed in this worktree pass:** the test now resolves the native binary through compile-time `CARGO_BIN_EXE_omx` first, avoiding false negatives caused by a stale `target/debug/omx`.
2. **Fixed in this worktree pass:** the tmux stub now uses POSIX-portable `;;` terminators instead of `;;&`, so the test remains valid under `/bin/sh` implementations such as `dash`.
3. **Fixed in this worktree pass:** the help assertion now checks the truthful detached-session wording (`auto HUD in tmux; detached tmux session when available`), and the shipped help fixture was updated to match.
4. **Still missing:** the detached-session test proves `new-session` / `split-window` / `attach-session`, but it does not yet prove resize/client-attached reconcile hook registration or rollback behavior.
5. **Still missing:** the inside-tmux test proves pane split + cleanup at a high level, but it does not yet distinguish stale-pane cleanup from cleanup of the newly created HUD pane.

Reviewer recommendation: treat that file as a good starting matrix, but require one more pass before final sign-off so the tests validate the full parity/rollback contract rather than only the happy-path shelling behavior.

Migration note (2026-03-12): native-first with tmux opt-in
- Detached tmux session is no longer automatic when not inside tmux.
- Opt-in signals: `--tmux` flag, `OMX_LAUNCH_TMUX=1`, or `OMX_LAUNCH_MODE=tmux`.
- Force native: `--no-tmux`, `OMX_LAUNCH_NO_TMUX=1`, `OMX_NO_TMUX=1`, or `OMX_LAUNCH_MODE=native`.
- Inside an attached tmux session, native launch retains tmux HUD split and cleanup to preserve operator UX.
