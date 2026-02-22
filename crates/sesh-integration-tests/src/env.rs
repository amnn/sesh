//! A contained environment to run tests in.
//!
//! Each environment is set-up within its own temporary directory, under the cargo target temp
//! directory, which is cleaned up when the environment is dropped. That temporary directory
//! includes a `bin` directory and a `home` directory.
//!
//! Binaries can be added to the environment, and commands can be run under a restricted env (can
//! only search for binaries in its own `bin` directory, current directory is set to `home`).
//!
//! NB. Environment isolation is a convenience to ensure tests are stable, not true isolation.

use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use tokio::fs;
use tokio::join;
use tokio::process::Command;
use which::which;

pub(crate) struct Env {
    _dir: tempfile::TempDir,
}

impl Env {
    /// Construct a new isolated environment rooted under `tmp`.
    ///
    /// The caller provides `tmp` so test harnesses can decide where temporary artifacts live (for
    /// example, under Cargo's per-test temporary directory).
    pub(crate) async fn new(tmp: impl AsRef<Path>) -> anyhow::Result<Self> {
        let dir = tempfile::tempdir_in(tmp).context("failed to create environment root")?;

        let (home, path) = join!(
            fs::create_dir(dir.path().join("home")),
            fs::create_dir(dir.path().join("bin")),
        );

        home.context("failed to create $HOME")?;
        path.context("failed to create $PATH")?;

        Ok(Self { _dir: dir })
    }

    /// Ensure the binary is available in the environment.
    ///
    /// The binary can either be specified by name (in which case it is fetched from the test's
    /// $PATH), or it can be specified by path, in which case it must exist and be executable.
    ///
    /// The binary is added to the environment's `bin` directory. On Unix systems, it is added by
    /// symlink, on Windows, it is added by hard link, and on other systems it is copied.
    ///
    /// Returns the path to the binary in the environment.
    pub(crate) async fn bin(&self, bin: impl AsRef<OsStr>) -> anyhow::Result<PathBuf> {
        let bin = bin.as_ref();
        self.bin_(bin)
            .await
            .with_context(|| format!("failed to add '{}' to environment", bin.display()))
    }

    async fn bin_(&self, bin: &OsStr) -> anyhow::Result<PathBuf> {
        let source = which(bin)?;
        let name = source
            .file_name()
            .context("missing binary name")?
            .to_str()
            .context("invalid binary name")?
            .to_owned();

        let mut target = self.path("bin");
        target.extend([&name]);

        if !target.exists() {
            link(&source, &target)
                .await
                .context("failed to link binary")?;
        }

        Ok(target)
    }

    /// Start a new command in this environment.
    ///
    /// Its `$HOME` and `$PATH` environment variables point inside the environment, and its current
    /// directory is also set to `$HOME`.
    pub(crate) fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);

        command
            .env_clear()
            .env("HOME", self.path("home"))
            .env("PATH", self.path("bin"))
            .current_dir(self.path("home"));

        command
    }

    /// Relativize `path` in this environment's context.
    pub(crate) fn path(&self, path: impl AsRef<Path>) -> PathBuf {
        self._dir.path().join(path)
    }
}

/// Make `source` available at `target`.
async fn link(source: &Path, target: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    fs::symlink(source, target).await?;

    #[cfg(windows)]
    fs::hard_link(source, target).await?;

    #[cfg(not(any(unix, windows)))]
    fs::copy(source, target).await?;

    Ok(())
}
