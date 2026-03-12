use crate::surface_runtime::{
    HudSurface, NativeBackend, SurfaceEventKind, SurfaceRuntime, TmuxBackend,
};
use omx_process::process_bridge::{CommandSpec, Platform, ProcessBridge, ProcessResult, StdioMode};
use omx_process::{
    SpawnErrorKind, build_tmux_command, build_tmux_kill_pane_command, build_tmux_pane_command,
    build_tmux_version_command,
};
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MADMAX_FLAG: &str = "--madmax";
const CODEX_BYPASS_FLAG: &str = "--dangerously-bypass-approvals-and-sandbox";
const HIGH_REASONING_FLAG: &str = "--high";
const XHIGH_REASONING_FLAG: &str = "--xhigh";
const SPARK_FLAG: &str = "--spark";
const MADMAX_SPARK_FLAG: &str = "--madmax-spark";
const CONFIG_FLAG: &str = "-c";
const REASONING_KEY: &str = "model_reasoning_effort";
const HUD_TMUX_HEIGHT_LINES: &str = "2";
const TMUX_ENV: &str = "TMUX";
const TMUX_PANE_ENV: &str = "TMUX_PANE";
const SHELL_ENV: &str = "SHELL";
// Compatibility/opt-in flags and env for tmux backend
const TMUX_OPT_IN_FLAG: &str = "--tmux";
const NO_TMUX_FLAG: &str = "--no-tmux";
const OMX_LAUNCH_TMUX_ENV: &str = "OMX_LAUNCH_TMUX";
const OMX_LAUNCH_MODE_ENV: &str = "OMX_LAUNCH_MODE"; // expected values: "tmux" | "native"
const OMX_LAUNCH_NO_TMUX_ENV: &str = "OMX_LAUNCH_NO_TMUX";
const OMX_NO_TMUX_ENV: &str = "OMX_NO_TMUX";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchExecution {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchError(String);

#[derive(Debug, Clone, PartialEq, Eq)]
struct TmuxPaneSnapshot {
    pane_id: String,
    pane_current_command: String,
    pane_start_command: String,
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LaunchError {}

#[allow(clippy::missing_errors_doc)]
pub fn run_launch(
    args: &[String],
    cwd: &Path,
    env: &BTreeMap<OsString, OsString>,
    help_output: &str,
) -> Result<LaunchExecution, LaunchError> {
    run_launch_with_stdio(args, cwd, env, help_output, StdioMode::Inherit)
}

fn run_launch_with_stdio(
    args: &[String],
    cwd: &Path,
    env: &BTreeMap<OsString, OsString>,
    help_output: &str,
    stdio_mode: StdioMode,
) -> Result<LaunchExecution, LaunchError> {
    if matches!(
        args.first().map(String::as_str),
        Some("--help" | "-h" | "help")
    ) {
        return Ok(LaunchExecution {
            stdout: help_output.as_bytes().to_vec(),
            stderr: Vec::new(),
            exit_code: 0,
        });
    }

    let bridge = ProcessBridge::new(Platform::detect(), env.clone());
    let normalized_args = normalize_launch_args(args);
    let codex_spec = build_codex_spec(&normalized_args, cwd, stdio_mode);

    let tmux_opt_in = tmux_opt_in(args, env);
    let tmux_forced_off = tmux_forced_off(args, env);

    if running_inside_tmux(env) && !tmux_forced_off {
        let session_name = current_tmux_session_name(&bridge, env)
            .unwrap_or_else(|| "attached-session".to_string());
        let runtime = SurfaceRuntime::new(TmuxBackend::new(session_name));
        return Ok(run_inside_tmux(&bridge, &codex_spec, cwd, env, &runtime));
    }

    if !tmux_forced_off && tmux_opt_in && tmux_available(&bridge) {
        let session_name = build_detached_session_name(cwd);
        let runtime = SurfaceRuntime::new(TmuxBackend::new(session_name.clone()));
        return Ok(run_detached_tmux(
            &bridge,
            &codex_spec,
            cwd,
            env,
            stdio_mode,
            &runtime,
            &session_name,
        ));
    }

    let runtime = SurfaceRuntime::new(NativeBackend::new("direct-launch"));
    let _ = runtime.event(
        runtime.leader_surface().id,
        SurfaceEventKind::FallbackDirect,
    );
    Ok(finalize_process_result(bridge.run(&codex_spec), "codex"))
}

fn build_codex_spec(args: &[String], cwd: &Path, stdio_mode: StdioMode) -> CommandSpec {
    let mut spec = CommandSpec::new("codex");
    spec.args = args.iter().map(OsString::from).collect();
    spec.cwd = Some(cwd.to_path_buf());
    spec.stdio_mode = stdio_mode;
    spec
}

fn run_inside_tmux(
    bridge: &ProcessBridge,
    codex_spec: &CommandSpec,
    cwd: &Path,
    env: &BTreeMap<OsString, OsString>,
    runtime: &SurfaceRuntime<TmuxBackend>,
) -> LaunchExecution {
    let current_pane = env_string(env, TMUX_PANE_ENV);
    for pane_id in list_hud_watch_panes(bridge, current_pane.as_deref()) {
        let _ = run_tmux_command(bridge, build_tmux_kill_pane_command(&pane_id));
    }

    let _ = runtime.event(
        runtime.leader_surface().id,
        SurfaceEventKind::LaunchRequested,
    );
    let hud_surface = runtime.hud_surface(true);
    let _ = runtime.event(hud_surface.id.clone(), SurfaceEventKind::HudRequested);
    let hud_pane = create_hud_watch_pane(bridge, cwd, env, &hud_surface);
    let codex_result = bridge.run(codex_spec);

    let mut cleanup_targets = list_hud_watch_panes(bridge, current_pane.as_deref());
    if let Some(pane_id) = hud_pane
        && !cleanup_targets.iter().any(|existing| existing == &pane_id)
    {
        cleanup_targets.push(pane_id);
    }
    for pane_id in cleanup_targets {
        let _ = runtime.event(hud_surface.id.clone(), SurfaceEventKind::CleanedUp);
        let _ = run_tmux_command(bridge, build_tmux_kill_pane_command(&pane_id));
    }

    finalize_process_result(codex_result, "codex")
}

fn run_detached_tmux(
    bridge: &ProcessBridge,
    codex_spec: &CommandSpec,
    cwd: &Path,
    env: &BTreeMap<OsString, OsString>,
    stdio_mode: StdioMode,
    runtime: &SurfaceRuntime<TmuxBackend>,
    session_name: &str,
) -> LaunchExecution {
    let _ = runtime.event(
        runtime.leader_surface().id,
        SurfaceEventKind::LaunchRequested,
    );
    let codex_cmd = build_tmux_pane_command(
        &codex_spec.program,
        &codex_spec.args,
        env.get(&OsString::from(SHELL_ENV)).map(OsString::as_os_str),
    );
    let hud_surface = runtime.hud_surface(true);
    let _ = runtime.event(hud_surface.id.clone(), SurfaceEventKind::HudRequested);

    let mut new_session = build_tmux_command(&[
        OsString::from("new-session"),
        OsString::from("-d"),
        OsString::from("-s"),
        OsString::from(session_name),
        OsString::from("-c"),
        cwd.as_os_str().to_os_string(),
        OsString::from(codex_cmd),
    ]);
    new_session.stdio_mode = StdioMode::Capture;
    if !run_tmux_command(bridge, new_session).is_some_and(|result| result.success()) {
        return finalize_process_result(bridge.run(codex_spec), "codex");
    }

    let mut split_hud = build_tmux_command(&[
        OsString::from("split-window"),
        OsString::from("-v"),
        OsString::from("-l"),
        OsString::from(HUD_TMUX_HEIGHT_LINES),
        OsString::from("-d"),
        OsString::from("-t"),
        OsString::from(session_name),
        OsString::from("-c"),
        cwd.as_os_str().to_os_string(),
        OsString::from("-P"),
        OsString::from("-F"),
        OsString::from("#{pane_id}"),
        OsString::from(build_hud_tmux_command(env, &hud_surface)),
    ]);
    split_hud.stdio_mode = StdioMode::Capture;
    if !run_tmux_command(bridge, split_hud).is_some_and(|result| result.success()) {
        let _ = run_tmux_command(bridge, kill_session_spec(session_name));
        return finalize_process_result(bridge.run(codex_spec), "codex");
    }

    let mut attach = build_tmux_command(&[
        OsString::from("attach-session"),
        OsString::from("-t"),
        OsString::from(session_name),
    ]);
    attach.stdio_mode = stdio_mode;
    let attach_result = match run_tmux_command(bridge, attach) {
        Some(result) if result.success() => {
            let _ = runtime.event(runtime.leader_surface().id, SurfaceEventKind::Attached);
            result
        }
        _ => {
            let _ = run_tmux_command(bridge, kill_session_spec(session_name));
            return finalize_process_result(bridge.run(codex_spec), "codex");
        }
    };

    finalize_process_result(attach_result, "tmux")
}

fn kill_session_spec(session_name: &str) -> CommandSpec {
    build_tmux_command(&[
        OsString::from("kill-session"),
        OsString::from("-t"),
        OsString::from(session_name),
    ])
}

fn tmux_available(bridge: &ProcessBridge) -> bool {
    bridge.run(&build_tmux_version_command()).success()
}

fn run_tmux_command(bridge: &ProcessBridge, spec: CommandSpec) -> Option<ProcessResult> {
    let result = bridge.run(&spec);
    if result.spawn_error_kind == Some(SpawnErrorKind::Missing) {
        None
    } else {
        Some(result)
    }
}

fn create_hud_watch_pane(
    bridge: &ProcessBridge,
    cwd: &Path,
    env: &BTreeMap<OsString, OsString>,
    hud_surface: &HudSurface,
) -> Option<String> {
    let hud_cmd = build_hud_tmux_command(env, hud_surface);
    let mut spec = build_tmux_command(&[
        OsString::from("split-window"),
        OsString::from("-v"),
        OsString::from("-l"),
        OsString::from(HUD_TMUX_HEIGHT_LINES),
        OsString::from("-d"),
        OsString::from("-c"),
        cwd.as_os_str().to_os_string(),
        OsString::from("-P"),
        OsString::from("-F"),
        OsString::from("#{pane_id}"),
        OsString::from(hud_cmd),
    ]);
    spec.stdio_mode = StdioMode::Capture;
    run_tmux_command(bridge, spec)
        .filter(|result| result.success())
        .and_then(|result| parse_pane_id(&String::from_utf8_lossy(&result.stdout)))
}

fn build_hud_tmux_command(env: &BTreeMap<OsString, OsString>, hud_surface: &HudSurface) -> String {
    let current_exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("omx"));
    let mut args = vec![OsString::from("hud")];
    if hud_surface.watch {
        args.push(OsString::from("--watch"));
    }
    build_tmux_pane_command(
        current_exe.as_os_str(),
        &args,
        env.get(&OsString::from(SHELL_ENV)).map(OsString::as_os_str),
    )
}

fn list_hud_watch_panes(bridge: &ProcessBridge, current_pane: Option<&str>) -> Vec<String> {
    let mut spec = build_tmux_command(&[
        OsString::from("list-panes"),
        OsString::from("-F"),
        OsString::from("#{pane_id}\t#{pane_current_command}\t#{pane_start_command}"),
    ]);
    spec.stdio_mode = StdioMode::Capture;
    let Some(result) = run_tmux_command(bridge, spec) else {
        return Vec::new();
    };
    if !result.success() {
        return Vec::new();
    }

    parse_tmux_pane_snapshot(&String::from_utf8_lossy(&result.stdout))
        .into_iter()
        .filter(|pane| Some(pane.pane_id.as_str()) != current_pane)
        .filter(|pane| is_hud_watch_pane(pane))
        .map(|pane| pane.pane_id)
        .collect()
}

fn parse_tmux_pane_snapshot(raw: &str) -> Vec<TmuxPaneSnapshot> {
    raw.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            let pane_id = parts.next()?.trim();
            let pane_current_command = parts.next()?.trim();
            let pane_start_command = parts.next()?.trim();
            Some(TmuxPaneSnapshot {
                pane_id: pane_id.to_string(),
                pane_current_command: pane_current_command.to_string(),
                pane_start_command: pane_start_command.to_string(),
            })
        })
        .collect()
}

fn is_hud_watch_pane(pane: &TmuxPaneSnapshot) -> bool {
    let start = pane.pane_start_command.to_ascii_lowercase();
    let current = pane.pane_current_command.to_ascii_lowercase();
    start.contains("hud") && start.contains("--watch")
        || current == "omx" && start.contains("--watch") && start.contains("hud")
}

fn parse_pane_id(raw: &str) -> Option<String> {
    let pane_id = raw.lines().next()?.trim();
    pane_id.starts_with('%').then(|| pane_id.to_string())
}

fn current_tmux_session_name(
    bridge: &ProcessBridge,
    env: &BTreeMap<OsString, OsString>,
) -> Option<String> {
    let target = env_string(env, TMUX_PANE_ENV);
    let mut args = vec![OsString::from("display-message"), OsString::from("-p")];
    if let Some(target) = target {
        args.push(OsString::from("-t"));
        args.push(OsString::from(target));
    }
    args.push(OsString::from("#S"));
    let mut spec = build_tmux_command(&args);
    spec.stdio_mode = StdioMode::Capture;
    run_tmux_command(bridge, spec)
        .filter(|result| result.success())
        .and_then(|result| {
            let session = String::from_utf8_lossy(&result.stdout).trim().to_string();
            (!session.is_empty()).then_some(session)
        })
}

fn build_detached_session_name(cwd: &Path) -> String {
    let dir_name = cwd
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("omx")
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("omx-{dir_name}-{stamp}")
}

fn running_inside_tmux(env: &BTreeMap<OsString, OsString>) -> bool {
    env.contains_key(&OsString::from(TMUX_ENV))
}

fn env_truthy(env: &BTreeMap<OsString, OsString>, key: &str) -> bool {
    env_string(env, key)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
        .unwrap_or(false)
}

fn tmux_opt_in(args: &[String], env: &BTreeMap<OsString, OsString>) -> bool {
    if args.iter().any(|a| a == TMUX_OPT_IN_FLAG) {
        return true;
    }
    if env_truthy(env, OMX_LAUNCH_TMUX_ENV) {
        return true;
    }
    matches!(
        env_string(env, OMX_LAUNCH_MODE_ENV).as_deref(),
        Some("tmux")
    )
}

fn tmux_forced_off(args: &[String], env: &BTreeMap<OsString, OsString>) -> bool {
    if args.iter().any(|a| a == NO_TMUX_FLAG) {
        return true;
    }
    env_truthy(env, OMX_LAUNCH_NO_TMUX_ENV)
        || env_truthy(env, OMX_NO_TMUX_ENV)
        || matches!(
            env_string(env, OMX_LAUNCH_MODE_ENV).as_deref(),
            Some("native")
        )
}

fn env_string(env: &BTreeMap<OsString, OsString>, key: &str) -> Option<String> {
    env.get(&OsString::from(key))
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.trim().is_empty())
}

fn finalize_process_result(result: ProcessResult, program_name: &str) -> LaunchExecution {
    if let Some(kind) = result.spawn_error_kind {
        let message = match kind {
            SpawnErrorKind::Missing => {
                format!("[omx] failed to launch {program_name}: executable not found in PATH")
            }
            SpawnErrorKind::Blocked => format!(
                "[omx] failed to launch {program_name}: executable is present but blocked in the current environment"
            ),
            SpawnErrorKind::Error => format!("[omx] failed to launch {program_name}"),
        };
        return LaunchExecution {
            stdout: Vec::new(),
            stderr: format!("{message}\n").into_bytes(),
            exit_code: 1,
        };
    }

    let mut stderr = result.stderr;
    let exit_code = result.status_code.unwrap_or(1);
    if let Some(signal) = result.terminating_signal {
        let signal_message = format!("[omx] {program_name} exited due to signal {signal}\n");
        stderr.extend_from_slice(signal_message.as_bytes());
    }

    LaunchExecution {
        stdout: result.stdout,
        stderr,
        exit_code,
    }
}

fn normalize_launch_args(args: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut wants_bypass = false;
    let mut has_bypass = false;
    let mut reasoning_mode: Option<&str> = None;
    let mut index = 0_usize;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            // compatibility/runtime selection flags (not passed to codex)
            TMUX_OPT_IN_FLAG | NO_TMUX_FLAG => {}
            "-w" | "--worktree" => {
                if args
                    .get(index + 1)
                    .is_some_and(|value| !value.starts_with('-'))
                {
                    index += 1;
                }
            }
            value if value.starts_with("--worktree=") => {}
            MADMAX_FLAG => wants_bypass = true,
            CODEX_BYPASS_FLAG => {
                wants_bypass = true;
                if !has_bypass {
                    normalized.push(arg.to_string());
                    has_bypass = true;
                }
            }
            HIGH_REASONING_FLAG => reasoning_mode = Some("high"),
            XHIGH_REASONING_FLAG => reasoning_mode = Some("xhigh"),
            SPARK_FLAG => {}
            MADMAX_SPARK_FLAG => wants_bypass = true,
            "--notify-temp" | "--discord" | "--slack" | "--telegram" => {}
            "--custom" => {
                if args
                    .get(index + 1)
                    .is_some_and(|value| !value.starts_with('-'))
                {
                    index += 1;
                }
            }
            value if value.starts_with("--custom=") => {}
            _ => normalized.push(arg.to_string()),
        }
        index += 1;
    }

    if wants_bypass && !has_bypass {
        normalized.push(CODEX_BYPASS_FLAG.to_string());
    }
    if let Some(mode) = reasoning_mode {
        normalized.push(CONFIG_FLAG.to_string());
        normalized.push(format!("{REASONING_KEY}=\"{mode}\""));
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::{normalize_launch_args, run_launch_with_stdio};
    use omx_process::process_bridge::StdioMode;
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::fs;
    use std::path::{Path, PathBuf};

    const HELP: &str = "top-level help\n";

    #[test]
    fn prints_top_level_help_for_help_variants() {
        let cwd = std::env::current_dir().expect("cwd");
        let env: BTreeMap<OsString, OsString> = std::env::vars_os().collect();
        for args in [
            vec!["--help".to_string()],
            vec!["-h".to_string()],
            vec!["help".to_string()],
        ] {
            let result = run_launch_with_stdio(&args, &cwd, &env, HELP, StdioMode::Capture)
                .expect("launch help");
            assert_eq!(result.stdout, HELP.as_bytes());
            assert!(result.stderr.is_empty());
            assert_eq!(result.exit_code, 0);
        }
    }

    #[cfg(unix)]
    #[test]
    fn launches_codex_directly_when_tmux_is_unavailable() {
        let cwd = temp_dir("launch-direct");
        let fake_bin = cwd.join("bin");
        fs::create_dir_all(&fake_bin).expect("create bin");
        let codex_path = fake_bin.join("codex");
        fs::write(
            &codex_path,
            "#!/bin/sh\nprintf 'fake-codex cwd=%s args=%s\\n' \"$PWD\" \"$*\"\n",
        )
        .expect("write codex");
        make_executable(&codex_path);

        let env = env_with_path(&fake_bin);
        let result = run_launch_with_stdio(
            &["--model".to_string(), "gpt-5".to_string()],
            &cwd,
            &env,
            HELP,
            StdioMode::Capture,
        )
        .expect("launch direct");

        let stdout = String::from_utf8(result.stdout).expect("utf8 stdout");
        assert_eq!(result.exit_code, 0);
        assert!(result.stderr.is_empty());
        assert!(stdout.contains("fake-codex"));
        assert!(stdout.contains(&format!("cwd={}", cwd.display())));
        assert!(stdout.contains("args=--model gpt-5"));
    }

    #[cfg(unix)]
    #[test]
    fn launches_inside_tmux_with_hud_split_and_cleanup() {
        let cwd = temp_dir("launch-inside-tmux");
        let fake_bin = cwd.join("bin");
        let log_path = cwd.join("tmux.log");
        fs::create_dir_all(&fake_bin).expect("create bin");
        write_fake_codex(&fake_bin.join("codex"));
        write_fake_tmux(&fake_bin.join("tmux"), &log_path);

        let mut env = env_with_path(&fake_bin);
        env.insert(OsString::from("TMUX"), OsString::from("/tmp/tmux-sock"));
        env.insert(OsString::from("TMUX_PANE"), OsString::from("%1"));
        env.insert(
            OsString::from("OMX_TMUX_LOG"),
            log_path.as_os_str().to_os_string(),
        );
        env.insert(
            OsString::from("OMX_TMUX_LIST_PANES"),
            OsString::from(
                "%1\tzsh\t/bin/zsh -l\n%9\tomx\t'/bin/sh' -lc 'exec /tmp/omx hud --watch'\n",
            ),
        );

        let result = run_launch_with_stdio(&[], &cwd, &env, HELP, StdioMode::Capture)
            .expect("launch inside tmux");

        let stdout = String::from_utf8(result.stdout).expect("utf8 stdout");
        let tmux_log = fs::read_to_string(&log_path).expect("tmux log");
        assert_eq!(result.exit_code, 0);
        assert!(stdout.contains("fake-codex"));
        assert!(tmux_log.contains("list-panes|-F|#{pane_id}"));
        assert!(tmux_log.contains("kill-pane|-t|%9"));
        assert!(tmux_log.contains("split-window|-v|-l|2|-d|-c|"));
        assert!(tmux_log.contains("kill-pane|-t|%77"));
    }

    #[cfg(unix)]
    #[test]
    fn launches_detached_tmux_session_with_hud_when_available() {
        let cwd = temp_dir("launch-detached-tmux");
        let fake_bin = cwd.join("bin");
        let log_path = cwd.join("tmux.log");
        fs::create_dir_all(&fake_bin).expect("create bin");
        write_fake_codex(&fake_bin.join("codex"));
        write_fake_tmux(&fake_bin.join("tmux"), &log_path);

        let mut env = env_with_path(&fake_bin);
        env.insert(
            OsString::from("OMX_TMUX_LOG"),
            log_path.as_os_str().to_os_string(),
        );
        // Opt-in to detached tmux behavior under the new native-first policy
        env.insert(OsString::from("OMX_LAUNCH_TMUX"), OsString::from("1"));

        let result = run_launch_with_stdio(
            &["--model".to_string(), "gpt-5".to_string()],
            &cwd,
            &env,
            HELP,
            StdioMode::Capture,
        )
        .expect("launch detached tmux");

        let tmux_log = fs::read_to_string(&log_path).expect("tmux log");
        let stdout = String::from_utf8(result.stdout).expect("utf8 stdout");
        assert_eq!(result.exit_code, 0);
        assert!(stdout.contains("attached"));
        assert!(tmux_log.contains("new-session|-d|-s|omx-"));
        assert!(tmux_log.contains("split-window|-v|-l|2|-d|-t|omx-"));
        assert!(tmux_log.contains("attach-session|-t|omx-"));
        assert!(tmux_log.contains("'\"'\"'codex'\"'\"'|'\"'\"'--model'\"'\"'|'\"'\"'gpt-5'\"'\"'"));
        assert!(tmux_log.contains("'\"'\"'hud'\"'\"'|'\"'\"'--watch'\"'\"'"));
    }

    #[test]
    fn reports_missing_codex_executable_in_path() {
        let cwd = std::env::current_dir().expect("cwd");
        let env = BTreeMap::from([(OsString::from("PATH"), OsString::from(""))]);
        let result = run_launch_with_stdio(&[], &cwd, &env, HELP, StdioMode::Capture)
            .expect("missing codex handled");
        let stderr = String::from_utf8(result.stderr).expect("utf8 stderr");
        assert_eq!(result.exit_code, 1);
        assert!(stderr.contains("failed to launch codex"));
        assert!(stderr.contains("executable not found in PATH"));
    }

    #[test]
    fn normalizes_launch_shorthand_flags_to_codex_args() {
        assert_eq!(
            normalize_launch_args(&["--xhigh".to_string(), "--madmax".to_string()]),
            vec![
                "--dangerously-bypass-approvals-and-sandbox".to_string(),
                "-c".to_string(),
                "model_reasoning_effort=\"xhigh\"".to_string()
            ]
        );
    }

    #[test]
    fn strips_worker_only_and_notify_temp_flags_from_leader_args() {
        assert_eq!(
            normalize_launch_args(&[
                "--notify-temp".to_string(),
                "--discord".to_string(),
                "--custom".to_string(),
                "openclaw:ops".to_string(),
                "--spark".to_string(),
                "--model".to_string(),
                "gpt-5".to_string()
            ]),
            vec!["--model".to_string(), "gpt-5".to_string()]
        );
    }

    #[test]
    fn strips_worktree_flags_before_launching_codex() {
        assert_eq!(
            normalize_launch_args(&[
                "--worktree".to_string(),
                "feature/demo".to_string(),
                "--yolo".to_string()
            ]),
            vec!["--yolo".to_string()]
        );
        assert_eq!(
            normalize_launch_args(&[
                "--worktree=feature/demo".to_string(),
                "--model".to_string(),
                "gpt-5".to_string()
            ]),
            vec!["--model".to_string(), "gpt-5".to_string()]
        );
    }

    #[cfg(unix)]
    fn temp_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("omx-launch-{label}-{nanos}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[cfg(unix)]
    fn env_with_path(fake_bin: &PathBuf) -> BTreeMap<OsString, OsString> {
        let mut env: BTreeMap<OsString, OsString> = std::env::vars_os().collect();
        let mut path = fake_bin.as_os_str().to_os_string();
        if let Some(existing) = std::env::var_os("PATH") {
            path.push(OsString::from(":"));
            path.push(existing);
        }
        env.insert(OsString::from("PATH"), path);
        env.remove(&OsString::from("TMUX"));
        env.remove(&OsString::from("TMUX_PANE"));
        env
    }

    #[cfg(unix)]
    fn write_fake_codex(path: &Path) {
        fs::write(
            path,
            "#!/bin/sh\nprintf 'fake-codex cwd=%s args=%s\\n' \"$PWD\" \"$*\"\n",
        )
        .expect("write codex");
        make_executable(path);
    }

    #[cfg(unix)]
    fn write_fake_tmux(path: &Path, log_path: &Path) {
        let script = format!(
            "#!/bin/sh\nLOG=\"{}\"\nprintf '%s\\n' \"$*\" | tr ' ' '|' >> \"$LOG\"\ncase \"$1\" in\n  -V)\n    echo 'tmux 3.4'\n    ;;\n  list-panes)\n    printf '%s' \"${{OMX_TMUX_LIST_PANES:-}}\"\n    ;;\n  split-window)\n    echo '%77'\n    ;;\n  attach-session)\n    echo 'attached'\n    ;;\n  *)\n    :\n    ;;\nesac\n",
            log_path.display()
        );
        fs::write(path, script).expect("write tmux");
        make_executable(path);
    }

    #[cfg(unix)]
    fn make_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path).expect("metadata");
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("chmod");
    }
}
