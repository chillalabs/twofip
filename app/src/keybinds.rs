use std::collections::HashMap;

use cosmic::iced::keyboard::key::Named;
use cosmic::iced::keyboard::Key;
use cosmic::widget::menu::action::MenuAction;
use cosmic::widget::menu::key_bind::{KeyBind, Modifier};

use crate::pane::PaneMessage;

/// Total-Commander-style actions triggerable by a global keyboard shortcut
/// and/or the file context menu.
///
/// Deliberately excludes anything dialog-local (rename text entry, delete
/// confirmation) — those are driven by their own dedicated messages since
/// they only make sense while their dialog is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    SwitchPane,
    /// Copies the selection into the *other* pane (F5).
    Copy,
    /// Moves the selection into the *other* pane (F6).
    Move,
    Delete,
    /// Shift+Delete / Shift+F8: delete for good, without the trash.
    DeletePermanently,
    Rename,
    NewFolder,
    NewTab,
    CloseTab,
    GoBack,
    GoForward,
    /// Goes to the parent folder.
    GoUp,
    SelectNext,
    SelectPrevious,
    /// Shift+↓ / Shift+↑: grow or shrink the selection by one row.
    ExtendSelectionDown,
    ExtendSelectionUp,
    /// Space: calculate the total size of the selected folders.
    CalculateSize,
    /// Shows the selected folder's contents and recursive sizes in the other pane.
    FolderSize,
    /// Unpacks the selected archives into the other panel (toolbar, Alt+F9).
    Extract,
    /// Unpacks the selected archives next to themselves (right-click).
    ExtractHere,
    /// Packs the selection into an archive next to it (right-click).
    CompressHere,
    /// Shows the folder tree beside the left panel, the right, both, or
    /// none (in turn).
    FolderTree,
    /// Home / End: first / last item; with Shift, extend the selection there.
    SelectFirst,
    SelectLast,
    ExtendSelectionToFirst,
    ExtendSelectionToLast,
    EnterSelectedFolder,
    /// Total Commander's quick filter: list only names matching typed text.
    QuickFilter,
    /// Opens the default terminal in the active pane's folder.
    Terminal,
    /// F3: the built-in viewer for text and images (and files no other
    /// app opens); other files open with their default app.
    View,
    /// Alt+F3: always the built-in viewer (hex for binary files).
    ViewBuiltIn,
    /// Ctrl+Q: the other panel shows the file under the cursor.
    QuickView,
    /// Ctrl+L: type a path into the active pane's path bar.
    EditPath,
    ListView,
    GridView,
    /// F4: opens the selected files in the text editor.
    Edit,
    /// Shift+F4: a new empty file, opened in the text editor.
    NewFile,
    /// Shows the active pane's next / previous tab (wrapping around).
    NextTab,
    PreviousTab,
    Favorites,
    Open,
    OpenWith,
    /// Puts the selection on the internal clipboard, to be moved on paste.
    Cut,
    /// Puts the selection on the internal clipboard, to be copied on paste.
    CopyToClipboard,
    /// Copies/moves the clipboard's contents into the active pane's folder.
    Paste,
    Compress,
    ShowDetails,
    SelectAll,
    /// Opens or closes the Settings side panel.
    Settings,
    ToggleHiddenFiles,
    /// Ctrl+R: re-reads the active tab's folder in both panels.
    Refresh,
    /// Ctrl+F: the Find Files dialog.
    Find,
    /// Help → About 2fip (no shortcut).
    About,
    /// F1: the online help, in the browser.
    Help,
    /// Ctrl+K: the "Connect to server" dialog (SFTP, FTP, FTPS).
    ConnectToServer,
    /// Closes the connection of the server the active panel shows.
    Disconnect,
    /// The Connections side panel (saved and open server connections).
    Connections,
    Quit,
    /// Right-click → Folder color…: the color picker for the selected
    /// folders.
    FolderColor,
}

impl MenuAction for Action {
    type Message = PaneMessage;

    fn message(&self) -> PaneMessage {
        PaneMessage::Action(*self)
    }
}

/// Second shortcuts for actions that already have one in
/// [`default_keybinds`]. Kept apart so menus keep showing the main one
/// (they show one shortcut per action).
pub fn extra_keybinds() -> Vec<(KeyBind, Action)> {
    vec![
        // Total Commander's rename (F2 still works).
        (
            KeyBind {
                modifiers: vec![Modifier::Shift],
                key: Key::Named(Named::F6),
            },
            Action::Rename,
        ),
    ]
}

/// The action for a key press: the main shortcuts first, then the extra ones.
pub fn action_for(
    keybinds: &HashMap<KeyBind, Action>,
    modifiers: cosmic::iced::keyboard::Modifiers,
    key: &Key,
) -> Option<Action> {
    keybinds
        .iter()
        .chain(extra_keybinds().iter().map(|(bind, action)| (bind, action)))
        .find_map(|(bind, action)| bind.matches(modifiers, key, None).then_some(*action))
}

pub fn default_keybinds() -> HashMap<KeyBind, Action> {
    let mut map = HashMap::new();

    let mut bind = |modifiers: &[Modifier], key: Key, action: Action| {
        map.insert(
            KeyBind {
                modifiers: modifiers.to_vec(),
                key,
            },
            action,
        );
    };

    bind(&[], Key::Named(Named::Tab), Action::SwitchPane);
    bind(&[], Key::Named(Named::F1), Action::Help);
    bind(&[], Key::Named(Named::F5), Action::Copy);
    bind(&[], Key::Named(Named::F6), Action::Move);
    bind(&[], Key::Named(Named::F8), Action::Delete);
    bind(
        &[Modifier::Shift],
        Key::Named(Named::F8),
        Action::DeletePermanently,
    );
    bind(
        &[Modifier::Shift],
        Key::Named(Named::Delete),
        Action::DeletePermanently,
    );
    bind(&[], Key::Named(Named::F2), Action::Rename);
    // Total Commander: Alt+F5 packs, Alt+F9 unpacks, with the toolbar
    // buttons' targets (the other panel). The right-click entries have no
    // shortcut.
    bind(&[Modifier::Alt], Key::Named(Named::F5), Action::Compress);
    bind(&[Modifier::Alt], Key::Named(Named::F9), Action::Extract);
    bind(&[], Key::Named(Named::F7), Action::NewFolder);
    bind(
        &[Modifier::Ctrl],
        Key::Character("t".into()),
        Action::NewTab,
    );
    bind(
        &[Modifier::Ctrl],
        Key::Character("w".into()),
        Action::CloseTab,
    );
    bind(
        &[Modifier::Alt],
        Key::Named(Named::ArrowLeft),
        Action::GoBack,
    );
    bind(
        &[Modifier::Alt],
        Key::Named(Named::ArrowRight),
        Action::GoForward,
    );
    bind(&[], Key::Named(Named::Backspace), Action::GoUp);
    bind(&[], Key::Named(Named::ArrowLeft), Action::GoUp);
    bind(
        &[],
        Key::Named(Named::ArrowRight),
        Action::EnterSelectedFolder,
    );
    bind(&[], Key::Named(Named::ArrowDown), Action::SelectNext);
    bind(&[], Key::Named(Named::ArrowUp), Action::SelectPrevious);
    bind(
        &[Modifier::Shift],
        Key::Named(Named::ArrowDown),
        Action::ExtendSelectionDown,
    );
    bind(
        &[Modifier::Shift],
        Key::Named(Named::ArrowUp),
        Action::ExtendSelectionUp,
    );
    // Space arrives as the character " " (there is no named Space key).
    bind(&[], Key::Character(" ".into()), Action::CalculateSize);
    bind(&[], Key::Named(Named::Home), Action::SelectFirst);
    bind(&[], Key::Named(Named::End), Action::SelectLast);
    bind(
        &[Modifier::Shift],
        Key::Named(Named::Home),
        Action::ExtendSelectionToFirst,
    );
    bind(
        &[Modifier::Shift],
        Key::Named(Named::End),
        Action::ExtendSelectionToLast,
    );
    bind(
        &[Modifier::Ctrl],
        Key::Character("s".into()),
        Action::QuickFilter,
    );
    bind(&[], Key::Named(Named::F9), Action::Terminal);
    bind(&[], Key::Named(Named::F3), Action::View);
    bind(&[Modifier::Alt], Key::Named(Named::F3), Action::ViewBuiltIn);
    bind(
        &[Modifier::Ctrl],
        Key::Character("l".into()),
        Action::EditPath,
    );
    bind(
        &[Modifier::Ctrl],
        Key::Character("1".into()),
        Action::ListView,
    );
    bind(
        &[Modifier::Ctrl],
        Key::Character("2".into()),
        Action::GridView,
    );
    bind(&[], Key::Named(Named::F4), Action::Edit);
    bind(&[Modifier::Shift], Key::Named(Named::F4), Action::NewFile);
    // Alt+Tab belongs to the desktop's window switcher, so tabs use Ctrl+Tab
    // (as in Total Commander and web browsers).
    bind(&[Modifier::Ctrl], Key::Named(Named::Tab), Action::NextTab);
    bind(
        &[Modifier::Ctrl, Modifier::Shift],
        Key::Named(Named::Tab),
        Action::PreviousTab,
    );
    // COSMIC itself closes the window on Alt+F4; this covers desktops that don't.
    bind(&[Modifier::Alt], Key::Named(Named::F4), Action::Quit);
    bind(
        &[Modifier::Ctrl],
        Key::Character("d".into()),
        Action::Favorites,
    );
    bind(
        &[Modifier::Ctrl],
        Key::Character("r".into()),
        Action::Refresh,
    );
    bind(&[Modifier::Ctrl], Key::Character("f".into()), Action::Find);
    bind(
        &[Modifier::Ctrl],
        Key::Character("k".into()),
        Action::ConnectToServer,
    );
    bind(&[], Key::Named(Named::Delete), Action::Delete);
    bind(&[], Key::Named(Named::Enter), Action::Open);
    bind(&[Modifier::Ctrl], Key::Character("x".into()), Action::Cut);
    bind(
        &[Modifier::Ctrl],
        Key::Character("c".into()),
        Action::CopyToClipboard,
    );
    bind(&[Modifier::Ctrl], Key::Character("v".into()), Action::Paste);
    bind(
        &[Modifier::Alt],
        Key::Named(Named::Enter),
        Action::ShowDetails,
    );
    bind(
        &[Modifier::Ctrl],
        Key::Character("a".into()),
        Action::SelectAll,
    );
    bind(
        &[Modifier::Ctrl],
        Key::Character(",".into()),
        Action::Settings,
    );
    bind(
        &[Modifier::Ctrl],
        Key::Character("h".into()),
        Action::ToggleHiddenFiles,
    );
    // Ctrl+Q is Quick View, as in Total Commander (Alt+F4 quits).
    bind(
        &[Modifier::Ctrl],
        Key::Character("q".into()),
        Action::QuickView,
    );

    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic::iced::keyboard::Modifiers;

    #[test]
    fn shift_f6_renames_like_f2_and_f6_still_moves() {
        let keybinds = default_keybinds();
        let f6 = Key::Named(Named::F6);
        let f2 = Key::Named(Named::F2);
        assert_eq!(
            action_for(&keybinds, Modifiers::SHIFT, &f6),
            Some(Action::Rename)
        );
        assert_eq!(
            action_for(&keybinds, Modifiers::empty(), &f2),
            Some(Action::Rename)
        );
        assert_eq!(
            action_for(&keybinds, Modifiers::empty(), &f6),
            Some(Action::Move)
        );
    }
}
