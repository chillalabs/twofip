use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use cosmic::app::context_drawer::{self, ContextDrawer};
use cosmic::app::{Core, Task};
use cosmic::iced::event::{listen_raw, Status};
use cosmic::iced::keyboard::key::Named;
use cosmic::iced::keyboard::{Event as KeyEvent, Key, Modifiers};
use cosmic::iced::{Alignment, Event, Length, Subscription};
use cosmic::widget;
use cosmic::widget::menu::key_bind::KeyBind;
use cosmic::{Application, Element};
use tokio_stream::{Stream, StreamExt};

use fs_ops::details::Details;
use fs_ops::favorites::Favorite;
use fs_ops::folder_colors::FolderColor;
use fs_ops::ops::{CancelHandle, ConflictHandle, ConflictResolution, OpEvent};
use fs_ops::search::SearchEvent;
use fs_ops::session::Session;
use fs_ops::settings::{ColorTheme, Corners, FontSize, IconStyle, Settings, ViewMode};
use fs_ops::user_dirs::UserDir;
use fs_ops::EntryKind;

use crate::connect::{ConnectState, ConnectStatus};
use crate::favorites_menu::{Config as FavoritesConfig, FavoritesMessage};
use crate::file_item::{format_modified, format_size, ListingOptions};
use crate::find::{FindState, FindStatus};
use crate::fl;
use crate::folder_size::Panel as FolderSizePanel;
use crate::keybinds::{default_keybinds, Action};
use crate::launch::{self, AppEntry};
use crate::localize;
use crate::menu_bar::menu_bar;
use crate::operation::{OpKind, OperationState};
use crate::pane::{home_dir, PaneMessage, PaneState, SelectMode};
use crate::tab::{matches_filter, tab_label};
use crate::themes;
use crate::viewer::{Viewer, ViewerMessage};
use crate::watcher;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaneId {
    Left,
    Right,
}

impl PaneId {
    fn other(self) -> Self {
        match self {
            PaneId::Left => PaneId::Right,
            PaneId::Right => PaneId::Left,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Pane(PaneId, PaneMessage),
    /// Fired by the bottom function-key button bar; identical to the matching keybind.
    Action(Action),
    /// An event of the operation with this id.
    Operation(u64, OpEvent),
    Key(Modifiers, Key),
    /// Every way of closing the app (Exit button, Ctrl+Q/Alt+F4, the window's
    /// close button, the compositor closing the window) ends here.
    ExitNow,
    /// Escape that a focused widget (e.g. the quick filter box) consumed.
    EscapeCaptured,
    /// A click on a pane's empty space: make it the active pane.
    ActivatePane(PaneId),
    ModifiersChanged(Modifiers),
    ConfirmDelete,
    CancelDelete,
    CancelOperation,
    /// Shows or hides the list of waiting operations.
    ToggleQueue,
    /// Takes a waiting operation out of the queue.
    RemoveQueued(u64),
    RenameInputChanged(String),
    ConfirmRename,
    CancelRename,
    /// The Folder color picker: a color (or `None`, Default) was chosen.
    PickFolderColor(Option<FolderColor>),
    CancelFolderColor,
    /// A rename finished (the old path, and the new one or the error).
    RenameFinished(PathBuf, Result<PathBuf, String>),
    ConflictRenameInputChanged(String),
    BeginConflictRename,
    ConfirmConflictRename,
    CancelConflictRename,
    NewFolderInputChanged(String),
    ConfirmNewFolder,
    CancelNewFolder,
    NewFolderFinished(Result<PathBuf, String>),
    /// Shift+F4 created this file (to open in the editor).
    NewFileFinished(Result<PathBuf, String>),
    /// The installed text editors, for the F4 setting: (desktop id, name).
    EditorsLoaded(Vec<(String, String)>),
    /// Index into the F4 editor list (0 = the system's text editor).
    EditorSelected(usize),
    /// A file edited with F4 on a server was saved: uploaded, or not.
    RemoteSaved(PathBuf, Result<(), String>),
    /// No longer watching this server file for saves.
    RemoteEditEnded(PathBuf),
    HideNotice(u64),
    ResolveConflict(ConflictResolution),
    /// The Ctrl+D favorites menu and its Configure dialog.
    Favorites(FavoritesMessage),
    /// Pack dialog: the archive's path, format, password and
    /// "Move to archive".
    PackPathChanged(String),
    PackFormatSelected(usize),
    PackPasswordChanged(String),
    PackTogglePassword,
    PackMoveToggled(bool),
    ConfirmPack,
    CancelPack,
    /// Extract dialog.
    UnpackKeepPathsToggled(bool),
    UnpackOverwriteToggled(bool),
    UnpackPasswordChanged(String),
    UnpackTogglePassword,
    ConfirmUnpack,
    CancelUnpack,
    /// Cancel in the "file already exists" dialog: stops the operation.
    CancelConflict,
    /// Apps available for the paths the "Open With" dialog was opened for.
    OpenWithLoaded(Vec<PathBuf>, Vec<AppEntry>),
    LaunchApp(usize),
    /// "Always use this app for this type of file" in Open With.
    OpenWithRemember(bool),
    /// Open With for these files (e.g. their type has no default app).
    OpenWithPaths(Vec<PathBuf>),
    CancelOpenWith,
    Launched,
    /// F3: show this file (the panel's files, for Next / Previous).
    OpenViewer(PathBuf, Vec<PathBuf>),
    Viewer(ViewerMessage),
    QuickView(ViewerMessage),
    /// Time to read the drive list again.
    RefreshDrives,
    DrivesLoaded(Vec<fs_ops::drives::Drive>),
    /// A drive picked in a panel's menu got mounted (or not).
    DriveMounted(PaneId, Result<PathBuf, String>),
    /// A drive was ejected (or not).
    DriveEjected(PaneId, Result<(), String>),
    /// A column resize event for whichever pane is resizing.
    ColumnResize(PaneMessage),
    /// Files dropped from another app through the window system (Explorer
    /// on Windows; on Linux, libcosmic's drag and drop handles drops).
    #[cfg(windows)]
    ExternalDrop(Vec<PathBuf>),
    /// Windows: the mouse button was released while dragging a tab.
    #[cfg(windows)]
    TabDragReleased,
    DetailsLoaded(Vec<PathBuf>, Result<Details, String>),
    CloseDetails,
    /// A scan finished (panel, scan generation, result).
    FolderSizeLoaded(PaneId, u64, Result<fs_ops::folder_size::Scan, String>),
    /// A scan's progress: entries and bytes counted so far.
    FolderSizeProgress(PaneId, u64, u64, u64),
    FolderSizeToggle(PaneId, PathBuf),
    FolderSizeSelect(PaneId, PathBuf),
    FolderSizeHover(PaneId, Option<(PathBuf, String, String)>),
    FolderSizeContextOpen(PaneId, PathBuf),
    FolderSizeMoveToTrash(PaneId),
    FolderSizeOpenInNewTab(PaneId),
    FolderSizeClose,
    ShowHiddenFilesToggled(bool),
    ThumbnailsToggled(bool),
    RowLinesToggled(bool),
    SeparateExtensionToggled(bool),
    /// Index into [`localize::LANGUAGES`]; applied immediately.
    LanguageSelected(usize),
    /// Index into [`ICON_STYLES`].
    IconStyleSelected(usize),
    /// Index into [`FONT_SIZES`].
    FontSizeSelected(usize),
    /// Index into [`COLOR_THEMES`].
    ColorThemeSelected(usize),
    /// Index into [`CORNERS`].
    CornersSelected(usize),
    /// A click on a Settings row: give it the keyboard focus.
    FocusSettingsRow(SettingsRow),
    CloseDrawer,
    ConnectProtocol(usize),
    ConnectHost(String),
    ConnectPort(String),
    ConnectUser(String),
    ConnectPassword(String),
    ConnectTogglePassword,
    ConnectSubmit,
    /// Trust the unknown server key shown, and connect.
    ConnectTrust,
    ConnectCancel,
    ConnectFinished(Result<PathBuf, fs_ops::vfs::ConnectError>),
    ConnectName(String),
    ConnectSave(bool),
    ConnectRemember(bool),
    /// Save the connection without connecting.
    ConnectSaveOnly,
    ConnectSavedOnly(Result<(), String>),
    ConnectionAdd,
    /// Open a saved connection (connecting first if needed).
    ConnectionOpen(String),
    /// Show an open, unsaved connection (by root).
    ConnectionOpenRoot(PathBuf),
    /// Close and open a saved connection again.
    ConnectionReconnect(String),
    ConnectionDisconnect(PathBuf),
    ConnectionEdit(String),
    ConnectionDelete(String),
    /// Save an open connection (by root).
    ConnectionSaveOpen(PathBuf),
    ConnectionChecked(PathBuf, Result<(), String>),
    ConnectionOpened {
        id: String,
        navigate: bool,
        result: Result<PathBuf, fs_ops::vfs::ConnectError>,
    },
    /// Opens a web address in the default browser.
    OpenUrl(&'static str),
    FindPatternChanged(String),
    FindStart,
    FindStop,
    /// Results of the search with this generation (see `FindState`).
    FindEvent(u64, SearchEvent),
    FindHighlight(usize),
    FindGoTo(usize),
    /// "View in Listbox": show all results as a tab.
    FindToListbox,
    FindClose,
    /// The inotify watcher is ready (carries its command sender) or a watched
    /// folder changed.
    WatcherEvent(watcher::WatcherEvent),
    /// The main window regained focus.
    WindowFocused(cosmic::iced::window::Id),
}

/// Index of "Skip" in `App::conflict_buttons`: the safe default focus.
const CONFLICT_SKIP: usize = 2;

/// Corner roundness steps offered in Settings, in dropdown order.
const CORNERS: [Corners; 4] = [
    Corners::Square,
    Corners::Small,
    Corners::Medium,
    Corners::Large,
];

fn corners_name(corners: Corners) -> String {
    match corners {
        Corners::Square => fl!("corners-square"),
        Corners::Small => fl!("corners-small"),
        Corners::Medium => fl!("corners-medium"),
        Corners::Large => fl!("corners-large"),
    }
}

fn suggested_conflict_rename_name(path: &Path) -> String {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return "copy".to_string();
    };
    let file_path = std::path::Path::new(file_name);
    let stem = file_path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(file_name);
    match file_path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => format!("{stem} (1).{ext}"),
        None => format!("{stem} (1)"),
    }
}

/// File name sizes offered in Settings, in dropdown order.
const FONT_SIZES: [FontSize; 5] = [
    FontSize::Medium,
    FontSize::Small,
    FontSize::Smaller,
    FontSize::VerySmall,
    FontSize::ExtraSmall,
];

/// The user's guide (`docs/` in the repository, on GitHub Pages).
const HELP_URL: &str = "https://chillalabs.github.io/twofip/";

/// The Settings panel's rows, top to bottom, for keyboard navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsRow {
    ShowHidden,
    SeparateExtension,
    Thumbnails,
    Language,
    ColorTheme,
    Corners,
    IconStyle,
    FontSize,
    RowLines,
    Editor,
}

const SETTINGS_ROWS: [SettingsRow; 10] = [
    SettingsRow::ShowHidden,
    SettingsRow::SeparateExtension,
    SettingsRow::Thumbnails,
    SettingsRow::Language,
    SettingsRow::ColorTheme,
    SettingsRow::Corners,
    SettingsRow::IconStyle,
    SettingsRow::FontSize,
    SettingsRow::RowLines,
    SettingsRow::Editor,
];

/// `index` moved by `step` (±1) within `len` options, wrapping around.
fn step_index(index: usize, step: isize, len: usize) -> usize {
    (index as isize + step).rem_euclid(len as isize) as usize
}

/// Color themes offered in Settings, in dropdown order.
const COLOR_THEMES: [ColorTheme; 16] = [
    ColorTheme::System,
    ColorTheme::Light,
    ColorTheme::Dark,
    ColorTheme::Windows,
    // 2fip's own palettes, alphabetically.
    ColorTheme::AyuDark,
    ColorTheme::CatppuccinMacchiato,
    ColorTheme::CatppuccinMocha,
    ColorTheme::Dracula,
    ColorTheme::Everforest,
    ColorTheme::GruvboxDark,
    ColorTheme::GruvboxMaterial,
    ColorTheme::Matrix,
    ColorTheme::MonokaiPro,
    ColorTheme::Nord,
    ColorTheme::SolarizedDark,
    ColorTheme::TokyoNightStorm,
];

fn color_theme_name(theme: ColorTheme) -> String {
    match theme {
        ColorTheme::System => fl!("color-theme-system"),
        ColorTheme::Light => fl!("color-theme-light"),
        ColorTheme::Dark => fl!("color-theme-dark"),
        // Proper names: the same in every language.
        ColorTheme::Dracula => "Dracula".to_string(),
        ColorTheme::Everforest => "Everforest".to_string(),
        ColorTheme::GruvboxMaterial => "Gruvbox Material".to_string(),
        ColorTheme::Nord => "Nord".to_string(),
        ColorTheme::TokyoNightStorm => "Tokyo Night Storm".to_string(),
        ColorTheme::CatppuccinMocha => "Catppuccin Mocha".to_string(),
        ColorTheme::CatppuccinMacchiato => "Catppuccin Macchiato".to_string(),
        ColorTheme::AyuDark => "Ayu Dark".to_string(),
        ColorTheme::Matrix => "Matrix".to_string(),
        ColorTheme::MonokaiPro => "Monokai Pro".to_string(),
        ColorTheme::SolarizedDark => "Solarized Dark".to_string(),
        ColorTheme::GruvboxDark => "Gruvbox Dark".to_string(),
        ColorTheme::Windows => "Windows".to_string(),
    }
}

/// Icon styles offered in Settings, in dropdown order.
const ICON_STYLES: [IconStyle; 5] = [
    IconStyle::Colorful,
    IconStyle::Vivid,
    IconStyle::Classic,
    IconStyle::Soft,
    IconStyle::Monochrome,
];

/// Dropdown label for a file name size, e.g. "Small (13 px)".
fn font_size_name(size: FontSize) -> String {
    let px = size.px();
    match size {
        FontSize::Medium => fl!("font-size-medium", px = px),
        FontSize::Small => fl!("font-size-small", px = px),
        FontSize::Smaller => fl!("font-size-smaller", px = px),
        FontSize::VerySmall => fl!("font-size-very-small", px = px),
        FontSize::ExtraSmall => fl!("font-size-tiny", px = px),
    }
}

fn icon_style_name(style: IconStyle) -> String {
    match style {
        IconStyle::Colorful => fl!("icon-style-colorful"),
        IconStyle::Monochrome => fl!("icon-style-monochrome"),
        IconStyle::Vivid => fl!("icon-style-vivid"),
        IconStyle::Classic => fl!("icon-style-classic"),
        IconStyle::Soft => fl!("icon-style-soft"),
    }
}

/// What the side panel (context drawer) is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DrawerPage {
    Settings,
    About,
    Connections,
}

/// Paths put aside by Cut/Copy, waiting for a Paste.
struct Clipboard {
    paths: Vec<PathBuf>,
    /// Move (rather than copy) on paste.
    cut: bool,
}

/// The Pack dialog.
struct PackState {
    sources: Vec<PathBuf>,
    /// The archive to create: folder and name.
    path: String,
    /// Where a bare name (no folder typed) goes: the dialog's default folder.
    folder: PathBuf,
    format: fs_ops::ops::ArchiveFormat,
    password: String,
    password_hidden: bool,
    move_files: bool,
}

/// The Extract dialog.
struct UnpackState {
    archives: Vec<PathBuf>,
    /// Where they go: the other panel's folder.
    dest: PathBuf,
    keep_paths: bool,
    overwrite: bool,
    password: String,
    password_hidden: bool,
}

/// An operation's events, whatever kind of stream produces them.
type OperationStream = Pin<Box<dyn Stream<Item = OpEvent> + Send>>;

/// An operation waiting in the queue: its kind, what it works on (shown in
/// the queue list) and how to start it.
struct QueuedOperation {
    id: u64,
    kind: OpKind,
    detail: String,
    start: Box<dyn FnOnce(CancelHandle, ConflictHandle) -> OperationStream + Send>,
}

struct OpenWithState {
    paths: Vec<PathBuf>,
    /// `None` while the installed applications are still loading.
    apps: Option<Vec<AppEntry>>,
    /// The first file's type, for "Always use this app".
    mime: Option<String>,
    /// Make the chosen app the default for `mime`.
    remember: bool,
}

struct DetailsState {
    paths: Vec<PathBuf>,
    /// `None` while sizes are still being calculated.
    result: Option<Result<Details, String>>,
}

struct RenameState {
    path: PathBuf,
    name: String,
}

struct ConflictRenameState {
    path: PathBuf,
    name: String,
}

struct NewFolderState {
    name: String,
    /// Shift+F4: a new file instead of a folder.
    file: bool,
}

pub struct App {
    core: Core,
    left: PaneState,
    right: PaneState,
    active_pane: PaneId,
    keybinds: HashMap<KeyBind, Action>,
    operation: Option<OperationState>,
    /// Operations asked for while another one runs, started in turn.
    queue: VecDeque<QueuedOperation>,
    /// Whether the progress panel lists the waiting operations.
    show_queue: bool,
    /// The id of the last operation started.
    last_operation_id: u64,
    /// The moment the last operation finished, to ignore inotify events that
    /// arrive just after it (their reload already happened on `Done`).
    last_op_done: Option<Instant>,
    /// The inotify watcher's command channel, once it reports ready.
    watcher: Option<watcher::CommandSender>,
    confirm_delete: Option<Vec<PathBuf>>,
    /// Whether the Delete dialog deletes for good (Shift+Delete) instead of
    /// moving to the trash.
    delete_permanently: bool,
    rename: Option<RenameState>,
    new_folder: Option<NewFolderState>,
    /// Set while `self.operation` has an `OpEvent::Conflict` awaiting an answer.
    pending_conflict: Option<(PathBuf, PathBuf)>,
    conflict_rename: Option<ConflictRenameState>,
    favorites: Vec<Favorite>,
    /// The Ctrl+D menu, while open: the entry the keyboard is on.
    favorites_menu: Option<usize>,
    /// The favorites' Configure dialog, while open.
    favorites_config: Option<FavoritesConfig>,
    /// The Delete dialog's buttons, so the keyboard can move focus between them.
    confirm_delete_button: widget::Id,
    cancel_delete_button: widget::Id,
    /// The Rename / New Folder / Compress dialogs' text field and buttons
    /// (only one dialog shows at a time), for moving focus with Tab.
    dialog_input: widget::Id,
    dialog_confirm: widget::Id,
    dialog_cancel: widget::Id,
    /// Which of those has keyboard focus: 0 text field, 1 confirm, 2 cancel.
    dialog_focus: usize,
    /// The "File Already Exists" buttons in on-screen order: Skip All,
    /// Replace All, Skip, Replace; and which one has keyboard focus.
    conflict_buttons: [widget::Id; 4],
    conflict_focus: usize,
    /// Which Delete dialog button has keyboard focus (true = Delete).
    delete_focus_on_confirm: bool,
    /// The Find Files dialog, while open.
    find: Option<FindState>,
    /// The "Connect to server" dialog, while open.
    connect: Option<ConnectState>,
    /// Its protocol names (the dropdown borrows them).
    connect_protocols: Vec<String>,
    /// The Connections panel's saved connections.
    saved_connections: Vec<fs_ops::connections::SavedConnection>,
    /// Their last known state, by connection root.
    connection_checks: HashMap<PathBuf, crate::connections_panel::Check>,
    /// The Settings row with keyboard focus (index into [`SETTINGS_ROWS`]).
    settings_cursor: usize,
    clipboard: Option<Clipboard>,
    compress: Option<PackState>,
    unpack: Option<UnpackState>,
    open_with: Option<OpenWithState>,
    details: Option<DetailsState>,
    folder_size: Option<FolderSizePanel>,
    /// The installed text editors (desktop id, name), for the F4 setting.
    editors: Vec<(String, String)>,
    /// Server files open in an editor (F4): saves are uploaded back.
    remote_edits: HashSet<PathBuf>,
    /// A short message at the bottom (id, text, is an error).
    notice: Option<(u64, String, bool)>,
    last_notice: u64,
    /// The F3 viewer, in place of the panels.
    viewer: Option<Viewer>,
    /// Quick View (Ctrl+Q): this panel shows the other one's file.
    quick_view: Option<(PaneId, Viewer)>,
    /// Windows: the tab being dragged, until the mouse is released.
    #[cfg(windows)]
    tab_drag: Option<crate::dnd::TabDrag>,
    /// The drive menus' entries (refreshed every few seconds).
    drives: Vec<fs_ops::drives::Drive>,
    settings: Settings,
    user_dirs: Arc<HashMap<PathBuf, UserDir>>,
    /// Colors the user picked for folders.
    folder_colors: Arc<fs_ops::folder_colors::FolderColors>,
    /// The Folder color picker, while open: the folders it colors.
    folder_color_picker: Option<Vec<PathBuf>>,
    /// Folders being moved (from, to) whose colors follow them once the
    /// move is done.
    color_moves: Vec<(PathBuf, PathBuf)>,
    drawer: Option<DrawerPage>,
    /// The session as last written to disk, to skip saving when unchanged.
    saved_session: Session,
    modifiers: Modifiers,
}

impl App {
    fn pane(&self, id: PaneId) -> &PaneState {
        match id {
            PaneId::Left => &self.left,
            PaneId::Right => &self.right,
        }
    }

    fn pane_mut(&mut self, id: PaneId) -> &mut PaneState {
        match id {
            PaneId::Left => &mut self.left,
            PaneId::Right => &mut self.right,
        }
    }

    /// Handles one message. `update` wraps this so the session is saved
    /// after every message, whichever way this returns.
    fn handle_message(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Pane(id, PaneMessage::Action(action)) => {
                self.set_active_pane(id);
                self.handle_action(action)
            }
            Message::Pane(
                id,
                PaneMessage::DropFiles {
                    dest,
                    paths,
                    is_move,
                },
            ) => {
                self.set_active_pane(id);
                self.left.clear_drop_hover();
                self.right.clear_drop_hover();
                // Nothing goes into search results ("View in Listbox").
                if self.pane(id).read_only() {
                    return Task::none();
                }
                self.drop_files(dest, paths, is_move)
            }
            Message::Pane(id, PaneMessage::MountDrive(device)) => {
                self.set_active_pane(id);
                cosmic::task::future(async move {
                    let result =
                        tokio::task::spawn_blocking(move || fs_ops::drives::mount(&device))
                            .await
                            .unwrap_or_else(|err| Err(err.to_string()));
                    Message::DriveMounted(id, result)
                })
            }
            Message::Pane(id, PaneMessage::EjectDrive(drive)) => {
                self.set_active_pane(id);
                // Panels showing the drive move home first, or it's busy.
                let mut tasks = Vec::new();
                for pane in [PaneId::Left, PaneId::Right] {
                    let on_drive = drive
                        .mount
                        .as_ref()
                        .is_some_and(|mount| self.pane(pane).current_dir().starts_with(mount));
                    if on_drive {
                        tasks.push(
                            self.pane_mut(pane)
                                .update(PaneMessage::Navigate(home_dir())),
                        );
                    }
                }
                let all = self.drives.clone();
                tasks.push(cosmic::task::future(async move {
                    let result =
                        tokio::task::spawn_blocking(move || fs_ops::drives::eject(&drive, &all))
                            .await
                            .unwrap_or_else(|err| Err(err.to_string()));
                    Message::DriveEjected(id, result)
                }));
                Task::batch(tasks)
            }
            Message::Pane(id, PaneMessage::ShowShellMenu) => {
                self.set_active_pane(id);
                self.show_shell_menu(id)
            }
            #[cfg(windows)]
            Message::Pane(_, PaneMessage::TabDragStarted(tab)) => {
                self.tab_drag = Some(tab);
                Task::none()
            }
            #[cfg(windows)]
            Message::TabDragReleased => {
                let Some(tab) = self.tab_drag.take() else {
                    return Task::none();
                };
                let target = match crate::windows::cursor_over_right_half() {
                    Some(true) => PaneId::Right,
                    Some(false) => PaneId::Left,
                    None => return Task::none(),
                };
                // Released on its own panel: nothing to do (a plain click).
                if target == tab.source {
                    return Task::none();
                }
                self.set_active_pane(target);
                self.pane_mut(target)
                    .update(PaneMessage::TabDropped(Some(tab)))
            }
            #[cfg(windows)]
            Message::Pane(id, PaneMessage::DragOut(paths)) => {
                self.set_active_pane(id);
                // Files on a server or inside an archive aren't files on
                // disk that other programs could take.
                if paths
                    .iter()
                    .any(|path| fs_ops::vfs::is_remote(path) || fs_ops::archive_fs::is_inside(path))
                {
                    return Task::none();
                }
                crate::shell_drag::drag(&paths);
                // A move into Explorer took the files away.
                self.reload_both_panes()
            }
            Message::Pane(id, msg) => {
                // A listing finishing its load isn't the user picking that
                // pane (after an operation both panes reload in the background),
                // and neither is a drag passing over it.
                if !matches!(
                    msg,
                    PaneMessage::DirLoaded(..)
                        | PaneMessage::TreeLoaded(..)
                        | PaneMessage::DropHover(..)
                        | PaneMessage::DropLeave { .. }
                ) {
                    self.set_active_pane(id);
                }
                self.pane_mut(id).update(msg)
            }
            Message::Action(action) => self.handle_action(action),
            Message::FolderSizeLoaded(target, generation, result) => {
                if let Some(panel) = self
                    .folder_size
                    .as_mut()
                    .filter(|panel| panel.target == target)
                {
                    panel.set_result(generation, result);
                }
                Task::none()
            }
            Message::FolderSizeProgress(target, generation, entries, bytes) => {
                if let Some(panel) = self
                    .folder_size
                    .as_mut()
                    .filter(|panel| panel.target == target)
                {
                    panel.set_progress(generation, entries, bytes);
                }
                Task::none()
            }
            Message::FolderSizeToggle(target, path) => {
                self.set_active_pane(target);
                if let Some(panel) = self.folder_size.as_mut() {
                    panel.toggle(path);
                }
                Task::none()
            }
            Message::FolderSizeSelect(target, path) => {
                self.set_active_pane(target);
                if let Some(panel) = self.folder_size.as_mut() {
                    panel.select(path);
                }
                Task::none()
            }
            Message::FolderSizeHover(target, hovered) => {
                if let Some(panel) = self
                    .folder_size
                    .as_mut()
                    .filter(|panel| panel.target == target)
                {
                    panel.set_hovered(hovered);
                }
                Task::none()
            }
            Message::FolderSizeContextOpen(target, path) => {
                self.set_active_pane(target);
                if let Some(panel) = self.folder_size.as_mut() {
                    panel.set_context_path(path);
                }
                Task::none()
            }
            Message::FolderSizeMoveToTrash(target) => {
                let Some(path) = self
                    .folder_size
                    .as_mut()
                    .filter(|panel| panel.target == target)
                    .and_then(FolderSizePanel::take_context_path)
                else {
                    return Task::none();
                };
                self.set_active_pane(target);
                self.delete_permanently = fs_ops::vfs::is_remote(&path);
                self.confirm_delete = Some(vec![path]);
                self.focus_delete_button(true)
            }
            Message::FolderSizeOpenInNewTab(target) => {
                let Some(path) = self
                    .folder_size
                    .as_mut()
                    .filter(|panel| panel.target == target)
                    .and_then(FolderSizePanel::take_context_path)
                else {
                    return Task::none();
                };
                self.set_active_pane(target);
                self.pane_mut(target).open_path_in_new_tab(path)
            }
            Message::FolderSizeClose => {
                self.folder_size = None;
                Task::none()
            }
            Message::ExitNow => self.exit_now(),
            Message::ActivatePane(id) => {
                let changed = id != self.active_pane;
                self.set_active_pane(id);
                // A freshly-focused pane shows what is on disk right now.
                if changed && self.can_auto_refresh() {
                    self.reload_active_pane()
                } else {
                    Task::none()
                }
            }
            Message::WatcherEvent(event) => match event {
                watcher::WatcherEvent::Ready(sender) => {
                    self.watcher = Some(sender);
                    self.sync_watcher();
                    Task::none()
                }
                watcher::WatcherEvent::Changed(path) => self.on_external_change(path),
            },
            Message::WindowFocused(id) => {
                if self.core.main_window_id() == Some(id) && self.can_auto_refresh() {
                    self.reload_active_pane()
                } else {
                    Task::none()
                }
            }
            Message::EscapeCaptured => {
                // Text boxes swallow Escape; still let one press close the
                // dialog they're in (e.g. Rename), the side panel (e.g. the
                // Favorites name field), the path editor or the filter.
                if self.dialog_open() || self.find.is_some() || self.viewer.is_some() {
                    return self.on_escape();
                }
                if self.drawer.is_some() {
                    self.close_drawer();
                    return Task::none();
                }
                if !self.pane_mut(self.active_pane).cancel_path_edit()
                    && self.pane(self.active_pane).filter_open()
                {
                    self.pane_mut(self.active_pane).close_filter();
                }
                Task::none()
            }
            Message::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers;
                Task::none()
            }
            Message::Key(modifiers, key) => {
                self.modifiers = modifiers;
                if key == Key::Named(Named::Escape) {
                    return self.on_escape();
                }
                if self.confirm_delete.is_some()
                    && !self.dialog_open_besides_delete()
                    && modifiers.is_empty()
                {
                    if let Some(task) = self.delete_dialog_key(&key) {
                        return task;
                    }
                }
                // Tab / → and Shift+Tab / ← move focus between the "File Already
                // Exists" buttons; Enter runs the focused one (the button itself
                // handles Enter).
                if self.pending_conflict.is_some() {
                    let forward = match (&key, modifiers) {
                        (Key::Named(Named::Tab), m) if m.is_empty() => Some(true),
                        (Key::Named(Named::Tab), m) if m == Modifiers::SHIFT => Some(false),
                        (Key::Named(Named::ArrowRight), m) if m.is_empty() => Some(true),
                        (Key::Named(Named::ArrowLeft), m) if m.is_empty() => Some(false),
                        _ => None,
                    };
                    if let Some(forward) = forward {
                        let count = self.conflict_buttons.len();
                        let step = if forward { 1 } else { count - 1 };
                        return self.focus_conflict_button(self.conflict_focus + step);
                    }
                }
                // Tab / Shift+Tab move focus around the text dialogs (libcosmic's
                // own keyboard navigation is off, see `init`).
                if key == Key::Named(Named::Tab)
                    && (modifiers.is_empty() || modifiers == Modifiers::SHIFT)
                    && self.text_dialog_showing()
                {
                    let step = if modifiers.shift() { 2 } else { 1 };
                    return self.focus_dialog_field(self.dialog_focus + step);
                }
                if key == Key::Named(Named::Tab)
                    && (modifiers.is_empty() || modifiers == Modifiers::SHIFT)
                {
                    if let Some(connect) = &mut self.connect {
                        return connect.move_focus(if modifiers.shift() { -1 } else { 1 });
                    }
                }
                if self.dialog_open() {
                    // A modal dialog is open; let its own controls (and `on_escape`)
                    // handle input instead of firing global shortcuts underneath it.
                    return Task::none();
                }
                if let Some(viewer) = &self.viewer {
                    // The viewer covers the panels: only its own keys, and
                    // Quit.
                    if let Some(message) = viewer.key(modifiers, &key) {
                        return self.update(Message::Viewer(message));
                    }
                    let action = crate::keybinds::action_for(&self.keybinds, modifiers, &key);
                    if action == Some(Action::Quit) {
                        return self.handle_action(Action::Quit);
                    }
                    return Task::none();
                }
                if self.find.is_some() {
                    // Find Files is modal: only Ctrl+F (to close it) and
                    // Quit get past it.
                    let action = self.keybinds.iter().find_map(|(bind, action)| {
                        bind.matches(modifiers, &key, None).then_some(*action)
                    });
                    if let Some(action @ (Action::Find | Action::Quit)) = action {
                        return self.handle_action(action);
                    }
                    return self.find_key(modifiers, &key);
                }
                if self.favorites_menu.is_some() {
                    return self.favorites_menu_key(modifiers, &key);
                }
                if let Some(task) = self.settings_key(modifiers, &key) {
                    return task;
                }
                let action = crate::keybinds::action_for(&self.keybinds, modifiers, &key);
                // A side panel keeps the keyboard: keys it doesn't use must not
                // reach the file panels behind it (e.g. Delete trashing files).
                // Only the shortcuts that switch panels or quit still work.
                if self.drawer.is_some()
                    && !matches!(
                        action,
                        Some(
                            Action::Favorites
                                | Action::Settings
                                | Action::About
                                | Action::Connections
                                | Action::ConnectToServer
                                | Action::Quit
                        )
                    )
                {
                    return Task::none();
                }
                match action {
                    Some(action) => self.handle_action(action),
                    None => Task::none(),
                }
            }
            // Late events of an operation that ended (e.g. cancelled) are
            // ignored: the next one in the queue may be running.
            Message::Operation(id, _) if self.operation.as_ref().is_none_or(|op| op.id != id) => {
                Task::none()
            }
            Message::Operation(_, event) => match event {
                OpEvent::Progress(progress) => {
                    if let Some(op) = &mut self.operation {
                        op.current_file = Some(progress.current_file);
                        op.files_done = progress.files_done;
                        op.files_total = progress.files_total;
                        op.bytes_done = progress.bytes_done;
                        op.bytes_total = progress.bytes_total;
                    }
                    Task::none()
                }
                OpEvent::Conflict { src, dest } => {
                    self.pending_conflict = Some((src, dest));
                    // Start on Skip: Enter then never overwrites anything.
                    self.focus_conflict_button(CONFLICT_SKIP)
                }
                OpEvent::Done | OpEvent::Cancelled => {
                    self.operation = None;
                    self.last_op_done = Some(Instant::now());
                    self.apply_color_moves();
                    let reload = self.reload_both_panes();
                    let folder_size = self.refresh_folder_size();
                    let next = self.start_next_operation();
                    Task::batch([reload, folder_size, next])
                }
                OpEvent::Error(err) => {
                    if let Some(op) = &mut self.operation {
                        op.error = Some(err);
                    }
                    Task::none()
                }
            },
            Message::ConfirmDelete => {
                let Some(sources) = self.confirm_delete.take() else {
                    return Task::none();
                };
                if sources
                    .iter()
                    .any(|path| fs_ops::archive_fs::is_inside(path))
                {
                    // Inside an archive: it's written again without them.
                    let detail = queue_detail(&sources, None);
                    self.spawn_operation(OpKind::Delete, detail, |cancel, _| {
                        fs_ops::ops::delete_in_archive(sources, cancel)
                    })
                } else if self.delete_permanently {
                    let detail = queue_detail(&sources, None);
                    self.spawn_operation(OpKind::Delete, detail, |cancel, _| {
                        fs_ops::ops::delete_permanently(sources, cancel)
                    })
                } else {
                    let detail = queue_detail(&sources, None);
                    self.spawn_operation(OpKind::Delete, detail, |cancel, _| {
                        fs_ops::ops::delete_to_trash(sources, cancel)
                    })
                }
            }
            Message::CancelDelete => {
                self.confirm_delete = None;
                Task::none()
            }
            Message::CancelOperation => {
                self.pending_conflict = None;
                if let Some(op) = self.operation.take() {
                    op.cancel.cancel();
                    // Unblock the background task if it's mid-`conflict.ask()` -
                    // otherwise it would sit there forever, never seeing the
                    // cancellation, since it isn't polling `cancel` while waiting.
                    op.conflict.respond(ConflictResolution::SkipAll);
                }
                // Cancel (or Close after an error) ends this one only: the
                // queue goes on.
                let reload = self.reload_both_panes();
                Task::batch([reload, self.start_next_operation()])
            }
            Message::ToggleQueue => {
                self.show_queue = !self.show_queue;
                Task::none()
            }
            Message::RemoveQueued(id) => {
                self.queue.retain(|queued| queued.id != id);
                Task::none()
            }
            Message::RenameInputChanged(value) => {
                if let Some(rename) = &mut self.rename {
                    rename.name = value;
                }
                Task::none()
            }
            Message::ConfirmRename => {
                let Some(rename) = self.rename.take() else {
                    return Task::none();
                };
                cosmic::task::future(async move {
                    let old = rename.path.clone();
                    let result = fs_ops::ops::rename(rename.path, rename.name).await;
                    Message::RenameFinished(old, result)
                })
            }
            Message::CancelRename => {
                self.rename = None;
                Task::none()
            }
            Message::PickFolderColor(color) => self.set_folder_color(color),
            Message::CancelFolderColor => {
                self.folder_color_picker = None;
                Task::none()
            }
            Message::RenameFinished(old, result) => {
                match result {
                    Ok(new) => {
                        self.color_moves.push((old, new));
                        self.apply_color_moves();
                    }
                    Err(err) => eprintln!("rename failed: {err}"),
                }
                self.reload_both_panes()
            }
            Message::ConflictRenameInputChanged(value) => {
                if let Some(rename) = &mut self.conflict_rename {
                    rename.name = value;
                }
                Task::none()
            }
            Message::BeginConflictRename => {
                let Some((_, dest)) = self.pending_conflict.clone() else {
                    return Task::none();
                };
                self.conflict_rename = Some(ConflictRenameState {
                    path: dest.clone(),
                    name: suggested_conflict_rename_name(&dest),
                });
                Task::none()
            }
            Message::ConfirmConflictRename => {
                let Some(rename) = self.conflict_rename.take() else {
                    return Task::none();
                };
                self.pending_conflict = None;
                if let Some(op) = &self.operation {
                    op.conflict.respond(ConflictResolution::Rename(rename.name));
                }
                Task::none()
            }
            Message::CancelConflictRename => {
                self.conflict_rename = None;
                Task::none()
            }
            Message::NewFolderInputChanged(value) => {
                if let Some(new_folder) = &mut self.new_folder {
                    new_folder.name = value;
                }
                Task::none()
            }
            Message::ConfirmNewFolder => {
                let Some(new_folder) = self.new_folder.take() else {
                    return Task::none();
                };
                let parent = self.pane(self.active_pane).current_dir();
                cosmic::task::future(async move {
                    if new_folder.file {
                        let result = fs_ops::ops::create_file(parent, new_folder.name).await;
                        return Message::NewFileFinished(result);
                    }
                    let result = fs_ops::ops::create_dir(parent, new_folder.name).await;
                    Message::NewFolderFinished(result)
                })
            }
            Message::CancelNewFolder => {
                self.new_folder = None;
                Task::none()
            }
            Message::NewFolderFinished(result) => {
                if let Err(err) = result {
                    eprintln!("failed to create folder: {err}");
                }
                self.reload_both_panes()
            }
            Message::NewFileFinished(result) => {
                let reload = self.reload_both_panes();
                match result {
                    Ok(path) => Task::batch([reload, self.edit_files(vec![path])]),
                    Err(err) => Task::batch([reload, self.show_notice(err, true)]),
                }
            }
            Message::EditorsLoaded(editors) => {
                self.editors = editors;
                Task::none()
            }
            Message::EditorSelected(index) => {
                self.focus_settings_row(SettingsRow::Editor);
                self.settings.editor = match index {
                    0 => String::new(),
                    index => match self.editors.get(index - 1) {
                        Some((id, _)) => id.clone(),
                        None => return Task::none(),
                    },
                };
                self.persist_settings();
                Task::none()
            }
            Message::RemoteSaved(remote, result) => {
                let name = tab_label(&remote);
                let reload = self.reload_both_panes();
                let notice = match result {
                    Ok(()) => self.show_notice(fl!("notice-uploaded", name = name), false),
                    Err(err) => self
                        .show_notice(fl!("notice-upload-failed", name = name, error = err), true),
                };
                Task::batch([reload, notice])
            }
            Message::RemoteEditEnded(remote) => {
                self.remote_edits.remove(&remote);
                Task::none()
            }
            Message::HideNotice(id) => {
                if self.notice.as_ref().is_some_and(|(shown, ..)| *shown == id) {
                    self.notice = None;
                }
                Task::none()
            }
            Message::ResolveConflict(resolution) => {
                self.pending_conflict = None;
                if let Some(op) = &self.operation {
                    op.conflict.respond(resolution);
                }
                Task::none()
            }
            Message::Favorites(message) => self.favorites_message(message),
            Message::PackPathChanged(value) => {
                if let Some(pack) = &mut self.compress {
                    pack.path = value;
                }
                Task::none()
            }
            Message::PackFormatSelected(index) => {
                if let Some(pack) = &mut self.compress {
                    if let Some(format) = fs_ops::ops::ArchiveFormat::ALL.get(index) {
                        pack.format = *format;
                        pack.path = with_archive_extension(&pack.path, *format);
                    }
                }
                Task::none()
            }
            Message::PackPasswordChanged(value) => {
                if let Some(pack) = &mut self.compress {
                    pack.password = value;
                }
                Task::none()
            }
            Message::PackTogglePassword => {
                if let Some(pack) = &mut self.compress {
                    pack.password_hidden = !pack.password_hidden;
                }
                Task::none()
            }
            Message::PackMoveToggled(move_files) => {
                if let Some(pack) = &mut self.compress {
                    pack.move_files = move_files;
                }
                Task::none()
            }
            Message::ConfirmPack => {
                let Some(pack) = self.compress.take() else {
                    return Task::none();
                };
                let path = pack.path.trim();
                if path.is_empty() || pack_problem(&pack).is_some() {
                    self.compress = Some(pack);
                    return Task::none();
                }
                // A bare name goes into the dialog's default folder.
                let archive = {
                    let typed = PathBuf::from(path);
                    if typed.is_absolute() {
                        typed
                    } else {
                        pack.folder.join(typed)
                    }
                };
                let options = fs_ops::ops::PackOptions {
                    format: pack.format,
                    password: (pack.format.encrypts() && !pack.password.is_empty())
                        .then(|| pack.password.clone()),
                    move_files: pack.move_files,
                };
                let sources = pack.sources;
                let detail = queue_detail(&sources, Some(&archive));
                self.spawn_operation(OpKind::Compress, detail, |cancel, _| {
                    fs_ops::ops::pack(sources, archive, options, cancel)
                })
            }
            Message::CancelPack => {
                self.compress = None;
                Task::none()
            }
            Message::UnpackKeepPathsToggled(keep) => {
                if let Some(unpack) = &mut self.unpack {
                    unpack.keep_paths = keep;
                }
                Task::none()
            }
            Message::UnpackOverwriteToggled(overwrite) => {
                if let Some(unpack) = &mut self.unpack {
                    unpack.overwrite = overwrite;
                }
                Task::none()
            }
            Message::UnpackPasswordChanged(value) => {
                if let Some(unpack) = &mut self.unpack {
                    unpack.password = value;
                }
                Task::none()
            }
            Message::UnpackTogglePassword => {
                if let Some(unpack) = &mut self.unpack {
                    unpack.password_hidden = !unpack.password_hidden;
                }
                Task::none()
            }
            Message::ConfirmUnpack => {
                let Some(unpack) = self.unpack.take() else {
                    return Task::none();
                };
                let options = fs_ops::ops::UnpackOptions {
                    keep_paths: unpack.keep_paths,
                    overwrite: unpack.overwrite,
                    password: (!unpack.password.is_empty()).then_some(unpack.password),
                };
                let (archives, dest) = (unpack.archives, unpack.dest);
                let detail = queue_detail(&archives, Some(&dest));
                self.spawn_operation(OpKind::Unpack, detail, |cancel, conflict| {
                    fs_ops::ops::unpack(archives, dest, options, cancel, conflict)
                })
            }
            Message::CancelUnpack => {
                self.unpack = None;
                Task::none()
            }
            Message::CancelConflict => {
                if let Some(operation) = &self.operation {
                    operation.cancel.cancel();
                    operation.conflict.respond(ConflictResolution::Skip);
                }
                self.pending_conflict = None;
                Task::none()
            }
            Message::OpenWithLoaded(paths, apps) => {
                if let Some(open_with) = &mut self.open_with {
                    // Ignore a slow load for a dialog that's since been reopened.
                    if open_with.paths == paths {
                        open_with.apps = Some(apps);
                    }
                }
                Task::none()
            }
            Message::LaunchApp(index) => {
                let Some(open_with) = self.open_with.take() else {
                    return Task::none();
                };
                let Some(app) = open_with.apps.and_then(|apps| apps.into_iter().nth(index)) else {
                    return Task::none();
                };
                let paths = open_with.paths;
                let remember = open_with.remember.then_some(open_with.mime).flatten();
                cosmic::task::future(async move {
                    if let Some(mime) = remember {
                        let id = app.id.clone();
                        let _ = tokio::task::spawn_blocking(move || {
                            launch::set_default_app(&id, &mime)
                        })
                        .await;
                    }
                    launch::launch(app, paths).await;
                    Message::Launched
                })
            }
            Message::OpenWithRemember(remember) => {
                if let Some(open_with) = &mut self.open_with {
                    open_with.remember = remember;
                }
                Task::none()
            }
            Message::OpenWithPaths(paths) => self.open_with_dialog_for(paths, true),
            Message::CancelOpenWith => {
                self.open_with = None;
                Task::none()
            }
            Message::Launched => Task::none(),
            Message::OpenViewer(path, files) => {
                let (viewer, task) = Viewer::open(path, files, false);
                self.viewer = Some(viewer);
                viewer_task(task, Message::Viewer)
            }
            Message::Viewer(message) => {
                let Some(viewer) = &mut self.viewer else {
                    return Task::none();
                };
                let (task, event) = viewer.update(message);
                let event_task = match event {
                    Some(crate::viewer::Event::Close) => {
                        self.viewer = None;
                        Task::none()
                    }
                    Some(crate::viewer::Event::OpenExternally(path)) => open_externally(path),
                    None => Task::none(),
                };
                Task::batch([viewer_task(task, Message::Viewer), event_task])
            }
            Message::QuickView(message) => {
                let Some((_, viewer)) = &mut self.quick_view else {
                    return Task::none();
                };
                let (task, event) = viewer.update(message);
                let event_task = match event {
                    Some(crate::viewer::Event::OpenExternally(path)) => open_externally(path),
                    _ => Task::none(),
                };
                Task::batch([viewer_task(task, Message::QuickView), event_task])
            }
            Message::RefreshDrives => refresh_drives(),
            Message::DrivesLoaded(drives) => {
                self.drives = drives;
                Task::none()
            }
            Message::DriveMounted(id, result) => {
                let open = match result {
                    Ok(mount) => self.pane_mut(id).update(PaneMessage::Navigate(mount)),
                    Err(error) => self.pane_mut(id).update(PaneMessage::ShowError(fl!(
                        "drive-mount-failed",
                        error = error
                    ))),
                };
                Task::batch([open, refresh_drives()])
            }
            Message::DriveEjected(id, result) => {
                let shown = match result {
                    Ok(()) => Task::none(),
                    Err(error) => self.pane_mut(id).update(PaneMessage::ShowError(fl!(
                        "drive-eject-failed",
                        error = error
                    ))),
                };
                Task::batch([shown, refresh_drives()])
            }
            Message::ColumnResize(message) => {
                let left = self.left.update(message.clone());
                let right = self.right.update(message);
                Task::batch([left, right])
            }
            #[cfg(windows)]
            Message::ExternalDrop(paths) => {
                let id = match crate::windows::cursor_over_right_half() {
                    Some(true) => PaneId::Right,
                    Some(false) => PaneId::Left,
                    None => self.active_pane,
                };
                self.set_active_pane(id);
                if self.pane(id).read_only() {
                    return Task::none();
                }
                let dest = self.pane(id).current_dir();
                // Copy, or move with Shift held, as for drops within 2fip.
                self.drop_files(dest, paths, crate::windows::shift_held())
            }
            Message::DetailsLoaded(paths, result) => {
                if let Some(details) = &mut self.details {
                    if details.paths == paths {
                        details.result = Some(result);
                    }
                }
                Task::none()
            }
            Message::CloseDetails => {
                self.details = None;
                Task::none()
            }
            Message::ShowHiddenFilesToggled(show) => {
                self.focus_settings_row(SettingsRow::ShowHidden);
                self.set_hide_hidden_files(!show)
            }
            Message::SeparateExtensionToggled(separate) => {
                self.focus_settings_row(SettingsRow::SeparateExtension);
                self.settings.separate_extension = separate;
                self.apply_settings()
            }
            Message::RowLinesToggled(show) => {
                self.focus_settings_row(SettingsRow::RowLines);
                self.settings.row_lines = show;
                self.apply_settings()
            }
            Message::ThumbnailsToggled(show) => {
                self.focus_settings_row(SettingsRow::Thumbnails);
                self.settings.show_thumbnails = show;
                self.apply_settings()
            }
            Message::LanguageSelected(index) => {
                self.focus_settings_row(SettingsRow::Language);
                if let Some((code, _)) = localize::LANGUAGES.get(index) {
                    self.settings.language = code.to_string();
                    // Every view reads its text through `fl!`, so the next
                    // redraw is already in the new language.
                    localize::set_language(code);
                    self.persist_settings();
                }
                Task::none()
            }
            Message::FocusSettingsRow(row) => {
                self.focus_settings_row(row);
                Task::none()
            }
            Message::ColorThemeSelected(index) => {
                self.focus_settings_row(SettingsRow::ColorTheme);
                if let Some(theme) = COLOR_THEMES.get(index) {
                    self.settings.color_theme = *theme;
                    self.persist_settings();
                    return self.apply_color_theme();
                }
                Task::none()
            }
            Message::CornersSelected(index) => {
                self.focus_settings_row(SettingsRow::Corners);
                if let Some(corners) = CORNERS.get(index) {
                    self.settings.corners = *corners;
                    self.persist_settings();
                    return self.apply_color_theme();
                }
                Task::none()
            }
            Message::FontSizeSelected(index) => {
                self.focus_settings_row(SettingsRow::FontSize);
                if let Some(size) = FONT_SIZES.get(index) {
                    self.settings.name_font_size = *size;
                    return self.apply_settings();
                }
                Task::none()
            }
            Message::IconStyleSelected(index) => {
                self.focus_settings_row(SettingsRow::IconStyle);
                if let Some(style) = ICON_STYLES.get(index) {
                    self.settings.icon_style = *style;
                    return self.apply_settings();
                }
                Task::none()
            }
            Message::CloseDrawer => {
                self.close_drawer();
                Task::none()
            }
            Message::FindPatternChanged(pattern) => {
                if let Some(find) = &mut self.find {
                    find.pattern = pattern;
                    find.input_focused = true;
                }
                Task::none()
            }
            Message::FindStart => self.start_find(),
            Message::ConnectProtocol(index) => {
                if let Some(connect) = &mut self.connect {
                    connect.set_protocol(index);
                }
                Task::none()
            }
            Message::ConnectHost(text) => {
                if let Some(connect) = &mut self.connect {
                    connect.host = text;
                }
                Task::none()
            }
            Message::ConnectPort(text) => {
                if let Some(connect) = &mut self.connect {
                    connect.port = text.chars().filter(char::is_ascii_digit).collect();
                }
                Task::none()
            }
            Message::ConnectUser(text) => {
                if let Some(connect) = &mut self.connect {
                    connect.user = text;
                }
                Task::none()
            }
            Message::ConnectPassword(text) => {
                if let Some(connect) = &mut self.connect {
                    connect.password = text;
                }
                Task::none()
            }
            Message::ConnectTogglePassword => {
                if let Some(connect) = &mut self.connect {
                    connect.password_hidden = !connect.password_hidden;
                }
                Task::none()
            }
            Message::ConnectSubmit => self.start_connect(false),
            Message::ConnectTrust => self.start_connect(true),
            Message::ConnectCancel => {
                self.connect = None;
                Task::none()
            }
            Message::ConnectFinished(result) => {
                let Some(connect) = &mut self.connect else {
                    return Task::none(); // Cancelled meanwhile.
                };
                match result {
                    Ok(path) => {
                        self.connect = None;
                        if let Some(location) = fs_ops::vfs::RemoteLocation::parse(&path) {
                            self.connection_checks.insert(
                                PathBuf::from(location.root()),
                                crate::connections_panel::Check::Connected,
                            );
                        }
                        self.pane_mut(self.active_pane)
                            .update(PaneMessage::Navigate(path))
                    }
                    Err(error) => {
                        connect.failed(error);
                        Task::none()
                    }
                }
            }
            Message::ConnectName(text) => {
                if let Some(connect) = &mut self.connect {
                    connect.name = text;
                }
                Task::none()
            }
            Message::ConnectSave(save) => {
                if let Some(connect) = &mut self.connect {
                    connect.save = save;
                }
                Task::none()
            }
            Message::ConnectRemember(remember) => {
                if let Some(connect) = &mut self.connect {
                    connect.remember_password = remember;
                }
                Task::none()
            }
            Message::ConnectSaveOnly => {
                let plan = match self.save_from_dialog() {
                    Ok(plan) => plan,
                    Err(message) => {
                        if let Some(connect) = &mut self.connect {
                            connect.status = ConnectStatus::Failed(message);
                        }
                        return Task::none();
                    }
                };
                cosmic::task::future(async move { Message::ConnectSavedOnly(plan.apply().await) })
            }
            Message::ConnectSavedOnly(result) => {
                match result {
                    Ok(()) => self.connect = None,
                    Err(err) => {
                        if let Some(connect) = &mut self.connect {
                            connect.status = ConnectStatus::Failed(err);
                        }
                    }
                }
                Task::none()
            }
            Message::ConnectionAdd => {
                let connect = ConnectState::new_saved();
                let focus = connect.focus_current();
                self.connect = Some(connect);
                focus
            }
            Message::ConnectionOpen(id) => {
                let Some(connection) = self.saved_connections.iter().find(|c| c.id == id) else {
                    return Task::none();
                };
                let root = connection.root();
                if fs_ops::vfs::is_connected(&root) {
                    let home = fs_ops::vfs::home(&root).unwrap_or(root);
                    return self
                        .pane_mut(self.active_pane)
                        .update(PaneMessage::Navigate(home));
                }
                self.open_saved_connection(&id, true)
            }
            Message::ConnectionOpenRoot(root) => {
                let home = fs_ops::vfs::home(&root).unwrap_or(root);
                self.pane_mut(self.active_pane)
                    .update(PaneMessage::Navigate(home))
            }
            Message::ConnectionReconnect(id) => {
                if let Some(connection) = self.saved_connections.iter().find(|c| c.id == id) {
                    fs_ops::vfs::disconnect(&connection.root());
                }
                self.open_saved_connection(&id, false)
            }
            Message::ConnectionOpened {
                id,
                navigate,
                result,
            } => {
                let Some(connection) = self.saved_connections.iter().find(|c| c.id == id).cloned()
                else {
                    return Task::none();
                };
                match result {
                    Ok(path) => {
                        self.connection_checks.insert(
                            connection.root(),
                            crate::connections_panel::Check::Connected,
                        );
                        // A connection no tab shows would be closed right
                        // away: show it.
                        if navigate || !self.server_in_use(&connection.root()) {
                            self.pane_mut(self.active_pane)
                                .update(PaneMessage::Navigate(path))
                        } else {
                            self.reload_both_panes()
                        }
                    }
                    Err(error) => {
                        // Let the user fix it: trust the key, type the password...
                        self.connection_checks.remove(&connection.root());
                        let mut connect = ConnectState::for_saved(&connection);
                        connect.failed(error);
                        let focus = connect.focus_current();
                        self.connect = Some(connect);
                        focus
                    }
                }
            }
            Message::ConnectionDisconnect(root) => {
                fs_ops::vfs::disconnect(&root);
                self.connection_checks.remove(&root);
                self.leave_server(&root)
            }
            Message::ConnectionEdit(id) => {
                let Some(connection) = self.saved_connections.iter().find(|c| c.id == id) else {
                    return Task::none();
                };
                let connect = ConnectState::for_saved(connection);
                let focus = connect.focus_current();
                self.connect = Some(connect);
                focus
            }
            Message::ConnectionDelete(id) => {
                let Some(index) = self.saved_connections.iter().position(|c| c.id == id) else {
                    return Task::none();
                };
                let removed = self.saved_connections.remove(index);
                self.persist_connections();
                if !removed.remember_password {
                    return Task::none();
                }
                cosmic::task::future(async move {
                    if let Err(err) = fs_ops::connections::delete_password(&removed.id).await {
                        eprintln!("failed to forget the password of {}: {err}", removed.name);
                    }
                    Message::Launched
                })
            }
            Message::ConnectionSaveOpen(root) => {
                let mut connect = ConnectState::new(&root);
                connect.save = true;
                let focus = connect.focus_current();
                self.connect = Some(connect);
                focus
            }
            Message::ConnectionChecked(root, result) => {
                let check = match result {
                    Ok(()) => crate::connections_panel::Check::Connected,
                    Err(err) => crate::connections_panel::Check::Lost(err),
                };
                self.connection_checks.insert(root, check);
                Task::none()
            }
            Message::OpenUrl(url) => {
                launch::open_with_default_app(std::path::Path::new(url));
                Task::none()
            }
            Message::FindStop => {
                if let Some(find) = &self.find {
                    find.cancel.cancel();
                }
                Task::none()
            }
            Message::FindEvent(generation, event) => {
                let Some(find) = &mut self.find else {
                    return Task::none();
                };
                if find.generation != generation {
                    return Task::none(); // From a search since replaced.
                }
                match event {
                    SearchEvent::Found(hits) => find.hits.extend(hits),
                    SearchEvent::Done { truncated } => {
                        find.status = FindStatus::Done { truncated };
                    }
                }
                Task::none()
            }
            Message::FindHighlight(index) => {
                if let Some(find) = &mut self.find {
                    find.cursor = Some(index);
                    find.input_focused = false;
                }
                unfocus_text_fields()
            }
            Message::FindGoTo(index) => {
                let Some(find) = self.find.take() else {
                    return Task::none();
                };
                find.cancel.cancel();
                match find.hits.into_iter().nth(index) {
                    Some(hit) => self.pane_mut(self.active_pane).reveal(hit.path),
                    None => Task::none(),
                }
            }
            Message::FindToListbox => {
                let Some(find) = self.find.take() else {
                    return Task::none();
                };
                find.cancel.cancel();
                let paths = find.hits.into_iter().map(|hit| hit.path).collect();
                let pattern = find.pattern.trim().to_string();
                self.pane_mut(self.active_pane)
                    .open_results(find.root, pattern, paths)
            }
            Message::FindClose => {
                self.close_find();
                Task::none()
            }
        }
    }

    /// The panes' current state, as saved for the next launch.
    fn session(&self) -> Session {
        Session {
            left: self.left.session(),
            right: self.right.session(),
            right_active: self.active_pane == PaneId::Right,
        }
    }

    /// Saves the session if it changed since the last save. Called after
    /// every message, so it survives the app being killed, not just a clean quit.
    fn save_session_if_changed(&mut self) {
        // Not on every mouse move of a column resize: once it ends.
        if self.left.is_resizing_columns() || self.right.is_resizing_columns() {
            return;
        }
        let session = self.session();
        if session == self.saved_session {
            return;
        }
        if let Err(err) = fs_ops::session::save(&session) {
            eprintln!("failed to save session: {err}");
        }
        self.saved_session = session;
    }

    /// Saves the session and ends the process right away.
    ///
    /// libcosmic's normal shutdown tears down the Wayland connection while a
    /// background thread (the Wayland event loop / clipboard) may still use
    /// it, which intermittently crashed with "Bad file descriptor" and a
    /// segfault in libwayland-client. Everything worth keeping (session,
    /// settings, favorites) is already on disk, so skipping that teardown
    /// loses nothing. The compositor cleans up the window when we exit.
    fn exit_now(&mut self) -> ! {
        self.save_session_if_changed();
        std::process::exit(0)
    }

    /// Makes `id` the active pane. The other pane's selection is hidden (and
    /// remembered), and `id`'s own hidden selection comes back.
    fn set_active_pane(&mut self, id: PaneId) {
        if id == self.active_pane {
            return;
        }
        self.pane_mut(self.active_pane).deactivate();
        self.active_pane = id;
        self.pane_mut(id).activate();
    }

    fn reload_both_panes(&mut self) -> Task<Message> {
        Task::batch(vec![self.left.reload(), self.right.reload()])
    }

    /// Reloads just the active pane's current folder.
    fn reload_active_pane(&self) -> Task<Message> {
        match self.active_pane {
            PaneId::Left => self.left.reload(),
            PaneId::Right => self.right.reload(),
        }
    }

    /// Whether an automatic refresh is safe right now: not mid-operation and
    /// not mid-edit (rename / new folder / path editor).
    fn can_auto_refresh(&self) -> bool {
        self.operation.is_none()
            && self.rename.is_none()
            && self.new_folder.is_none()
            && !self.left.path_edit_active()
            && !self.right.path_edit_active()
    }

    /// Points the inotify watcher at the panes' current local folders.
    fn sync_watcher(&self) {
        let Some(sender) = &self.watcher else {
            return;
        };
        let mut paths = HashSet::new();
        for dir in [self.left.current_dir(), self.right.current_dir()] {
            // Real folders only (not servers, not archives).
            if !fs_ops::vfs::is_remote(&dir) && dir.is_dir() {
                paths.insert(dir);
            }
        }
        sender.watch(paths);
    }

    /// A watched folder changed on disk: reload the pane(s) showing it.
    fn on_external_change(&mut self, path: PathBuf) -> Task<Message> {
        if !self.can_auto_refresh() {
            return Task::none();
        }
        // Our own operations already reload on `Done`; ignore the inotify
        // events that land just after.
        if self
            .last_op_done
            .is_some_and(|done| done.elapsed() < Duration::from_millis(500))
        {
            return Task::none();
        }
        let left = self.left.current_dir();
        let right = self.right.current_dir();
        let mut tasks = Vec::new();
        if !fs_ops::vfs::is_remote(&left) && path.starts_with(&left) {
            tasks.push(self.left.reload());
        }
        if !fs_ops::vfs::is_remote(&right) && path.starts_with(&right) {
            tasks.push(self.right.reload());
        }
        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
    }

    fn handle_action(&mut self, action: Action) -> Task<Message> {
        match action {
            Action::SwitchPane => {
                self.set_active_pane(self.active_pane.other());
                // Switching panes is a moment to show the freshly-activated
                // pane's current state, like the other explicit refreshes.
                if self.can_auto_refresh() {
                    self.reload_active_pane()
                } else {
                    Task::none()
                }
            }
            Action::NextTab => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::CycleTab(true)),
            Action::PreviousTab => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::CycleTab(false)),
            Action::ListView | Action::GridView => {
                // Only the active pane's active tab changes; the session saves it.
                let view_mode = if action == Action::GridView {
                    ViewMode::Grid
                } else {
                    ViewMode::List
                };
                self.pane_mut(self.active_pane).set_view_mode(view_mode);
                Task::none()
            }
            Action::EditPath => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::StartPathEdit),
            Action::View | Action::ViewBuiltIn => {
                let pane = self.pane(self.active_pane);
                let Some(path) = pane
                    .cursor_path()
                    .into_iter()
                    .chain(pane.selected_paths())
                    .find(|path| !self.is_folder(path))
                else {
                    return Task::none();
                };
                let files = pane.file_paths();
                let built_in = action == Action::ViewBuiltIn;
                cosmic::task::future(async move {
                    // PDFs, videos, documents… open in their own app; text,
                    // images and files nothing else opens, in the viewer.
                    if !built_in {
                        if let Some(local) = launch::local_paths(vec![path.clone()]).await.pop() {
                            let check = local.clone();
                            let external = tokio::task::spawn_blocking(move || {
                                crate::viewer::wants_own_app(&check)
                            })
                            .await
                            .unwrap_or(false);
                            if external {
                                launch::open_with_default_app(&local);
                                return Message::Launched;
                            }
                        }
                    }
                    Message::OpenViewer(path, files)
                })
            }
            Action::QuickView => {
                if self.quick_view.take().is_some() {
                    return Task::none();
                }
                self.folder_size = None;
                let target = self.active_pane.other();
                let Some(path) = self.pane(self.active_pane).cursor_path() else {
                    let (viewer, _) = Viewer::open(PathBuf::new(), Vec::new(), true);
                    self.quick_view = Some((target, viewer));
                    return Task::none();
                };
                let (viewer, task) = Viewer::open(path, Vec::new(), true);
                self.quick_view = Some((target, viewer));
                viewer_task(task, Message::QuickView)
            }
            Action::Edit => {
                let paths = self.pane(self.active_pane).selected_paths();
                self.edit_files(paths)
            }
            Action::Terminal => {
                // Inside an archive: in the folder holding the archive.
                let dir = self.pane(self.active_pane).current_dir();
                let dir = fs_ops::archive_fs::split(&dir)
                    .and_then(|(archive, _)| archive.parent().map(PathBuf::from))
                    .unwrap_or(dir);
                launch::open_terminal(&dir);
                Task::none()
            }
            Action::Copy => self.start_operation(OpKind::Copy),
            Action::Move => self.start_operation(OpKind::Move),
            Action::Delete | Action::DeletePermanently => {
                let sources = self.pane(self.active_pane).selected_paths();
                if sources.is_empty() {
                    return Task::none();
                }
                // Servers and archives have no trash: deleting there is
                // always permanent.
                self.delete_permanently = action == Action::DeletePermanently
                    || sources.iter().any(|path| {
                        fs_ops::vfs::is_remote(path) || fs_ops::archive_fs::is_inside(path)
                    });
                self.confirm_delete = Some(sources);
                // Focus "Delete" so Enter confirms (Escape still cancels).
                self.focus_delete_button(true)
            }
            Action::Rename
                if fs_ops::archive_fs::in_archive(&self.pane(self.active_pane).current_dir()) =>
            {
                Task::none()
            }
            Action::NewFolder if self.pane(self.active_pane).read_only() => Task::none(),
            Action::Rename => {
                let mut sources = self.pane(self.active_pane).selected_paths();
                if sources.len() != 1 {
                    // Renaming multiple items at once needs a batch-rename tool; out of v1 scope.
                    return Task::none();
                }
                let path = sources.pop().unwrap();
                let name = tab_label(&path);
                self.rename = Some(RenameState { path, name });
                self.focus_dialog_field(0)
            }
            Action::NewFolder => {
                self.new_folder = Some(NewFolderState {
                    name: fl!("new-folder-default-name"),
                    file: false,
                });
                self.focus_dialog_field(0)
            }
            Action::NewFile => {
                let pane = self.pane(self.active_pane);
                if pane.read_only() || fs_ops::archive_fs::in_archive(&pane.current_dir()) {
                    return Task::none();
                }
                self.new_folder = Some(NewFolderState {
                    name: fl!("new-file-default-name"),
                    file: true,
                });
                self.focus_dialog_field(0)
            }
            Action::NewTab => self.pane_mut(self.active_pane).update(PaneMessage::NewTab),
            Action::CloseTab => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::CloseActiveTab),
            Action::GoBack => self.pane_mut(self.active_pane).update(PaneMessage::GoBack),
            Action::GoForward => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::GoForward),
            Action::GoUp => self.pane_mut(self.active_pane).update(PaneMessage::GoUp),
            Action::SelectNext => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::SelectNext),
            Action::CalculateSize => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::CalculateDirSizes),
            Action::FolderSize => self.open_folder_size(),
            Action::FolderTree => {
                self.settings.folder_tree = self.settings.folder_tree.next();
                self.persist_settings();
                self.apply_folder_tree()
            }
            Action::SelectFirst
            | Action::SelectLast
            | Action::ExtendSelectionToFirst
            | Action::ExtendSelectionToLast => {
                let last = matches!(action, Action::SelectLast | Action::ExtendSelectionToLast);
                let extend = matches!(
                    action,
                    Action::ExtendSelectionToFirst | Action::ExtendSelectionToLast
                );
                self.pane_mut(self.active_pane)
                    .update(PaneMessage::SelectEdge { last, extend })
            }
            Action::ExtendSelectionDown => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::ExtendSelection(true)),
            Action::ExtendSelectionUp => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::ExtendSelection(false)),
            Action::SelectPrevious => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::SelectPrevious),
            Action::QuickFilter => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::OpenFilter),
            Action::EnterSelectedFolder => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::EnterSelectedFolder),
            Action::Favorites => {
                if self.favorites_menu.take().is_none() {
                    self.close_drawer();
                    // Start on the current folder's favorite, if it has one.
                    let current = self.pane(self.active_pane).current_dir();
                    let at = self
                        .favorites
                        .iter()
                        .position(|favorite| favorite.path == current);
                    self.favorites_menu = Some(at.unwrap_or(0));
                }
                Task::none()
            }
            Action::Open => {
                if let Some(dir) = self.pane(self.active_pane).selected_single_dir() {
                    return self
                        .pane_mut(self.active_pane)
                        .update(PaneMessage::Navigate(dir));
                }
                // An archive opens as a folder, like in Total Commander.
                if let [single] = self.pane(self.active_pane).selected_paths().as_slice() {
                    if opens_as_folder(single) {
                        return self
                            .pane_mut(self.active_pane)
                            .update(PaneMessage::Navigate(single.clone()));
                    }
                }
                // Folders in a multi-selection are skipped: we can't navigate
                // into several at once, and handing them to `xdg-open` would
                // open them in a different file manager.
                Task::batch(
                    self.pane(self.active_pane)
                        .selected_paths()
                        .into_iter()
                        .filter(|path| !path.is_dir())
                        .map(launch::open),
                )
            }
            Action::OpenWith => {
                let paths = self.pane(self.active_pane).selected_paths();
                self.open_with_dialog_for(paths, false)
            }
            Action::Cut | Action::CopyToClipboard => {
                let paths = self.pane(self.active_pane).selected_paths();
                if !paths.is_empty() {
                    self.clipboard = Some(Clipboard {
                        paths,
                        cut: action == Action::Cut,
                    });
                }
                Task::none()
            }
            Action::Paste => self.paste(),
            Action::Compress | Action::CompressHere => {
                let sources = self.pane(self.active_pane).selected_paths();
                if sources.is_empty()
                    || sources.iter().any(|path| {
                        fs_ops::vfs::is_remote(path) || fs_ops::archive_fs::is_inside(path)
                    })
                {
                    return Task::none();
                }
                // The right-click menu packs next to the files; the toolbar
                // button into the other panel's folder (this one's when that
                // can't take files: a server, an archive, search results).
                let here = self.pane(self.active_pane).current_dir();
                let other_pane = self.pane(self.active_pane.other());
                let dir = if action == Action::CompressHere
                    || other_pane.read_only()
                    || fs_ops::vfs::is_remote(&other_pane.current_dir())
                {
                    here
                } else {
                    other_pane.current_dir()
                };
                let name = default_archive_name(&sources, &dir);
                self.compress = Some(PackState {
                    sources,
                    path: dir.join(name).display().to_string(),
                    folder: dir.clone(),
                    format: fs_ops::ops::ArchiveFormat::Zip,
                    password: String::new(),
                    password_hidden: true,
                    move_files: false,
                });
                self.focus_dialog_field(0)
            }
            Action::ExtractHere | Action::Extract => {
                let archives: Vec<PathBuf> = self
                    .pane(self.active_pane)
                    .selected_paths()
                    .into_iter()
                    .filter(|path| {
                        !fs_ops::vfs::is_remote(path)
                            && path.is_file()
                            && fs_ops::ops::ArchiveFormat::of(path).is_some()
                    })
                    .collect();
                // The toolbar button unpacks into the other panel; the
                // right-click menu next to the archive.
                let dest = if action == Action::ExtractHere {
                    self.pane(self.active_pane).current_dir()
                } else {
                    self.pane(self.active_pane.other()).current_dir()
                };
                if archives.is_empty()
                    || fs_ops::vfs::is_remote(&dest)
                    || fs_ops::archive_fs::in_archive(&dest)
                {
                    return Task::none();
                }
                self.unpack = Some(UnpackState {
                    archives,
                    dest,
                    keep_paths: true,
                    overwrite: false,
                    password: String::new(),
                    password_hidden: true,
                });
                Task::none()
            }
            Action::FolderColor => {
                let pane = self.pane(self.active_pane);
                let folders: Vec<PathBuf> = pane
                    .selected_paths()
                    .into_iter()
                    .filter(|path| pane.is_listed_folder(path))
                    .collect();
                if !folders.is_empty() {
                    self.folder_color_picker = Some(folders);
                }
                Task::none()
            }
            Action::ShowDetails => {
                let paths = self.pane(self.active_pane).selected_paths();
                if paths.is_empty() {
                    return Task::none();
                }
                self.details = Some(DetailsState {
                    paths: paths.clone(),
                    result: None,
                });
                cosmic::task::future(async move {
                    let result = fs_ops::details::details(paths.clone()).await;
                    Message::DetailsLoaded(paths, result)
                })
            }
            Action::SelectAll => self
                .pane_mut(self.active_pane)
                .update(PaneMessage::SelectAll),
            Action::Settings => {
                self.toggle_drawer(DrawerPage::Settings);
                Task::none()
            }
            Action::Refresh => self.reload_both_panes(),
            Action::Find => self.toggle_find(),
            Action::About => self.toggle_about(),
            Action::Help => {
                launch::open_with_default_app(std::path::Path::new(HELP_URL));
                Task::none()
            }
            Action::ConnectToServer => {
                self.close_drawer();
                self.close_find();
                let connect = ConnectState::new(&self.pane(self.active_pane).current_dir());
                let focus = connect.focus_current();
                self.connect = Some(connect);
                focus
            }
            Action::Connections => {
                self.close_find();
                self.toggle_drawer(DrawerPage::Connections);
                if self.drawer == Some(DrawerPage::Connections) {
                    self.check_connections()
                } else {
                    Task::none()
                }
            }
            Action::Disconnect => {
                let current = self.pane(self.active_pane).current_dir();
                if !fs_ops::vfs::is_remote(&current) {
                    return Task::none();
                }
                fs_ops::vfs::disconnect(&current);
                self.pane_mut(self.active_pane)
                    .update(PaneMessage::Navigate(home_dir()))
            }
            Action::ToggleHiddenFiles => {
                self.set_hide_hidden_files(!self.settings.hide_hidden_files)
            }
            Action::Quit => self.exit_now(),
        }
    }

    fn listing_options(&self) -> ListingOptions {
        ListingOptions {
            hide_hidden: self.settings.hide_hidden_files,
            icon_style: self.settings.icon_style,
            user_dirs: self.user_dirs.clone(),
            folder_colors: self.folder_colors.clone(),
            show_thumbnails: self.settings.show_thumbnails,
            row_lines: self.settings.row_lines,
            font_size: self.settings.name_font_size,
            separate_extension: self.settings.separate_extension,
        }
    }

    /// Saves the settings and re-applies them to both panes' listings.
    /// Saves the settings and re-applies them to both panes' listings
    /// (returning any thumbnail loading that newly needs to start).
    fn apply_settings(&mut self) -> Task<Message> {
        let options = self.listing_options();
        if let Some(panel) = &mut self.folder_size {
            panel.set_options(options.clone());
        }
        let left = self.left.set_options(options.clone());
        let right = self.right.set_options(options);
        self.persist_settings();
        Task::batch([left, right])
    }

    fn set_hide_hidden_files(&mut self, hide: bool) -> Task<Message> {
        self.settings.hide_hidden_files = hide;
        self.apply_settings()
    }

    /// Switches 2fip's colors to the chosen theme, with the chosen corners;
    /// "System" goes back to the desktop's current theme (libcosmic keeps it
    /// up to date meanwhile).
    fn apply_color_theme(&self) -> Task<Message> {
        let theme = match self.settings.color_theme {
            ColorTheme::System => self.core.system_theme().clone(),
            ColorTheme::Light => cosmic::Theme::light(),
            ColorTheme::Dark => cosmic::Theme::dark(),
            ColorTheme::Windows => themes::windows_like(),
            own => themes::build(own).unwrap_or_else(cosmic::Theme::dark),
        };
        let system = self.settings.color_theme == ColorTheme::System;
        cosmic::command::set_theme(themes::with_corners(&theme, self.settings.corners, system))
    }

    fn persist_settings(&self) {
        if let Err(err) = fs_ops::settings::save(&self.settings) {
            eprintln!("failed to save settings: {err}");
        }
    }

    /// Opens the side panel on `page`, or closes it if it's already showing `page`.
    fn toggle_drawer(&mut self, page: DrawerPage) {
        if self.drawer == Some(page) {
            self.close_drawer();
        } else {
            self.drawer = Some(page);
            self.core.set_show_context(true);
            if page == DrawerPage::Settings {
                self.settings_cursor = 0;
            }
        }
    }

    /// While the Settings panel is open it has the keyboard: Tab/↓ and
    /// Shift+Tab/↑ move between settings, Space/Enter toggle a checkbox (or
    /// pick the next option), →/← pick the next/previous option. Returns
    /// `None` for other keys (e.g. Escape closes the panel as usual).
    fn settings_key(&mut self, modifiers: Modifiers, key: &Key) -> Option<Task<Message>> {
        if self.drawer != Some(DrawerPage::Settings) {
            return None;
        }
        let rows = SETTINGS_ROWS.len();
        let plain = modifiers.is_empty();
        let row = SETTINGS_ROWS[self.settings_cursor.min(rows - 1)];
        match key {
            Key::Named(Named::Tab) if plain => {
                self.settings_cursor = step_index(self.settings_cursor, 1, rows);
            }
            Key::Named(Named::Tab) if modifiers == Modifiers::SHIFT => {
                self.settings_cursor = step_index(self.settings_cursor, -1, rows);
            }
            Key::Named(Named::ArrowDown) if plain => {
                self.settings_cursor = step_index(self.settings_cursor, 1, rows);
            }
            Key::Named(Named::ArrowUp) if plain => {
                self.settings_cursor = step_index(self.settings_cursor, -1, rows);
            }
            Key::Named(Named::ArrowRight | Named::Enter) if plain => {
                return Some(self.change_setting(row, 1));
            }
            Key::Character(c) if plain && c.as_str() == " " => {
                return Some(self.change_setting(row, 1));
            }
            Key::Named(Named::ArrowLeft) if plain => return Some(self.change_setting(row, -1)),
            _ => return None,
        }
        Some(Task::none())
    }

    /// Moves the Settings keyboard focus to `row`.
    fn focus_settings_row(&mut self, row: SettingsRow) {
        if let Some(index) = SETTINGS_ROWS.iter().position(|r| *r == row) {
            self.settings_cursor = index;
        }
    }

    /// Toggles a checkbox setting, or moves a list setting `step` options.
    fn change_setting(&mut self, row: SettingsRow, step: isize) -> Task<Message> {
        let message = match row {
            SettingsRow::ShowHidden => {
                // Shown now ⇔ hidden before: flip it.
                Message::ShowHiddenFilesToggled(self.settings.hide_hidden_files)
            }
            SettingsRow::SeparateExtension => {
                Message::SeparateExtensionToggled(!self.settings.separate_extension)
            }
            SettingsRow::Thumbnails => Message::ThumbnailsToggled(!self.settings.show_thumbnails),
            SettingsRow::RowLines => Message::RowLinesToggled(!self.settings.row_lines),
            SettingsRow::Editor => {
                let current = self
                    .editors
                    .iter()
                    .position(|(id, _)| *id == self.settings.editor)
                    .map_or(0, |index| index + 1);
                Message::EditorSelected(step_index(current, step, self.editors.len() + 1))
            }
            SettingsRow::Language => {
                let current = localize::LANGUAGES
                    .iter()
                    .position(|(code, _)| *code == self.settings.language)
                    .unwrap_or(0);
                Message::LanguageSelected(step_index(current, step, localize::LANGUAGES.len()))
            }
            SettingsRow::ColorTheme => {
                let current = COLOR_THEMES
                    .iter()
                    .position(|theme| *theme == self.settings.color_theme)
                    .unwrap_or(0);
                Message::ColorThemeSelected(step_index(current, step, COLOR_THEMES.len()))
            }
            SettingsRow::Corners => {
                let current = CORNERS
                    .iter()
                    .position(|corners| *corners == self.settings.corners)
                    .unwrap_or(0);
                Message::CornersSelected(step_index(current, step, CORNERS.len()))
            }
            SettingsRow::IconStyle => {
                let current = ICON_STYLES
                    .iter()
                    .position(|style| *style == self.settings.icon_style)
                    .unwrap_or(0);
                Message::IconStyleSelected(step_index(current, step, ICON_STYLES.len()))
            }
            SettingsRow::FontSize => {
                let current = FONT_SIZES
                    .iter()
                    .position(|size| *size == self.settings.name_font_size)
                    .unwrap_or(0);
                Message::FontSizeSelected(step_index(current, step, FONT_SIZES.len()))
            }
        };
        self.update(message)
    }

    /// Wraps a Settings row, outlined in the accent color when it has the
    /// keyboard focus.
    fn settings_row<'a>(
        &self,
        row: SettingsRow,
        content: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let focused = SETTINGS_ROWS.get(self.settings_cursor) == Some(&row);
        let framed = widget::container(content)
            .padding(4)
            .class(cosmic::theme::Container::custom(move |theme| {
                if !focused {
                    return widget::container::Style::default();
                }
                // A tinted background plus a border, so the focus stands out
                // in every color theme.
                let cosmic = theme.cosmic();
                let accent: cosmic::iced::Color = cosmic.accent_color().into();
                widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(cosmic::iced::Color {
                        a: 0.18,
                        ..accent
                    })),
                    border: cosmic::iced::Border {
                        color: accent,
                        width: 2.0,
                        radius: cosmic.radius_s().into(),
                    },
                    ..Default::default()
                }
            }));
        // A click on the row's title or description (anything that doesn't
        // handle the click itself) moves the focus here.
        widget::mouse_area(framed)
            .on_press(Message::FocusSettingsRow(row))
            .into()
    }

    /// A Settings checkbox row: the checkbox before the title and description
    /// (like libcosmic's own checkbox rows), toggled by a click anywhere on
    /// the row, with the keyboard focus outline.
    fn checkbox_row<'a>(
        &self,
        row: SettingsRow,
        title: String,
        description: String,
        checked: bool,
        on_toggle: fn(bool) -> Message,
    ) -> Element<'a, Message> {
        let content = widget::settings::item::builder(title)
            .description(description)
            .icon(widget::checkbox(checked).on_toggle(on_toggle))
            .control(widget::Space::new());
        // The whole row toggles (and so also takes the focus, see `update`).
        self.settings_row(
            row,
            widget::mouse_area(content).on_press(on_toggle(!checked)),
        )
    }

    /// The color picker's choice: `color` (or none) for its folders.
    fn set_folder_color(&mut self, color: Option<FolderColor>) -> Task<Message> {
        let Some(folders) = self.folder_color_picker.take() else {
            return Task::none();
        };
        let mut colors = (*self.folder_colors).clone();
        for folder in folders {
            match color {
                Some(color) => {
                    colors.insert(folder, color);
                }
                None => {
                    colors.remove(&folder);
                }
            }
        }
        self.store_folder_colors(colors)
    }

    /// The Folder color picker: the colors as large samples (one click
    /// picks), then Default and Cancel.
    fn folder_color_dialog<'a>(&'a self, folders: &'a [PathBuf]) -> Element<'a, Message> {
        let title = match folders {
            [one] => fl!("folder-color-one", name = tab_label(one)),
            _ => fl!("folder-color-many", count = folders.len()),
        };
        // The color they all have now, to mark it.
        let current = folders
            .iter()
            .map(|folder| self.folder_colors.get(folder).copied())
            .reduce(|a, b| if a == b { a } else { None })
            .flatten();
        let any_colored = folders
            .iter()
            .any(|folder| self.folder_colors.contains_key(folder));
        let mut swatches = widget::Row::new().spacing(6).align_y(Alignment::Center);
        for color in FolderColor::ALL {
            let sample = widget::icon(crate::icon_sets::color_swatch(color)).size(36);
            let button = widget::button::custom(sample)
                .padding(4)
                .class(if current == Some(color) {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::Icon
                })
                .on_press(Message::PickFolderColor(Some(color)));
            swatches = swatches.push(themes::tooltip(
                button,
                widget::text(crate::context_menu::color_name(color)),
                widget::tooltip::Position::Bottom,
            ));
        }
        widget::dialog()
            .title(title)
            .control(swatches)
            .primary_action(
                widget::button::standard(fl!("cancel")).on_press(Message::CancelFolderColor),
            )
            .secondary_action(
                widget::button::standard(fl!("folder-color-default"))
                    // Whenever any of them has a color, even different ones.
                    .on_press_maybe(any_colored.then_some(Message::PickFolderColor(None))),
            )
            .into()
    }

    /// Keeps `colors`, saves them and redraws the listings.
    fn store_folder_colors(
        &mut self,
        colors: fs_ops::folder_colors::FolderColors,
    ) -> Task<Message> {
        if let Err(err) = fs_ops::folder_colors::save(&colors) {
            eprintln!("failed to save folder colors: {err}");
        }
        self.folder_colors = Arc::new(colors);
        self.apply_settings()
    }

    /// Before moving `sources` into `dest`: remembers the colored folders
    /// among them (or inside them), so their colors can follow.
    fn note_color_moves(&mut self, sources: &[PathBuf], dest: &Path) {
        for source in sources {
            let colored = self
                .folder_colors
                .keys()
                .any(|path| path.starts_with(source));
            if let (true, Some(name)) = (colored, source.file_name()) {
                self.color_moves.push((source.clone(), dest.join(name)));
            }
        }
    }

    /// After a move or rename: the colors of folders that are now in their
    /// new place go with them (a move that failed or was skipped leaves the
    /// old folder there, and its color with it).
    fn apply_color_moves(&mut self) {
        if self.color_moves.is_empty() {
            return;
        }
        let mut colors = (*self.folder_colors).clone();
        let mut changed = false;
        for (from, to) in std::mem::take(&mut self.color_moves) {
            if !from.exists() && to.exists() {
                changed |= fs_ops::folder_colors::moved(&mut colors, &from, &to);
            }
        }
        if changed {
            // The panels reload after a move or rename anyway.
            let _ = self.store_folder_colors(colors);
        }
    }

    /// The Ctrl+D menu and the Configure dialog.
    fn favorites_message(&mut self, message: FavoritesMessage) -> Task<Message> {
        let current = self.pane(self.active_pane).current_dir();
        match message {
            FavoritesMessage::Open(index) => {
                self.favorites_menu = None;
                let Some(favorite) = self.favorites.get(index).cloned() else {
                    return Task::none();
                };
                return self
                    .pane_mut(self.active_pane)
                    .update(PaneMessage::Navigate(favorite.path));
            }
            FavoritesMessage::ToggleCurrent => {
                self.favorites_menu = None;
                match self
                    .favorites
                    .iter()
                    .position(|favorite| favorite.path == current)
                {
                    Some(index) => {
                        self.favorites.remove(index);
                    }
                    None => self.favorites.push(Favorite {
                        name: tab_label(&current),
                        path: current,
                    }),
                }
                self.persist_favorites();
            }
            FavoritesMessage::Configure => {
                self.favorites_menu = None;
                self.favorites_config = Some(FavoritesConfig::new(&self.favorites, &current));
            }
            FavoritesMessage::CloseMenu => self.favorites_menu = None,
            FavoritesMessage::Ok => {
                if let Some(config) = self.favorites_config.take() {
                    self.favorites = config.favorites();
                    self.persist_favorites();
                }
            }
            FavoritesMessage::Cancel => self.favorites_config = None,
            message => {
                let Some(config) = &mut self.favorites_config else {
                    return Task::none();
                };
                match message {
                    FavoritesMessage::Select(index) => config.selected = Some(index),
                    FavoritesMessage::Add => config.add(&current),
                    FavoritesMessage::Delete => config.delete(),
                    FavoritesMessage::Move(up) => config.move_selected(up),
                    FavoritesMessage::Title(text) => {
                        if let Some(draft) = config.selected_mut() {
                            draft.name = text;
                        }
                    }
                    FavoritesMessage::Path(text) => {
                        if let Some(draft) = config.selected_mut() {
                            draft.path = text;
                        }
                    }
                    FavoritesMessage::UseCurrent => {
                        if let Some(draft) = config.selected_mut() {
                            draft.path = current.display().to_string();
                        }
                    }
                    _ => {}
                }
            }
        }
        Task::none()
    }

    /// The Ctrl+D menu has the keyboard while it's open: ↑/↓ (Home/End)
    /// move, Enter picks; "-" adds or removes the current folder and "*"
    /// opens Configure, as in Total Commander; Ctrl+D closes it.
    fn favorites_menu_key(&mut self, modifiers: Modifiers, key: &Key) -> Task<Message> {
        let Some(cursor) = self.favorites_menu else {
            return Task::none();
        };
        let len = crate::favorites_menu::menu_len(&self.favorites);
        let plain = modifiers.is_empty() || modifiers == Modifiers::SHIFT;
        let next = match key {
            Key::Named(Named::ArrowDown) => (cursor + 1) % len,
            Key::Named(Named::ArrowUp) => (cursor + len - 1) % len,
            Key::Named(Named::Home) => 0,
            Key::Named(Named::End) => len - 1,
            Key::Named(Named::Enter) => {
                let message = crate::favorites_menu::menu_message(&self.favorites, cursor);
                return self.favorites_message(message);
            }
            Key::Character(c) if plain && c.as_str() == "-" => {
                return self.favorites_message(FavoritesMessage::ToggleCurrent);
            }
            Key::Character(c) if plain && c.as_str() == "*" => {
                return self.favorites_message(FavoritesMessage::Configure);
            }
            _ => {
                if crate::keybinds::action_for(&self.keybinds, modifiers, key)
                    == Some(Action::Favorites)
                {
                    self.favorites_menu = None;
                }
                return Task::none();
            }
        };
        self.favorites_menu = Some(next);
        Task::none()
    }

    /// `frame` (a panel) with the Ctrl+D menu over its top left corner,
    /// when it's the active panel and the menu is open.
    fn with_favorites_menu<'a>(
        &'a self,
        frame: Element<'a, Message>,
        id: PaneId,
    ) -> Element<'a, Message> {
        let Some(cursor) = self.favorites_menu.filter(|_| id == self.active_pane) else {
            return frame;
        };
        let current = self.pane(id).current_dir();
        let menu =
            crate::favorites_menu::menu(&self.favorites, &current, cursor).map(Message::Favorites);
        widget::popover(frame)
            .popup(menu)
            // Below the tabs and the path bar, by the file list, like Total
            // Commander's.
            .position(widget::popover::Position::Point(cosmic::iced::Point::new(
                12.0, 72.0,
            )))
            .on_close(Message::Favorites(FavoritesMessage::CloseMenu))
            .into()
    }

    fn close_drawer(&mut self) {
        self.drawer = None;
        self.core.set_show_context(false);
    }

    /// Shows the About side panel, or closes it.
    fn toggle_about(&mut self) -> Task<Message> {
        self.close_find();
        self.toggle_drawer(DrawerPage::About);
        Task::none()
    }

    /// Connects with the dialog's settings (`trust`: the user accepted an
    /// unknown server key); the result comes back as `ConnectFinished`.
    fn start_connect(&mut self, trust: bool) -> Task<Message> {
        let Some(connect) = &mut self.connect else {
            return Task::none();
        };
        if connect.status == ConnectStatus::Connecting {
            return Task::none();
        }
        let params = match connect.params(trust) {
            Ok(params) => params,
            Err(message) => {
                connect.status = ConnectStatus::Failed(message);
                return Task::none();
            }
        };
        let plan = match self.save_from_dialog() {
            Ok(plan) => plan,
            Err(message) => {
                if let Some(connect) = &mut self.connect {
                    connect.status = ConnectStatus::Failed(message);
                }
                return Task::none();
            }
        };
        if let Some(connect) = &mut self.connect {
            connect.status = ConnectStatus::Connecting;
        }
        cosmic::task::future(async move {
            if let Err(err) = plan.apply().await {
                return Message::ConnectFinished(Err(fs_ops::vfs::ConnectError::Other(err)));
            }
            let mut params = params;
            if params.password.is_none() {
                params.password = plan.stored_password().await;
            }
            Message::ConnectFinished(fs_ops::vfs::connect(params).await)
        })
    }

    /// Saves the dialog's connection when "Save in Connections" is ticked
    /// (adding it, or updating the one being edited), and returns the
    /// keyring work for its password.
    fn save_from_dialog(&mut self) -> Result<crate::connect::KeyringPlan, String> {
        let Some(connect) = &mut self.connect else {
            return Ok(Default::default());
        };
        if !connect.save {
            return Ok(Default::default());
        }
        let id = connect
            .editing
            .clone()
            .unwrap_or_else(fs_ops::connections::new_id);
        let saved = connect.saved(id.clone())?;
        // Later attempts (e.g. after trusting a key) update this one.
        connect.editing = Some(id.clone());
        let plan = connect.keyring_plan(Some(id.clone()), &saved.name);
        match self.saved_connections.iter_mut().find(|c| c.id == id) {
            Some(existing) => *existing = saved,
            None => self.saved_connections.push(saved),
        }
        self.persist_connections();
        Ok(plan)
    }

    /// Whether any tab, in either panel, shows a folder on the server `root`.
    fn server_in_use(&self, root: &std::path::Path) -> bool {
        [PaneId::Left, PaneId::Right]
            .into_iter()
            .flat_map(|id| self.pane(id).tab_dirs())
            .any(|dir| {
                fs_ops::vfs::RemoteLocation::parse(&dir)
                    .is_some_and(|location| std::path::Path::new(&location.root()) == root)
            })
    }

    /// Closes the connections no tab shows anymore (e.g. their last tab was
    /// closed). Not while an operation runs: it may be copying from one.
    fn close_unused_connections(&mut self) {
        if self.operation.is_some() {
            return;
        }
        for root in fs_ops::vfs::open_connections() {
            if !self.server_in_use(&root) {
                fs_ops::vfs::disconnect(&root);
                self.connection_checks.remove(&root);
            }
        }
    }

    fn persist_connections(&self) {
        if let Err(err) = fs_ops::connections::save(&self.saved_connections) {
            eprintln!("failed to save connections: {err}");
        }
    }

    /// Checks every open connection, for the Connections panel's status.
    fn check_connections(&mut self) -> Task<Message> {
        let roots = fs_ops::vfs::open_connections();
        for root in &roots {
            self.connection_checks
                .insert(root.clone(), crate::connections_panel::Check::Busy);
        }
        Task::batch(roots.into_iter().map(|root| {
            cosmic::task::future(async move {
                let result = fs_ops::vfs::check(&root).await;
                Message::ConnectionChecked(root, result)
            })
        }))
    }

    /// Opens the saved connection `id` (with its stored password); the
    /// result comes back as `ConnectionOpened`.
    fn open_saved_connection(&mut self, id: &str, navigate: bool) -> Task<Message> {
        let Some(connection) = self.saved_connections.iter().find(|c| c.id == id).cloned() else {
            return Task::none();
        };
        self.connection_checks
            .insert(connection.root(), crate::connections_panel::Check::Busy);
        let id = id.to_string();
        cosmic::task::future(async move {
            let password = if connection.remember_password {
                fs_ops::connections::load_password(&connection.id)
                    .await
                    .ok()
                    .flatten()
            } else {
                None
            };
            let result = fs_ops::vfs::connect(connection.params(password)).await;
            Message::ConnectionOpened {
                id,
                navigate,
                result,
            }
        })
    }

    /// Sends both panels showing the server `root` home (it was closed).
    fn leave_server(&mut self, root: &std::path::Path) -> Task<Message> {
        let mut tasks = Vec::new();
        for id in [PaneId::Left, PaneId::Right] {
            let current = self.pane(id).current_dir();
            let on_it = fs_ops::vfs::RemoteLocation::parse(&current)
                .is_some_and(|location| std::path::Path::new(&location.root()) == root);
            if on_it {
                tasks.push(self.pane_mut(id).update(PaneMessage::Navigate(home_dir())));
            }
        }
        Task::batch(tasks)
    }

    /// Opens the Find Files dialog on the active panel's folder, or closes it.
    fn toggle_find(&mut self) -> Task<Message> {
        if self.find.is_some() {
            self.close_find();
            return Task::none();
        }
        self.close_drawer();
        let find = FindState::new(self.pane(self.active_pane).current_dir());
        let focus = widget::text_input::focus(find.input_id.clone());
        self.find = Some(find);
        focus
    }

    fn close_find(&mut self) {
        if let Some(find) = self.find.take() {
            find.cancel.cancel();
        }
    }

    /// Starts searching for the typed name (stopping any search still running).
    fn start_find(&mut self) -> Task<Message> {
        let include_hidden = !self.settings.hide_hidden_files;
        let Some(find) = &mut self.find else {
            return Task::none();
        };
        let pattern = find.pattern.trim().to_string();
        if pattern.is_empty() {
            return Task::none();
        }
        find.cancel.cancel();
        find.cancel = CancelHandle::new();
        find.generation += 1;
        find.hits.clear();
        find.cursor = None;
        find.status = FindStatus::Searching;
        let generation = find.generation;
        let search = fs_ops::search::search(
            find.root.clone(),
            move |name| matches_filter(name, &pattern),
            include_hidden,
            crate::find::RESULT_LIMIT,
            find.cancel.clone(),
        );
        cosmic::task::stream(search.map(move |event| Message::FindEvent(generation, event)))
    }

    /// Keys while Find Files is open: Tab switches between the name field
    /// and the results; in the results ↑/↓, PageUp/PageDown and Home/End
    /// move the highlight and Enter opens it. The name field handles its
    /// own typing (Enter there searches).
    fn find_key(&mut self, modifiers: Modifiers, key: &Key) -> Task<Message> {
        let Some(find) = &mut self.find else {
            return Task::none();
        };
        let tab = *key == Key::Named(Named::Tab)
            && (modifiers.is_empty() || modifiers == Modifiers::SHIFT);
        let down_from_input =
            find.input_focused && modifiers.is_empty() && *key == Key::Named(Named::ArrowDown);
        if tab || down_from_input {
            if find.input_focused && !find.hits.is_empty() {
                find.input_focused = false;
                find.cursor = find.cursor.or(Some(0));
                return Task::batch([unfocus_text_fields(), find.move_cursor(0)]);
            }
            if !find.input_focused {
                find.input_focused = true;
                return widget::text_input::focus(find.input_id.clone());
            }
            return Task::none();
        }
        if find.input_focused || !modifiers.is_empty() {
            return Task::none();
        }
        let page = crate::find::page_rows();
        match key {
            Key::Named(Named::ArrowUp) if find.cursor == Some(0) => {
                // Up from the first result goes back to the name field.
                find.input_focused = true;
                widget::text_input::focus(find.input_id.clone())
            }
            Key::Named(Named::ArrowDown) => find.move_cursor(1),
            Key::Named(Named::ArrowUp) => find.move_cursor(-1),
            Key::Named(Named::PageDown) => find.move_cursor(page),
            Key::Named(Named::PageUp) => find.move_cursor(-page),
            Key::Named(Named::Home) => find.move_cursor(isize::MIN / 2),
            Key::Named(Named::End) => find.move_cursor(isize::MAX / 2),
            Key::Named(Named::Enter) => match find.cursor {
                Some(index) => self.update(Message::FindGoTo(index)),
                None => Task::none(),
            },
            _ => Task::none(),
        }
    }

    fn settings_page(&self) -> Element<'_, Message> {
        // Option names are built on every draw, so they follow the language.
        let language_names: Vec<String> = localize::LANGUAGES
            .iter()
            .map(|(code, name)| match *code {
                "system" => fl!("language-system"),
                _ => name.to_string(),
            })
            .collect();
        let selected_language = localize::LANGUAGES
            .iter()
            .position(|(code, _)| *code == self.settings.language)
            .or(Some(0));
        let icon_style_names: Vec<String> = ICON_STYLES.into_iter().map(icon_style_name).collect();
        let selected_icon_style = ICON_STYLES
            .iter()
            .position(|style| *style == self.settings.icon_style);
        let font_size_names: Vec<String> = FONT_SIZES.into_iter().map(font_size_name).collect();
        let color_theme_names: Vec<String> =
            COLOR_THEMES.into_iter().map(color_theme_name).collect();
        let selected_color_theme = COLOR_THEMES
            .iter()
            .position(|theme| *theme == self.settings.color_theme);
        let corners_names: Vec<String> = CORNERS.into_iter().map(corners_name).collect();
        let selected_corners = CORNERS
            .iter()
            .position(|corners| *corners == self.settings.corners);
        let selected_font_size = FONT_SIZES
            .iter()
            .position(|size| *size == self.settings.name_font_size);

        let general = widget::settings::section()
            .title(fl!("settings-general"))
            .add(self.checkbox_row(
                SettingsRow::ShowHidden,
                fl!("settings-show-hidden"),
                fl!("settings-show-hidden-description"),
                !self.settings.hide_hidden_files,
                Message::ShowHiddenFilesToggled,
            ))
            .add(self.checkbox_row(
                SettingsRow::SeparateExtension,
                fl!("settings-separate-ext"),
                fl!("settings-separate-ext-description"),
                self.settings.separate_extension,
                Message::SeparateExtensionToggled,
            ))
            .add(self.checkbox_row(
                SettingsRow::Thumbnails,
                fl!("settings-thumbnails"),
                fl!("settings-thumbnails-description"),
                self.settings.show_thumbnails,
                Message::ThumbnailsToggled,
            ))
            .add(self.settings_row(
                SettingsRow::Language,
                widget::settings::item(
                    fl!("settings-language"),
                    widget::dropdown(language_names, selected_language, Message::LanguageSelected),
                ),
            ));

        let theme = widget::settings::section()
            .title(fl!("settings-theme"))
            .add(
                self.settings_row(
                    SettingsRow::ColorTheme,
                    widget::settings::item::builder(fl!("settings-color-theme"))
                        .description(fl!("settings-color-theme-description"))
                        .control(widget::dropdown(
                            color_theme_names,
                            selected_color_theme,
                            Message::ColorThemeSelected,
                        )),
                ),
            )
            .add(
                self.settings_row(
                    SettingsRow::Corners,
                    widget::settings::item::builder(fl!("settings-corners"))
                        .description(fl!("settings-corners-description"))
                        .control(widget::dropdown(
                            corners_names,
                            selected_corners,
                            Message::CornersSelected,
                        )),
                ),
            )
            .add(
                self.settings_row(
                    SettingsRow::IconStyle,
                    widget::settings::item::builder(fl!("settings-icon-style"))
                        .description(fl!("settings-icon-style-description"))
                        .control(widget::dropdown(
                            icon_style_names,
                            selected_icon_style,
                            Message::IconStyleSelected,
                        )),
                ),
            )
            .add(
                self.settings_row(
                    SettingsRow::FontSize,
                    widget::settings::item::builder(fl!("settings-font-size"))
                        .description(fl!("settings-font-size-description"))
                        .control(widget::dropdown(
                            font_size_names,
                            selected_font_size,
                            Message::FontSizeSelected,
                        )),
                ),
            )
            .add(self.checkbox_row(
                SettingsRow::RowLines,
                fl!("settings-row-lines"),
                fl!("settings-row-lines-description"),
                self.settings.row_lines,
                Message::RowLinesToggled,
            ));

        let mut editor_names = vec![fl!("editor-system-default")];
        editor_names.extend(self.editors.iter().map(|(_, name)| name.clone()));
        let selected_editor = self
            .editors
            .iter()
            .position(|(id, _)| *id == self.settings.editor)
            .map_or(Some(0), |index| Some(index + 1));
        let programs = widget::settings::section()
            .title(fl!("settings-programs"))
            .add(
                self.settings_row(
                    SettingsRow::Editor,
                    widget::settings::item::builder(fl!("settings-editor"))
                        .description(fl!("settings-editor-description"))
                        .control(widget::dropdown(
                            editor_names,
                            selected_editor,
                            Message::EditorSelected,
                        )),
                ),
            );

        widget::settings::view_column(vec![general.into(), theme.into(), programs.into()]).into()
    }

    /// Windows: Explorer's right-click menu for the pane's selection; runs
    /// the 2fip action it picks (or refreshes after a Windows command).
    #[cfg(windows)]
    fn show_shell_menu(&mut self, id: PaneId) -> Task<Message> {
        let paths = self.pane(id).selected_paths();
        let paste_destination = (paths.len() == 1 && paths[0].is_dir()).then(|| paths[0].clone());
        // 2fip's own "Folder color…" goes at the end of Explorer's menu.
        let pane = self.pane(id);
        let folder_color = paths
            .iter()
            .any(|path| pane.is_listed_folder(path))
            .then(|| fl!("folder-color-ellipsis"));
        match (
            crate::shell_menu::show(&paths, folder_color),
            paste_destination,
        ) {
            (Some(Action::Paste), Some(dest)) => self.paste_to(dest),
            (Some(action), _) => self.handle_action(action),
            (None, _) => Task::none(),
        }
    }

    #[cfg(not(windows))]
    fn show_shell_menu(&mut self, _id: PaneId) -> Task<Message> {
        Task::none()
    }

    /// Files dropped into `dest` (drag and drop, from 2fip or another app):
    /// copied, or moved if the drag asked for it or Shift is held. Files
    /// already in `dest`, and folders dropped into themselves, are skipped.
    fn drop_files(&mut self, dest: PathBuf, paths: Vec<PathBuf>, is_move: bool) -> Task<Message> {
        let sources = drop_sources(&dest, paths);
        if sources.is_empty() {
            return Task::none();
        }
        let detail = queue_detail(&sources, Some(&dest));
        if is_move || self.modifiers.shift() {
            self.note_color_moves(&sources, &dest);
            self.spawn_operation(OpKind::Move, detail, |cancel, conflict| {
                fs_ops::ops::move_paths(sources, dest, cancel, conflict)
            })
        } else {
            self.spawn_operation(OpKind::Copy, detail, |cancel, conflict| {
                fs_ops::ops::copy(sources, dest, cancel, conflict)
            })
        }
    }

    fn open_folder_size(&mut self) -> Task<Message> {
        self.quick_view = None;
        if self
            .folder_size
            .as_ref()
            .is_some_and(|panel| panel.target == self.active_pane)
        {
            self.folder_size = None;
            return Task::none();
        }
        // The selected folder, or (nothing selected) the folder the panel
        // shows: e.g. `/` or `C:\` for a whole drive.
        let Some(root) = self.folder_size_root() else {
            return Task::none();
        };
        let target = self.active_pane.other();
        self.folder_size = Some(FolderSizePanel::loading(
            target,
            root,
            self.listing_options(),
        ));
        self.refresh_folder_size()
    }

    /// Opens the Open With dialog for `paths` and loads the apps. `remember`
    /// starts "Always use this app" ticked (no default app yet).
    fn open_with_dialog_for(&mut self, paths: Vec<PathBuf>, remember: bool) -> Task<Message> {
        let Some(first) = paths.first().cloned() else {
            return Task::none();
        };
        let mime = fs_ops::mime::mime_type(&first);
        self.open_with = Some(OpenWithState {
            paths: paths.clone(),
            apps: None,
            mime: mime.clone(),
            remember,
        });
        cosmic::task::future(async move {
            let apps = tokio::task::spawn_blocking(move || launch::load_apps(mime.as_deref()))
                .await
                .unwrap_or_default();
            Message::OpenWithLoaded(paths, apps)
        })
    }

    /// Shows the folder tree beside the panels the setting names.
    fn apply_folder_tree(&mut self) -> Task<Message> {
        let mode = self.settings.folder_tree;
        Task::batch([
            self.left.set_tree_visible(mode.left()),
            self.right.set_tree_visible(mode.right()),
        ])
    }

    /// What Folder Size would analyze: the single selected folder, else the
    /// active panel's folder; never a server folder.
    fn folder_size_root(&self) -> Option<PathBuf> {
        let pane = self.pane(self.active_pane);
        let root = pane
            .selected_single_dir()
            .or_else(|| pane.selected_paths().is_empty().then(|| pane.current_dir()))?;
        (!fs_ops::vfs::is_remote(&root) && !fs_ops::archive_fs::in_archive(&root)).then_some(root)
    }

    /// (Re)scans the Folder Size panel's folder, reporting progress a few
    /// times a second until the result comes.
    fn refresh_folder_size(&mut self) -> Task<Message> {
        let Some(panel) = self.folder_size.as_mut() else {
            return Task::none();
        };
        let (generation, progress) = panel.start_scan();
        let target = panel.target;
        let root = panel.root.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(8);
        tokio::spawn(async move {
            let scan_progress = progress.clone();
            let mut job = tokio::task::spawn_blocking(move || {
                fs_ops::folder_size::scan(&root, &scan_progress)
            });
            let mut tick = tokio::time::interval(std::time::Duration::from_millis(250));
            loop {
                tokio::select! {
                    result = &mut job => {
                        let result = result
                            .unwrap_or_else(|err| Err(format!("folder size task failed: {err}")));
                        let _ = tx.send(Message::FolderSizeLoaded(target, generation, result)).await;
                        break;
                    }
                    _ = tick.tick() => {
                        let (entries, bytes) = progress.counted();
                        let progress_message =
                            Message::FolderSizeProgress(target, generation, entries, bytes);
                        if tx.send(progress_message).await.is_err() {
                            progress.cancel();
                            break;
                        }
                    }
                }
            }
        });
        cosmic::task::stream(tokio_stream::wrappers::ReceiverStream::new(rx))
    }

    /// Copies (or, after a Cut, moves) the clipboard's paths into the active
    /// pane's current folder.
    fn paste(&mut self) -> Task<Message> {
        let dest = self.pane(self.active_pane).current_dir();
        self.paste_to(dest)
    }

    fn paste_to(&mut self, dest: PathBuf) -> Task<Message> {
        // Pasting always targets the active pane: never search results.
        if self.pane(self.active_pane).read_only() {
            return Task::none();
        }
        let Some(clipboard) = &self.clipboard else {
            return Task::none();
        };
        if !clipboard.cut {
            // Pasted into the folder they came from: duplicates named
            // "name - Copy", as in Explorer. Anything else is copied.
            let (same_folder, sources): (Vec<PathBuf>, Vec<PathBuf>) =
                clipboard.paths.iter().cloned().partition(|path| {
                    path.parent() == Some(dest.as_path())
                        && !fs_ops::vfs::is_remote(path)
                        && !fs_ops::archive_fs::is_inside(path)
                });
            let mut tasks = Vec::new();
            if !same_folder.is_empty() {
                let detail = queue_detail(&same_folder, Some(&dest));
                let suffix = fl!("copy-suffix");
                tasks.push(
                    self.spawn_operation(OpKind::Copy, detail, move |cancel, conflict| {
                        fs_ops::ops::duplicate(same_folder, suffix, cancel, conflict)
                    }),
                );
            }
            if !sources.is_empty() {
                let detail = queue_detail(&sources, Some(&dest));
                tasks.push(
                    self.spawn_operation(OpKind::Copy, detail, |cancel, conflict| {
                        fs_ops::ops::copy(sources, dest, cancel, conflict)
                    }),
                );
            }
            return Task::batch(tasks);
        }
        // Moving something into the folder it's already in is a no-op.
        let sources: Vec<PathBuf> = clipboard
            .paths
            .iter()
            .filter(|path| path.parent() != Some(dest.as_path()))
            .cloned()
            .collect();
        if sources.is_empty() {
            return Task::none();
        }
        // Cut items only move once; they're gone from their old place after.
        self.clipboard = None;
        let detail = queue_detail(&sources, Some(&dest));
        self.note_color_moves(&sources, &dest);
        self.spawn_operation(OpKind::Move, detail, |cancel, conflict| {
            fs_ops::ops::move_paths(sources, dest, cancel, conflict)
        })
    }

    /// Focuses button `index` of the "File Already Exists" dialog.
    fn focus_conflict_button(&mut self, index: usize) -> Task<Message> {
        self.conflict_focus = index % self.conflict_buttons.len();
        widget::button::focus(self.conflict_buttons[self.conflict_focus].clone())
    }

    /// Whether a dialog with a text field (Rename, New Folder, Compress) is
    /// the one showing.
    fn text_dialog_showing(&self) -> bool {
        self.pending_conflict.is_none()
            && (self.rename.is_some() || self.new_folder.is_some() || self.compress.is_some())
    }

    /// Focuses the text dialog's text field (0, with its text selected),
    /// confirm button (1) or Cancel (2).
    fn focus_dialog_field(&mut self, index: usize) -> Task<Message> {
        self.dialog_focus = index % 3;
        match self.dialog_focus {
            0 => Task::batch([
                widget::text_input::focus(self.dialog_input.clone()),
                widget::text_input::select_all(self.dialog_input.clone()),
            ]),
            1 => widget::button::focus(self.dialog_confirm.clone()),
            _ => widget::button::focus(self.dialog_cancel.clone()),
        }
    }

    /// Moves keyboard focus to the Delete dialog's Delete (`true`) or Cancel button.
    fn focus_delete_button(&mut self, confirm: bool) -> Task<Message> {
        self.delete_focus_on_confirm = confirm;
        let id = if confirm {
            &self.confirm_delete_button
        } else {
            &self.cancel_delete_button
        };
        widget::button::focus(id.clone())
    }

    /// Keyboard handling for the Delete dialog: ← focuses Cancel (the left
    /// button), → focuses Delete, Tab switches, and Enter runs the focused one.
    fn delete_dialog_key(&mut self, key: &Key) -> Option<Task<Message>> {
        match key {
            Key::Named(Named::ArrowLeft) => Some(self.focus_delete_button(false)),
            Key::Named(Named::ArrowRight) => Some(self.focus_delete_button(true)),
            Key::Named(Named::Tab) => Some(self.focus_delete_button(!self.delete_focus_on_confirm)),
            // A focused button consumes Enter itself, so this only runs if focus
            // was lost (e.g. a click elsewhere in the dialog) - never twice.
            Key::Named(Named::Enter) => Some(if self.delete_focus_on_confirm {
                self.update(Message::ConfirmDelete)
            } else {
                self.update(Message::CancelDelete)
            }),
            _ => None,
        }
    }

    /// True while a dialog other than the Delete confirmation is showing
    /// (e.g. a file conflict raised by an operation that's still running).
    fn dialog_open_besides_delete(&self) -> bool {
        self.rename.is_some()
            || self.new_folder.is_some()
            || self.pending_conflict.is_some()
            || self.compress.is_some()
            || self.unpack.is_some()
            || self.open_with.is_some()
            || self.details.is_some()
            || self.connect.is_some()
            || self.favorites_config.is_some()
            || self.folder_color_picker.is_some()
    }

    /// True while any modal dialog is showing.
    fn dialog_open(&self) -> bool {
        self.confirm_delete.is_some() || self.dialog_open_besides_delete()
    }

    fn persist_favorites(&self) {
        if let Err(err) = fs_ops::favorites::save(&self.favorites) {
            eprintln!("failed to save favorites: {err}");
        }
    }

    /// Starts a copy or move of the active pane's selection into the other
    /// pane's current directory (queued while another operation runs).
    /// No-ops if nothing is selected.
    fn start_operation(&mut self, kind: OpKind) -> Task<Message> {
        let sources = self.pane(self.active_pane).selected_paths();
        // F5/F6 can't copy or move into search results ("View in Listbox").
        if sources.is_empty() || self.pane(self.active_pane.other()).read_only() {
            return Task::none();
        }
        let dest = self.pane(self.active_pane.other()).current_dir();
        let detail = queue_detail(&sources, Some(&dest));
        match kind {
            OpKind::Copy => self.spawn_operation(kind, detail, |cancel, conflict| {
                fs_ops::ops::copy(sources, dest, cancel, conflict)
            }),
            OpKind::Move => {
                self.note_color_moves(&sources, &dest);
                self.spawn_operation(kind, detail, |cancel, conflict| {
                    fs_ops::ops::move_paths(sources, dest, cancel, conflict)
                })
            }
            OpKind::Delete | OpKind::Compress | OpKind::Unpack => Task::none(),
        }
    }

    /// Tracks a new background operation and forwards its events to `update`.
    /// Starts an operation, or queues it while another one runs (it starts
    /// by itself when that one ends).
    fn spawn_operation<S>(
        &mut self,
        kind: OpKind,
        detail: String,
        start: impl FnOnce(CancelHandle, ConflictHandle) -> S + Send + 'static,
    ) -> Task<Message>
    where
        S: Stream<Item = OpEvent> + Send + 'static,
    {
        self.last_operation_id += 1;
        let queued = QueuedOperation {
            id: self.last_operation_id,
            kind,
            detail,
            start: Box::new(move |cancel, conflict| {
                Box::pin(start(cancel, conflict)) as OperationStream
            }),
        };
        self.queue.push_back(queued);
        self.start_next_operation()
    }

    /// F4: opens `paths` in the editor. Files on a server are watched:
    /// each save in the editor is uploaded back.
    fn edit_files(&mut self, paths: Vec<PathBuf>) -> Task<Message> {
        if paths.is_empty() {
            return Task::none();
        }
        let editor = Some(self.settings.editor.clone()).filter(|id| !id.is_empty());
        let mut tasks = Vec::new();
        for remote in paths.iter().filter(|path| fs_ops::vfs::is_remote(path)) {
            if self.remote_edits.insert(remote.clone()) {
                tasks.push(watch_remote_edit(remote.clone()));
            }
        }
        tasks.push(cosmic::task::future(async move {
            launch::edit(paths, editor).await;
            Message::Launched
        }));
        Task::batch(tasks)
    }

    /// A short message at the bottom, e.g. "Saved to the server" (red for
    /// an error), with a button to close it.
    fn notice_bar(&self) -> Option<Element<'_, Message>> {
        let (id, text, error) = self.notice.as_ref()?;
        let (id, error) = (*id, *error);
        let row = widget::Row::new()
            .spacing(12)
            .align_y(Alignment::Center)
            .push(widget::text::body(text.clone()).width(Length::Fill))
            .push(widget::button::standard(fl!("close")).on_press(Message::HideNotice(id)));
        Some(
            widget::container(row)
                .padding([6, 14])
                .width(Length::Fill)
                .class(cosmic::theme::Container::custom(move |theme| {
                    let cosmic = theme.cosmic();
                    let background: cosmic::iced::Color = if error {
                        cosmic.destructive_color().into()
                    } else {
                        cosmic.accent_color().into()
                    };
                    let foreground: cosmic::iced::Color = cosmic.on_accent_color().into();
                    widget::container::Style {
                        text_color: Some(foreground),
                        icon_color: Some(foreground),
                        background: Some(cosmic::iced::Background::Color(background)),
                        ..Default::default()
                    }
                }))
                .into(),
        )
    }

    /// Shows `text` at the bottom for a few seconds.
    fn show_notice(&mut self, text: String, error: bool) -> Task<Message> {
        self.last_notice += 1;
        let id = self.last_notice;
        self.notice = Some((id, text, error));
        cosmic::task::future(async move {
            tokio::time::sleep(Duration::from_secs(if error { 10 } else { 4 })).await;
            Message::HideNotice(id)
        })
    }

    /// Quick View: shows the file under the other panel's cursor. The
    /// panel showing it can't be the active one.
    fn sync_quick_view(&mut self) -> Task<Message> {
        let Some((target, _)) = &self.quick_view else {
            return Task::none();
        };
        let target = *target;
        if self.active_pane == target {
            self.set_active_pane(target.other());
        }
        let cursor = self.pane(target.other()).cursor_path();
        let Some((_, viewer)) = &mut self.quick_view else {
            return Task::none();
        };
        match cursor {
            Some(path) if path != viewer.file() => {
                viewer_task(viewer.show(path), Message::QuickView)
            }
            _ => Task::none(),
        }
    }

    /// Whether `path` is a folder (in the panel's listing, so it works on
    /// servers and in archives too).
    fn is_folder(&self, path: &Path) -> bool {
        self.pane(self.active_pane).is_listed_folder(path)
    }

    /// Starts the first queued operation, if none is running.
    fn start_next_operation(&mut self) -> Task<Message> {
        if self.operation.is_some() {
            return Task::none();
        }
        let Some(next) = self.queue.pop_front() else {
            return Task::none();
        };
        let id = next.id;
        let cancel = CancelHandle::new();
        let conflict = ConflictHandle::new();
        self.operation = Some(OperationState::new(
            id,
            next.kind,
            cancel.clone(),
            conflict.clone(),
        ));
        cosmic::task::stream(
            (next.start)(cancel, conflict).map(move |event| Message::Operation(id, event)),
        )
    }
}

impl Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "io.github.chillalabs.TwoFip";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, _flags: Self::Flags) -> (Self, Task<Message>) {
        // Outside COSMIC: the desktop's icon theme when COSMIC's is missing.
        crate::fallback_icons::choose_theme();
        // libcosmic's built-in keyboard navigation moves widget focus on Tab,
        // which fights our Tab = "switch pane" (and a focused widget then
        // swallows later Tabs). Turn it off; Escape is handled in `Message::Key`.
        core.set_keyboard_nav(false);
        let home = home_dir();
        // Settings saved under the app's old name (Cosmic Commander) move to
        // ~/.config/2fip the first time 2fip starts.
        fs_ops::settings::migrate_legacy_config();
        let settings = fs_ops::settings::load();
        // Before anything is drawn: every text is looked up in this language.
        localize::set_language(&settings.language);
        let user_dirs = Arc::new(fs_ops::user_dirs::load());
        let folder_colors = Arc::new(fs_ops::folder_colors::load());
        let options = ListingOptions {
            hide_hidden: settings.hide_hidden_files,
            icon_style: settings.icon_style,
            user_dirs: user_dirs.clone(),
            folder_colors: folder_colors.clone(),
            show_thumbnails: settings.show_thumbnails,
            row_lines: settings.row_lines,
            font_size: settings.name_font_size,
            separate_extension: settings.separate_extension,
        };
        // Reopen the panes as they were when the app was last used.
        let session = fs_ops::session::load();
        let (left_dirs, left_active) = session.left.restore(&home);
        let (right_dirs, right_active) = session.right.restore(&home);
        let (left, left_task) = PaneState::new(
            PaneId::Left,
            left_dirs,
            &session.left.tab_column_widths,
            left_active,
            options.clone(),
        );
        let (right, right_task) = PaneState::new(
            PaneId::Right,
            right_dirs,
            &session.right.tab_column_widths,
            right_active,
            options,
        );
        let app = App {
            core,
            left,
            right,
            active_pane: if session.right_active {
                PaneId::Right
            } else {
                PaneId::Left
            },
            keybinds: default_keybinds(),
            operation: None,
            queue: VecDeque::new(),
            show_queue: false,
            last_operation_id: 0,
            last_op_done: None,
            watcher: None,
            confirm_delete: None,
            delete_permanently: false,
            rename: None,
            new_folder: None,
            pending_conflict: None,
            conflict_rename: None,
            favorites: fs_ops::favorites::load(),
            favorites_menu: None,
            favorites_config: None,
            find: None,
            connect: None,
            connect_protocols: crate::connect::protocol_names(),
            saved_connections: fs_ops::connections::load(),
            connection_checks: HashMap::new(),
            settings_cursor: 0,
            confirm_delete_button: widget::Id::unique(),
            cancel_delete_button: widget::Id::unique(),
            dialog_input: widget::Id::unique(),
            dialog_confirm: widget::Id::unique(),
            dialog_cancel: widget::Id::unique(),
            dialog_focus: 0,
            conflict_buttons: std::array::from_fn(|_| widget::Id::unique()),
            conflict_focus: CONFLICT_SKIP,
            delete_focus_on_confirm: true,
            clipboard: None,
            compress: None,
            unpack: None,
            open_with: None,
            details: None,
            folder_size: None,
            editors: Vec::new(),
            remote_edits: HashSet::new(),
            notice: None,
            last_notice: 0,
            viewer: None,
            quick_view: None,
            #[cfg(windows)]
            tab_drag: None,
            drives: Vec::new(),
            settings,
            user_dirs,
            folder_colors,
            color_moves: Vec::new(),
            folder_color_picker: None,
            drawer: None,
            saved_session: session,
            modifiers: Modifiers::empty(),
        };
        // A theme of 2fip's own (or other corners) replaces the desktop's
        // from the first frame on ("System" with COSMIC's standard corners is
        // libcosmic's default, nothing to do).
        let theme_task = if app.settings.color_theme == ColorTheme::System
            && app.settings.corners == Corners::default()
        {
            Task::none()
        } else {
            app.apply_color_theme()
        };
        let drives_task = refresh_drives();
        let editors_task = cosmic::task::future(async {
            let editors = tokio::task::spawn_blocking(launch::text_editors)
                .await
                .unwrap_or_default();
            Message::EditorsLoaded(editors)
        });
        let mut app = app;
        let tree_task = app.apply_folder_tree();
        (
            app,
            Task::batch(vec![
                left_task,
                right_task,
                theme_task,
                drives_task,
                tree_task,
                editors_task,
            ]),
        )
    }

    fn update(&mut self, message: Self::Message) -> Task<Message> {
        let task = self.handle_message(message);
        let task = Task::batch([task, self.sync_quick_view()]);
        self.save_session_if_changed();
        self.close_unused_connections();
        // Keep the inotify watch set tracking the panes' current folders.
        self.sync_watcher();
        task
    }

    fn on_app_exit(&mut self) -> Option<Message> {
        Some(Message::ExitNow)
    }

    fn on_close_requested(&self, id: cosmic::iced::window::Id) -> Option<Message> {
        (self.core.main_window_id() == Some(id)).then_some(Message::ExitNow)
    }

    fn header_start(&self) -> Vec<Element<'_, Message>> {
        let pane = self.pane(self.active_pane);
        let current = pane.current_dir();
        let on_server = fs_ops::vfs::is_remote(&current);
        // The selection is in the panel's folder, so where that is says
        // where the files are (no per-file checks on every redraw).
        let in_archive = fs_ops::archive_fs::in_archive(&current);
        let local_folder = !on_server && !in_archive;
        let selected = pane.selected_paths();
        let other = self.pane(self.active_pane.other());
        let tools = crate::menu_bar::Tools {
            can_find: local_folder,
            can_folder_size: self.folder_size_root().is_some(),
            can_pack: local_folder && !selected.is_empty(),
            can_extract: local_folder
                && selected
                    .iter()
                    .any(|path| fs_ops::ops::ArchiveFormat::of(path).is_some())
                && !other.read_only()
                && !fs_ops::vfs::is_remote(&other.current_dir()),
            folder_tree: self.settings.folder_tree,
            quick_view: self.quick_view.is_some(),
            can_back: pane.can_go_back(),
            can_forward: pane.can_go_forward(),
            can_up: pane.can_go_up(),
        };
        menu_bar(
            &self.keybinds,
            self.clipboard.is_some(),
            pane.view_mode(),
            on_server,
            tools,
        )
    }

    fn context_drawer(&self) -> Option<ContextDrawer<'_, Message>> {
        let drawer = match self.drawer? {
            DrawerPage::Settings => {
                context_drawer::context_drawer(self.settings_page(), Message::CloseDrawer)
                    .title(fl!("settings"))
            }
            DrawerPage::Connections => context_drawer::context_drawer(
                crate::connections_panel::view(&self.saved_connections, &self.connection_checks),
                Message::CloseDrawer,
            )
            .title(fl!("connections")),
            DrawerPage::About => context_drawer::context_drawer(
                crate::about::view(Self::APP_ID),
                Message::CloseDrawer,
            )
            .title(fl!("about-2fip")),
        };
        Some(drawer)
    }

    /// The desktop's theme changed: libcosmic applies it as is, so give it
    /// 2fip's corners again.
    fn system_theme_update(
        &mut self,
        _keys: &[&'static str],
        new_theme: &cosmic::cosmic_theme::Theme,
    ) -> Task<Message> {
        if self.settings.color_theme != ColorTheme::System
            || self.settings.corners == Corners::default()
        {
            return Task::none();
        }
        let theme = cosmic::Theme::system(Arc::new(new_theme.clone()));
        cosmic::command::set_theme(themes::with_corners(&theme, self.settings.corners, true))
    }

    fn on_escape(&mut self) -> Task<Message> {
        if self.favorites_menu.take().is_some()
            || self.favorites_config.take().is_some()
            || self.folder_color_picker.take().is_some()
        {
            return Task::none();
        }
        if !self.dialog_open() && self.viewer.take().is_some() {
            return Task::none();
        }
        if self.rename.take().is_some() {
            return Task::none();
        }
        if self.connect.take().is_some() {
            return Task::none();
        }
        if self.confirm_delete.take().is_some() {
            return Task::none();
        }
        if self.new_folder.take().is_some() {
            return Task::none();
        }
        if self.compress.take().is_some()
            || self.unpack.take().is_some()
            || self.open_with.take().is_some()
            || self.details.take().is_some()
        {
            return Task::none();
        }
        if self.pending_conflict.take().is_some() {
            // Skip is the safe default: unlike Cancel, it can't destroy data.
            if let Some(op) = &self.operation {
                op.conflict.respond(ConflictResolution::Skip);
            }
            return Task::none();
        }
        if self.find.is_some() {
            self.close_find();
            return Task::none();
        }
        if self.pane_mut(self.active_pane).cancel_path_edit() {
            return Task::none();
        }
        if self.pane(self.active_pane).filter_open() {
            self.pane_mut(self.active_pane).close_filter();
            return Task::none();
        }
        if let Some(op) = self.operation.take() {
            op.cancel.cancel();
            return Task::none();
        }
        if self.drawer.is_some() {
            self.close_drawer();
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let keys = if self.left.is_resizing_columns() || self.right.is_resizing_columns() {
            Subscription::batch([key_subscription(), column_resize_subscription()])
        } else {
            key_subscription()
        };
        #[allow(unused_mut)]
        let mut subscriptions = vec![
            keys,
            watcher::subscription().map(Message::WatcherEvent),
            focus_subscription(),
            // A USB stick plugged in shows up in the drive menus.
            cosmic::iced::time::every(Duration::from_secs(3)).map(|_| Message::RefreshDrives),
        ];
        #[cfg(windows)]
        if self.tab_drag.is_some() {
            subscriptions.push(tab_drag_subscription());
        }
        Subscription::batch(subscriptions)
    }

    fn dialog(&self) -> Option<Element<'_, Message>> {
        if let Some(folders) = &self.folder_color_picker {
            return Some(self.folder_color_dialog(folders));
        }

        if let Some(config) = &self.favorites_config {
            return Some(crate::favorites_menu::config_dialog(config).map(Message::Favorites));
        }

        if let Some(rename) = &self.conflict_rename {
            return Some(
                widget::dialog()
                    .title(fl!("rename"))
                    .body(fl!("renaming", name = tab_label(&rename.path)))
                    .control(
                        widget::text_input(fl!("new-name"), rename.name.as_str())
                            .id(self.dialog_input.clone())
                            .on_input(Message::ConflictRenameInputChanged)
                            .on_submit(|_| Message::ConfirmConflictRename),
                    )
                    .primary_action(
                        widget::button::suggested(fl!("rename"))
                            .id(self.dialog_confirm.clone())
                            .on_press(Message::ConfirmConflictRename),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("cancel"))
                            .id(self.dialog_cancel.clone())
                            .on_press(Message::CancelConflictRename),
                    )
                    .into(),
            );
        }

        if let Some((src, dest)) = &self.pending_conflict {
            let body = fl!(
                "conflict-body",
                existing = tab_label(dest),
                new = tab_label(src)
            );
            let [skip_all_id, replace_all_id, skip_id, replace_id] = self.conflict_buttons.clone();
            let all_buttons = widget::Row::new()
                .spacing(8)
                .push(
                    widget::button::standard(fl!("skip-all"))
                        .id(skip_all_id)
                        .on_press(Message::ResolveConflict(ConflictResolution::SkipAll)),
                )
                .push(
                    widget::button::standard(fl!("replace-all"))
                        .id(replace_all_id)
                        .on_press(Message::ResolveConflict(ConflictResolution::ReplaceAll)),
                )
                .push(
                    widget::button::standard(fl!("rename")).on_press(Message::BeginConflictRename),
                )
                .push(widget::button::standard(fl!("cancel")).on_press(Message::CancelConflict));
            return Some(
                widget::dialog()
                    .title(fl!("conflict-title"))
                    .body(body)
                    .control(all_buttons)
                    .primary_action(
                        widget::button::destructive(fl!("replace"))
                            .id(replace_id)
                            .on_press(Message::ResolveConflict(ConflictResolution::Replace)),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("skip"))
                            .id(skip_id)
                            .on_press(Message::ResolveConflict(ConflictResolution::Skip)),
                    )
                    .into(),
            );
        }

        if let Some(pack) = &self.compress {
            return Some(pack_dialog(
                pack,
                self.dialog_input.clone(),
                self.dialog_confirm.clone(),
                self.dialog_cancel.clone(),
            ));
        }

        if let Some(unpack) = &self.unpack {
            return Some(unpack_dialog(unpack));
        }

        if let Some(open_with) = &self.open_with {
            return Some(open_with_dialog(open_with));
        }

        if let Some(details) = &self.details {
            return Some(details_dialog(details));
        }

        if let Some(connect) = &self.connect {
            return Some(crate::connect::view(connect, &self.connect_protocols));
        }

        if let Some(find) = &self.find {
            return Some(crate::find::view(find));
        }

        if let Some(new_folder) = &self.new_folder {
            return Some(
                widget::dialog()
                    .title(if new_folder.file {
                        fl!("new-file")
                    } else {
                        fl!("new-folder")
                    })
                    .control(
                        widget::text_input(
                            if new_folder.file {
                                fl!("file-name")
                            } else {
                                fl!("folder-name")
                            },
                            new_folder.name.as_str(),
                        )
                        .id(self.dialog_input.clone())
                        .on_input(Message::NewFolderInputChanged)
                        .on_submit(|_| Message::ConfirmNewFolder),
                    )
                    .primary_action(
                        widget::button::suggested(fl!("create"))
                            .id(self.dialog_confirm.clone())
                            .on_press(Message::ConfirmNewFolder),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("cancel"))
                            .id(self.dialog_cancel.clone())
                            .on_press(Message::CancelNewFolder),
                    )
                    .into(),
            );
        }

        if let Some(rename) = &self.rename {
            return Some(
                widget::dialog()
                    .title(fl!("rename"))
                    .body(fl!("renaming", name = tab_label(&rename.path)))
                    .control(
                        widget::text_input(fl!("new-name"), rename.name.as_str())
                            .id(self.dialog_input.clone())
                            .on_input(Message::RenameInputChanged)
                            .on_submit(|_| Message::ConfirmRename),
                    )
                    .primary_action(
                        widget::button::suggested(fl!("rename"))
                            .id(self.dialog_confirm.clone())
                            .on_press(Message::ConfirmRename),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("cancel"))
                            .id(self.dialog_cancel.clone())
                            .on_press(Message::CancelRename),
                    )
                    .into(),
            );
        }

        if let Some(sources) = &self.confirm_delete {
            let (title, body) = if self.delete_permanently {
                let body = match sources.as_slice() {
                    [single] => fl!("delete-permanently-one", name = tab_label(single)),
                    many => fl!("delete-permanently-many", count = many.len()),
                };
                (fl!("delete-permanently"), body)
            } else {
                let body = match sources.as_slice() {
                    [single] => fl!("delete-one", name = tab_label(single)),
                    many => fl!("delete-many", count = many.len()),
                };
                (fl!("delete"), body)
            };
            return Some(
                widget::dialog()
                    .title(title.clone())
                    .body(body)
                    .primary_action(
                        widget::button::destructive(title)
                            .id(self.confirm_delete_button.clone())
                            .on_press(Message::ConfirmDelete),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("cancel"))
                            .id(self.cancel_delete_button.clone())
                            .on_press(Message::CancelDelete),
                    )
                    .into(),
            );
        }

        None
    }

    /// The progress panel of a running copy / move / delete / compress: a
    /// strong accent-colored bar (red on error) with a thick progress line,
    /// the percentage, how much is done and the current file.
    fn footer(&self) -> Option<Element<'_, Message>> {
        let Some(op) = self.operation.as_ref() else {
            return self.notice_bar();
        };
        let failed = op.error.is_some();
        let percent = (op.fraction() * 100.0).round() as u32;

        let title = match &op.error {
            Some(_) => op.kind.failed_label(),
            None => fl!(
                "op-progress",
                operation = op.kind.label(),
                percent = percent
            ),
        };
        let mut amounts = Vec::new();
        if op.files_total > 0 {
            amounts.push(fl!(
                "op-files-done",
                done = op.files_done,
                total = op.files_total
            ));
        }
        if op.bytes_total > 0 {
            amounts.push(fl!(
                "op-bytes-done",
                done = format_size(op.bytes_done),
                total = format_size(op.bytes_total)
            ));
        }
        let detail = match (&op.error, &op.current_file) {
            (Some(err), _) => err.clone(),
            (None, Some(file)) => format!("{}  —  {}", amounts.join("  ·  "), tab_label(file)),
            (None, None) => fl!("op-preparing"),
        };

        // The theme's colors, resolved now: the bar's style takes plain colors.
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();
        let background: cosmic::iced::Color = if failed {
            cosmic.destructive_color().into()
        } else {
            cosmic.accent_color().into()
        };
        let foreground: cosmic::iced::Color = cosmic.on_accent_color().into();

        let header = widget::Row::new()
            .spacing(12)
            .align_y(Alignment::Center)
            .push(widget::text::title4(title))
            .push(
                widget::text::body(detail)
                    .width(Length::Fill)
                    .wrapping(cosmic::iced::core::text::Wrapping::None),
            )
            .push_maybe((!self.queue.is_empty()).then(|| {
                let arrow = if self.show_queue { "▴" } else { "▾" };
                widget::button::standard(format!(
                    "{}  {arrow}",
                    fl!("op-queued", count = self.queue.len())
                ))
                .on_press(Message::ToggleQueue)
            }))
            .push(
                widget::button::standard(if failed { fl!("close") } else { fl!("cancel") })
                    .on_press(Message::CancelOperation),
            );
        let bar = widget::determinate_linear(op.fraction())
            .width(Length::Fill)
            .girth(10)
            .class(
                cosmic::widget::progress_bar::style::Class::default()
                    .bar_color(foreground)
                    .track_color(cosmic::iced::Color {
                        a: 0.3,
                        ..foreground
                    }),
            );

        let mut panel = widget::Column::new().spacing(8).push(header).push(bar);
        if self.show_queue {
            // The waiting operations, in the order they will run.
            for (position, queued) in self.queue.iter().enumerate() {
                panel = panel.push(
                    widget::Row::new()
                        .spacing(12)
                        .align_y(Alignment::Center)
                        .push(widget::text::body(format!("{}.", position + 1)))
                        .push(widget::text::heading(queued.kind.label()))
                        .push(
                            widget::text::body(queued.detail.clone())
                                .width(Length::Fill)
                                .wrapping(cosmic::iced::core::text::Wrapping::None),
                        )
                        .push(
                            widget::button::standard(fl!("op-queue-remove"))
                                .on_press(Message::RemoveQueued(queued.id)),
                        ),
                );
            }
        }
        Some(
            widget::container(panel)
                .padding([10, 14])
                .width(Length::Fill)
                .class(cosmic::theme::Container::custom(move |theme| {
                    widget::container::Style {
                        text_color: Some(foreground),
                        icon_color: Some(foreground),
                        background: Some(cosmic::iced::Background::Color(background)),
                        border: cosmic::iced::Border {
                            radius: theme.cosmic().radius_s().into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }))
                .into(),
        )
    }

    fn view(&self) -> Element<'_, Message> {
        if let Some(viewer) = &self.viewer {
            return viewer.view().map(Message::Viewer);
        }
        // How a click on a file changes the selection (Ctrl / Shift held).
        let select_mode = SelectMode::from_modifiers(self.modifiers);
        let can_paste = self.clipboard.is_some();
        let left = match (self.folder_size.as_ref(), self.quick_view.as_ref()) {
            (_, Some((PaneId::Left, viewer))) => viewer.view().map(Message::QuickView),
            (Some(panel), _) if panel.target == PaneId::Left => panel.view(),
            _ => self
                .left
                .view(
                    self.active_pane == PaneId::Left,
                    select_mode,
                    &self.keybinds,
                    can_paste,
                    &self.drives,
                )
                .map(|msg| Message::Pane(PaneId::Left, msg)),
        };
        let right = match (self.folder_size.as_ref(), self.quick_view.as_ref()) {
            (_, Some((PaneId::Right, viewer))) => viewer.view().map(Message::QuickView),
            (Some(panel), _) if panel.target == PaneId::Right => panel.view(),
            _ => self
                .right
                .view(
                    self.active_pane == PaneId::Right,
                    select_mode,
                    &self.keybinds,
                    can_paste,
                    &self.drives,
                )
                .map(|msg| Message::Pane(PaneId::Right, msg)),
        };

        let panes = widget::Row::new()
            .spacing(PANE_GAP)
            .push(self.with_favorites_menu(
                pane_frame(left, PaneId::Left, self.active_pane == PaneId::Left),
                PaneId::Left,
            ))
            .push(self.with_favorites_menu(
                pane_frame(right, PaneId::Right, self.active_pane == PaneId::Right),
                PaneId::Right,
            ))
            .width(Length::Fill)
            .height(Length::Fill);

        widget::Column::new()
            .push(panes)
            .push(function_key_bar())
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

/// Total-Commander-style function-key button row, always visible at the
/// bottom of the window as a mouse-accessible mirror of the matching keybinds.
fn function_key_bar() -> Element<'static, Message> {
    // `button::standard(..).width(Fill)` only widens the label inside the
    // button, so build the button around a centered label and make the button
    // itself fill: every key then gets an equal share of the window's width.
    // "F2" etc. stay as they are; only the action's name is translated.
    let key_button = |key: &str, label: String, action: Action| {
        let label = format!("{key} {label}");
        widget::button::custom(
            widget::text(label)
                .width(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center),
        )
        .class(cosmic::theme::Button::Standard)
        .padding([6, 4])
        .width(Length::Fill)
        .on_press(Message::Action(action))
    };

    widget::Row::new()
        .spacing(4)
        .padding(4)
        .push(key_button("F2", fl!("fkey-rename"), Action::Rename))
        .push(key_button("F3", fl!("fkey-view"), Action::View))
        .push(key_button("F4", fl!("fkey-edit"), Action::Edit))
        .push(key_button("F5", fl!("fkey-copy"), Action::Copy))
        .push(key_button("F6", fl!("fkey-move"), Action::Move))
        .push(key_button("F7", fl!("fkey-mkdir"), Action::NewFolder))
        .push(key_button("F8", fl!("fkey-delete"), Action::Delete))
        .push(key_button("F9", fl!("fkey-terminal"), Action::Terminal))
        .push(key_button("Alt+F4", fl!("fkey-exit"), Action::Quit))
        .width(Length::Fill)
        .into()
}

/// While a list column is being resized: where the mouse goes, and when the
/// button is let go (anywhere in the window).
fn column_resize_subscription() -> Subscription<Message> {
    use cosmic::iced::mouse;
    listen_raw(|event, _status, _window| match event {
        Event::Mouse(mouse::Event::CursorMoved { position }) => Some(Message::ColumnResize(
            PaneMessage::ColumnResizeMoved(position.x),
        )),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
            Some(Message::ColumnResize(PaneMessage::ColumnResizeEnded))
        }
        _ => None,
    })
}

fn key_subscription() -> Subscription<Message> {
    listen_raw(|event, status, _window| match event {
        // Skip keys a focused widget already handled, e.g. typing Delete or
        // Ctrl+A in the Favorites name field mustn't act on the file listing.
        Event::Keyboard(KeyEvent::KeyPressed { key, modifiers, .. })
            if status == Status::Ignored =>
        {
            Some(Message::Key(modifiers, key))
        }
        Event::Keyboard(KeyEvent::KeyPressed {
            key: Key::Named(Named::Escape),
            ..
        }) => Some(Message::EscapeCaptured),
        Event::Keyboard(KeyEvent::ModifiersChanged(modifiers)) => {
            Some(Message::ModifiersChanged(modifiers))
        }
        #[cfg(windows)]
        Event::Window(cosmic::iced::window::Event::FileDropped(paths)) => {
            Some(Message::ExternalDrop(paths))
        }
        _ => None,
    })
}

/// Windows, while a tab is dragged: the release of the left button, wherever
/// the mouse is (the panel under it opens the tab).
#[cfg(windows)]
fn tab_drag_subscription() -> Subscription<Message> {
    listen_raw(|event, _status, _window| match event {
        Event::Mouse(cosmic::iced::mouse::Event::ButtonReleased(
            cosmic::iced::mouse::Button::Left,
        )) => Some(Message::TabDragReleased),
        _ => None,
    })
}

/// Fires when the main window regains focus, so the active pane can refresh.
/// Reads the drive list in the background (it runs `lsblk`).
fn refresh_drives() -> Task<Message> {
    cosmic::task::future(async {
        let drives = tokio::task::spawn_blocking(fs_ops::drives::list)
            .await
            .unwrap_or_default();
        Message::DrivesLoaded(drives)
    })
}

fn focus_subscription() -> Subscription<Message> {
    listen_raw(|event, _status, window| match event {
        Event::Window(cosmic::iced::window::Event::Focused) => Some(Message::WindowFocused(window)),
        _ => None,
    })
}

fn open_with_dialog(state: &OpenWithState) -> Element<'_, Message> {
    let body = match state.paths.as_slice() {
        [single] => fl!("open-with-one", name = tab_label(single)),
        many => fl!("open-with-many", count = many.len()),
    };

    let control: Element<'_, Message> = match &state.apps {
        None => widget::text(fl!("loading-apps")).into(),
        Some(apps) if apps.is_empty() => widget::text(fl!("no-apps")).into(),
        Some(apps) => {
            let mut list = widget::Column::new().spacing(2);
            let mut shown_recommended_header = false;
            let mut shown_other_header = false;
            for (index, app) in apps.iter().enumerate() {
                if app.recommended && !shown_recommended_header {
                    list = list.push(widget::text::heading(fl!("recommended-apps")));
                    shown_recommended_header = true;
                } else if !app.recommended && !app.is_default && !shown_other_header {
                    list = list.push(widget::text::heading(fl!("other-apps")));
                    shown_other_header = true;
                }
                let mut row = widget::Row::new()
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(widget::icon(app.icon.clone()).size(24))
                    .push(widget::text(app.name.clone()).width(Length::Fill));
                if app.is_default {
                    row = row.push(widget::text::caption(fl!("default-app")));
                }
                list = list.push(
                    widget::button::custom(row)
                        .class(cosmic::theme::Button::MenuItem)
                        .width(Length::Fill)
                        .on_press(Message::LaunchApp(index)),
                );
            }
            widget::scrollable(list).height(Length::Fixed(360.0)).into()
        }
    };

    // Remembering is per type: only when the files have one.
    let remember = state.mime.as_ref().map(|mime| {
        widget::checkbox(state.remember)
            .label(fl!("open-with-remember", mime = mime.clone()))
            .on_toggle(Message::OpenWithRemember)
    });
    let control = widget::Column::new()
        .spacing(12)
        .push(control)
        .push_maybe(remember);

    widget::dialog()
        .title(fl!("open-with"))
        .body(body)
        .control(control)
        .secondary_action(widget::button::standard(fl!("cancel")).on_press(Message::CancelOpenWith))
        .into()
}

fn details_dialog(state: &DetailsState) -> Element<'_, Message> {
    let mut rows: Vec<(String, String)> = Vec::new();
    let location = state
        .paths
        .first()
        .and_then(|path| path.parent())
        .map(|parent| parent.display().to_string())
        .unwrap_or_default();

    let title = match state.paths.as_slice() {
        [single] => tab_label(single),
        many => fl!("details-items-title", count = many.len()),
    };

    match &state.result {
        None => rows.push((fl!("details-size-label"), fl!("details-calculating"))),
        Some(Err(err)) => rows.push((fl!("details-error"), err.clone())),
        Some(Ok(details)) => {
            let contents = if details.contained_files + details.contained_dirs > 0 {
                format!(
                    " — {}",
                    fl!(
                        "details-contents",
                        files = details.contained_files,
                        folders = details.contained_dirs
                    )
                )
            } else {
                String::new()
            };
            let size = format!(
                "{}{contents}",
                fl!(
                    "details-size",
                    size = format_size(details.total_size),
                    bytes = details.total_size
                )
            );

            match &details.single {
                None => {
                    rows.push((fl!("details-items"), details.item_count.to_string()));
                    rows.push((fl!("details-location"), location));
                    rows.push((fl!("details-total-size"), size));
                }
                Some(entry) => {
                    let kind = match entry.kind {
                        EntryKind::Dir => fl!("kind-folder"),
                        EntryKind::File => fl!("kind-file"),
                        EntryKind::Symlink => fl!("kind-symlink"),
                    };
                    let kind = match &entry.mime_type {
                        Some(mime) => format!("{kind} ({mime})"),
                        None => kind.to_string(),
                    };
                    rows.push((fl!("details-name"), tab_label(&entry.path)));
                    rows.push((fl!("details-type"), kind));
                    rows.push((fl!("details-location"), location));
                    if let Some(target) = &entry.symlink_target {
                        rows.push((fl!("details-link-target"), target.display().to_string()));
                    }
                    rows.push((fl!("details-size-label"), size));
                    let time = |t: Option<std::time::SystemTime>| {
                        t.map(format_modified)
                            .unwrap_or_else(|| fl!("details-unknown"))
                    };
                    rows.push((fl!("details-modified"), time(entry.modified)));
                    rows.push((fl!("details-accessed"), time(entry.accessed)));
                    rows.push((fl!("details-created"), time(entry.created)));
                    rows.push((
                        fl!("details-permissions"),
                        format!(
                            "{} ({:o})",
                            fs_ops::details::format_mode(entry.mode),
                            entry.mode
                        ),
                    ));
                    rows.push((fl!("details-owner"), entry.owner.clone()));
                    rows.push((fl!("details-group"), entry.group.clone()));
                }
            }
        }
    }

    let mut grid = widget::Column::new().spacing(6);
    for (label, value) in rows {
        grid = grid.push(
            widget::Row::new()
                .spacing(12)
                .push(widget::text::heading(label).width(Length::Fixed(110.0)))
                .push(widget::text(value).width(Length::Fill)),
        );
    }

    widget::dialog()
        .title(title)
        .control(grid)
        .primary_action(widget::button::standard(fl!("close")).on_press(Message::CloseDetails))
        .into()
}

/// `<name>.zip` for a single item (dropping a file's own extension), else
/// `Archive.zip`, numbered if that name is already taken in `dir`.
/// The Pack dialog: where the archive goes, its format, a password (zip
/// and 7z) and "Move to archive".
fn pack_dialog(
    pack: &PackState,
    input: widget::Id,
    confirm: widget::Id,
    cancel: widget::Id,
) -> Element<'_, Message> {
    use fs_ops::ops::ArchiveFormat;
    let body = match pack.sources.as_slice() {
        [single] => fl!("pack-one", name = tab_label(single)),
        many => fl!("pack-many", count = many.len()),
    };
    let formats: Vec<&'static str> = ArchiveFormat::ALL.iter().map(|f| f.extension()).collect();
    let selected = ArchiveFormat::ALL.iter().position(|f| *f == pack.format);
    let password = widget::secure_input(
        if pack.format.encrypts() {
            fl!("pack-password")
        } else {
            fl!("pack-password-unavailable")
        },
        pack.password.as_str(),
        Some(Message::PackTogglePassword),
        pack.password_hidden,
    )
    // Ignored unless the format encrypts (the placeholder says so).
    .on_input(Message::PackPasswordChanged);
    let problem = pack_problem(pack).map(|problem| {
        widget::text::caption(problem).class(cosmic::theme::Text::Color(
            cosmic::iced::Color::from_rgb(0.85, 0.25, 0.25),
        ))
    });
    let controls = widget::Column::new()
        .spacing(10)
        .push(widget::text::caption(fl!("pack-path")))
        .push(
            widget::text_input(fl!("pack-path"), pack.path.as_str())
                .id(input)
                .on_input(Message::PackPathChanged)
                .on_submit(|_| Message::ConfirmPack),
        )
        .push(
            widget::Row::new()
                .spacing(12)
                .align_y(Alignment::Center)
                .push(widget::text::body(fl!("pack-format")))
                .push(widget::dropdown(
                    formats,
                    selected,
                    Message::PackFormatSelected,
                )),
        )
        .push(password)
        .push(
            widget::checkbox(pack.move_files)
                .label(fl!("pack-move"))
                .on_toggle(Message::PackMoveToggled),
        )
        .push_maybe(problem);
    widget::dialog()
        .title(fl!("pack"))
        .body(body)
        .control(controls)
        .primary_action(
            widget::button::suggested(fl!("pack"))
                .id(confirm)
                .on_press_maybe(pack_problem(pack).is_none().then_some(Message::ConfirmPack)),
        )
        .secondary_action(
            widget::button::standard(fl!("cancel"))
                .id(cancel)
                .on_press(Message::CancelPack),
        )
        .into()
}

/// Why the Pack dialog's choices can't be packed (gz and xz hold a single
/// file), if they can't.
fn pack_problem(pack: &PackState) -> Option<String> {
    let one_file = matches!(pack.sources.as_slice(), [single] if single.is_file());
    (pack.format.single_file() && !one_file).then(|| fl!("pack-single-file-only"))
}

/// `path` with its archive extension replaced by `format`'s.
fn with_archive_extension(path: &str, format: fs_ops::ops::ArchiveFormat) -> String {
    let lower = path.to_lowercase();
    let known = [
        ".tar.gz", ".tar.xz", ".zip", ".tar", ".tgz", ".txz", ".gz", ".xz", ".7z",
    ];
    let stem = known
        .iter()
        .find(|suffix| lower.ends_with(*suffix))
        .map_or(path, |suffix| &path[..path.len() - suffix.len()]);
    format!("{stem}.{}", format.extension())
}

/// The Extract dialog: unpack into the other panel, with or without the
/// stored folders, replacing existing files or not, with a password.
fn unpack_dialog(unpack: &UnpackState) -> Element<'_, Message> {
    let body = match unpack.archives.as_slice() {
        [single] => fl!(
            "unpack-one",
            name = tab_label(single),
            dest = unpack.dest.display().to_string()
        ),
        many => fl!(
            "unpack-many",
            count = many.len(),
            dest = unpack.dest.display().to_string()
        ),
    };
    let controls = widget::Column::new()
        .spacing(10)
        .push(
            widget::checkbox(unpack.keep_paths)
                .label(fl!("unpack-keep-paths"))
                .on_toggle(Message::UnpackKeepPathsToggled),
        )
        .push(
            widget::checkbox(unpack.overwrite)
                .label(fl!("unpack-overwrite"))
                .on_toggle(Message::UnpackOverwriteToggled),
        )
        .push(
            widget::secure_input(
                fl!("unpack-password"),
                unpack.password.as_str(),
                Some(Message::UnpackTogglePassword),
                unpack.password_hidden,
            )
            .on_input(Message::UnpackPasswordChanged)
            .on_submit(|_| Message::ConfirmUnpack),
        );
    widget::dialog()
        .title(fl!("unpack"))
        .body(body)
        .control(controls)
        .primary_action(widget::button::suggested(fl!("unpack")).on_press(Message::ConfirmUnpack))
        .secondary_action(widget::button::standard(fl!("cancel")).on_press(Message::CancelUnpack))
        .into()
}

/// Whether `path` is an archive on disk that opens as a folder (Enter,
/// double-click).
pub fn opens_as_folder(path: &std::path::Path) -> bool {
    fs_ops::archive_fs::browsable(path) && path.is_file()
}

fn default_archive_name(sources: &[PathBuf], dir: &std::path::Path) -> String {
    let stem = match sources {
        [single] if single.is_dir() => tab_label(single),
        [single] => single
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| tab_label(single)),
        // Several items: the folder they are in, like Total Commander;
        // "Archive" only when that has no name (e.g. `/`).
        [first, ..] => first
            .parent()
            .and_then(|parent| parent.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| fl!("default-archive-name")),
        [] => fl!("default-archive-name"),
    };
    let mut name = format!("{stem}.zip");
    let mut n = 2;
    while dir.join(&name).exists() {
        name = format!("{stem} ({n}).zip");
        n += 1;
    }
    name
}

/// Space between the two panes, in pixels.
const PANE_GAP: u16 = 5;
/// Width of the border drawn around each pane, in pixels.
const PANE_BORDER: f32 = 2.0;

/// Encloses a pane in a rounded rectangle. The active pane's border uses the
/// accent color; the other uses the text color (white in a dark theme).
/// A click anywhere in it (even on empty space) makes it the active pane;
/// files, buttons, etc. handle their own clicks first.
fn pane_frame(content: Element<'_, Message>, id: PaneId, is_active: bool) -> Element<'_, Message> {
    let frame = widget::container(content)
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .class(cosmic::theme::Container::custom(move |theme| {
            let cosmic = theme.cosmic();
            let color = if is_active {
                cosmic.accent_color()
            } else {
                cosmic.on_bg_color()
            };
            widget::container::Style {
                border: cosmic::iced::Border {
                    color: color.into(),
                    width: PANE_BORDER,
                    radius: cosmic.radius_s().into(),
                },
                ..Default::default()
            }
        }));
    widget::mouse_area(frame)
        .on_press(Message::ActivatePane(id))
        .on_right_press(Message::ActivatePane(id))
        .into()
}

/// The dropped `paths` that can go into `dest`: not the ones already in it,
/// and not a folder into itself or one of its own subfolders.
fn drop_sources(dest: &std::path::Path, paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths
        .into_iter()
        .filter(|path| path.parent() != Some(dest) && !dest.starts_with(path))
        .collect()
}

/// Takes the keyboard focus away from whichever text field has it.
fn unfocus_text_fields() -> Task<Message> {
    cosmic::iced::runtime::task::widget(cosmic::iced::core::widget::operation::focusable::unfocus())
}

/// The keyboard highlight for a list row: the accent color, like a selected
/// file in the listing.
pub(crate) fn highlight_style(
    theme: &cosmic::Theme,
    highlighted: bool,
) -> widget::container::Style {
    if !highlighted {
        return widget::container::Style::default();
    }
    let cosmic = theme.cosmic();
    widget::container::Style {
        icon_color: Some(cosmic.on_accent_color().into()),
        text_color: Some(cosmic.on_accent_color().into()),
        background: Some(cosmic::iced::Background::Color(
            cosmic.accent_color().into(),
        )),
        border: cosmic::iced::Border {
            radius: cosmic.radius_s().into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// What a queued operation works on, for the queue list: "report.pdf" or
/// "3 items", and "→ Downloads" when it has a destination.
fn queue_detail(sources: &[PathBuf], dest: Option<&Path>) -> String {
    let items = match sources {
        [one] => tab_label(one),
        _ => fl!("status-items", count = sources.len()),
    };
    match dest {
        Some(dest) => format!("{items}  →  {}", tab_label(dest)),
        None => items,
    }
}

/// Watches the local copy of the server file `remote` while it's open in
/// an editor (F4), and uploads it each time it's saved. Ends when the
/// connection closes, or after a day.
fn watch_remote_edit(remote: PathBuf) -> Task<Message> {
    use cosmic::iced::futures::SinkExt;
    let Some(local) = fs_ops::vfs::local_copy_path(&remote) else {
        return Task::none();
    };
    let stream = cosmic::iced::stream::channel(4, async move |mut output| {
        let modified = |path: &Path| {
            std::fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok()
        };
        // The download runs at the same time: its result is the starting point.
        let mut seen = None;
        for _ in 0..(24 * 60 * 60 / 2) {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if !fs_ops::vfs::is_connected(&remote) {
                break;
            }
            let now = modified(&local);
            if seen.is_none() {
                seen = now;
                continue;
            }
            if now.is_none() || now == seen {
                continue;
            }
            // Wait until the editor has finished writing.
            tokio::time::sleep(Duration::from_millis(500)).await;
            seen = modified(&local);
            let result = fs_ops::vfs::upload(&local, &remote, &mut |_| {}, &|| false).await;
            let _ = output
                .send(Message::RemoteSaved(remote.clone(), result))
                .await;
        }
        let _ = output.send(Message::RemoteEditEnded(remote)).await;
    });
    cosmic::task::stream(stream)
}

/// A viewer's task, as one of the app's.
fn viewer_task(
    task: cosmic::Task<ViewerMessage>,
    wrap: fn(ViewerMessage) -> Message,
) -> Task<Message> {
    task.map(move |message| cosmic::Action::App(wrap(message)))
}

/// "Open with default app" from the viewer (a local copy for files on a
/// server or in an archive).
fn open_externally(path: PathBuf) -> Task<Message> {
    cosmic::task::future(async move {
        if let Some(local) = launch::local_paths(vec![path]).await.pop() {
            launch::open_with_default_app(&local);
        }
        Message::Launched
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn several_items_pack_under_their_folders_name() {
        let dir = std::env::temp_dir().join(format!("2fip-pack-name-{}", std::process::id()));
        let photos = dir.join("Holidays");
        std::fs::create_dir_all(&photos).unwrap();
        let a = photos.join("a.jpg");
        let b = photos.join("b.jpg");
        assert_eq!(
            default_archive_name(&[a.clone(), b], &photos),
            "Holidays.zip"
        );
        assert_eq!(default_archive_name(&[a], &photos), "a.zip");
        // Taken: numbered.
        std::fs::write(photos.join("Holidays.zip"), b"").unwrap();
        let c = photos.join("c.jpg");
        let d = photos.join("d.jpg");
        assert_eq!(default_archive_name(&[c, d], &photos), "Holidays (2).zip");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn stepping_through_options_wraps_around() {
        assert_eq!(step_index(0, 1, 3), 1);
        assert_eq!(step_index(2, 1, 3), 0);
        assert_eq!(step_index(0, -1, 3), 2);
        assert_eq!(step_index(1, -1, 3), 0);
    }

    #[test]
    fn drops_skip_files_already_there_and_folders_into_themselves() {
        let dest = std::path::Path::new("/home/me/docs");
        let dropped = vec![
            PathBuf::from("/home/me/docs/a.txt"),  // already in docs
            PathBuf::from("/home/me/docs"),        // docs into itself
            PathBuf::from("/home/me"),             // parent into its child
            PathBuf::from("/home/me/music/b.mp3"), // fine
            PathBuf::from("/tmp/c"),               // fine
        ];
        assert_eq!(
            drop_sources(dest, dropped),
            [
                PathBuf::from("/home/me/music/b.mp3"),
                PathBuf::from("/tmp/c")
            ]
        );
    }

    #[test]
    fn every_settings_row_is_listed_once() {
        for row in SETTINGS_ROWS {
            assert_eq!(SETTINGS_ROWS.iter().filter(|r| **r == row).count(), 1);
        }
    }
}
