//! Folder colors the user picks (right-click → Folder color), kept by path
//! in `folder-colors.json` next to the settings. Nothing is written into
//! the folders themselves, so other apps don't see them.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The colors a folder can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FolderColor {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Pink,
    Grey,
}

impl FolderColor {
    /// In the order the menu offers them.
    pub const ALL: [FolderColor; 8] = [
        FolderColor::Red,
        FolderColor::Orange,
        FolderColor::Yellow,
        FolderColor::Green,
        FolderColor::Blue,
        FolderColor::Purple,
        FolderColor::Pink,
        FolderColor::Grey,
    ];
}

pub type FolderColors = HashMap<PathBuf, FolderColor>;

/// The saved colors; empty if there are none or the file can't be read
/// (colors are a convenience, never a reason not to start).
pub fn load() -> FolderColors {
    load_from(&colors_path())
}

pub fn save(colors: &FolderColors) -> Result<(), String> {
    save_to(&colors_path(), colors)
}

/// After `from` was renamed or moved to `to`: its color, and those of the
/// folders inside it, follow it. Returns whether anything changed.
pub fn moved(colors: &mut FolderColors, from: &Path, to: &Path) -> bool {
    let moving: Vec<PathBuf> = colors
        .keys()
        .filter(|path| path.starts_with(from))
        .cloned()
        .collect();
    for old in &moving {
        if let Some(color) = colors.remove(old) {
            let rest = old.strip_prefix(from).unwrap_or(Path::new(""));
            let new = if rest.as_os_str().is_empty() {
                to.to_path_buf()
            } else {
                to.join(rest)
            };
            colors.insert(new, color);
        }
    }
    !moving.is_empty()
}

fn load_from(path: &Path) -> FolderColors {
    let Ok(contents) = fs::read_to_string(path) else {
        return FolderColors::new();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

fn save_to(path: &Path, colors: &FolderColors) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    // Sorted, so the file stays readable and diffs stay small.
    let sorted: std::collections::BTreeMap<_, _> = colors.iter().collect();
    let json = serde_json::to_string_pretty(&sorted)
        .map_err(|err| format!("failed to serialize folder colors: {err}"))?;
    fs::write(path, json).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn colors_path() -> PathBuf {
    crate::settings::config_dir().join("folder-colors.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("folder-colors.json");
        let mut colors = FolderColors::new();
        colors.insert(PathBuf::from("/home/me/Work"), FolderColor::Red);
        colors.insert(PathBuf::from("/home/me/Photos 2024"), FolderColor::Green);

        save_to(&path, &colors).unwrap();

        assert_eq!(load_from(&path), colors);
        assert!(load_from(&dir.path().join("missing.json")).is_empty());
    }

    #[test]
    fn colors_follow_a_renamed_folder_and_its_subfolders() {
        let mut colors = FolderColors::new();
        colors.insert(PathBuf::from("/a/project"), FolderColor::Blue);
        colors.insert(PathBuf::from("/a/project/docs"), FolderColor::Pink);
        colors.insert(PathBuf::from("/a/projects"), FolderColor::Grey);

        assert!(moved(
            &mut colors,
            Path::new("/a/project"),
            Path::new("/b/work")
        ));

        assert_eq!(colors.get(Path::new("/b/work")), Some(&FolderColor::Blue));
        assert_eq!(
            colors.get(Path::new("/b/work/docs")),
            Some(&FolderColor::Pink)
        );
        // A different folder whose name merely starts the same stays put.
        assert_eq!(
            colors.get(Path::new("/a/projects")),
            Some(&FolderColor::Grey)
        );
        assert!(!moved(
            &mut colors,
            Path::new("/nothing"),
            Path::new("/else")
        ));
    }
}
