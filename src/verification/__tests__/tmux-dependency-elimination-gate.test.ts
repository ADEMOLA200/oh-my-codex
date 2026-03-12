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

describe('tmux dependency elimination release gate artifacts', () => {
  it('requires phase plan artifacts and frozen acceptance language for phases 1 through 3', () => {
    const root = repoRoot();
    const prdPath = join(root, '.omx', 'plans', 'prd-tmux-dependency-elimination.md');
    const specPath = join(root, '.omx', 'plans', 'test-spec-tmux-dependency-elimination.md');
    const prd = read(prdPath);
    const spec = read(specPath);

    assert.match(prd, /Phase 1 — Establish one runtime boundary/);
    assert.match(prd, /Phase 2 — Build native control-plane equivalents/);
    assert.match(prd, /Phase 3 — Cut over product paths and freeze tmux/);
    assert.match(prd, /authority map exists for launch, inspect, nudge, recover, and shutdown/i);
    assert.match(spec, /Product runtime starts and operates without tmux installed/i);
    assert.match(spec, /Team lifecycle works through native\/session control/i);
    assert.match(spec, /UX parity: launch, inspect, nudge, recover, shutdown/i);
  });

  it('requires native-first authority docs and tmux compat quarantine docs', () => {
    const root = repoRoot();
    const runtimeMetadata = read(join(root, 'docs', 'rust', 'team-layout-runtime-metadata.md'));
    const compatQuarantine = read(join(root, 'docs', 'compat', 'notify-tmux-quarantine.md'));
    const hooksDoc = read(join(root, 'docs', 'hooks-extension.md'));

    assert.match(runtimeMetadata, /"layout_mode": "native_equivalent"/);
    assert.match(runtimeMetadata, /"no_tmux": true/);
    assert.match(runtimeMetadata, /tmux session name .* optional metadata/i);

    assert.match(compatQuarantine, /tmux\.ts — COMPAT-ONLY/);
    assert.match(compatQuarantine, /sdk\.ts — COMPAT-ONLY/);
    assert.match(compatQuarantine, /DISABLED by default\. Enable explicitly with `OMX_COMPAT_TMUX=1\|true\|yes`/);
    assert.match(compatQuarantine, /Force-disable with `OMX_NO_TMUX=1`/);

    assert.match(hooksDoc, /tmux backend \(compat-only\)/i);
    assert.match(hooksDoc, /native-first/i);
    assert.match(hooksDoc, /Treat tmux operations .* as optional compatibility helpers/i);
  });

  it('requires regression tests that prove native-first runtime and tmux compat fences', () => {
    const root = repoRoot();
    const runtimeTests = read(join(root, 'src', 'team', '__tests__', 'runtime.test.ts'));
    const sdkFenceTests = read(join(root, 'src', 'hooks', 'extensibility', '__tests__', 'tmux-sdk-compat-fence.test.ts'));
    const notificationFenceTests = read(join(root, 'src', 'notifications', '__tests__', 'tmux-compat-fence.test.ts'));
    const compatGateTests = read(join(root, 'src', 'compat', '__tests__', 'tmux-optin-gate.test.ts'));
    const doctorTests = read(join(root, 'src', 'cli', '__tests__', 'doctor-team.test.ts'));
    const sparkshellTests = read(join(root, 'src', 'cli', '__tests__', 'sparkshell-cli.test.ts'));

    assert.match(runtimeTests, /startTeam supports prompt launch mode without tmux and pipes trigger text via stdin/);
    assert.match(runtimeTests, /shutdownTeam force-kills prompt workers that ignore SIGTERM/);
    assert.match(runtimeTests, /monitorTeam emits worker_state_changed, worker_idle, and task_completed events based on transitions/);
    assert.match(runtimeTests, /sendWorkerMessage hook-preferred path keeps leader mailbox native-first even when leader pane exists/);
    assert.match(runtimeTests, /sendWorkerMessage hook-preferred path uses tmux leader nudge only when compat tmux is explicitly enabled/);

    assert.match(sdkFenceTests, /sendKeys returns no_backend when OMX_NO_TMUX=1/);
    assert.match(sdkFenceTests, /native_equivalent\/no_tmux/);
    assert.match(notificationFenceTests, /returns null\/empty when OMX_NO_TMUX=1/);
    assert.match(notificationFenceTests, /native_equivalent\/no_tmux/);
    assert.match(compatGateTests, /does not require OMX_COMPAT_TMUX when tmux is present/);
    assert.match(doctorTests, /does not emit resume_blocker for prompt-mode teams without tmux sessions/);
    assert.match(sparkshellTests, /parseSparkShellFallbackInvocation/);
    assert.match(sparkshellTests, /--tmux-pane/);
    assert.match(sparkshellTests, /adaptive backend for qualifying read-only explore tasks/i);
  });
});
