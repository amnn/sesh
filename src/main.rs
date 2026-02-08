use sesh::skim;
use sesh::tmux;

fn main() -> anyhow::Result<()> {
    tmux::ensure()?;

    let sessions = tmux::sessions()?;
    skim::run(sessions);
    Ok(())
}
