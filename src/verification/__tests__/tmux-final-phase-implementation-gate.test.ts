import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

function repoRoot(): string {
  return join(process.cwd());
}

function mustExist(path: string): void {
  assert.equal(existsSync(path), true, `missing required artifact: ${path}`);
}

function read(path: string): string {
  mustExist(path);
  return readFileSync(path, 'utf-8');
}

describe('tmux final phase implementation release gate artifacts', () => {
  it('requires the final phase prd and test spec to lock the Phase 3 cutover before Phase 4 deletion', () => {
    const root = repoRoot();
    const prd = read(join(root, '.omx', 'plans', 'prd-tmux-final-phase-implementation.md'));
    const spec = read(join(root, '.omx', 'plans', 'test-spec-tmux-final-phase-implementation.md'));

    assert.match(prd, /Choose Option A\./);
    assert.match(prd, /Step 1 — Finish the remaining Phase 3 authority cutover/);
    assert.match(prd, /Step 2 — Close the Phase 3 verification gap/);
    assert.match(prd, /Step 3 — Execute Phase 4 deletion as a dedicated cleanup slice/);
    assert.match(prd, /Step 4 — Final verification and release-readiness signoff/);
    assert.match(prd, /omx sparkshell --tmux-pane/i);
    assert.match(prd, /operator-only compat surfaces/i);
    assert.match(prd, /Available Agent Types Roster/i);
    assert.match(prd, /`architect`/);
    assert.match(prd, /`executor`/);
    assert.match(prd, /`debugger`/);

    assert.match(spec, /Default product runtime works without tmux authority/i);
    assert.match(spec, /Team\/runtime inspection and cleanup rely on state\/transcript\/session-native evidence/i);
    assert.match(spec, /Phase 4 removes tmux runtime debt without breaking native-first UX/i);
    assert.match(spec, /Phase 4 deletion complete or residual exceptions explicitly approved/i);
  });

  it('requires docs and regression suites to preserve native-first authority and operator-only tmux inspection', () => {
    const root = repoRoot();
    const migrationDoc = read(join(root, 'docs', 'rust', 'native-surface-runtime-migration.md'));
    const teamTests = read(join(root, 'src', 'cli', '__tests__', 'team.test.ts'));
    const doctorTests = read(join(root, 'src', 'cli', '__tests__', 'doctor-team.test.ts'));
    const runtimeTests = read(join(root, 'src', 'team', '__tests__', 'runtime.test.ts'));
    const watcherTests = read(join(root, 'src', 'hooks', '__tests__', 'notify-fallback-watcher.test.ts'));
    const leaderNudgeTests = read(join(root, 'src', 'hooks', '__tests__', 'notify-hook-team-leader-nudge.test.ts'));
    const dispatchHookTests = read(join(root, 'src', 'hooks', '__tests__', 'notify-hook-team-dispatch.test.ts'));

    assert.match(migrationDoc, /operator-only compatibility inspection aid until Phase 4/i);
    assert.match(migrationDoc, /team inspection guidance prefers heartbeat\/status\/task\/mailbox\/monitor-snapshot paths before pane-tail inspection/i);
    assert.match(migrationDoc, /any remaining tmux-only surface is either compat-only or explicitly deferred to a later deletion\/removal phase/i);

    assert.match(teamTests, /inspect_state_worker-1:/);
    assert.match(teamTests, /inspect_worker_status_path_worker-1:/);
    assert.match(teamTests, /inspect_next: omx sparkshell --tmux-pane/);
    assert.match(teamTests, /sparkshell_hint: omx sparkshell --tmux-pane <pane-id> --tail-lines 400/);

    assert.match(doctorTests, /does not emit resume_blocker for prompt-mode teams without tmux sessions/);
    assert.match(runtimeTests, /startTeam supports prompt launch mode without tmux and pipes trigger text via stdin/);
    assert.match(runtimeTests, /shutdownTeam force-kills prompt workers that ignore SIGTERM/);
    assert.match(watcherTests, /keeps Ralph continue steer compat-only when OMX_NO_TMUX=1/);
    assert.match(leaderNudgeTests, /treats tmux leader nudges as compat-only when OMX_NO_TMUX=1/);
    assert.match(dispatchHookTests, /keeps compat-only team-dispatch pending when OMX_NO_TMUX=1/);
  });
});
