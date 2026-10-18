use std::collections::HashMap;

use cosmic::iced::{Alignment, Length};
use cosmic::widget::menu::action::MenuAction;
use cosmic::widget::menu::key_bind::KeyBind;
use cosmic::widget::menu::{self, Item as MenuItem, ItemHeight, ItemWidth};
use cosmic::widget::RcElementWrapper;
use cosmic::widget::{self, icon};
use cosmic::Element;
use fs_ops::settings::ViewMode;

use crate::app::Message;
use crate::fl;
use crate::keybinds::Action;
use crate::themes;

/// An [`Action`] fired from the top menu bar. `Action`'s own [`MenuAction`]
/// impl produces a pane message (for the file context menu); this one
/// produces an app-level message instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopMenuAction(Action);

impl MenuAction for TopMenuAction {
    type Message = Message;

    fn message(&self) -> Message {
        Message::Action(self.0)
    }
}

/// The header's left side: the File / Edit / View / Help menus, a separator, and
/// the Refresh and Find buttons.
pub fn menu_bar<'a>(
    keybinds: &HashMap<KeyBind, Action>,
    can_paste: bool,
    view_mode: ViewMode,
    on_server: bool,
    tools: Tools,
) -> Vec<Element<'a, Message>> {
    // Same shortcuts, re-keyed to the wrapper so the menu can show them.
    let keybinds: HashMap<KeyBind, TopMenuAction> = keybinds
        .iter()
        .map(|(bind, action)| (bind.clone(), TopMenuAction(*action)))
        .collect();

    let button = |label: String, icon_name: &'static str, action: Action| {
        MenuItem::Button(
            label,
            Some(icon::from_name(icon_name).handle()),
            TopMenuAction(action),
        )
    };
    let root = |label: String| RcElementWrapper::new(Element::from(menu::root(label)));

    let file = vec![
        button(fl!("new-tab"), "tab-new-symbolic", Action::NewTab),
        button(fl!("new-folder"), "folder-new-symbolic", Action::NewFolder),
        MenuItem::Divider,
        button(
            fl!("connect-to-server"),
            "network-server-symbolic",
            Action::ConnectToServer,
        ),
        button(
            fl!("connections"),
            "network-workgroup-symbolic",
            Action::Connections,
        ),
        if on_server {
            button(
                fl!("disconnect"),
                "network-offline-symbolic",
                Action::Disconnect,
            )
        } else {
            MenuItem::ButtonDisabled(
                fl!("disconnect"),
                Some(icon::from_name("network-offline-symbolic").handle()),
                TopMenuAction(Action::Disconnect),
            )
        },
        MenuItem::Divider,
        button(fl!("close-tab"), "window-close-symbolic", Action::CloseTab),
        button(fl!("quit"), "application-exit-symbolic", Action::Quit),
    ];

    let paste = if can_paste {
        button(fl!("paste"), "edit-paste-symbolic", Action::Paste)
    } else {
        MenuItem::ButtonDisabled(
            fl!("paste"),
            Some(icon::from_name("edit-paste-symbolic").handle()),
            TopMenuAction(Action::Paste),
        )
    };
    let edit = vec![
        button(fl!("cut"), "edit-cut-symbolic", Action::Cut),
        button(fl!("copy"), "edit-copy-symbolic", Action::CopyToClipboard),
        paste,
        MenuItem::Divider,
        button(
            fl!("select-all"),
            "edit-select-all-symbolic",
            Action::SelectAll,
        ),
        MenuItem::Divider,
        button(fl!("rename-ellipsis"), "edit-symbolic", Action::Rename),
        button(fl!("delete"), "user-trash-symbolic", Action::Delete),
        button(
            fl!("delete-permanently"),
            "edit-delete-symbolic",
            Action::DeletePermanently,
        ),
    ];

    let check = |label: String, icon_name: &'static str, checked: bool, action: Action| {
        MenuItem::CheckBox(
            label,
            Some(icon::from_name(icon_name).handle()),
            checked,
            TopMenuAction(action),
        )
    };
    let view = vec![
        check(
            fl!("list-view"),
            "view-list-symbolic",
            view_mode == ViewMode::List,
            Action::ListView,
        ),
        check(
            fl!("grid-view"),
            "view-grid-symbolic",
            view_mode == ViewMode::Grid,
            Action::GridView,
        ),
        MenuItem::Divider,
        button(fl!("favorites"), "starred-symbolic", Action::Favorites),
        MenuItem::Divider,
        button(
            fl!("settings"),
            "preferences-system-symbolic",
            Action::Settings,
        ),
    ];

    let help = vec![
        button(fl!("help"), "help-browser-symbolic", Action::Help),
        MenuItem::Divider,
        button(fl!("about-2fip"), "help-about-symbolic", Action::About),
    ];

    let bar = menu::bar(vec![
        menu::Tree::with_children(root(fl!("menu-file")), menu::items(&keybinds, file)),
        menu::Tree::with_children(root(fl!("menu-edit")), menu::items(&keybinds, edit)),
        menu::Tree::with_children(root(fl!("menu-view")), menu::items(&keybinds, view)),
        menu::Tree::with_children(root(fl!("menu-help")), menu::items(&keybinds, help)),
    ])
    .item_height(ItemHeight::Dynamic(40))
    .item_width(ItemWidth::Uniform(260))
    .spacing(4.0);
    let separator = widget::container(widget::divider::vertical::default())
        .height(Length::Fixed(20.0))
        .padding([0, 4]);
    // The tooltip goes below: the buttons sit at the top of the window, and
    // libcosmic's default (above) would cover the title bar.
    // Plain icon buttons, no background; one that can't act now is greyed
    // out (disabled).
    let tool = |icon: icon::Handle, tooltip: String, action: Action, enabled: bool| {
        themes::tooltip(
            widget::button::icon(icon).on_press_maybe(enabled.then_some(Message::Action(action))),
            widget::text(tooltip),
            widget::tooltip::Position::Bottom,
        )
    };
    let svg = |bytes: &'static [u8]| icon::from_svg_bytes(bytes).symbolic(true);

    vec![
        bar.into(),
        widget::container(separator)
            .height(Length::Fill)
            .align_y(Alignment::Center)
            .into(),
        tool(
            icon::from_name("view-refresh-symbolic").handle(),
            fl!("tooltip-refresh"),
            Action::Refresh,
            true,
        )
        .into(),
        tool(
            icon::from_name("system-search-symbolic").handle(),
            fl!("tooltip-find"),
            Action::Find,
            tools.can_find,
        )
        .into(),
        tool(
            icon::from_name("network-server-symbolic").handle(),
            fl!("tooltip-connections"),
            Action::Connections,
            true,
        )
        .into(),
        tool(
            svg(include_bytes!("../resources/icons/folder-size.svg")),
            fl!("folder-size"),
            Action::FolderSize,
            tools.can_folder_size,
        )
        .into(),
        tool(
            svg(include_bytes!("../resources/icons/pack.svg")),
            fl!("tooltip-pack"),
            Action::Compress,
            tools.can_pack,
        )
        .into(),
        tool(
            svg(include_bytes!("../resources/icons/unpack.svg")),
            fl!("tooltip-unpack"),
            Action::Extract,
            tools.can_extract,
        )
        .into(),
        tree_button(tools.folder_tree),
    ]
}

/// What the toolbar buttons can do right now (the others are greyed out).
pub struct Tools {
    /// Find: not in server folders or archives.
    pub can_find: bool,
    pub can_folder_size: bool,
    /// Pack: something selected, on this computer.
    pub can_pack: bool,
    /// Extract: an archive selected, and a folder in the other panel that
    /// takes files.
    pub can_extract: bool,
    pub folder_tree: fs_ops::settings::TreeMode,
}

/// "Show hierarchy": its icon shows which panels have the folder tree, and
/// each click moves on (left, right, both, none).
fn tree_button<'a>(mode: fs_ops::settings::TreeMode) -> Element<'a, Message> {
    use fs_ops::settings::TreeMode;
    let (svg, tooltip): (&'static [u8], String) = match mode {
        TreeMode::Off => (
            include_bytes!("../resources/icons/tree-off.svg"),
            fl!("tooltip-tree-off"),
        ),
        TreeMode::Left => (
            include_bytes!("../resources/icons/tree-left.svg"),
            fl!("tooltip-tree-left"),
        ),
        TreeMode::Right => (
            include_bytes!("../resources/icons/tree-right.svg"),
            fl!("tooltip-tree-right"),
        ),
        TreeMode::Both => (
            include_bytes!("../resources/icons/tree-both.svg"),
            fl!("tooltip-tree-both"),
        ),
    };
    themes::tooltip(
        widget::button::icon(icon::from_svg_bytes(svg).symbolic(true))
            .on_press(Message::Action(Action::FolderTree)),
        widget::text(tooltip),
        widget::tooltip::Position::Bottom,
    )
    .into()
}
