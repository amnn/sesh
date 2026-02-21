//! Builds the `sesh` binary in the workspace to make available during testing.

use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::ensure;

use tokio::process::Command;
use tokio::sync::OnceCell;

static SESH: OnceCell<PathBuf> = OnceCell::const_new();

/// Ensure the `sesh` binary is built and return its path in the target directory.
pub(crate) async fn binary() -> anyhow::Result<&'static PathBuf> {
    SESH.get_or_try_init(|| async {
        let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        root.extend(["..", ".."]);

        let build = Command::new("cargo")
            .current_dir(&root)
            .args(["build", "--bin", "sesh"])
            .status()
            .await
            .context("failed to execute 'cargo build --bin sesh'")?;

        ensure!(build.success(), "'cargo build --bin sesh' failed");

        let mut path = root;
        path.extend(["target", "debug", "sesh"]);
        Ok(path)
    })
    .await
}
