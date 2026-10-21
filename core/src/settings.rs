//! User preferences, persisted as JSON next to the favorites.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Hide dotfiles (names starting with `.`) from the file listings.
    pub hide_hidden_files: bool,
    /// UI language: "system" (follow the desktop), "en", "es" (Spain) or
    /// "es-419" (Latin America).
    pub language: String,
    pub icon_style: IconStyle,
    /// Show previews of images, PDFs, videos, ... instead of type icons.
    pub show_thumbnails: bool,
    /// List view: a thin line between rows, like COSMIC Files.
    pub row_lines: bool,
    /// Text size of file and folder names in the listings.
    pub name_font_size: FontSize,
    /// List view: show the extension in its own "Ext" column (like Total
    /// Commander) instead of as part of the name.
    pub separate_extension: bool,
    /// 2fip's colors: the desktop's theme, or one of 2fip's own.
    pub color_theme: ColorTheme,
    /// How rounded the window, buttons, menus and panels are.
    pub corners: Corners,
    /// Which panels show the folder tree beside their files.
    pub folder_tree: TreeMode,
    /// The F4 editor: a desktop app id, or empty for the system's text
    /// editor.
    pub editor: String,
}

/// Which panels show the folder tree ("Show hierarchy"); the toolbar button
/// goes through them in this order.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TreeMode {
    #[default]
    Off,
    Left,
    Right,
    Both,
}

impl TreeMode {
    /// The next one: off → left → right → both → off.
    pub fn next(self) -> Self {
        match self {
            TreeMode::Off => TreeMode::Left,
            TreeMode::Left => TreeMode::Right,
            TreeMode::Right => TreeMode::Both,
            TreeMode::Both => TreeMode::Off,
        }
    }

    pub fn left(self) -> bool {
        matches!(self, TreeMode::Left | TreeMode::Both)
    }

    pub fn right(self) -> bool {
        matches!(self, TreeMode::Right | TreeMode::Both)
    }
}

/// Roundness of the window, buttons, menus and panels, from square corners
/// to COSMIC's standard round look.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Corners {
    Square,
    Small,
    Medium,
    /// COSMIC's standard roundness.
    #[default]
    Large,
}

/// 2fip's color theme (only this app; the desktop keeps its own). Saved as
/// kebab-case names ("tokyo-night-storm"); the one-word names are unchanged
/// from earlier versions ("system", "dracula").
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorTheme {
    /// Follow COSMIC's appearance (light/dark, accent color).
    #[default]
    System,
    /// COSMIC's standard light theme, whatever the desktop uses.
    Light,
    /// COSMIC's standard dark theme, whatever the desktop uses.
    Dark,
    /// A flatter, Windows-like light look: blue accent, light-gray surfaces.
    Windows,
    /// The Dracula palette (dark).
    Dracula,
    Everforest,
    GruvboxMaterial,
    Nord,
    TokyoNightStorm,
    CatppuccinMocha,
    CatppuccinMacchiato,
    AyuDark,
    /// Green-on-black.
    Matrix,
    MonokaiPro,
    SolarizedDark,
    GruvboxDark,
}

/// Text size for file and folder names; smaller sizes fit more files on
/// screen (the list's rows get shorter too).
/// Saved as kebab-case names; the names of earlier versions still load
/// ("default" is Medium, "tiny" was 11 px and is now Very small, the
/// short-lived 9 px "smallest" is now Tiny).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontSize {
    /// libcosmic's normal body text size, and the largest (the short-lived
    /// "large" and "larger" load as Medium).
    #[default]
    #[serde(alias = "default", alias = "large", alias = "larger")]
    Medium,
    Small,
    Smaller,
    #[serde(alias = "tiny")]
    VerySmall,
    /// The smallest size (10 px), shown as "Tiny".
    #[serde(alias = "smallest")]
    ExtraSmall,
}

impl FontSize {
    /// Size in pixels.
    pub fn px(self) -> u16 {
        match self {
            FontSize::Medium => 14,
            FontSize::Small => 13,
            FontSize::Smaller => 12,
            FontSize::VerySmall => 11,
            FontSize::ExtraSmall => 10,
        }
    }
}

/// Widths of the list view's columns in pixels; Name takes the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ColumnWidths {
    pub ext: u16,
    pub size: u16,
    pub modified: u16,
}

impl Default for ColumnWidths {
    fn default() -> Self {
        Self {
            ext: 70,
            size: 120,
            modified: 180,
        }
    }
}

/// How a pane lays out its files (saved per pane in the session).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    /// Rows with Name / Size / Modified columns.
    #[default]
    List,
    /// Large icons with the name below, wrapping to fill the width.
    Grid,
}

/// How file and folder icons are drawn in the listings.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconStyle {
    /// Full-color icons from the icon theme, like COSMIC Files.
    #[default]
    Colorful,
    /// Single-color symbolic icons.
    Monochrome,
    /// 2fip's own bright icons: a color and symbol per file type, a color
    /// per special folder.
    Vivid,
    /// 2fip's own icons in a Windows-like look: yellow folders with a
    /// colored badge, white pages with a colored symbol.
    Classic,
    /// 2fip's own icons in a macOS-like look: light-blue folders, white
    /// pages with a colored symbol and band.
    Soft,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hide_hidden_files: true,
            language: "system".to_string(),
            icon_style: IconStyle::default(),
            show_thumbnails: true,
            row_lines: false,
            name_font_size: FontSize::default(),
            separate_extension: true,
            color_theme: ColorTheme::default(),
            corners: Corners::default(),
            folder_tree: TreeMode::default(),
            editor: String::new(),
        }
    }
}

/// Loads the saved settings, falling back to defaults if none have been
/// saved yet or the file can't be parsed. Fields missing from an older file
/// get their default value.
pub fn load() -> Settings {
    load_from(&settings_path())
}

/// Persists `settings`, creating the config directory if needed.
pub fn save(settings: &Settings) -> Result<(), String> {
    save_to(&settings_path(), settings)
}

fn load_from(path: &Path) -> Settings {
    let Some(value) = fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str::<serde_json::Value>(&contents).ok())
    else {
        return Settings::default();
    };
    // Up to 0.2.4 the Windows look was a checkbox on top of the color theme.
    let windows_style = value.get("windows_style") == Some(&serde_json::Value::Bool(true));
    let mut settings: Settings = serde_json::from_value(value).unwrap_or_default();
    if windows_style {
        settings.color_theme = ColorTheme::Windows;
    }
    settings
}

fn save_to(path: &Path, settings: &Settings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(settings)
        .map_err(|err| format!("failed to serialize settings: {err}"))?;
    fs::write(path, json).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

/// The app's config folder name under `~/.config`.
const APP_DIR: &str = "2fip";
/// The folders the app used under its old names (pa2, and before that
/// cosmic-commander), newest first.
const LEGACY_APP_DIRS: [&str; 2] = ["pa2", "cosmic-commander"];

/// `$XDG_CONFIG_HOME/2fip`, or `~/.config/2fip`.
pub(crate) fn config_dir() -> PathBuf {
    config_base().join(APP_DIR)
}

/// `$XDG_CONFIG_HOME`, or `~/.config` (see [`crate::sandbox`] for Flatpak).
fn config_base() -> PathBuf {
    crate::sandbox::config_home()
}

/// Moves the settings, favorites and session saved under one of the app's old
/// names (`~/.config/pa2` or `~/.config/cosmic-commander`) to its current
/// folder, once: only if the new one doesn't exist yet, taking the newest old
/// folder there is. Call before loading anything.
pub fn migrate_legacy_config() {
    if let Err(err) = migrate_in(&config_base()) {
        eprintln!("failed to move the old settings folder: {err}");
    }
}

fn migrate_in(base: &Path) -> std::io::Result<()> {
    let new = base.join(APP_DIR);
    if new.exists() {
        return Ok(());
    }
    let old = LEGACY_APP_DIRS
        .iter()
        .map(|name| base.join(name))
        .find(|old| old.is_dir());
    if let Some(old) = old {
        fs::rename(old, new)?;
    }
    Ok(())
}

/// Whether `name` is a hidden file by Unix convention.
pub fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load_from(&dir.path().join("settings.json")),
            Settings::default()
        );
    }

    #[test]
    fn round_trips_through_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("settings.json");
        let settings = Settings {
            hide_hidden_files: false,
            language: "en".to_string(),
            icon_style: IconStyle::Monochrome,
            show_thumbnails: false,
            row_lines: true,
            name_font_size: FontSize::Smaller,
            separate_extension: false,
            color_theme: ColorTheme::Dracula,
            corners: Corners::Small,
            folder_tree: TreeMode::Both,
            editor: "org.gnome.TextEditor".to_string(),
        };

        save_to(&path, &settings).unwrap();

        assert_eq!(load_from(&path), settings);
    }

    #[test]
    fn missing_fields_get_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, br#"{ "hide_hidden_files": false }"#).unwrap();

        let loaded = load_from(&path);

        assert!(!loaded.hide_hidden_files);
        assert_eq!(loaded.language, "system");
        assert_eq!(loaded.icon_style, IconStyle::Colorful);
        assert_eq!(loaded.name_font_size, FontSize::Medium);
        assert!(loaded.separate_extension);
        assert_eq!(loaded.color_theme, ColorTheme::System);
    }

    #[test]
    fn font_sizes_get_smaller_and_default_matches_libcosmic() {
        assert_eq!(FontSize::Medium.px(), 14);
        let sizes = [
            FontSize::Medium,
            FontSize::Small,
            FontSize::Smaller,
            FontSize::VerySmall,
            FontSize::ExtraSmall,
        ];
        assert!(sizes.windows(2).all(|pair| pair[0].px() > pair[1].px()));
        assert_eq!(FontSize::ExtraSmall.px(), 10);
    }

    #[test]
    fn the_old_windows_checkbox_becomes_the_windows_theme() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            br#"{ "color_theme": "nord", "windows_style": true }"#,
        )
        .unwrap();
        assert_eq!(load_from(&path).color_theme, ColorTheme::Windows);
    }

    #[test]
    fn old_font_size_names_still_load() {
        let old: FontSize = serde_json::from_str(r#""default""#).unwrap();
        assert_eq!(old, FontSize::Medium);
        let old: FontSize = serde_json::from_str(r#""tiny""#).unwrap();
        assert_eq!(old, FontSize::VerySmall);
        // 9 px, removed again in 0.2.5.
        let old: FontSize = serde_json::from_str(r#""smallest""#).unwrap();
        assert_eq!(old, FontSize::ExtraSmall);
        // 16 and 18 px, never released: back to the normal size.
        for name in [r#""large""#, r#""larger""#] {
            let old: FontSize = serde_json::from_str(name).unwrap();
            assert_eq!(old, FontSize::Medium);
        }
    }

    #[test]
    fn moves_the_old_config_folder_once() {
        let base = tempfile::tempdir().unwrap();
        let old = base.path().join(LEGACY_APP_DIRS[1]);
        fs::create_dir(&old).unwrap();
        fs::write(old.join("favorites.json"), b"[]").unwrap();

        migrate_in(base.path()).unwrap();
        let new = base.path().join(APP_DIR);
        assert!(new.join("favorites.json").exists());
        assert!(!old.exists());

        // An existing new folder is never overwritten by a stale old one.
        fs::create_dir(&old).unwrap();
        fs::write(old.join("favorites.json"), b"stale").unwrap();
        migrate_in(base.path()).unwrap();
        assert_eq!(fs::read(new.join("favorites.json")).unwrap(), b"[]");
    }

    #[test]
    fn prefers_the_newest_old_config_folder() {
        let base = tempfile::tempdir().unwrap();
        for (name, content) in [("pa2", "newest"), ("cosmic-commander", "oldest")] {
            let old = base.path().join(name);
            fs::create_dir(&old).unwrap();
            fs::write(old.join("favorites.json"), content).unwrap();
        }

        migrate_in(base.path()).unwrap();
        let new = base.path().join(APP_DIR);
        assert_eq!(fs::read(new.join("favorites.json")).unwrap(), b"newest");
        assert!(!base.path().join("pa2").exists());
    }

    #[test]
    fn color_themes_are_saved_by_name() {
        let saved = serde_json::to_string(&ColorTheme::TokyoNightStorm).unwrap();
        assert_eq!(saved, "\"tokyo-night-storm\"");
        let old: ColorTheme = serde_json::from_str("\"dracula\"").unwrap();
        assert_eq!(old, ColorTheme::Dracula);
    }

    #[test]
    fn dotfiles_are_hidden() {
        assert!(is_hidden(".bashrc"));
        assert!(!is_hidden("notes.txt"));
    }
}
