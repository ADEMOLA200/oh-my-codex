# Autopilot runtime-controller release gate

This gate turns the approved milestone review into a concrete sign-off checklist for the autopilot-only runtime-controller upgrade.

## Review summary
- The current repo still exposes autopilot as a staged pipeline in `skills/autopilot/SKILL.md` and `src/pipeline/orchestrator.ts`.
- Existing brownfield seams are strong enough for an incremental controller upgrade because planning, team execution, Ralph verification, mode state, and team runtime already exist as reusable boundaries.
- The highest-risk failure remains dual truth between a new controller ledger and the legacy `pipeline_stage_results` compatibility view.

## Required sign-off matrix

| ID | Requirement | Evidence to collect |
| --- | --- | --- |
| A1 | `$autopilot` entry stays stable | smoke test invoking the preserved autopilot entry path |
| A2 | One authoritative controller entrypoint exists | unit/integration coverage proving compatibility wrappers delegate to a single controller owner |
| A3 | Controller decisions are explainable | persisted decision-log assertions including action, trigger, and evidence summary |
| A4 | Pipeline state is derived compatibility only | tests showing controller state and compatibility phase outputs stay coherent |
| A5 | Team handoff reuses existing staffing/runtime seams | integration coverage around `src/team/followup-planner.ts` and `src/team/runtime.ts` outputs |
| A6 | Ralph handoff respects exclusive-mode rules | tests proving autopilot and Ralph do not overlap as active modes |
| A7 | Resume/cancel remains intact | smoke coverage for start → persist → resume/cancel lifecycle |
| A8 | Known limits are documented | docs update that names controller boundaries and non-goals |

## Structural-fit checks
- `src/modes/base.ts` remains the lifecycle source for autopilot activity and exclusivity.
- `src/pipeline/stages/ralplan.ts` remains a planning adapter, not the controller brain.
- `src/pipeline/stages/team-exec.ts` continues to emit team execution descriptors instead of inventing a second team runtime.
- `src/pipeline/stages/ralph-verify.ts` remains a verification adapter and only becomes active through an explicit controller decision.
- `src/team/followup-planner.ts` remains the source for launch hints, staffing, and verification-lane guidance.
- `src/team/runtime.ts` remains the owner of mailbox/dispatch/task lifecycle behavior.

## Review notes for final integration
- Do not ship a controller implementation that leaves `runPipeline(...)` and the controller each owning separate progress state.
- Do not document the milestone as a new user-facing command; the visible contract is still `$autopilot`.
- Do not bypass existing team/runtime or mode-state primitives just to make the controller feel more novel.
- If the controller needs richer logs, add them to autopilot-owned state first and mirror only the compatibility fields required by existing tooling.
