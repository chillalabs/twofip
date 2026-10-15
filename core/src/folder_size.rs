use std::cmp::Reverse;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub children: Vec<Entry>,
}

/// Builds a size-sorted hierarchy for a local directory.
pub fn scan(root: &Path) -> Result<Entry, String> {
    if crate::vfs::is_remote(root) {
        return Err("folder size is not available for server folders".to_string());
    }

    let metadata = fs::symlink_metadata(root)
        .map_err(|err| format!("failed to read {}: {err}", root.display()))?;
    if !metadata.is_dir() {
        return Err(format!("{} is not a directory", root.display()));
    }

    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| root.display().to_string());
    // Extra threads for subfolders, on top of the calling one.
    let spare_threads = std::thread::available_parallelism().map_or(1, |n| n.get()) - 1;
    scan_directory(root, name, &AtomicUsize::new(spare_threads))
}

/// Takes one of the spare threads, if any is left.
fn take_thread(spare: &AtomicUsize) -> bool {
    spare
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_sub(1))
        .is_ok()
}

/// Scans `path` recursively. Subfolders go to spare threads while there are
/// any (the rest are scanned on this one), so a big tree uses every core.
fn scan_directory(path: &Path, name: String, spare: &AtomicUsize) -> Result<Entry, String> {
    let read_dir =
        fs::read_dir(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let mut children = Vec::new();
    let mut subfolders = Vec::new();

    for entry in read_dir.flatten() {
        // The type comes with the listing, and the size is read relative to
        // the open folder: no extra lookup of the full path. Neither follows
        // symbolic links.
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let child_name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_dir() {
            subfolders.push((entry.path(), child_name));
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        children.push(Entry {
            name: child_name,
            path: entry.path(),
            size: metadata.len(),
            is_dir: false,
            children: Vec::new(),
        });
    }

    std::thread::scope(|scope| {
        let mut threads = Vec::new();
        for (child_path, child_name) in subfolders {
            if take_thread(spare) {
                threads.push(scope.spawn(move || {
                    let result = scan_directory(&child_path, child_name, spare);
                    spare.fetch_add(1, Ordering::AcqRel);
                    result
                }));
            } else if let Ok(child) = scan_directory(&child_path, child_name, spare) {
                children.push(child);
            }
        }
        for thread in threads {
            if let Ok(Ok(child)) = thread.join() {
                children.push(child);
            }
        }
    });

    // Largest first; the lower-case names are made once per entry, not on
    // every comparison.
    children.sort_by_cached_key(|child| (Reverse(child.size), child.name.to_lowercase()));
    let size = children.iter().map(|child| child.size).sum();

    Ok(Entry {
        name,
        path: path.to_path_buf(),
        size,
        is_dir: true,
        children,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregates_directory_sizes_and_sorts_children() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("small.txt"), b"123").unwrap();
        fs::write(root.join("nested").join("large.txt"), b"12345678").unwrap();
        fs::write(root.join("nested").join("tiny.txt"), b"1").unwrap();

        let tree = scan(&root).unwrap();

        assert_eq!(tree.size, 12);
        assert!(tree.is_dir);
        assert_eq!(tree.children[0].name, "nested");
        assert_eq!(tree.children[0].size, 9);
        assert_eq!(tree.children[0].children[0].name, "large.txt");
        assert_eq!(tree.children[1].name, "small.txt");
        assert!(!tree.children[1].is_dir);
    }
}
