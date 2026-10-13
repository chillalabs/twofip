//! Persistent, user-named directory bookmarks (Total Commander's "hotlist").

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Favorite {
    pub name: String,
    pub path: PathBuf,
}

/// Why a favorite can't be added: one already saved clashes with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateReason {
    /// That folder is already saved.
    Path,
    /// Another favorite already uses that name.
    Name,
}

/// The existing favorite `name`/`path` would duplicate, and why. `name` is
/// expected to be trimmed already. Path wins over name: if both clash, the
/// entry found is the same one either way.
pub fn find_duplicate(
    favorites: &[Favorite],
    name: &str,
    path: &Path,
) -> Option<(usize, DuplicateReason)> {
    if let Some(index) = favorites.iter().position(|favorite| favorite.path == path) {
        return Some((index, DuplicateReason::Path));
    }
    favorites
        .iter()
        .position(|favorite| favorite.name == name)
        .map(|index| (index, DuplicateReason::Name))
}

/// Loads the persisted favorites list. Returns an empty list if none has
/// been saved yet or the file can't be parsed - favorites are a convenience,
/// not critical state, so a corrupt file shouldn't stop the app from starting.
pub fn load() -> Vec<Favorite> {
    load_from(&favorites_path())
}

/// Persists the favorites list, creating the config directory if needed.
pub fn save(favorites: &[Favorite]) -> Result<(), String> {
    save_to(&favorites_path(), favorites)
}

fn load_from(path: &Path) -> Vec<Favorite> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

fn save_to(path: &Path, favorites: &[Favorite]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(favorites)
        .map_err(|err| format!("failed to serialize favorites: {err}"))?;
    fs::write(path, json).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn favorites_path() -> PathBuf {
    crate::settings::config_dir().join("favorites.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("favorites.json");

        assert!(load_from(&path).is_empty());
    }

    #[test]
    fn round_trips_through_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("favorites.json");
        let favorites = vec![
            Favorite {
                name: "Projects".to_string(),
                path: PathBuf::from("/home/user/projects"),
            },
            Favorite {
                name: "Downloads".to_string(),
                path: PathBuf::from("/home/user/downloads"),
            },
        ];

        save_to(&path, &favorites).unwrap();
        let loaded = load_from(&path);

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].name, "Projects");
        assert_eq!(loaded[0].path, PathBuf::from("/home/user/projects"));
        assert_eq!(loaded[1].name, "Downloads");
    }

    #[test]
    fn corrupt_file_yields_empty_list_instead_of_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("favorites.json");
        fs::write(&path, b"not json").unwrap();

        assert!(load_from(&path).is_empty());
    }

    fn saved() -> Vec<Favorite> {
        vec![
            Favorite {
                name: "Projects".to_string(),
                path: PathBuf::from("/home/user/projects"),
            },
            Favorite {
                name: "Downloads".to_string(),
                path: PathBuf::from("/home/user/downloads"),
            },
        ]
    }

    #[test]
    fn the_same_folder_under_another_name_is_a_duplicate() {
        assert_eq!(
            find_duplicate(&saved(), "Work", Path::new("/home/user/projects")),
            Some((0, DuplicateReason::Path))
        );
    }

    #[test]
    fn another_folder_under_a_used_name_is_a_duplicate() {
        assert_eq!(
            find_duplicate(&saved(), "Downloads", Path::new("/mnt/usb")),
            Some((1, DuplicateReason::Name))
        );
    }

    #[test]
    fn a_trailing_slash_does_not_hide_a_duplicate() {
        assert_eq!(
            find_duplicate(&saved(), "Work", Path::new("/home/user/projects/")),
            Some((0, DuplicateReason::Path))
        );
    }

    #[test]
    fn a_clash_on_both_name_and_folder_reports_the_folder() {
        assert_eq!(
            find_duplicate(&saved(), "Projects", Path::new("/home/user/projects")),
            Some((0, DuplicateReason::Path))
        );
    }

    #[test]
    fn a_new_name_on_a_new_folder_is_not_a_duplicate() {
        assert_eq!(
            find_duplicate(&saved(), "Music", Path::new("/home/user/music")),
            None
        );
    }

    #[test]
    fn an_empty_list_has_no_duplicates() {
        assert_eq!(
            find_duplicate(&[], "Projects", Path::new("/home/user/projects")),
            None
        );
    }
}
