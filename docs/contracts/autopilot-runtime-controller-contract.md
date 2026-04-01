# Autopilot runtime-controller contract

This document records the approved autopilot-only runtime-controller milestone contract from `/home/doyun/Desktop/auRun/.omx/plans/prd-autopilot-runtime-controller.md` and `/home/doyun/Desktop/auRun/.omx/plans/test-spec-autopilot-runtime-controller.md` against the current OMX seams.

## Scope
- Keep the visible `$autopilot` entry contract stable.
- Change autopilot internals only.
- Reuse existing runtime, team, and mode-state seams before adding new orchestration layers.
- Treat this contract as the structural-fit checklist for implementation and sign-off.

## Brownfield evidence snapshot

| Concern | Current seam | Evidence |
| --- | --- | --- |
| Visible autopilot UX already exists at the skill layer | `skills/autopilot/SKILL.md` | The skill still describes autopilot as a staged execution workflow, so the milestone must preserve the shell while upgrading the engine. |
| Autopilot state persistence already exists | `src/modes/base.ts` | `autopilot` is an exclusive mode, so controller + Ralph handoff must respect the shared lifecycle rules. |
| Current autopilot engine is sequential | `src/pipeline/orchestrator.ts` | `runPipeline(...)` persists `stage:<name>` progress and executes stages in fixed order. |
| Planning/team/verification are already adapter-shaped | `src/pipeline/stages/ralplan.ts`, `src/pipeline/stages/team-exec.ts`, `src/pipeline/stages/ralph-verify.ts` | Existing stages already return descriptors/artifacts instead of owning a richer controller ledger. |
| Team staffing and launch hints already exist | `src/team/followup-planner.ts` | Launch hints and verification lanes are already derived from bounded task routing. |
| Team runtime/state machinery already exists | `src/team/runtime.ts` | Team runtime already owns mailbox/dispatch/task lifecycle and should be reused instead of recreated inside autopilot. |

## Required milestone shape

### 1. Stable external contract
- `$autopilot <task>` remains the user-facing entry.
- Existing high-level expectations remain recognizable: autopilot still plans, executes, verifies, and completes.
- Compatibility readers such as HUD/mode-state surfaces must keep seeing coherent autopilot progress.

### 2. One authoritative controller state
- The milestone must introduce one canonical autopilot controller state/decision source.
- Any pipeline stage progress that remains should be a derived compatibility view, not a competing source of truth.
- Decision logs must include at least: selected action, trigger, and evidence/rationale summary.

### 3. Existing seams stay first-class
- `src/modes/base.ts` remains the lifecycle/state boundary for `autopilot` mode activity.
- `src/pipeline/*` remains a compatibility shell/fallback surface, not the conceptual owner of adaptive decisions.
- `src/team/followup-planner.ts` and `src/team/runtime.ts` remain the execution/runtime integration seams.
- Ralph verification remains an adapter path, not a parallel always-on owner.

### 4. Mode-conflict safety
- `autopilot` and `ralph` must not overlap as active exclusive modes.
- Any Ralph verification handoff must be explicit: finish or suspend autopilot ownership before Ralph becomes active.
- Resume/cancel behavior must keep using the shared mode-state contract so existing tooling continues to work.

### 5. Team-ready execution handoff
- Bootstrap launch still uses the current `omx team N:role ...` surface.
- Richer staffing remains documented as post-launch lane allocation guidance, matching `src/team/followup-planner.ts`.
- Autopilot-scoped instructions must stay bounded to autopilot non-goals; no stealth redesign of `team` or `ralph`.

## Structural-fit review notes
- **Current gap:** `skills/autopilot/SKILL.md` still documents a rigid staged pipeline. The milestone documentation should explain controller-style decision making without changing the visible entry UX.
- **Current gap:** `src/pipeline/orchestrator.ts` is currently linear and stage-index driven. A controller implementation must prevent dual truth between controller state and `pipeline_stage_results`.
- **Current strength:** existing stage adapters are already thin descriptor producers, which makes them viable controller actions instead of logic owners.
- **Current strength:** follow-up staffing/verification planning is already reusable and should remain the path for team + Ralph handoff details.

## Sign-off evidence required
- Unit coverage for controller state transitions, decision policy, entrypoint ownership, and mode-conflict guards.
- Integration coverage showing controller compatibility with pipeline state, team runtime/follow-up planning, and Ralph handoff behavior.
- E2E smoke coverage showing preserved `$autopilot` entry, adaptive verification, resume/cancel integrity, and directly executable team handoff artifacts.
- Documentation updates that describe the controller contract and known limitations without renaming the user-facing skill.
