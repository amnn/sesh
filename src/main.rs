use std::time::Duration;

use anyhow::Context as _;
use sesh::skim;
use sesh::tmux::Tmux;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tmux = Tmux::new(Duration::from_millis(100)).context("Failed to initialize tmux")?;

    let mut sessions = tmux.sessions();
    while let Ok(names) = sessions.has_changed().await {
        let tmux = tmux.clone();
        let names = names.to_vec();

        // `skim` sets up its own tokio runtime, so move it onto a blocking thread to avoid
        // conflicts.
        tokio::task::spawn_blocking(move || skim::run(tmux, names)).await?;
    }

    Ok(())
}
