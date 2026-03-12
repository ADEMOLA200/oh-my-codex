# Team Layout Runtime Metadata (native surface)

This document describes the state emitted by the native team runtime that the HUD and other surfaces consume.

Example (rendered by `TeamLayout::team_mode_state_json()`):

```json
{
  "active": true,
  "team_name": "<name>",
  "agent_count": 3,
  "current_phase": "team-exec",
  "worker_launch_mode": "prompt",
  "runtime_session_id": "<uuid>",
  "tmux_session": null,
  "layout_mode": "native_equivalent",
  "layout_density": "balanced",
  "layout_signature": "leader-primary | workers-secondary-stack | hud-footer",
  "layout_columns": 144,
  "layout_rows": 48,
  "hud_mode": "watch",
  "no_tmux": true
}
```

Key fields:
- `layout_mode`: `native_equivalent` indicates a tmux-identical UX expressed via native surfaces (tmux optional).
- `no_tmux`: `true` when operating without tmux. When `false`, tmux flows are compatibility/opt-in only.
- `layout_signature`: stable semantic layout for the operator view; used by HUD rendering.
- `hud_mode`: `watch` for continuous statusline updates; other presets map to different verbosity.

Consumers:
- `omx hud` renders a native line/watch using this snapshot.
- Compatibility flows (`omx tmux-hook`) should treat these fields as authoritative for layout semantics, not as pane directives.

Notes:
- Keep this schema stable; add fields behind defaults and update docs/help when behavior changes.
- The tmux session name (if any) remains optional metadata and should not be relied on in native-first integrations.
