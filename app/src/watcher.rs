//! Watches the panes' local folders for external changes, so they refresh
//! themselves without polling (Linux: inotify via `notify`; a no-op elsewhere).

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use cosmic::iced::futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use cosmic::iced::futures::stream::unfold;
use cosmic::iced::futures::{Stream, StreamExt};
use cosmic::iced::Subscription;

/// Sent from the watcher's background task to the app.
#[derive(Debug, Clone)]
pub enum WatcherEvent {
    /// The watcher is running; keep this sender and drive it with
    /// [`CommandSender::watch`].
    Ready(CommandSender),
    /// Something changed in one of the watched folders.
    Changed(PathBuf),
}

/// Sends watch commands to the watcher's background task.
#[derive(Debug, Clone)]
pub struct CommandSender(UnboundedSender<Msg>);

impl CommandSender {
    /// Watch exactly this set of folders, unwatching everything else.
    pub fn watch(&self, paths: HashSet<PathBuf>) {
        let _ = self.0.unbounded_send(Msg::Watch(paths));
    }
}

/// What flows through the watcher's channel: commands from the app, change
/// events from the OS.
#[derive(Debug)]
enum Msg {
    Watch(HashSet<PathBuf>),
    Changed(PathBuf),
}

#[cfg(unix)]
pub fn subscription() -> Subscription<WatcherEvent> {
    Subscription::run(build_stream)
}

#[cfg(not(unix))]
pub fn subscription() -> Subscription<WatcherEvent> {
    Subscription::none()
}

#[cfg(unix)]
fn build_stream() -> impl Stream<Item = WatcherEvent> {
    unfold(State::Ready, run)
}

#[cfg(unix)]
enum State {
    Ready,
    Running {
        watcher: notify::RecommendedWatcher,
        rx: UnboundedReceiver<Msg>,
        watched: HashSet<PathBuf>,
    },
}

#[cfg(unix)]
async fn run(state: State) -> Option<(WatcherEvent, State)> {
    match state {
        State::Ready => {
            let (tx, rx) = unbounded();
            let notify_tx = tx.clone();
            let watcher =
                match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                    if let Ok(event) = res {
                        if !is_relevant(event.kind) {
                            return;
                        }
                        for path in event.paths {
                            let _ = notify_tx.unbounded_send(Msg::Changed(path));
                        }
                    }
                }) {
                    Ok(watcher) => watcher,
                    // Couldn't set up inotify (e.g. no watches available); give up.
                    Err(_) => return None,
                };
            Some((
                WatcherEvent::Ready(CommandSender(tx)),
                State::Running {
                    watcher,
                    rx,
                    watched: HashSet::new(),
                },
            ))
        }
        State::Running {
            mut watcher,
            mut rx,
            mut watched,
        } => loop {
            let first = rx.next().await?;
            match first {
                Msg::Watch(want) => {
                    watched = apply_watch(&mut watcher, watched, want);
                }
                Msg::Changed(path) => {
                    // Coalesce a burst: keep draining until 150 ms pass with no
                    // further change, then report a single path.
                    let mut pending = vec![path];
                    loop {
                        match tokio::time::timeout(Duration::from_millis(150), rx.next()).await {
                            Ok(Some(Msg::Changed(p))) => pending.push(p),
                            Ok(Some(Msg::Watch(want))) => {
                                watched = apply_watch(&mut watcher, watched, want);
                            }
                            Ok(None) => return None,
                            Err(_) => break,
                        }
                    }
                    let changed = pending.into_iter().next()?;
                    return Some((
                        WatcherEvent::Changed(changed),
                        State::Running {
                            watcher,
                            rx,
                            watched,
                        },
                    ));
                }
            }
        },
    }
}

#[cfg(unix)]
fn apply_watch(
    watcher: &mut notify::RecommendedWatcher,
    watched: HashSet<PathBuf>,
    want: HashSet<PathBuf>,
) -> HashSet<PathBuf> {
    use notify::Watcher;
    for path in watched.difference(&want) {
        let _ = watcher.unwatch(path);
    }
    for path in want.difference(&watched) {
        let _ = watcher.watch(path, notify::RecursiveMode::NonRecursive);
    }
    want
}

/// Whether a change event means the folder's *listing* changed. We ignore
/// `Access` (open/close/read) — our own `list_dir` opens the folder, which
/// would otherwise loop — and metadata-only modifications.
#[cfg(unix)]
fn is_relevant(kind: notify::EventKind) -> bool {
    use notify::event::ModifyKind;
    matches!(
        kind,
        notify::EventKind::Create(_)
            | notify::EventKind::Remove(_)
            | notify::EventKind::Modify(ModifyKind::Data(_))
            | notify::EventKind::Modify(ModifyKind::Name(_))
    )
}
