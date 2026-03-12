import { describe, it, beforeEach, afterEach } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";

// Import relative to this test file so it resolves to dist/notifications/tmux.js after build
import {
  getCurrentTmuxSession,
  getCurrentTmuxPaneId,
  getTeamTmuxSessions,
  captureTmuxPane,
  formatTmuxInfo,
} from "../tmux.js";

async function writeTeamState(cwd: string, payload: Record<string, unknown>) {
  const stateDir = join(cwd, ".omx", "state");
  await mkdir(stateDir, { recursive: true });
  await writeFile(join(stateDir, "team-state.json"), JSON.stringify(payload));
}

describe("notifications: tmux compatibility fence", () => {
  const originalCwd = process.cwd();
  const originalEnv = { ...process.env };
  let wd: string;

  beforeEach(async () => {
    wd = await mkdtemp(join(tmpdir(), "omx-notify-fence-"));
    process.chdir(wd);
    // Clean gating env by default
    delete process.env.OMX_NO_TMUX;
    delete process.env.OMX_LAUNCH_NO_TMUX;
    delete process.env.OMX_LAUNCH_MODE;
    delete process.env.TMUX;
    delete process.env.TMUX_PANE;
  });

  afterEach(async () => {
    process.chdir(originalCwd);
    process.env = { ...originalEnv };
    await rm(wd, { recursive: true, force: true });
  });

  it("returns null/empty when OMX_NO_TMUX=1 (env fence)", async () => {
    process.env.OMX_NO_TMUX = "1";

    assert.equal(getCurrentTmuxSession(), null);
    assert.equal(getCurrentTmuxPaneId(), null);
    assert.equal(formatTmuxInfo(), null);
    assert.deepEqual(getTeamTmuxSessions("demo"), []);
    assert.equal(captureTmuxPane("%1", 5), null);
  });

  it("returns null/empty when team-state declares native_equivalent/no_tmux (state fence)", async () => {
    await writeTeamState(wd, { layout_mode: "native_equivalent", no_tmux: true });

    assert.equal(getCurrentTmuxSession(), null);
    assert.equal(getCurrentTmuxPaneId(), null);
    assert.equal(formatTmuxInfo(), null);
    assert.deepEqual(getTeamTmuxSessions("demo"), []);
    assert.equal(captureTmuxPane("%1", 5), null);
  });
});

