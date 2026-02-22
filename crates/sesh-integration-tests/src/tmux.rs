use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::ensure;
use tokio::process::Command;

use crate::env::Env;

/// Handle for managing a `tmux` server.
pub(crate) struct Tmux {
    bin: PathBuf,
    socket: PathBuf,
}

impl Tmux {
    /// Start a `tmux` server in the given `env`ironment.
    ///
    /// Fails if the tmux socket file already exists, the `tmux` binary can't be added to the
    /// environment, or the server fails to start.
    pub(crate) async fn new(env: &Env) -> anyhow::Result<Self> {
        let socket = env.path("tmux.sock");
        ensure!(!socket.exists(), "tmux socket already exists");

        let bin = env.bin("tmux").await?;
        let tmux = Self { bin, socket };

        let new = tmux
            .command(env)
            .args(["new-session", "-d"])
            .args(["-x", "160", "-y", "100"])
            .output()
            .await
            .context("failed to execute 'tmux new'")?;

        ensure!(
            new.status.success(),
            "'tmux new' failed: {}",
            String::from_utf8_lossy(&new.stderr),
        );

        Ok(tmux)
    }

    /// Build a `tmux` command in the given `env`ironment.
    fn command(&self, env: &Env) -> Command {
        let mut command = env.command("tmux");
        command.arg("-S").arg(self.socket.as_os_str());
        command
    }
}

impl Drop for Tmux {
    fn drop(&mut self) {
        let _ = std::process::Command::new(&self.bin)
            .arg("-S")
            .arg(self.socket.as_os_str())
            .arg("kill-server")
            .status();
    }
}
