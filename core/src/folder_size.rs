use std::fs;
use std::path::{Path, PathBuf};

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
    scan_directory(root, name)
}

fn scan_directory(path: &Path, name: String) -> Result<Entry, String> {
    let read_dir =
        fs::read_dir(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let mut children = Vec::new();

    for entry in read_dir.flatten() {
        let child_path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&child_path) else {
            continue;
        };
        let child_name = entry.file_name().to_string_lossy().into_owned();
        let child = if metadata.is_dir() {
            match scan_directory(&child_path, child_name) {
                Ok(child) => child,
                Err(_) => continue,
            }
        } else {
            Entry {
                name: child_name,
                path: child_path,
                size: metadata.len(),
                is_dir: false,
                children: Vec::new(),
            }
        };
        children.push(child);
    }

    children.sort_by(|left, right| {
        right
            .size
            .cmp(&left.size)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
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
