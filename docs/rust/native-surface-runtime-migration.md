# Native Surface Runtime Migration

Status: in-progress (2026-03-12)
Owner: Team broader-native-tmux-identical

## Objective
Deliver a tmux-identical operator experience using a native runtime surface while keeping tmux as a compatibility backend — not the default runtime authority.

This migration aligns launch, HUD, team layout/runtime metadata, hooks/notifications targeting, and docs/help so they tell one consistent truth.

## Inputs (authoritative)
- Context: `.omx/context/broader-native-tmux-identical-migration-20260312T043751Z.md`
- Context: `.omx/context/native-pane-runtime-omx-20260312T035817Z.md`
- PRD: `.omx/plans/prd-native-pane-runtime-omx.md`
- Test Spec: `.omx/plans/test-spec-native-pane-runtime-omx.md`
- Parity gap PRD/spec: `.omx/plans/prd-omx-shell-parity-gap-closure.md`, `.omx/plans/test-spec-omx-shell-parity-gap-closure.md`
- Implementation plan: `.omx/plans/impl-omx-shell-parity-gap-closure.md`

## Scope for this slice (docs-first)
- Review current help/doc surfaces for truthfulness vs. implemented behavior.
- Update/author docs to reflect the broader native migration and compatibility fence for tmux-backed flows.
- Call out what is “surface-parity” (visibility/UX) vs. “operational parity” (live orchestration semantics).

Non-goals for this slice: landing the remaining detached-session live orchestration in Rust (tracked separately in code/PRD).

## Operator experience contract
- Normal path: native prompt-mode workers (`omx team …`) and HUD (`omx hud --watch`) — works with or without tmux.
- Compatibility path: tmux-specific flows (e.g., `omx tmux-hook …`) when explicitly requested or integrated.
- Help output and fixtures must match shipped behavior. Keep `crates/omx-cli/src/lib.rs` and `src/compat/fixtures/help.stdout.txt` in lockstep.

## Verification
- Rust: `cargo test --workspace`
- TypeScript: `npx tsc --noEmit`
- Lint (biome): `npm run -s lint`
- Focused parity tests: native launch parity tests under `crates/omx-cli/tests/*` (inside-tmux vs detached-session vs no-tmux)

## Follow-ups (next slices)
- Broaden validation of the detached-session parity path (more scenarios, failure-rollbacks under load).
- Hook/notification targeting against abstract surfaces (not tmux panes).
- Team runtime metadata refinements and docs/screens for native layouts.

## Quick usage
```bash
# Native team run (no tmux required)
omx team 3:executor "short scoped task"

# HUD statusline (inline or in a split when inside tmux)
omx hud --watch

# Optional compatibility workflow
omx tmux-hook status
```
