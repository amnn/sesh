use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Context as _;
use shlex::Quoter;

use crate::parser;
use crate::parser::Key;
use crate::parser::Line;
use crate::parser::LineKind;

const DEFAULT_HOME: &str = "/home/test";
const DEFAULT_PANE: &str = "zz-sesh-ui-runner:0.0";
const RUNNER_SESSION: &str = "zz-sesh-ui-runner";
const SETTLE_MAX_POLLS: usize = 20;
const SETTLE_POLL_DELAY_MS: u64 = 80;

static BUILD_LOCK: Mutex<()> = Mutex::new(());
static CASE_COUNTER: AtomicU64 = AtomicU64::new(0);
static SESH_BIN: OnceLock<PathBuf> = OnceLock::new();

struct Runner {
    current_pane: String,
    env_path: String,
    socket: String,
    tmux_tmp_dir: PathBuf,
    transcript: String,
    work_dir: PathBuf,
}

pub struct CaseResult {
    pub snapshot_name: String,
    pub transcript: String,
}

/// Run a markdown integration scenario against a headless tmux-backed `sesh` instance.
pub fn run_case_file(path: &Path) -> anyhow::Result<CaseResult> {
    let script = fs::read_to_string(path)?;
    let script = parser::Script::parse(&script);
    let socket = unique_socket_name();
    let temp = tempfile::tempdir()?;
    let work_dir = temp.path().join("work");
    fs::create_dir_all(&work_dir)?;

    let tmux_tmp_dir = PathBuf::from(format!("/tmp/{socket}"));
    if tmux_tmp_dir.exists() {
        fs::remove_dir_all(&tmux_tmp_dir).with_context(|| {
            format!(
                "failed to remove stale tmux tmpdir '{}'",
                tmux_tmp_dir.display()
            )
        })?;
    }
    fs::create_dir_all(&tmux_tmp_dir)?;
    let _server = TmuxServer::new(socket.clone(), tmux_tmp_dir.clone());

    let env_path = build_test_path(temp.path(), &work_dir)?;
    let runner_cmd = format!(
        "cd {} && env HOME={} PATH={} SESH_TEST_WORKDIR={} {} cli",
        shell_quote_str(&work_dir.to_string_lossy())?,
        DEFAULT_HOME,
        shell_quote_str(&env_path)?,
        shell_quote_str(&work_dir.to_string_lossy())?,
        shell_quote_str(&sesh_binary()?.to_string_lossy())?,
    );

    run_tmux(
        &tmux_tmp_dir,
        &[
            "-L",
            &socket,
            "new-session",
            "-d",
            "-s",
            RUNNER_SESSION,
            "-x",
            "120",
            "-y",
            "40",
            &runner_cmd,
        ],
    )?;

    let mut runner = Runner {
        current_pane: DEFAULT_PANE.to_owned(),
        env_path,
        socket,
        tmux_tmp_dir,
        transcript: String::new(),
        work_dir,
    };

    for line in script.lines {
        execute_line(&mut runner, line)?;
    }

    let snapshot_name = path
        .strip_prefix("tests/cases")?
        .to_string_lossy()
        .replace('/', "__");

    Ok(CaseResult {
        snapshot_name,
        transcript: runner.transcript,
    })
}

fn append_line(transcript: &mut String, line: &str) {
    transcript.push_str(line);
    transcript.push('\n');
}

fn execute_line(runner: &mut Runner, line: Line) -> anyhow::Result<()> {
    append_line(&mut runner.transcript, line.raw);
    let raw = line.raw.to_owned();

    match line.kind {
        LineKind::Error { message } => {
            append_line(&mut runner.transcript, &format!("ERROR: {message}"));
            Ok(())
        }
        LineKind::Text => Ok(()),
        LineKind::Sh { args } => run_sh(runner, &raw, args),
        LineKind::Tmux { args } => run_tmux_directive(runner, &raw, args),
        LineKind::Pane { target } => {
            runner.current_pane = target.to_owned();
            Ok(())
        }
        LineKind::Keys { keys } => run_keys(runner, &raw, keys),
        LineKind::Snap { filters } => run_snap(runner, &raw, filters),
    }
}

fn run_sh(runner: &Runner, raw: &str, args: Vec<String>) -> anyhow::Result<()> {
    let output = Command::new(&args[0])
        .args(args.iter().skip(1))
        .current_dir(&runner.work_dir)
        .env("HOME", DEFAULT_HOME)
        .env("PATH", &runner.env_path)
        .env("SESH_TEST_WORKDIR", &runner.work_dir)
        .output()
        .with_context(|| format!("failed to run :sh command: '{raw}'"))?;

    anyhow::ensure!(
        output.status.success(),
        ":sh command failed for '{raw}': {}",
        String::from_utf8_lossy(&output.stderr)
    );

    maybe_settle(runner)
}

fn run_tmux_directive(runner: &Runner, raw: &str, args: Vec<String>) -> anyhow::Result<()> {
    let mut tmux_args = vec!["-L".to_owned(), runner.socket.clone()];
    tmux_args.extend(args);
    run_tmux_owned(&runner.tmux_tmp_dir, &tmux_args)
        .with_context(|| format!("failed to run :tmux command: '{raw}'"))?;
    maybe_settle(runner)
}

fn run_keys(runner: &Runner, raw: &str, keys: Vec<Key>) -> anyhow::Result<()> {
    for key in keys {
        match key {
            Key::Text(text) => {
                run_tmux_owned(
                    &runner.tmux_tmp_dir,
                    &[
                        "-L".to_owned(),
                        runner.socket.clone(),
                        "send-keys".to_owned(),
                        "-t".to_owned(),
                        runner.current_pane.clone(),
                        "-l".to_owned(),
                        text,
                    ],
                )
                .with_context(|| format!("failed to send :keys input for '{raw}'"))?;
            }
            other => {
                let key_name = tmux_key_name(other);
                run_tmux_owned(
                    &runner.tmux_tmp_dir,
                    &[
                        "-L".to_owned(),
                        runner.socket.clone(),
                        "send-keys".to_owned(),
                        "-t".to_owned(),
                        runner.current_pane.clone(),
                        key_name.to_owned(),
                    ],
                )
                .with_context(|| format!("failed to send :keys input for '{raw}'"))?;
            }
        }
    }

    maybe_settle(runner)
}

fn run_snap(runner: &mut Runner, raw: &str, filter: Vec<parser::Filter>) -> anyhow::Result<()> {
    maybe_settle(runner)?;

    let mut captured = run_tmux(
        &runner.tmux_tmp_dir,
        &[
            "-L",
            &runner.socket,
            "capture-pane",
            "-p",
            "-t",
            &runner.current_pane,
        ],
    )
    .with_context(|| format!("failed to capture pane for '{raw}'"))?;

    for replacement in filter {
        captured = replacement
            .patt
            .replace_all(&captured, replacement.repl)
            .into_owned();
    }

    runner.transcript.push_str("```terminal\n");
    runner.transcript.push_str(&captured);
    if !captured.ends_with('\n') {
        runner.transcript.push('\n');
    }
    runner.transcript.push_str("```\n");

    Ok(())
}

fn tmux_key_name(key: Key) -> &'static str {
    match key {
        Key::Backspace => "BSpace",
        Key::Ctrl => "C",
        Key::Down => "Down",
        Key::Enter => "Enter",
        Key::Esc => "Escape",
        Key::Left => "Left",
        Key::Opt => "M",
        Key::Right => "Right",
        Key::Shift => "S",
        Key::Space => "Space",
        Key::Tab => "Tab",
        Key::Text(_) => unreachable!("text keys handled separately"),
        Key::Up => "Up",
    }
}

fn maybe_settle(runner: &Runner) -> anyhow::Result<()> {
    let mut previous = String::new();
    let mut stable_reads = 0;
    for _ in 0..SETTLE_MAX_POLLS {
        let capture = run_tmux(
            &runner.tmux_tmp_dir,
            &[
                "-L",
                &runner.socket,
                "capture-pane",
                "-p",
                "-t",
                &runner.current_pane,
            ],
        )?;

        if capture == previous {
            stable_reads += 1;
            if stable_reads >= 2 {
                break;
            }
        } else {
            stable_reads = 0;
        }

        previous = capture;
        std::thread::sleep(Duration::from_millis(SETTLE_POLL_DELAY_MS));
    }

    Ok(())
}

fn build_test_path(temp_dir: &Path, work_dir: &Path) -> anyhow::Result<String> {
    let bin_dir = temp_dir.join("bin");
    fs::create_dir_all(&bin_dir)?;

    fs::write(work_dir.join("jj.stdout"), "")?;
    fs::write(work_dir.join("jj.stderr"), "")?;
    fs::write(work_dir.join("jj.exit"), "0")?;

    let script = bin_dir.join("jj");
    let body = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\nworkdir={}\ncat \"$workdir/jj.stdout\"\ncat \"$workdir/jj.stderr\" >&2\nexit \"$(cat \"$workdir/jj.exit\")\"\n",
        shell_quote_str(&work_dir.to_string_lossy())?
    );
    fs::write(script.as_path(), body)?;
    fs::set_permissions(script.as_path(), fs::Permissions::from_mode(0o755))?;

    let mut path = OsString::from(bin_dir.as_os_str());
    if let Some(old_path) = std::env::var_os("PATH") {
        path.push(":");
        path.push(old_path);
    }

    path.into_string()
        .map_err(|_| anyhow::anyhow!("test PATH contains non-UTF8 values"))
}

fn run_tmux(tmux_tmp_dir: &Path, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("tmux")
        .env("TMUX_TMPDIR", tmux_tmp_dir)
        .args(args)
        .output()
        .with_context(|| format!("failed to run tmux command: tmux {}", args.join(" ")))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(anyhow::anyhow!(
            "tmux command failed: tmux {}\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr),
        ))
    }
}

fn run_tmux_owned(tmux_tmp_dir: &Path, args: &[String]) -> anyhow::Result<String> {
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    run_tmux(tmux_tmp_dir, &borrowed)
}

fn sesh_binary() -> anyhow::Result<&'static PathBuf> {
    if let Some(path) = SESH_BIN.get() {
        return Ok(path);
    }

    let _guard = BUILD_LOCK
        .lock()
        .map_err(|_| anyhow::anyhow!("failed to lock binary build guard"))?;

    if let Some(path) = SESH_BIN.get() {
        return Ok(path);
    }

    let root = workspace_root();
    let status = Command::new("cargo")
        .arg("build")
        .arg("-p")
        .arg("sesh")
        .arg("--bin")
        .arg("sesh")
        .current_dir(&root)
        .status()
        .context("failed to build sesh binary for integration tests")?;
    anyhow::ensure!(status.success(), "building sesh binary failed");

    let bin = root.join("target").join("debug").join("sesh");
    let _ = SESH_BIN.set(bin);
    Ok(SESH_BIN
        .get()
        .expect("SESH_BIN must be initialized after successful build"))
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir should have parent")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

fn unique_socket_name() -> String {
    let counter = CASE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after unix epoch")
        .as_nanos();
    format!("sesh-it-{}-{nanos}-{counter}", std::process::id())
}

fn shell_quote_str(value: &str) -> anyhow::Result<String> {
    Quoter::new()
        .quote(value)
        .map(|quoted| quoted.into_owned())
        .context("failed to shell-quote value")
}

struct TmuxServer {
    socket: String,
    tmux_tmp_dir: PathBuf,
}

impl TmuxServer {
    fn new(socket: String, tmux_tmp_dir: PathBuf) -> Self {
        Self {
            socket,
            tmux_tmp_dir,
        }
    }
}

impl Drop for TmuxServer {
    fn drop(&mut self) {
        let _ = run_tmux(&self.tmux_tmp_dir, &["-L", &self.socket, "kill-server"]);
        let _ = fs::remove_dir_all(&self.tmux_tmp_dir);
    }
}
