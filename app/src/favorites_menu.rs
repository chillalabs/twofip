//! Favorites, as Total Commander's directory hotlist: Ctrl+D shows a small
//! menu at the top left of the active panel (the favorites, then "Add /
//! Remove current folder" and "Configure…"), and Configure… edits the list
//! in a dialog. Changes there are kept only with OK.

use std::path::{Path, PathBuf};

use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;
use fs_ops::favorites::Favorite;

use crate::fl;
use crate::tab::tab_label;

/// Width of the Ctrl+D menu.
const MENU_WIDTH: f32 = 280.0;
/// Height of the Configure dialog's list.
const LIST_HEIGHT: f32 = 260.0;

#[derive(Debug, Clone)]
pub enum FavoritesMessage {
    /// Menu: go to this favorite.
    Open(usize),
    /// Menu: add the active panel's folder, or remove it if it's a favorite.
    ToggleCurrent,
    /// Menu: open the Configure dialog.
    Configure,
    /// Menu: closed without choosing.
    CloseMenu,
    // The Configure dialog.
    Select(usize),
    Add,
    Delete,
    /// One place up (`true`) or down.
    Move(bool),
    Title(String),
    Path(String),
    /// The folder field gets the active panel's folder.
    UseCurrent,
    Ok,
    Cancel,
}

/// A favorite while it's being edited (text fields, checked on OK).
#[derive(Debug, Clone)]
pub struct Draft {
    pub name: String,
    pub path: String,
}

/// The Configure dialog: a copy of the list, saved only with OK.
#[derive(Debug, Clone)]
pub struct Config {
    pub items: Vec<Draft>,
    pub selected: Option<usize>,
}

impl Config {
    /// The dialog for `favorites`, with `current`'s favorite selected (or
    /// the first one).
    pub fn new(favorites: &[Favorite], current: &Path) -> Self {
        let items: Vec<Draft> = favorites
            .iter()
            .map(|favorite| Draft {
                name: favorite.name.clone(),
                path: favorite.path.display().to_string(),
            })
            .collect();
        let selected = favorites
            .iter()
            .position(|favorite| favorite.path == current)
            .or((!items.is_empty()).then_some(0));
        Self { items, selected }
    }

    /// The edited list: entries without a folder are dropped, and an empty
    /// title becomes the folder's name.
    pub fn favorites(&self) -> Vec<Favorite> {
        self.items
            .iter()
            .filter_map(|draft| {
                let path = draft.path.trim();
                if path.is_empty() {
                    return None;
                }
                let path = PathBuf::from(path);
                let name = match draft.name.trim() {
                    "" => tab_label(&path),
                    name => name.to_string(),
                };
                Some(Favorite { name, path })
            })
            .collect()
    }

    /// Adds `path` (the active panel's folder) after the selected entry and
    /// selects it; if it's already in the list, selects that one instead.
    pub fn add(&mut self, path: &Path) {
        let text = path.display().to_string();
        if let Some(existing) = self.items.iter().position(|draft| draft.path == text) {
            self.selected = Some(existing);
            return;
        }
        let at = self
            .selected
            .map_or(self.items.len(), |selected| selected + 1);
        self.items.insert(
            at,
            Draft {
                name: tab_label(path),
                path: text,
            },
        );
        self.selected = Some(at);
    }

    pub fn delete(&mut self) {
        let Some(selected) = self.selected.filter(|s| *s < self.items.len()) else {
            return;
        };
        self.items.remove(selected);
        self.selected = match self.items.len() {
            0 => None,
            len => Some(selected.min(len - 1)),
        };
    }

    pub fn move_selected(&mut self, up: bool) {
        let Some(selected) = self.selected else {
            return;
        };
        let target = if up {
            selected.checked_sub(1)
        } else {
            Some(selected + 1)
        };
        if let Some(target) = target.filter(|target| *target < self.items.len()) {
            self.items.swap(selected, target);
            self.selected = Some(target);
        }
    }

    pub fn selected_mut(&mut self) -> Option<&mut Draft> {
        self.items.get_mut(self.selected?)
    }
}

/// How many entries the Ctrl+D menu has: the favorites, then "Add / Remove
/// current folder" and "Configure…".
pub fn menu_len(favorites: &[Favorite]) -> usize {
    favorites.len() + 2
}

/// What the Ctrl+D menu entry at `index` does.
pub fn menu_message(favorites: &[Favorite], index: usize) -> FavoritesMessage {
    match index.checked_sub(favorites.len()) {
        None => FavoritesMessage::Open(index),
        Some(0) => FavoritesMessage::ToggleCurrent,
        Some(_) => FavoritesMessage::Configure,
    }
}

/// The Ctrl+D menu. `current` is the active panel's folder (ticked when
/// it's a favorite); `cursor` is the entry the keyboard is on.
pub fn menu<'a>(
    favorites: &'a [Favorite],
    current: &Path,
    cursor: usize,
) -> Element<'a, FavoritesMessage> {
    let is_favorite = favorites.iter().any(|favorite| favorite.path == current);
    let mut column = widget::Column::new().spacing(1).width(Length::Fill);
    if favorites.is_empty() {
        column = column.push(
            widget::container(widget::text::caption(fl!("favorites-empty"))).padding([6, 12]),
        );
    }
    for (index, favorite) in favorites.iter().enumerate() {
        column = column.push(menu_item(
            favorite.name.clone(),
            Some(favorite.path == current),
            cursor == index,
            FavoritesMessage::Open(index),
        ));
    }
    column = column
        .push(widget::divider::horizontal::light())
        .push(menu_item(
            if is_favorite {
                fl!("favorites-remove-current")
            } else {
                fl!("favorites-add-current")
            },
            None,
            cursor == favorites.len(),
            FavoritesMessage::ToggleCurrent,
        ))
        .push(menu_item(
            fl!("favorites-configure"),
            None,
            cursor == favorites.len() + 1,
            FavoritesMessage::Configure,
        ));
    widget::container(column)
        .padding(4)
        .width(Length::Fixed(MENU_WIDTH))
        .class(cosmic::theme::Container::Dropdown)
        .into()
}

/// A menu entry: a tick column (`Some(true)`: ticked; `None`: no column
/// content), the label, and the keyboard highlight.
fn menu_item<'a>(
    label: String,
    ticked: Option<bool>,
    highlighted: bool,
    message: FavoritesMessage,
) -> Element<'a, FavoritesMessage> {
    let tick: Element<'a, FavoritesMessage> = if ticked == Some(true) {
        widget::icon::from_name("object-select-symbolic")
            .size(16)
            .into()
    } else {
        widget::Space::new().width(Length::Fixed(16.0)).into()
    };
    let row = widget::Row::new()
        .spacing(8)
        .align_y(Alignment::Center)
        .push(tick)
        .push(
            widget::text::body(label)
                .wrapping(cosmic::iced::widget::text::Wrapping::None)
                .width(Length::Fill),
        );
    if !highlighted {
        return widget::button::custom(row)
            .padding([5, 10])
            .width(Length::Fill)
            .class(cosmic::theme::Button::MenuItem)
            .on_press(message)
            .into();
    }
    // The menu-item button paints its own background, which would hide the
    // keyboard highlight: the highlighted entry is a tinted row instead.
    widget::mouse_area(
        widget::container(row)
            .padding([5, 10])
            .width(Length::Fill)
            .class(cosmic::theme::Container::custom(|theme| {
                let mut background = theme.cosmic().accent_color();
                background.alpha = 0.3;
                widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(background.into())),
                    border: cosmic::iced::Border {
                        radius: theme.cosmic().radius_s().into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })),
    )
    .on_press(message)
    .interaction(cosmic::iced::mouse::Interaction::Pointer)
    .into()
}

/// The Configure dialog: the list on the left with its buttons on the
/// right, and the selected entry's title and folder below.
pub fn config_dialog(config: &Config) -> Element<'_, FavoritesMessage> {
    let mut list = widget::Column::new().spacing(1).width(Length::Fill);
    if config.items.is_empty() {
        list = list
            .push(widget::container(widget::text::caption(fl!("favorites-empty"))).padding([6, 8]));
    }
    for (index, draft) in config.items.iter().enumerate() {
        let selected = config.selected == Some(index);
        let label = if draft.name.trim().is_empty() {
            draft.path.clone()
        } else {
            draft.name.clone()
        };
        let row = widget::button::custom(
            widget::text::body(label).wrapping(cosmic::iced::widget::text::Wrapping::None),
        )
        .padding([4, 8])
        .width(Length::Fill)
        .class(if selected {
            cosmic::theme::Button::Suggested
        } else {
            cosmic::theme::Button::MenuItem
        })
        .on_press(FavoritesMessage::Select(index));
        list = list.push(row);
    }
    let list = widget::container(widget::scrollable(list))
        .padding(4)
        .width(Length::Fill)
        .height(Length::Fixed(LIST_HEIGHT))
        .class(cosmic::theme::Container::List);

    let has_selection = config.selected.is_some();
    let side_button = |label: String, message: Option<FavoritesMessage>| {
        widget::button::standard(label)
            .width(Length::Fill)
            .on_press_maybe(message)
    };
    let last = config.items.len().saturating_sub(1);
    let buttons = widget::Column::new()
        .spacing(6)
        .width(Length::Fixed(210.0))
        .push(side_button(
            fl!("favorites-add-item"),
            Some(FavoritesMessage::Add),
        ))
        .push(side_button(
            fl!("favorites-delete-item"),
            has_selection.then_some(FavoritesMessage::Delete),
        ))
        .push(side_button(
            fl!("favorites-up"),
            (config.selected.is_some_and(|s| s > 0)).then_some(FavoritesMessage::Move(true)),
        ))
        .push(side_button(
            fl!("favorites-down"),
            (config.selected.is_some_and(|s| s < last)).then_some(FavoritesMessage::Move(false)),
        ));

    let selected = config.selected.and_then(|index| config.items.get(index));
    let field = |label: String,
                 value: &str,
                 on_input: fn(String) -> FavoritesMessage,
                 button: Option<(String, FavoritesMessage)>| {
        let mut input = widget::text_input("", value.to_string()).width(Length::Fill);
        if selected.is_some() {
            input = input.on_input(on_input);
        }
        let mut row = widget::Row::new()
            .spacing(6)
            .align_y(Alignment::Center)
            .push(widget::text::body(label).width(Length::Fixed(130.0)))
            .push(input);
        if let Some((tip, message)) = button {
            row = row.push(crate::themes::tooltip(
                widget::button::icon(widget::icon::from_name("folder-open-symbolic"))
                    .on_press_maybe(selected.is_some().then_some(message)),
                widget::text(tip),
                widget::tooltip::Position::Top,
            ));
        }
        row
    };
    let empty = String::new();
    let fields = widget::Column::new()
        .spacing(8)
        .push(field(
            fl!("favorites-title"),
            selected.map_or(&empty, |draft| &draft.name),
            FavoritesMessage::Title,
            None,
        ))
        .push(field(
            fl!("favorites-folder"),
            selected.map_or(&empty, |draft| &draft.path),
            FavoritesMessage::Path,
            Some((fl!("favorites-use-current"), FavoritesMessage::UseCurrent)),
        ));

    let control = widget::Column::new()
        .spacing(12)
        .push(widget::Row::new().spacing(12).push(list).push(buttons))
        .push(fields);

    widget::dialog()
        .title(fl!("favorites-configure-title"))
        .control(control)
        .primary_action(widget::button::suggested(fl!("ok")).on_press(FavoritesMessage::Ok))
        .secondary_action(
            widget::button::standard(fl!("cancel")).on_press(FavoritesMessage::Cancel),
        )
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn favorite(name: &str, path: &str) -> Favorite {
        Favorite {
            name: name.to_string(),
            path: PathBuf::from(path),
        }
    }

    #[test]
    fn edits_stay_in_the_draft_until_turned_back_into_favorites() {
        let saved = vec![
            favorite("Docs", "/home/me/Documents"),
            favorite("Tmp", "/tmp"),
        ];
        let mut config = Config::new(&saved, Path::new("/tmp"));
        assert_eq!(config.selected, Some(1));

        config.add(Path::new("/home/me/Music"));
        assert_eq!(config.selected, Some(2));
        config.move_selected(true);
        assert_eq!(config.selected, Some(1));
        if let Some(draft) = config.selected_mut() {
            draft.name = "My music".to_string();
        }
        config.items[0].name = "  ".to_string();
        // Adding a folder already in the list selects it instead.
        config.add(Path::new("/tmp"));
        assert_eq!(config.selected, Some(2));
        config.delete();
        assert_eq!(config.selected, Some(1));

        let edited = config.favorites();
        assert_eq!(edited.len(), 2);
        assert_eq!(edited[0].name, "Documents");
        assert_eq!(edited[1].name, "My music");
    }

    #[test]
    fn menu_entries_after_the_favorites_are_add_and_configure() {
        let saved = vec![favorite("Docs", "/d")];
        assert_eq!(menu_len(&saved), 3);
        assert!(matches!(menu_message(&saved, 0), FavoritesMessage::Open(0)));
        assert!(matches!(
            menu_message(&saved, 1),
            FavoritesMessage::ToggleCurrent
        ));
        assert!(matches!(
            menu_message(&saved, 2),
            FavoritesMessage::Configure
        ));
    }
}
