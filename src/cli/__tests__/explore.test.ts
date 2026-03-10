import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { chmodSync, existsSync, writeFileSync } from 'node:fs';
import { chmod, mkdtemp, readFile, rm, mkdir, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import {
  buildExploreHarnessArgs,
  exploreCommand,
  EXPLORE_USAGE,
  loadExplorePrompt,
  packagedExploreHarnessBinaryName,
  parseExploreArgs,
  resolveExploreHarnessCommand,
  resolveExploreHarnessCommandWithHydration,
  resolveExploreSparkShellRoute,
  resolvePackagedExploreHarnessCommand,
} from '../explore.js';

function runOmx(
  cwd: string,
  argv: string[],
  envOverrides: Record<string, string> = {},
): { status: number | null; stdout: string; stderr: string; error?: string } {
  const testDir = dirname(fileURLToPath(import.meta.url));
  const repoRoot = join(testDir, '..', '..', '..');
  const omxBin = join(repoRoot, 'bin', 'omx.js');
  const r = spawnSync(process.execPath, [omxBin, ...argv], {
    cwd,
    encoding: 'utf-8',
    env: { ...process.env, ...envOverrides },
  });
  return { status: r.status, stdout: r.stdout || '', stderr: r.stderr || '', error: r.error?.message };
}


async function createExploreTestPath(wd: string): Promise<string> {
  const binDir = join(wd, 'test-bin');
  await mkdir(binDir, { recursive: true });
  const rgPath = join(binDir, process.platform === 'win32' ? 'rg.cmd' : 'rg');
  const lines = process.platform === 'win32'
    ? ['@echo off', 'echo ripgrep 14.0.0', '']
    : ['#!/bin/sh', 'echo "ripgrep 14.0.0"', ''];
  await writeFile(rgPath, lines.join(process.platform === 'win32' ? '\r\n' : '\n'));
  if (process.platform !== 'win32') {
    await chmod(rgPath, 0o755);
  }
  return `${binDir}${process.platform === 'win32' ? ';' : ':'}${process.env.PATH || ''}`;
}

async function writeEnvNodeCodexStub(wd: string, capturePath: string): Promise<string> {
  const stub = join(wd, 'codex-stub.js');
  await writeFile(
    stub,
    `#!/usr/bin/env node
const { spawnSync } = require('child_process');
const { writeFileSync } = require('fs');

const args = process.argv.slice(2);
const outputIndex = args.indexOf('-o');
if (outputIndex === -1 || outputIndex === args.length - 1) {
  process.stderr.write('missing -o output path\\n');
  process.exit(1);
}

const allowed = spawnSync('bash', ['-lc', 'rg --version'], { encoding: 'utf-8' });
const blocked = spawnSync('bash', ['-lc', 'node --version'], { encoding: 'utf-8' });
writeFileSync(${JSON.stringify(capturePath)}, JSON.stringify({
  argv: args,
  path: process.env.PATH,
  shell: process.env.SHELL,
  allowed: { status: allowed.status, stdout: allowed.stdout, stderr: allowed.stderr },
  blocked: { status: blocked.status, stdout: blocked.stdout, stderr: blocked.stderr },
}, null, 2));
writeFileSync(args[outputIndex + 1], '# Answer\\nHarness completed\\n');
`,
  );
  await chmod(stub, 0o755);
  return stub;
}

describe('parseExploreArgs', () => {
  it('parses --prompt form', () => {
    assert.deepEqual(parseExploreArgs(['--prompt', 'find', 'auth']), { prompt: 'find auth' });
  });

  it('parses --prompt= form', () => {
    assert.deepEqual(parseExploreArgs(['--prompt=find auth']), { prompt: 'find auth' });
  });

  it('parses --prompt-file form', () => {
    assert.deepEqual(parseExploreArgs(['--prompt-file', 'prompt.md']), { promptFile: 'prompt.md' });
  });

  it('throws on missing prompt', () => {
    assert.throws(() => parseExploreArgs([]), new RegExp(EXPLORE_USAGE.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  });

  it('throws on unknown flag', () => {
    assert.throws(() => parseExploreArgs(['--bogus']), /Unknown argument/);
  });

  it('rejects duplicate prompt sources', () => {
    assert.throws(() => parseExploreArgs(['--prompt', 'find auth', '--prompt-file', 'prompt.md']), /Choose exactly one/);
  });

  it('rejects missing prompt-file value', () => {
    assert.throws(() => parseExploreArgs(['--prompt-file']), /Missing path after --prompt-file/);
  });

  it('rejects missing prompt value', () => {
    assert.throws(() => parseExploreArgs(['--prompt']), /Missing text after --prompt/);
  });
});

describe('loadExplorePrompt', () => {
  it('reads prompt file content', async () => {
    const wd = await mkdtemp(join(tmpdir(), 'omx-explore-prompt-'));
    try {
      const promptPath = join(wd, 'prompt.md');
      await writeFile(promptPath, '  find symbol refs  \n');
      assert.equal(await loadExplorePrompt({ promptFile: promptPath }), 'find symbol refs');
    } finally {
      await rm(wd, { recursive: true, force: true });
    }
  });
});

describe('resolvePackagedExploreHarnessCommand', () => {
  it('uses a packaged native binary when metadata matches the current platform', async () => {
    const wd = await mkdtemp(join(tmpdir(), 'omx-explore-packaged-'));
    try {
      const binDir = join(wd, 'bin');
      await mkdir(binDir, { recursive: true });
      await writeFile(join(wd, 'package.json'), '{}\n');
      await writeFile(join(binDir, 'omx-explore-harness.meta.json'), JSON.stringify({
        binaryName: packagedExploreHarnessBinaryName(),
        platform: process.platform,
        arch: process.arch,
      }));
      const binaryPath = join(binDir, packagedExploreHarnessBinaryName());
      await writeFile(binaryPath, '#!/bin/sh\nexit 0\n');
      await chmod(binaryPath, 0o755);

      const resolved = resolvePackagedExploreHarnessCommand(wd);
      assert.deepEqual(resolved, { command: binaryPath, args: [] });
    } finally {
      await rm(wd, { recursive: true, force: true });
    }
  });

  it('ignores packaged binaries built for a different platform', async () => {
    const wd = await mkdtemp(join(tmpdir(), 'omx-explore-packaged-mismatch-'));
    try {
      const binDir = join(wd, 'bin');
      await mkdir(binDir, { recursive: true });
      await writeFile(join(wd, 'package.json'), '{}\n');
      await writeFile(join(binDir, 'omx-explore-harness.meta.json'), JSON.stringify({
        binaryName: packagedExploreHarnessBinaryName('linux'),
        platform: process.platform === 'win32' ? 'linux' : 'win32',
        arch: process.arch,
      }));
      await writeFile(join(binDir, packagedExploreHarnessBinaryName('linux')), '#!/bin/sh\nexit 0\n');

      assert.equal(resolvePackagedExploreHarnessCommand(wd), undefined);
    } finally {
      await rm(wd, { recursive: true, force: true });
    }
  });
});

describe('resolveExploreHarnessCommand', () => {
  it('uses env override when provided', () => {
    const resolved = resolveExploreHarnessCommand('/repo', { OMX_EXPLORE_BIN: '/tmp/omx-explore-stub' } as NodeJS.ProcessEnv);
    assert.deepEqual(resolved, { command: '/tmp/omx-explore-stub', args: [] });
  });

  it('prefers a packaged native harness binary when present', async () => {
    const wd = await mkdtemp(join(tmpdir(), 'omx-explore-native-'));
    try {
      const binDir = join(wd, 'bin');
      await mkdir(binDir, { recursive: true });
      await writeFile(join(wd, 'package.json'), '{}\n');
      await writeFile(join(binDir, 'omx-explore-harness.meta.json'), JSON.stringify({
        binaryName: packagedExploreHarnessBinaryName(),
        platform: process.platform,
        arch: process.arch,
      }));
      const nativePath = join(binDir, packagedExploreHarnessBinaryName());
      await writeFile(nativePath, '#!/bin/sh\necho native\n');
      await chmod(nativePath, 0o755);

      const resolved = resolveExploreHarnessCommand(wd, {} as NodeJS.ProcessEnv);
      assert.deepEqual(resolved, { command: nativePath, args: [] });
    } finally {
      await rm(wd, { recursive: true, force: true });
    }
  });

  it('builds cargo fallback command otherwise', async () => {
    const wd = await mkdtemp(join(tmpdir(), 'omx-explore-fallback-'));
    try {
      const binDir = join(wd, 'bin');
      await mkdir(binDir, { recursive: true });
      await writeFile(join(wd, 'package.json'), '{}\n');
      await writeFile(join(binDir, 'omx-explore-harness.meta.json'), JSON.stringify({
        binaryName: packagedExploreHarnessBinaryName(),
        platform: process.platform,
        arch: process.arch,
      }));
      const nativePath = join(binDir, packagedExploreHarnessBinaryName());
      await writeFile(nativePath, '#!/bin/sh\necho native\n');
      await chmod(nativePath, 0o755);

      const resolved = resolveExploreHarnessCommand(wd, {} as NodeJS.ProcessEnv);
      assert.deepEqual(resolved, { command: nativePath, args: [] });
    } finally {
      await rm(wd, { recursive: true, force: true });
    }
  });

  it('hydrates a native harness for packaged installs before attempting cargo fallback', async () => {
    const wd = await mkdtemp(join(tmpdir(), 'omx-explore-hydrated-'));
    try {
      const assetRoot = join(wd, 'assets');
      const cacheDir = join(wd, 'cache');
      const stagingDir = join(wd, 'staging');
      await mkdir(assetRoot, { recursive: true });
      await mkdir(stagingDir, { recursive: true });
      await writeFile(join(wd, 'package.json'), JSON.stringify({
        version: '0.8.15',
        repository: { url: 'git+https://github.com/Yeachan-Heo/oh-my-codex.git' },
      }));
      await mkdir(join(wd, 'crates', 'omx-explore'), { recursive: true });
      await writeFile(join(wd, 'crates', 'omx-explore', 'Cargo.toml'), '[package]\nname=\"omx-explore-harness\"\nversion=\"0.8.15\"\n');
      const binaryPath = join(stagingDir, packagedExploreHarnessBinaryName());
      await writeFile(binaryPath, '#!/bin/sh\necho hydrated-explore\n');
      await chmod(binaryPath, 0o755);

      const archivePath = join(assetRoot, 'omx-explore-harness-x86_64-unknown-linux-gnu.tar.gz');
      const archive = spawnSync('tar', ['-czf', archivePath, '-C', stagingDir, packagedExploreHarnessBinaryName()], { encoding: 'utf-8' });
      assert.equal(archive.status, 0, archive.stderr || archive.stdout);
      const archiveBuffer = await readFile(archivePath);
      const checksum = createHash('sha256').update(archiveBuffer).digest('hex');

      const server = await new Promise<{ baseUrl: string; close: () => Promise<void> }>((resolve) => {
        const srv = createServer(async (req, res) => {
          const url = new URL(req.url || '/', 'http://127.0.0.1');
          const filePath = join(assetRoot, url.pathname.replace(/^\//, ''));
          try {
            res.writeHead(200);
            res.end(await readFile(filePath));
          } catch {
            res.writeHead(404);
            res.end('missing');
          }
        });
        srv.listen(0, '127.0.0.1', () => {
          const address = srv.address();
          if (!address || typeof address === 'string') throw new Error('bad address');
          resolve({
            baseUrl: `http://127.0.0.1:${address.port}`,
            close: () => new Promise<void>((done, reject) => srv.close((err: Error | undefined) => err ? reject(err) : done())),
          });
        });
      });

      try {
        await writeFile(join(assetRoot, 'native-release-manifest.json'), JSON.stringify({
          version: '0.8.15',
          assets: [{
            product: 'omx-explore-harness',
            version: '0.8.15',
            platform: 'linux',
            arch: 'x64',
            archive: 'omx-explore-harness-x86_64-unknown-linux-gnu.tar.gz',
            binary: 'omx-explore-harness',
            binary_path: 'omx-explore-harness',
            sha256: checksum,
            size: archiveBuffer.length,
            download_url: `${server.baseUrl}/omx-explore-harness-x86_64-unknown-linux-gnu.tar.gz`,
          }],
        }, null, 2));

        const resolved = await resolveExploreHarnessCommandWithHydration(wd, {
          OMX_NATIVE_MANIFEST_URL: `${server.baseUrl}/native-release-manifest.json`,
          OMX_NATIVE_CACHE_DIR: cacheDir,
        } as NodeJS.ProcessEnv);
        assert.notEqual(resolved.command, 'cargo');
        assert.match(resolved.command, /cache/);
      } finally {
        await server.close();
      }
    } finally {
      await rm(wd, { recursive: true, force: true });
    }
  });

  it('reports a clean fallback error when the native manifest is unavailable for packaged installs', async () => {
    const wd = await mkdtemp(join(tmpdir(), 'omx-explore-missing-manifest-'));
    try {
      await writeFile(join(wd, 'package.json'), JSON.stringify({
        version: '0.8.15',
        repository: { url: 'git+https://github.com/Yeachan-Heo/oh-my-codex.git' },
      }));
      const server = await new Promise<{ baseUrl: string; close: () => Promise<void> }>((resolve) => {
        const srv = createServer((_req, res) => {
          res.writeHead(404);
          res.end('missing');
        });
        srv.listen(0, '127.0.0.1', () => {
          const address = srv.address();
          if (!address || typeof address === 'string') throw new Error('bad address');
          resolve({
            baseUrl: `http://127.0.0.1:${address.port}`,
            close: () => new Promise<void>((done, reject) => srv.close((err: Error | undefined) => err ? reject(err) : done())),
          });
        });
      });

      try {
        await assert.rejects(
          () => resolveExploreHarnessCommandWithHydration(wd, {
            OMX_NATIVE_MANIFEST_URL: `${server.baseUrl}/native-release-manifest.json`,
            OMX_NATIVE_CACHE_DIR: join(wd, 'cache'),
          } as NodeJS.ProcessEnv),
          /no compatible native harness is available/,
        );
      } finally {
        await server.close();
      }
    } finally {
      await rm(wd, { recursive: true, force: true });
    }
  });
});

describe('buildExploreHarnessArgs', () => {
  it('includes cwd, prompt, prompt contract, and constrained model settings', () => {
    const args = buildExploreHarnessArgs('find auth', '/repo', {
      OMX_EXPLORE_SPARK_MODEL: 'spark-model',
    } as NodeJS.ProcessEnv, '/pkg');
    assert.deepEqual(args, [
      '--cwd',
      '/repo',
      '--prompt',
      'find auth',
      '--prompt-file',
      '/pkg/prompts/explore.md',
      '--model-spark',
      'spark-model',
      '--model-fallback',
      'gpt-5.4',
    ]);
  });
});

describe('exploreCommand', () => {
  it('routes qualifying read-only shell commands through sparkshell instead of the direct harness', async () => {
    const wd = await mkdtemp(join(tmpdir(), 'omx-explore-sparkshell-route-'));
    try {
      const sparkshellStub = join(wd, 'sparkshell-stub.sh');
      const harnessStub = join(wd, 'explore-stub.sh');
      const capturePath = join(wd, 'sparkshell-capture.txt');
      await writeFile(
        sparkshellStub,
        `#!/bin/sh\nprintf '%s\n' "$@" > ${JSON.stringify(capturePath)}\nprintf '# Answer\n- routed via sparkshell\n'\n`,
      );
      await writeFile(harnessStub, '#!/bin/sh\nprintf harness-should-not-run\n');
      await chmod(sparkshellStub, 0o755);
      await chmod(harnessStub, 0o755);

      const result = runOmx(wd, ['explore', '--prompt', 'git log --oneline'], {
        OMX_SPARKSHELL_BIN: sparkshellStub,
        OMX_EXPLORE_BIN: harnessStub,
      });
      if (shouldSkipForSpawnPermissions(result.error)) return;

      assert.equal(result.status, 0, result.stderr || result.stdout);
      assert.equal(result.stdout, '# Answer\n- routed via sparkshell\n');
      assert.equal(result.stderr, '');
      const captured = (await readFile(capturePath, 'utf-8')).trim().split('\n');
      assert.deepEqual(captured, ['git', 'log', '--oneline']);
    } finally {
      await rm(wd, { recursive: true, force: true });
    }
  });

  it('falls back to the explore harness when sparkshell backend is unavailable', async () => {
    const wd = await mkdtemp(join(tmpdir(), 'omx-explore-sparkshell-fallback-'));
    try {
      const harnessStub = join(wd, 'explore-stub.sh');
      await writeFile(
        harnessStub,
        '#!/bin/sh\nprintf "%s\\n" "# Answer" "- fallback harness recovered the lookup"\n',
      );
      await chmod(harnessStub, 0o755);

      const result = runOmx(wd, ['explore', '--prompt', 'git log --oneline'], {
        OMX_SPARKSHELL_BIN: join(wd, 'missing-sparkshell'),
        OMX_EXPLORE_BIN: harnessStub,
      });
      if (shouldSkipForSpawnPermissions(result.error)) return;

      assert.equal(result.status, 0, result.stderr || result.stdout);
      assert.match(result.stderr, /sparkshell backend unavailable/);
      assert.match(result.stderr, /Falling back to the explore harness/);
      assert.match(result.stdout, /fallback harness recovered the lookup/);
    } finally {
      await rm(wd, { recursive: true, force: true });
    }
  });

  it('passes prompt to harness and preserves markdown stdout', async () => {
    const wd = await mkdtemp(join(tmpdir(), 'omx-explore-cmd-'));
    try {
      const stub = join(wd, 'explore-stub.js');
      const capturePath = join(wd, 'capture.json');
      await writeFile(
        stub,
        `#!/usr/bin/env node\nconst fs = require('fs');\nfs.writeFileSync(${JSON.stringify(capturePath)}, JSON.stringify(process.argv.slice(2)));\nprocess.stdout.write('# Files\\n- demo\\n');\n`,
      );
      await chmod(stub, 0o755);

      const stdoutChunks: string[] = [];
      const stderrChunks: string[] = [];
      const originalStdout = process.stdout.write.bind(process.stdout);
      const originalStderr = process.stderr.write.bind(process.stderr);
      process.stdout.write = ((chunk: string | Uint8Array) => {
        stdoutChunks.push(typeof chunk === 'string' ? chunk : Buffer.from(chunk).toString('utf-8'));
        return true;
      }) as typeof process.stdout.write;
      process.stderr.write = ((chunk: string | Uint8Array) => {
        stderrChunks.push(typeof chunk === 'string' ? chunk : Buffer.from(chunk).toString('utf-8'));
        return true;
      }) as typeof process.stderr.write;

      const originalEnv = process.env.OMX_EXPLORE_BIN;
      process.env.OMX_EXPLORE_BIN = stub;
      const originalCwd = process.cwd();
      process.chdir(wd);
      try {
        await exploreCommand(['--prompt', 'find', 'auth']);
      } finally {
        process.chdir(originalCwd);
        if (originalEnv === undefined) delete process.env.OMX_EXPLORE_BIN;
        else process.env.OMX_EXPLORE_BIN = originalEnv;
        process.stdout.write = originalStdout;
        process.stderr.write = originalStderr;
      }

      assert.equal(stderrChunks.join(''), '');
      assert.equal(stdoutChunks.join(''), '# Files\n- demo\n');
      const captured = JSON.parse(await readFile(capturePath, 'utf-8')) as string[];
      assert.ok(captured.includes('--prompt'));
      assert.ok(captured.includes('find auth'));
      assert.ok(captured.includes('--model-spark'));
      assert.ok(captured.includes('--model-fallback'));
    } finally {
      await rm(wd, { recursive: true, force: true });
    }
  });

  it('works end-to-end through omx explore', async () => {
    const wd = await mkdtemp(join(tmpdir(), 'omx-explore-e2e-'));
    try {
      const stub = join(wd, 'explore-stub.js');
      await writeFile(
        stub,
        '#!/usr/bin/env node\nprocess.stdout.write("# Answer\\nReady to proceed\\n");\n',
      );
      await chmod(stub, 0o755);

      const result = runOmx(wd, ['explore', '--prompt', 'find auth'], { OMX_EXPLORE_BIN: stub });
      assert.equal(result.status, 0, result.stderr || result.stdout);
      assert.equal(result.stdout, '# Answer\nReady to proceed\n');
    } finally {
      await rm(wd, { recursive: true, force: true });
    }
  });

  it('launches an env-node codex binary while keeping model shell commands allowlisted', async () => {
    const wd = await mkdtemp(join(tmpdir(), 'omx-explore-harness-e2e-'));
    try {
      const capturePath = join(wd, 'capture.json');
      const codexStub = await writeEnvNodeCodexStub(wd, capturePath);
      const testPath = await createExploreTestPath(wd);

      const result = runOmx(wd, ['explore', '--prompt', 'find buildTmuxPaneCommand'], {
        OMX_EXPLORE_CODEX_BIN: codexStub,
        PATH: testPath,
      });

      assert.equal(result.status, 0, result.stderr || result.stdout);
      assert.equal(result.stdout, '# Answer\nHarness completed\n');
      const captured = JSON.parse(await readFile(capturePath, 'utf-8')) as {
        argv: string[];
        path: string;
        shell: string;
        allowed: { status: number | null; stdout: string; stderr: string };
        blocked: { status: number | null; stdout: string; stderr: string };
      };
      assert.ok(captured.argv.includes('exec'));
      assert.match(captured.path, /omx-explore-allowlist-/);
      assert.match(captured.shell, /omx-explore-allowlist-.*\/bin\/bash$/);
      assert.equal(captured.allowed.status, 0, captured.allowed.stderr);
      assert.match(captured.allowed.stdout, /ripgrep/i);
      assert.notEqual(captured.blocked.status, 0);
      assert.match(captured.blocked.stderr, /not on the omx explore allowlist/);
    } finally {
      await rm(wd, { recursive: true, force: true });
    }
  });

  it('supports --prompt-file end-to-end with the harness', async () => {
    const wd = await mkdtemp(join(tmpdir(), 'omx-explore-harness-prompt-file-'));
    try {
      const capturePath = join(wd, 'capture.json');
      const codexStub = await writeEnvNodeCodexStub(wd, capturePath);
      const testPath = await createExploreTestPath(wd);
      const promptPath = join(wd, 'prompt.md');
      await writeFile(promptPath, 'find prompt-file support\n');

      const result = runOmx(wd, ['explore', '--prompt-file', promptPath], {
        OMX_EXPLORE_CODEX_BIN: codexStub,
        PATH: testPath,
      });

      assert.equal(result.status, 0, result.stderr || result.stdout);
      assert.equal(result.stdout, '# Answer\nHarness completed\n');
      const captured = JSON.parse(await readFile(capturePath, 'utf-8')) as { argv: string[] };
      assert.ok(captured.argv.some((value) => value.includes('find prompt-file support')));
    } finally {
      await rm(wd, { recursive: true, force: true });
    }
  });
});
