import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { chmodSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

// Import from built output after tsc: dist/notifications/tmux-detector.js
import { isTmuxAvailable } from '../../notifications/tmux-detector.js';
import { createHookPluginSdk } from '../../hooks/extensibility/sdk.js';

function baseEvent() {
  return {
    schema_version: '1',
    event: 'turn-complete',
    timestamp: new Date().toISOString(),
    source: 'native',
    context: {},
  } as const;
}

describe('compat: tmux availability defaults', () => {
  const originalEnv = { ...process.env } as NodeJS.ProcessEnv;
  const originalCwd = process.cwd();
  let wd: string;
  let fakeBin: string;

  beforeEach(async () => {
    wd = await mkdtemp(join(tmpdir(), 'omx-compat-tmux-optin-'));
    fakeBin = await mkdtemp(join(tmpdir(), 'omx-compat-tmux-bin-'));
    process.chdir(wd);
    delete process.env.OMX_NO_TMUX;
    delete process.env.OMX_LAUNCH_NO_TMUX;
    delete process.env.OMX_LAUNCH_MODE;
    delete process.env.TMUX;
    delete process.env.TMUX_PANE;
    const tmuxPath = join(fakeBin, 'tmux');
    await writeFile(tmuxPath, '#!/bin/sh\nexit 0\n');
    chmodSync(tmuxPath, 0o755);
    process.env.PATH = `${fakeBin}:${originalEnv.PATH ?? ''}`;
  });

  afterEach(async () => {
    process.chdir(originalCwd);
    process.env = { ...originalEnv } as NodeJS.ProcessEnv;
    await rm(wd, { recursive: true, force: true });
    await rm(fakeBin, { recursive: true, force: true });
  });

  it('does not require OMX_COMPAT_TMUX when tmux is present', async () => {
    assert.equal(isTmuxAvailable(), true);

    const sdk = createHookPluginSdk({ cwd: wd, pluginName: 'demo', event: baseEvent(), sideEffectsEnabled: true });
    const res = await sdk.tmux.sendKeys({ text: 'echo hi', submit: false });
    assert.equal(res.ok, false);
    assert.equal(res.reason, 'target_missing');
  });
});
