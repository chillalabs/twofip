//! Icons for systems without COSMIC's icon theme (Fedora, Ubuntu or KDE
//! without COSMIC, minimal containers): libcosmic looks icons up in the
//! "Cosmic" theme and then only in "hicolor", so most of them were blank.
//!
//! - Outside COSMIC, the desktop's own icon theme (Adwaita, Breeze…) is
//!   used when the Cosmic theme isn't installed.
//! - Every symbolic icon 2fip uses is also built in, as a last resort: at
//!   start they're written to a small "hicolor" folder in the cache, which
//!   the icon lookup merges with the system's hicolor (searched last, so
//!   any icon the system has wins).
//! - With no icon theme at all, files and folders use 2fip's Vivid icons
//!   instead of the theme's (Colorful style).
//!
//! The icons come from the COSMIC, Pop and Adwaita themes (CC BY-SA, see
//! `resources/fallback-icons/README.md`).

use std::sync::atomic::{AtomicBool, Ordering};

/// The built-in icons: (name, SVG).
#[cfg_attr(not(all(unix, not(target_os = "macos"))), allow(dead_code))]
const ICONS: &[(&str, &[u8])] = &[
    (
        "accessories-text-editor-symbolic",
        include_bytes!("../resources/fallback-icons/accessories-text-editor-symbolic.svg"),
    ),
    (
        "application-exit-symbolic",
        include_bytes!("../resources/fallback-icons/application-exit-symbolic.svg"),
    ),
    (
        "disk-usage-analyzer-symbolic",
        include_bytes!("../resources/fallback-icons/disk-usage-analyzer-symbolic.svg"),
    ),
    (
        "document-edit-symbolic",
        include_bytes!("../resources/fallback-icons/document-edit-symbolic.svg"),
    ),
    (
        "document-new-symbolic",
        include_bytes!("../resources/fallback-icons/document-new-symbolic.svg"),
    ),
    (
        "document-open-symbolic",
        include_bytes!("../resources/fallback-icons/document-open-symbolic.svg"),
    ),
    (
        "document-properties-symbolic",
        include_bytes!("../resources/fallback-icons/document-properties-symbolic.svg"),
    ),
    (
        "document-save-symbolic",
        include_bytes!("../resources/fallback-icons/document-save-symbolic.svg"),
    ),
    (
        "document-viewer-symbolic",
        include_bytes!("../resources/fallback-icons/document-viewer-symbolic.svg"),
    ),
    (
        "edit-copy-symbolic",
        include_bytes!("../resources/fallback-icons/edit-copy-symbolic.svg"),
    ),
    (
        "edit-cut-symbolic",
        include_bytes!("../resources/fallback-icons/edit-cut-symbolic.svg"),
    ),
    (
        "edit-delete-symbolic",
        include_bytes!("../resources/fallback-icons/edit-delete-symbolic.svg"),
    ),
    (
        "edit-paste-symbolic",
        include_bytes!("../resources/fallback-icons/edit-paste-symbolic.svg"),
    ),
    (
        "edit-select-all-symbolic",
        include_bytes!("../resources/fallback-icons/edit-select-all-symbolic.svg"),
    ),
    (
        "edit-symbolic",
        include_bytes!("../resources/fallback-icons/edit-symbolic.svg"),
    ),
    (
        "folder-documents-symbolic",
        include_bytes!("../resources/fallback-icons/folder-documents-symbolic.svg"),
    ),
    (
        "folder-download-symbolic",
        include_bytes!("../resources/fallback-icons/folder-download-symbolic.svg"),
    ),
    (
        "folder-music-symbolic",
        include_bytes!("../resources/fallback-icons/folder-music-symbolic.svg"),
    ),
    (
        "folder-new-symbolic",
        include_bytes!("../resources/fallback-icons/folder-new-symbolic.svg"),
    ),
    (
        "folder-open-symbolic",
        include_bytes!("../resources/fallback-icons/folder-open-symbolic.svg"),
    ),
    (
        "folder-pictures-symbolic",
        include_bytes!("../resources/fallback-icons/folder-pictures-symbolic.svg"),
    ),
    (
        "folder-publicshare-symbolic",
        include_bytes!("../resources/fallback-icons/folder-publicshare-symbolic.svg"),
    ),
    (
        "folder-symbolic",
        include_bytes!("../resources/fallback-icons/folder-symbolic.svg"),
    ),
    (
        "folder-templates-symbolic",
        include_bytes!("../resources/fallback-icons/folder-templates-symbolic.svg"),
    ),
    (
        "folder-videos-symbolic",
        include_bytes!("../resources/fallback-icons/folder-videos-symbolic.svg"),
    ),
    (
        "go-down-symbolic",
        include_bytes!("../resources/fallback-icons/go-down-symbolic.svg"),
    ),
    (
        "go-next-symbolic",
        include_bytes!("../resources/fallback-icons/go-next-symbolic.svg"),
    ),
    (
        "go-previous-symbolic",
        include_bytes!("../resources/fallback-icons/go-previous-symbolic.svg"),
    ),
    (
        "go-up-symbolic",
        include_bytes!("../resources/fallback-icons/go-up-symbolic.svg"),
    ),
    (
        "help-about-symbolic",
        include_bytes!("../resources/fallback-icons/help-about-symbolic.svg"),
    ),
    (
        "help-browser-symbolic",
        include_bytes!("../resources/fallback-icons/help-browser-symbolic.svg"),
    ),
    (
        "list-add-symbolic",
        include_bytes!("../resources/fallback-icons/list-add-symbolic.svg"),
    ),
    (
        "media-eject-symbolic",
        include_bytes!("../resources/fallback-icons/media-eject-symbolic.svg"),
    ),
    (
        "network-offline-symbolic",
        include_bytes!("../resources/fallback-icons/network-offline-symbolic.svg"),
    ),
    (
        "network-server-symbolic",
        include_bytes!("../resources/fallback-icons/network-server-symbolic.svg"),
    ),
    (
        "network-workgroup-symbolic",
        include_bytes!("../resources/fallback-icons/network-workgroup-symbolic.svg"),
    ),
    (
        "object-select-symbolic",
        include_bytes!("../resources/fallback-icons/object-select-symbolic.svg"),
    ),
    (
        "package-x-generic-symbolic",
        include_bytes!("../resources/fallback-icons/package-x-generic-symbolic.svg"),
    ),
    (
        "pan-down-symbolic",
        include_bytes!("../resources/fallback-icons/pan-down-symbolic.svg"),
    ),
    (
        "pan-end-symbolic",
        include_bytes!("../resources/fallback-icons/pan-end-symbolic.svg"),
    ),
    (
        "pan-up-symbolic",
        include_bytes!("../resources/fallback-icons/pan-up-symbolic.svg"),
    ),
    (
        "pencil-symbolic",
        include_bytes!("../resources/fallback-icons/pencil-symbolic.svg"),
    ),
    (
        "preferences-system-symbolic",
        include_bytes!("../resources/fallback-icons/preferences-system-symbolic.svg"),
    ),
    (
        "starred-symbolic",
        include_bytes!("../resources/fallback-icons/starred-symbolic.svg"),
    ),
    (
        "system-run-symbolic",
        include_bytes!("../resources/fallback-icons/system-run-symbolic.svg"),
    ),
    (
        "system-search-symbolic",
        include_bytes!("../resources/fallback-icons/system-search-symbolic.svg"),
    ),
    (
        "tab-new-symbolic",
        include_bytes!("../resources/fallback-icons/tab-new-symbolic.svg"),
    ),
    (
        "text-x-generic-symbolic",
        include_bytes!("../resources/fallback-icons/text-x-generic-symbolic.svg"),
    ),
    (
        "user-desktop-symbolic",
        include_bytes!("../resources/fallback-icons/user-desktop-symbolic.svg"),
    ),
    (
        "user-home-symbolic",
        include_bytes!("../resources/fallback-icons/user-home-symbolic.svg"),
    ),
    (
        "user-trash-symbolic",
        include_bytes!("../resources/fallback-icons/user-trash-symbolic.svg"),
    ),
    (
        "view-dual-symbolic",
        include_bytes!("../resources/fallback-icons/view-dual-symbolic.svg"),
    ),
    (
        "view-grid-symbolic",
        include_bytes!("../resources/fallback-icons/view-grid-symbolic.svg"),
    ),
    (
        "view-list-symbolic",
        include_bytes!("../resources/fallback-icons/view-list-symbolic.svg"),
    ),
    (
        "view-refresh-symbolic",
        include_bytes!("../resources/fallback-icons/view-refresh-symbolic.svg"),
    ),
    (
        "window-close-symbolic",
        include_bytes!("../resources/fallback-icons/window-close-symbolic.svg"),
    ),
    (
        "zoom-fit-best-symbolic",
        include_bytes!("../resources/fallback-icons/zoom-fit-best-symbolic.svg"),
    ),
    (
        "zoom-in-symbolic",
        include_bytes!("../resources/fallback-icons/zoom-in-symbolic.svg"),
    ),
    (
        "zoom-out-symbolic",
        include_bytes!("../resources/fallback-icons/zoom-out-symbolic.svg"),
    ),
];

#[cfg_attr(not(all(unix, not(target_os = "macos"))), allow(dead_code))]
const INDEX_THEME: &str = "[Icon Theme]\nName=Hicolor\nComment=2fip's built-in fallback icons\nDirectories=scalable/actions\n\n[scalable/actions]\nSize=16\nMinSize=1\nMaxSize=512\nType=Scalable\n";

/// Whether the icon theme has file and folder icons (else the Colorful
/// style draws 2fip's own).
static THEMED_FILE_ICONS: AtomicBool = AtomicBool::new(true);

/// At start, before any icon is looked up (it changes `XDG_DATA_DIRS`).
pub fn install() {
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let base = fs_ops::sandbox::cache_home().join("2fip/fallback-icons");
        let hicolor = base.join("icons/hicolor");
        if write_icons(&hicolor).is_err() {
            return;
        }
        let current = std::env::var_os("XDG_DATA_DIRS")
            .filter(|dirs| !dirs.is_empty())
            .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
        let mut dirs: Vec<std::path::PathBuf> = std::env::split_paths(&current).collect();
        if !dirs.contains(&base) {
            // Last: the system's own icons come first.
            dirs.push(base);
        }
        if let Ok(joined) = std::env::join_paths(dirs) {
            std::env::set_var("XDG_DATA_DIRS", joined);
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn write_icons(hicolor: &std::path::Path) -> std::io::Result<()> {
    let actions = hicolor.join("scalable/actions");
    std::fs::create_dir_all(&actions)?;
    let index = hicolor.join("index.theme");
    if std::fs::read(&index).ok().as_deref() != Some(INDEX_THEME.as_bytes()) {
        std::fs::write(index, INDEX_THEME)?;
    }
    for (name, svg) in ICONS {
        let path = actions.join(format!("{name}.svg"));
        // Written once (and again only if 2fip's copy changed).
        if std::fs::read(&path).ok().as_deref() != Some(*svg) {
            std::fs::write(path, svg)?;
        }
    }
    Ok(())
}

/// Once 2fip runs: outside COSMIC, without the Cosmic icon theme, use the
/// desktop's icon theme; and note whether there are themed file icons.
pub fn choose_theme() {
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let cosmic_desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .is_ok_and(|desktop| desktop.to_ascii_uppercase().contains("COSMIC"));
        let has_cosmic_theme = cosmic::widget::icon::from_name("folder")
            .path()
            .is_some_and(|path| path.to_string_lossy().contains("/Cosmic/"));
        if !cosmic_desktop && !has_cosmic_theme {
            if let Some(theme) = desktop_icon_theme() {
                cosmic::icon_theme::set_default(theme);
            }
        }
        let themed = cosmic::widget::icon::from_name("folder").path().is_some()
            && cosmic::widget::icon::from_name("text-x-generic")
                .path()
                .is_some();
        THEMED_FILE_ICONS.store(themed, Ordering::Relaxed);
    }
}

/// Whether the icon theme has file and folder icons.
pub fn themed_file_icons() -> bool {
    THEMED_FILE_ICONS.load(Ordering::Relaxed)
}

/// The desktop's icon theme: KDE's from `kdeglobals`, else GNOME's (and
/// other GTK desktops') from gsettings or GTK's `settings.ini`.
#[cfg(all(unix, not(target_os = "macos")))]
fn desktop_icon_theme() -> Option<String> {
    let config = fs_ops::sandbox::config_home();
    let ini_value = |file: &str, section: &str, key: &str| -> Option<String> {
        let text = std::fs::read_to_string(config.join(file)).ok()?;
        let mut in_section = false;
        for line in text.lines().map(str::trim) {
            if line.starts_with('[') {
                in_section = line == section;
            } else if in_section {
                if let Some((name, value)) = line.split_once('=') {
                    if name.trim() == key && !value.trim().is_empty() {
                        return Some(value.trim().to_string());
                    }
                }
            }
        }
        None
    };
    let kde = std::env::var("XDG_CURRENT_DESKTOP")
        .is_ok_and(|desktop| desktop.to_ascii_uppercase().contains("KDE"));
    if kde {
        return ini_value("kdeglobals", "[Icons]", "Theme").or_else(|| Some("breeze".to_string()));
    }
    let gsettings = fs_ops::sandbox::host_command("gsettings")
        .args(["get", "org.gnome.desktop.interface", "icon-theme"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .trim_matches('\'')
                .to_string()
        })
        .filter(|theme| !theme.is_empty());
    gsettings
        .or_else(|| ini_value("gtk-3.0/settings.ini", "[Settings]", "gtk-icon-theme-name"))
        .or_else(|| Some("Adwaita".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run by hand on a system without icon themes (e.g. a minimal Fedora
    /// container): `cargo test -p twofip -- --ignored --nocapture lookup`.
    #[test]
    #[ignore]
    fn lookup_finds_every_icon_after_install() {
        install();
        choose_theme();
        let missing: Vec<&str> = ICONS
            .iter()
            .map(|(name, _)| *name)
            .filter(|name| cosmic::widget::icon::from_name(*name).path().is_none())
            .collect();
        println!(
            "theme: {}, themed file icons: {}, missing: {missing:?}",
            cosmic::icon_theme::default(),
            themed_file_icons()
        );
        assert!(missing.is_empty());
    }

    #[test]
    fn every_built_in_icon_is_an_svg() {
        assert!(ICONS.len() > 50);
        for (name, svg) in ICONS {
            assert!(name.ends_with("-symbolic"), "{name}");
            assert!(
                std::str::from_utf8(svg).is_ok_and(|text| text.contains("<svg")),
                "{name}"
            );
        }
    }
}
