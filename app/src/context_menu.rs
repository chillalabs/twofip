use std::collections::HashMap;

use cosmic::widget::icon;
use cosmic::widget::menu::key_bind::KeyBind;
use cosmic::widget::menu::{self, Item as MenuItem};

use crate::file_item::FileItem;
use crate::fl;
use crate::keybinds::Action;
use crate::pane::PaneMessage;

/// The right-click menu for a file or folder in the listing. Actions apply to
/// the pane's selection (right-clicking an unselected item selects it first),
/// and shortcut hints come from `keybinds`.
pub fn item_menu(
    keybinds: &HashMap<KeyBind, Action>,
    item: &FileItem,
    can_paste: bool,
) -> Option<Vec<menu::Tree<PaneMessage>>> {
    let button = |label: String, icon_name: &'static str, action: Action| {
        MenuItem::Button(label, Some(icon::from_name(icon_name).handle()), action)
    };

    let mut items = vec![button(fl!("open"), "document-open-symbolic", Action::Open)];
    if !item.is_dir() {
        items.push(button(
            fl!("open-with-ellipsis"),
            "system-run-symbolic",
            Action::OpenWith,
        ));
        items.push(button(
            fl!("view"),
            "document-viewer-symbolic",
            Action::View,
        ));
        items.push(button(
            fl!("edit"),
            "accessories-text-editor-symbolic",
            Action::Edit,
        ));
    } else {
        items.push(button(
            fl!("calculate-size"),
            "disk-usage-analyzer-symbolic",
            Action::CalculateSize,
        ));
        items.push(folder_color_item(|action| action));
    }
    items.extend([
        MenuItem::Divider,
        button(fl!("cut"), "edit-cut-symbolic", Action::Cut),
        button(fl!("copy"), "edit-copy-symbolic", Action::CopyToClipboard),
        if can_paste {
            button(fl!("paste"), "edit-paste-symbolic", Action::Paste)
        } else {
            MenuItem::ButtonDisabled(
                fl!("paste"),
                Some(icon::from_name("edit-paste-symbolic").handle()),
                Action::Paste,
            )
        },
        MenuItem::Divider,
        button(fl!("rename-ellipsis"), "edit-symbolic", Action::Rename),
        button(
            fl!("compress-ellipsis"),
            "package-x-generic-symbolic",
            Action::CompressHere,
        ),
    ]);
    // Archives can be unpacked too.
    if fs_ops::ops::ArchiveFormat::of(&item.path).is_some() && !item.is_dir() {
        items.push(button(
            fl!("extract-ellipsis"),
            "package-x-generic-symbolic",
            Action::ExtractHere,
        ));
    }
    items.extend([
        MenuItem::Divider,
        button(fl!("delete"), "user-trash-symbolic", Action::Delete),
        MenuItem::Divider,
        button(
            fl!("show-details"),
            "document-properties-symbolic",
            Action::ShowDetails,
        ),
    ]);

    Some(menu::items(keybinds, items))
}

/// "Folder color…" (opens the color picker), with a color sample as its
/// icon. `wrap` turns the action into the menu's own action type.
pub fn folder_color_item<A: cosmic::widget::menu::Action>(
    wrap: impl Fn(Action) -> A,
) -> MenuItem<A, String> {
    MenuItem::Button(
        fl!("folder-color-ellipsis"),
        Some(crate::icon_sets::color_swatch(
            fs_ops::folder_colors::FolderColor::Blue,
        )),
        wrap(Action::FolderColor),
    )
}

/// A folder color's name, in the UI language.
pub fn color_name(color: fs_ops::folder_colors::FolderColor) -> String {
    use fs_ops::folder_colors::FolderColor;
    match color {
        FolderColor::Red => fl!("color-red"),
        FolderColor::Orange => fl!("color-orange"),
        FolderColor::Yellow => fl!("color-yellow"),
        FolderColor::Green => fl!("color-green"),
        FolderColor::Blue => fl!("color-blue"),
        FolderColor::Purple => fl!("color-purple"),
        FolderColor::Pink => fl!("color-pink"),
        FolderColor::Grey => fl!("color-grey"),
    }
}

/// The right-click menu for empty space in a file listing.
pub fn background_menu(
    keybinds: &HashMap<KeyBind, Action>,
    can_paste: bool,
) -> Option<Vec<menu::Tree<PaneMessage>>> {
    let button = |label: String, icon_name: &'static str, action: Action| {
        MenuItem::Button(label, Some(icon::from_name(icon_name).handle()), action)
    };

    let items = vec![
        button(fl!("new-folder"), "folder-new-symbolic", Action::NewFolder),
        button(fl!("new-file"), "document-new-symbolic", Action::NewFile),
        if can_paste {
            button(fl!("paste"), "edit-paste-symbolic", Action::Paste)
        } else {
            MenuItem::ButtonDisabled(
                fl!("paste"),
                Some(icon::from_name("edit-paste-symbolic").handle()),
                Action::Paste,
            )
        },
        MenuItem::Divider,
        button(
            fl!("select-all"),
            "edit-select-all-symbolic",
            Action::SelectAll,
        ),
        button(
            fl!("tooltip-refresh"),
            "view-refresh-symbolic",
            Action::Refresh,
        ),
    ];

    Some(menu::items(keybinds, items))
}
