use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::ensure;
use dashmap::DashMap;
use tokio::process::Command;
use tokio::sync::watch;
use tokio_util::task::AbortOnDropHandle;
use tracing::error;
use which::which;

use crate::preview::preview;

/// TODO: Replace with config
const MIN_PANE_HEIGHT: usize = 5;

/// Client for extracting information from `tmux`.
#[derive(Clone)]
pub struct Tmux(Arc<State>);

/// Subscriber over the current list of tmux sessions. Its `has_changed` method will trigger when
/// the active session list is updated.
pub struct Sessions {
    current: Option<Vec<String>>,
    rx: watch::Receiver<Option<Vec<String>>>,
}

struct State {
    /// Shared state cached from `tmux` by the background refresh task.
    inner: Arc<Inner>,

    /// Channel for receiving notifications when the list of sessions has changed.
    rx_sessions: watch::Receiver<Option<Vec<String>>>,

    /// Handle for the task that periodically polls `tmux` for session/pane information. This task
    /// is automatically aborted when the last `Tmux` instance is dropped.
    #[allow(unused)]
    handle: AbortOnDropHandle<()>,
}

#[derive(Default)]
struct Inner {
    active_session: RwLock<Option<String>>,
    sessions: RwLock<Arc<BTreeMap<String, Vec<String>>>>,
    panes: DashMap<String, Vec<String>>,
}

impl Tmux {
    /// Create a new client for extracting information from `tmux`.
    ///
    /// This operation spawns a task that periodically polls `tmux` for the list of panes and for
    /// their contents, according to `poll_interval`.
    ///
    /// This can fail if `tmux` is not found on `$PATH`.
    pub fn new(poll_interval: Duration) -> anyhow::Result<Self> {
        ensure!(which("tmux").is_ok(), "'tmux' not found in PATH");

        let inner = Arc::new(Inner::default());
        let (tx, rx) = watch::channel(None);
        let handle = tokio::spawn(refresh(inner.clone(), tx, poll_interval));

        Ok(Self(Arc::new(State {
            inner,
            rx_sessions: rx,
            handle: AbortOnDropHandle::new(handle),
        })))
    }

    /// Set the currently focussed tmux session. Panes from this session will have their contents
    /// periodically updated.
    pub fn focus_session(&self, session: Option<String>) {
        let mut active = self.0.inner.active_session.write().unwrap();
        *active = session;
    }

    /// Subscribe to changes in the list of tmux sessions. The returned `Sessions` instance will
    /// trigger whenever the session list is updated by the background refresh task. The session list
    /// is provided as a slice of session names, ordered alphabetically.
    pub fn sessions(&self) -> Sessions {
        let mut rx = self.0.rx_sessions.clone();
        rx.mark_changed();
        Sessions { current: None, rx }
    }

    /// Render a stacked pane preview for the active session.
    ///
    /// Returns an empty string when no pane list is available for the requested session. Pane
    /// contents are read from the most recent background refresh and rendered with [`preview`].
    pub fn preview(&self, session: &str, width: usize, height: usize) -> String {
        let Some(pane_ids) = ({
            let sessions = self.0.inner.sessions.read().unwrap();
            sessions.get(session).cloned()
        }) else {
            return "".to_owned();
        };

        let panes: Vec<_> = pane_ids
            .iter()
            .filter_map(|id| self.0.inner.panes.get(id))
            .collect();

        preview(width, height, MIN_PANE_HEIGHT, panes.into_iter())
    }
}

impl Sessions {
    /// Wait until the session list has changed relative to the last time this method was called.
    pub async fn has_changed(&mut self) -> Result<&[String], watch::error::RecvError> {
        self.current = self
            .rx
            .wait_for(|sessions| {
                let Some(new) = sessions else {
                    return false;
                };

                let Some(curr) = self.current.as_ref() else {
                    return true;
                };

                new != curr
            })
            .await?
            .clone();

        // SAFETY: The `wait_for` condition guarantees that the session list will not be `None`.
        Ok(self.current.as_ref().unwrap())
    }
}

/// Background polling loop that keeps session/pane state fresh.
///
/// Each outer-loop iteration refreshes the global session->pane map, then snapshots the active
/// session and walks its pane IDs to refresh pane content. Before refreshing each pane, it
/// re-checks the active session; if focus has changed, it aborts the inner loop and starts the
/// next outer iteration.
///
/// Polling cadence is driven by `tokio::time::interval`: one tick before the session refresh and
/// one tick before each pane refresh.
///
/// Whenever the session list is refreshed, a notification is sent on `tx` with the new session
/// list.
async fn refresh(
    state: Arc<Inner>,
    tx: watch::Sender<Option<Vec<String>>>,
    poll_interval: Duration,
) {
    let mut interval = tokio::time::interval(poll_interval);

    loop {
        interval.tick().await;

        let Ok(sessions) = sessions()
            .await
            .inspect_err(|e| error!("tmux session list refresh failed: {e:?}"))
        else {
            continue;
        };

        // Notify listeners of the new session list.
        tx.send(Some(sessions.keys().cloned().collect())).ok();

        {
            let mut state = state.sessions.write().unwrap();
            *state = Arc::new(sessions);
        }

        let Some(session) = state.active_session.read().unwrap().clone() else {
            continue;
        };

        let pane_ids = {
            let sessions = state.sessions.read().unwrap();
            sessions.get(&session).cloned()
        };

        for pane_id in pane_ids.into_iter().flatten() {
            interval.tick().await;

            let is_same_session = {
                let current = state.active_session.read().unwrap();
                current.as_deref() == Some(session.as_str())
            };

            if !is_same_session {
                break;
            }

            let Ok(pane) = pane_content(&pane_id)
                .await
                .inspect_err(|e| error!(pane_id, "failed to capture pane: {e:?}"))
            else {
                continue;
            };

            state.panes.insert(pane_id, pane);
        }
    }
}

/// Query `tmux` for the current pane IDs in every session.
///
/// Returns a map from session name to pane IDs, preserving the order returned by `tmux list-panes
/// -a` within each session.
///
/// Malformed rows are skipped. Returns an error if invoking `tmux` fails or if `tmux` exits with a
/// non-zero status.
async fn sessions() -> anyhow::Result<BTreeMap<String, Vec<String>>> {
    let output = Command::new("tmux")
        .args(["list-panes", "-a", "-F", "#{session_name}\t#{pane_id}"])
        .output()
        .await
        .context("failed to run 'tmux list-panes'")?;

    ensure!(
        output.status.success(),
        "error running 'tmux list-panes': {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let mut sessions = BTreeMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some((session, pane)) = line.split_once('\t') else {
            continue;
        };

        sessions
            .entry(session.to_owned())
            .or_insert(vec![])
            .push(pane.to_owned());
    }

    Ok(sessions)
}

/// Capture the visible and scrollback text for a pane.
///
/// Runs `tmux capture-pane -ep -t <pane_id>` and returns the captured output as individual lines.
/// Returns an error if invoking `tmux` fails or if `tmux` exits with a non-zero status.
async fn pane_content(pane_id: &str) -> anyhow::Result<Vec<String>> {
    let output = Command::new("tmux")
        .args(["capture-pane", "-ep", "-t", pane_id])
        .output()
        .await
        .context("failed to run 'tmux capture-pane'")?;

    ensure!(
        output.status.success(),
        "error running 'tmux capture-pane': {}",
        String::from_utf8_lossy(&output.stderr),
    );

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.to_owned())
        .collect())
}
