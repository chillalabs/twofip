//! Copy and paste into the same folder (Ctrl+C, Ctrl+V): each item is
//! duplicated next to itself as "name - Copy.ext", then "name - Copy (2).ext"
//! and so on, as Windows Explorer does.

use std::path::{Path, PathBuf};

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;

use super::engine::{copy_planned_files, StopReason};
use super::plan::plan_tree;
use super::{CancelHandle, ConflictHandle, OpEvent};

/// Duplicates each of `sources` (files or folders) in its own folder.
/// `suffix` is the translated word for "Copy".
pub fn duplicate(
    sources: Vec<PathBuf>,
    suffix: String,
    cancel: CancelHandle,
    conflict: ConflictHandle,
) -> impl Stream<Item = OpEvent> {
    let (tx, rx) = mpsc::channel(16);
    tokio::spawn(run(sources, suffix, cancel, conflict, tx));
    ReceiverStream::new(rx)
}

async fn run(
    sources: Vec<PathBuf>,
    suffix: String,
    cancel: CancelHandle,
    conflict: ConflictHandle,
    tx: mpsc::Sender<OpEvent>,
) {
    let planned = tokio::task::spawn_blocking(move || {
        let mut planned = Vec::new();
        let mut taken = Vec::new();
        for source in &sources {
            let target = copy_name(source, &suffix, &taken);
            // Every folder, empty ones too (the file plan only has files).
            if source.is_dir() && !source.is_symlink() {
                for entry in walkdir::WalkDir::new(source) {
                    let entry = entry
                        .map_err(|err| format!("failed to walk {}: {err}", source.display()))?;
                    if entry.file_type().is_dir() {
                        let rel = entry.path().strip_prefix(source).unwrap_or(entry.path());
                        std::fs::create_dir_all(target.join(rel)).map_err(|err| {
                            format!("failed to create {}: {err}", target.join(rel).display())
                        })?;
                    }
                }
            }
            planned.extend(plan_tree(source, &target)?);
            taken.push(target);
        }
        Ok::<_, String>(planned)
    })
    .await
    .map_err(|err| format!("copy planning task failed: {err}"))
    .and_then(|planned| planned);
    let planned = match planned {
        Ok(planned) => planned,
        Err(err) => {
            let _ = tx.send(OpEvent::Error(err)).await;
            return;
        }
    };
    let files_total = planned.len();
    let bytes_total: u64 = planned.iter().map(|file| file.size).sum();
    let event =
        match copy_planned_files(&planned, files_total, bytes_total, &cancel, &conflict, &tx).await
        {
            Ok(()) => OpEvent::Done,
            Err(StopReason::Cancelled) => OpEvent::Cancelled,
            Err(StopReason::Error(err)) => OpEvent::Error(err),
        };
    let _ = tx.send(event).await;
}

/// The first free "name - Copy.ext" / "name - Copy (n).ext" next to
/// `source` (not on disk, and not in `taken`).
fn copy_name(source: &Path, suffix: &str, taken: &[PathBuf]) -> PathBuf {
    let parent = source.parent().unwrap_or(Path::new(""));
    let file_name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    // Folders keep dots in their names; files split off the extension
    // (".bashrc" has none).
    let (stem, extension) = match file_name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() && !source.is_dir() => {
            (stem.to_string(), Some(extension.to_string()))
        }
        _ => (file_name.clone(), None),
    };
    let name = |number: usize| {
        let copy = match number {
            1 => format!("{stem} - {suffix}"),
            n => format!("{stem} - {suffix} ({n})"),
        };
        match &extension {
            Some(extension) => format!("{copy}.{extension}"),
            None => copy,
        }
    };
    (1..)
        .map(|number| parent.join(name(number)))
        .find(|candidate| {
            !candidate.exists() && !candidate.is_symlink() && !taken.contains(candidate)
        })
        .unwrap_or_else(|| parent.join(name(1)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::StreamExt;

    #[test]
    fn names_follow_explorer() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("readme.txt");
        std::fs::write(&file, b"x").unwrap();
        let folder = dir.path().join("photos.2024");
        std::fs::create_dir(&folder).unwrap();
        let dotfile = dir.path().join(".bashrc");
        std::fs::write(&dotfile, b"x").unwrap();

        assert_eq!(
            copy_name(&file, "Copy", &[]),
            dir.path().join("readme - Copy.txt")
        );
        assert_eq!(
            copy_name(&folder, "Copy", &[]),
            dir.path().join("photos.2024 - Copy")
        );
        assert_eq!(
            copy_name(&dotfile, "Copia", &[]),
            dir.path().join(".bashrc - Copia")
        );

        std::fs::write(dir.path().join("readme - Copy.txt"), b"x").unwrap();
        assert_eq!(
            copy_name(&file, "Copy", &[]),
            dir.path().join("readme - Copy (2).txt")
        );
        let taken = [dir.path().join("readme - Copy (2).txt")];
        assert_eq!(
            copy_name(&file, "Copy", &taken),
            dir.path().join("readme - Copy (3).txt")
        );
    }

    #[tokio::test]
    async fn duplicates_files_and_folders_next_to_themselves() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("readme.txt"), b"hello").unwrap();
        std::fs::create_dir_all(dir.path().join("a/empty")).unwrap();
        std::fs::write(dir.path().join("a/inside.txt"), b"in").unwrap();

        let events: Vec<OpEvent> = duplicate(
            vec![dir.path().join("readme.txt"), dir.path().join("a")],
            "Copy".to_string(),
            CancelHandle::new(),
            ConflictHandle::new(),
        )
        .collect()
        .await;

        assert!(matches!(events.last(), Some(OpEvent::Done)));
        assert_eq!(
            std::fs::read(dir.path().join("readme - Copy.txt")).unwrap(),
            b"hello"
        );
        assert_eq!(
            std::fs::read(dir.path().join("a - Copy/inside.txt")).unwrap(),
            b"in"
        );
        assert!(dir.path().join("a - Copy/empty").is_dir());
        // The originals are untouched.
        assert_eq!(
            std::fs::read(dir.path().join("readme.txt")).unwrap(),
            b"hello"
        );
    }
}
