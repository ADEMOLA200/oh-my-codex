#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn native_binary_path() -> PathBuf {
    std::env::var("CARGO_BIN_EXE_omx")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root().join("target/debug/omx"))
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("omx-native-launch-legacy-{label}-{nanos}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn make_executable(path: &Path) {
    let metadata = fs::metadata(path).expect("metadata");
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("chmod");
}

fn write_codex_stub(path: &Path, log_path: &Path) {
    fs::write(
        path,
        format!(
            concat!(
                "#!/bin/sh\n",
                "printf 'codex args:%s\\n' \"$*\" >> '{}'\n",
                "printf 'codex cwd:%s\\n' \"$PWD\" >> '{}'\n",
                "exit 0\n"
            ),
            log_path.display(),
            log_path.display(),
        ),
    )
    .expect("write codex stub");
    make_executable(path);
}

fn write_tmux_stub(path: &Path, log_path: &Path) {
    fs::write(
        path,
        format!(
            concat!(
                "#!/bin/sh\n",
                "printf 'tmux %s\\n' \"$*\" >> '{}'\n",
                "cmd=\"$1\"\n",
                "case \"$cmd\" in\n",
                "  -V)\n",
                "    printf 'tmux 3.4\\n'\n",
                "    ;;\n",
                "  list-panes)\n",
                "    printf ''\n",
                "    ;;\n",
                "  display-message)\n",
                "    printf 'fixture-session\\n'\n",
                "    ;;\n",
                "  split-window)\n",
                "    printf '%%42\\n'\n",
                "    ;;\n",
                "  list-sessions)\n",
                "    printf 'fixture-session\\n'\n",
                "    ;;\n",
                "  *)\n",
                "    ;;\n",
                "esac\n",
                "exit 0\n"
            ),
            log_path.display(),
        ),
    )
    .expect("write tmux stub");
    make_executable(path);
}

fn base_command(binary: &Path, cwd: &Path, home: &Path, path: &str) -> Command {
    let mut command = Command::new(binary);
    command.current_dir(cwd);
    command.env_clear();
    command.env("HOME", home);
    command.env("PATH", path);
    command
}

fn assert_log_mentions_hud_watch(log: &str) {
    assert!(log.contains("hud"), "log:\n{log}");
    assert!(log.contains("--watch"), "log:\n{log}");
}

#[test]
fn inside_tmux_launch_uses_tmux_hud_bootstrap_and_cleanup() {
    let cwd = temp_dir("inside-tmux");
    let home = cwd.join("home");
    let fake_bin = cwd.join("bin");
    let log_path = cwd.join("launch.log");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&fake_bin).expect("create bin");

    write_codex_stub(&fake_bin.join("codex"), &log_path);
    write_tmux_stub(&fake_bin.join("tmux"), &log_path);

    let output = base_command(
        &native_binary_path(),
        &cwd,
        &home,
        fake_bin.display().to_string().as_str(),
    )
    .env("TMUX", "/tmp/fake-tmux,123,0")
    .env("TMUX_PANE", "%1")
    .arg("--model")
    .arg("gpt-5")
    .output()
    .expect("run native launch in tmux");

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    let log = fs::read_to_string(&log_path).unwrap_or_default();

    assert!(output.status.success(), "{stderr}{stdout}\nlog:\n{log}");
    assert!(log.contains("codex args:--model gpt-5"), "log:\n{log}");
    assert!(log.contains("tmux split-window"), "log:\n{log}");
    assert_log_mentions_hud_watch(&log);
    assert!(log.contains("tmux kill-pane"), "log:\n{log}");
}

#[test]
fn outside_tmux_launch_creates_detached_tmux_session_with_hud_and_attach_when_opted_in() {
    let cwd = temp_dir("detached-session");
    let home = cwd.join("home");
    let fake_bin = cwd.join("bin");
    let log_path = cwd.join("launch.log");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&fake_bin).expect("create bin");

    write_codex_stub(&fake_bin.join("codex"), &log_path);
    write_tmux_stub(&fake_bin.join("tmux"), &log_path);

    let output = base_command(
        &native_binary_path(),
        &cwd,
        &home,
        fake_bin.display().to_string().as_str(),
    )
    .env("OMX_LAUNCH_TMUX", "1")
    .arg("--model")
    .arg("gpt-5")
    .output()
    .expect("run native launch outside tmux");

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    let log = fs::read_to_string(&log_path).unwrap_or_default();

    assert!(output.status.success(), "{stderr}{stdout}\nlog:\n{log}");
    assert!(log.contains("tmux new-session"), "log:\n{log}");
    assert!(log.contains("tmux split-window"), "log:\n{log}");
    assert!(log.contains("tmux attach-session"), "log:\n{log}");
    assert_log_mentions_hud_watch(&log);
    assert!(
        !log.contains("codex args:--model gpt-5"),
        "detached path should launch codex through tmux when opted in, not directly in the parent process\nlog:\n{log}"
    );
}

#[test]
fn falls_back_to_direct_codex_when_tmux_is_unavailable_or_not_opted_in() {
    let cwd = temp_dir("no-tmux");
    let home = cwd.join("home");
    let fake_bin = cwd.join("bin");
    let log_path = cwd.join("launch.log");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&fake_bin).expect("create bin");

    write_codex_stub(&fake_bin.join("codex"), &log_path);

    let output = base_command(
        &native_binary_path(),
        &cwd,
        &home,
        fake_bin.display().to_string().as_str(),
    )
    .arg("--xhigh")
    .output()
    .expect("run native launch without tmux");

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    let log = fs::read_to_string(&log_path).unwrap_or_default();

    assert!(output.status.success(), "{stderr}{stdout}\nlog:\n{log}");
    assert!(
        log.contains("codex args:-c model_reasoning_effort=\"xhigh\""),
        "log:\n{log}"
    );
    assert!(!log.contains("tmux "), "log:\n{log}");
}

#[test]
fn help_mentions_tmux_hud_auto_attach_contract() {
    let cwd = temp_dir("help");
    let home = cwd.join("home");
    fs::create_dir_all(&home).expect("create home");

    let output = base_command(&native_binary_path(), &cwd, &home, "/usr/bin:/bin")
        .arg("--help")
        .output()
        .expect("run native help");

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(output.status.success(), "{stderr}{stdout}");
    assert!(stdout.contains("HUD auto-attaches only when already inside tmux"));
    assert!(stdout.contains("detached tmux session available with --tmux or OMX_LAUNCH_TMUX=1"));
}
