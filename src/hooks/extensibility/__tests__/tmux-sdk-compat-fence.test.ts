import { describe, it, beforeEach, afterEach } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";

import { createHookPluginSdk } from "../sdk.js";
import type { HookEventEnvelope } from "../types.js";

async function writeTeamState(cwd: string, payload: Record<string, unknown>) {
  const stateDir = join(cwd, ".omx", "state");
  await mkdir(stateDir, { recursive: true });
  await writeFile(join(stateDir, "team-state.json"), JSON.stringify(payload));
}

function baseEvent(): HookEventEnvelope {
  return {
    schema_version: "1",
    event: "turn-complete",
    timestamp: new Date().toISOString(),
    source: "native",
    context: {},
  };
}

describe("hook sdk: tmux compatibility fence", () => {
  const originalCwd = process.cwd();
  const originalEnv = { ...process.env } as NodeJS.ProcessEnv;
  let wd: string;

  beforeEach(async () => {
    wd = await mkdtemp(join(tmpdir(), "omx-hook-fence-"));
    process.chdir(wd);
    delete process.env.OMX_NO_TMUX;
    delete process.env.OMX_LAUNCH_NO_TMUX;
    delete process.env.OMX_LAUNCH_MODE;
    delete process.env.TMUX;
    delete process.env.TMUX_PANE;
  });

  afterEach(async () => {
    process.chdir(originalCwd);
    process.env = { ...originalEnv } as NodeJS.ProcessEnv;
    await rm(wd, { recursive: true, force: true });
  });

  it("sendKeys returns no_backend when OMX_NO_TMUX=1", async () => {
    process.env.OMX_NO_TMUX = "1";
    const sdk = createHookPluginSdk({ cwd: wd, pluginName: "demo", event: baseEvent(), sideEffectsEnabled: true });
    const res = await sdk.tmux.sendKeys({ text: "echo hi", submit: false });
    assert.equal(res.ok, false);
    assert.equal(res.reason, "no_backend");
  });

  it("sendKeys returns no_backend when team-state declares native_equivalent/no_tmux", async () => {
    await writeTeamState(wd, { layout_mode: "native_equivalent", no_tmux: true });
    const sdk = createHookPluginSdk({ cwd: wd, pluginName: "demo", event: baseEvent(), sideEffectsEnabled: true });
    const res = await sdk.tmux.sendKeys({ text: "echo hi", submit: false });
    assert.equal(res.ok, false);
    assert.equal(res.reason, "no_backend");
  });
});

