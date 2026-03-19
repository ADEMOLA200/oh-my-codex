#!/usr/bin/env node
import { existsSync } from 'node:fs';
const legacy = process.env.OMX_SPARKSHELL_MANIFEST;
if (legacy && !existsSync(legacy) && legacy.includes('native/omx-sparkshell/Cargo.toml')) {
  process.env.OMX_SPARKSHELL_MANIFEST = legacy.replace('native/omx-sparkshell/Cargo.toml', 'crates/omx-sparkshell/Cargo.toml');
}
await import('../dist/scripts/build-sparkshell.js');
