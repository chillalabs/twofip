use std::path::Path;

use tokio::fs;

use crate::entry::{DirEntry, EntryKind};
use crate::error::FsError;

/// Lists the immediate children of `path`. Does not recurse into subdirectories.
///
/// Symlinks are reported as `EntryKind::Symlink` without following them, so
/// a symlink pointing at a directory is not (yet) treated as navigable.
pub async fn list_dir(path: &Path) -> Result<Vec<DirEntry>, FsError> {
    if crate::vfs::is_remote(path) {
        return crate::vfs::list(path).await.map_err(FsError::Remote);
    }
    let mut read_dir = fs::read_dir(path)
        .await
        .map_err(|source| FsError::ReadDir {
            path: path.to_path_buf(),
            source,
        })?;

    let mut entries = Vec::new();
    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|source| FsError::ReadDir {
            path: path.to_path_buf(),
            source,
        })?
    {
        let entry_path = entry.path();
        let metadata = entry.metadata().await.map_err(|source| FsError::Metadata {
            path: entry_path.clone(),
            source,
        })?;

        let kind = if metadata.is_symlink() {
            EntryKind::Symlink
        } else if metadata.is_dir() {
            EntryKind::Dir
        } else {
            EntryKind::File
        };

        entries.push(DirEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry_path,
            kind,
            size: metadata.len(),
            modified: metadata.modified().ok(),
        });
    }

    Ok(entries)
}

/// Entries for a list of paths, e.g. search results shown in a tab: each
/// one named by its path relative to `root` (`docs/notes.txt`), so files
/// from different folders can be told apart. Paths that no longer exist are
/// left out. Local paths only.
pub async fn list_paths(root: &Path, paths: &[std::path::PathBuf]) -> Vec<DirEntry> {
    let mut entries = Vec::with_capacity(paths.len());
    for path in paths {
        let Ok(metadata) = fs::symlink_metadata(path).await else {
            continue;
        };
        let kind = if metadata.is_symlink() {
            EntryKind::Symlink
        } else if metadata.is_dir() {
            EntryKind::Dir
        } else {
            EntryKind::File
        };
        let name = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
        entries.push(DirEntry {
            name,
            path: path.clone(),
            kind,
            size: metadata.len(),
            modified: metadata.modified().ok(),
        });
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lists_paths_relative_to_the_root_and_skips_missing_ones() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("a.txt"), b"abc").unwrap();
        let paths = vec![sub.join("a.txt"), dir.path().join("gone.txt"), sub.clone()];

        let entries = list_paths(dir.path(), &paths).await;

        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].name,
            Path::new("sub").join("a.txt").to_string_lossy()
        );
        assert_eq!(entries[0].size, 3);
        assert_eq!(entries[1].kind, EntryKind::Dir);
    }
    use std::fs;

    #[tokio::test]
    async fn lists_files_and_dirs() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();

        let mut entries = list_dir(dir.path()).await.unwrap();
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "a.txt");
        assert_eq!(entries[0].kind, EntryKind::File);
        assert_eq!(entries[0].size, 5);
        assert_eq!(entries[1].name, "sub");
        assert_eq!(entries[1].kind, EntryKind::Dir);
    }

    #[tokio::test]
    async fn missing_dir_errors() {
        let missing = std::path::PathBuf::from("/nonexistent/2fip-test-path");
        assert!(list_dir(&missing).await.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn detects_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, b"data").unwrap();
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let entries = list_dir(dir.path()).await.unwrap();
        let link_entry = entries.iter().find(|e| e.name == "link.txt").unwrap();
        assert_eq!(link_entry.kind, EntryKind::Symlink);
    }
}
