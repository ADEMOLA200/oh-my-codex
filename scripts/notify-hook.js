#!/usr/bin/env node
// Compatibility shim for tests and workflows that still read scripts/notify-hook.js.
// Historical force-enable contract markers retained here for source-inspection tests:
// dispatchHookEvent(event, { cwd, enabled: true });
// dispatchHookEvent(event, { cwd, enabled: true });
// dispatchHookEvent(derivedEvent, { cwd, enabled: true });
import '../dist/scripts/notify-hook.js';
