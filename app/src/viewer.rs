//! The F3 viewer (Total Commander's Lister): text, hex and images inside
//! 2fip, without starting another app. Small text files are fully loaded
//! (select and copy work); big files and hex are read page by page, so a
//! file of several GB opens at once. Quick View (Ctrl+Q) shows the same
//! viewer in the other panel.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use cosmic::iced::keyboard::key::Named;
use cosmic::iced::keyboard::{Key, Modifiers};
use cosmic::iced::mouse::ScrollDelta;
use cosmic::iced::widget::text::LineHeight;
use cosmic::iced::{Alignment, ContentFit, Length, Size};
use cosmic::widget::{self, text_editor};
use cosmic::{Element, Task};

use crate::fl;

/// Text files up to this size are loaded whole, for selecting and copying;
/// bigger ones are read page by page.
const WHOLE_TEXT_MAX: u64 = 2 * 1024 * 1024;
/// How much of a big file is kept in memory around the shown part.
const WINDOW_BEFORE: u64 = 64 * 1024;
const WINDOW_AFTER: u64 = 192 * 1024;
/// A line longer than this is cut in the paged view.
const MAX_LINE: usize = 4000;
const FONT_SIZE: u16 = 13;
const ROW_HEIGHT: f32 = 18.0;
const HEX_ROW: u64 = 16;
/// Files whose first bytes hold a NUL byte are shown as hex.
const SNIFF: usize = 8 * 1024;

/// How the file is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Text,
    Hex,
    Image,
}

/// What a file turned out to be, once read.
#[derive(Debug, Clone)]
pub enum Loaded {
    Image {
        width: u32,
        height: u32,
    },
    Svg,
    /// A small text file, decoded.
    Text(String),
    /// A big text file or a binary one: read page by page.
    Paged {
        binary: bool,
    },
}

#[derive(Debug, Clone)]
pub enum ViewerMessage {
    /// The file was read (load generation, local copy, result).
    Loaded(u64, PathBuf, u64, Result<Loaded, String>),
    Editor(text_editor::Action),
    Scroll(ScrollDelta),
    /// The scroll bar moved: 0 = top, 1 = end.
    ScrollTo(f64),
    /// The paged view's size, to know how many rows fit.
    Resized(Size),
    SetMode(Mode),
    ToggleWrap,
    ZoomIn,
    ZoomOut,
    ZoomFit,
    Next,
    Previous,
    OpenExternally,
    Close,
    SearchInput(String),
    SearchNext,
    SearchPrevious,
    /// A search in a big file finished (search generation, match offset).
    SearchDone(u64, Option<u64>),
}

/// What the app has to do after a viewer message.
pub enum Event {
    Close,
    /// Open this file (the original path) with its default app.
    OpenExternally(PathBuf),
}

enum Body {
    Loading,
    Failed(String),
    Image(widget::image::Handle, u32, u32),
    Svg(widget::svg::Handle),
    Text {
        content: text_editor::Content,
        text: String,
    },
    Paged(Paged),
}

pub struct Viewer {
    /// The file as the panel knows it (maybe on a server or in an archive).
    pub path: PathBuf,
    /// The files of the panel, in its order, for Next / Previous.
    files: Vec<PathBuf>,
    /// In Quick View: no Next / Previous, no Close (the panel decides).
    quick: bool,
    generation: u64,
    local: Option<PathBuf>,
    size: u64,
    /// The mode chosen with the keys / buttons, kept for the next file.
    wanted: Option<Mode>,
    mode: Mode,
    binary: bool,
    wrap: bool,
    /// `None` = fit to the window.
    zoom: Option<f32>,
    body: Body,
    search: String,
    search_generation: u64,
    searching: bool,
    not_found: bool,
    /// Byte position of the last match in a small text file.
    text_hit: Option<usize>,
}

impl Viewer {
    /// A viewer for `path` (`files`: the panel's files, for Next / Previous),
    /// and the task that reads it.
    pub fn open(path: PathBuf, files: Vec<PathBuf>, quick: bool) -> (Self, Task<ViewerMessage>) {
        let mut viewer = Self {
            path: path.clone(),
            files,
            quick,
            generation: 0,
            local: None,
            size: 0,
            wanted: None,
            mode: Mode::Text,
            binary: false,
            wrap: true,
            zoom: None,
            body: Body::Loading,
            search: String::new(),
            search_generation: 0,
            searching: false,
            not_found: false,
            text_hit: None,
        };
        let task = viewer.load(path, None);
        (viewer, task)
    }

    /// Shows another file in the same viewer (Next / Previous, Quick View).
    pub fn show(&mut self, path: PathBuf) -> Task<ViewerMessage> {
        self.path = path.clone();
        self.body = Body::Loading;
        self.text_hit = None;
        self.not_found = false;
        self.load(path, self.wanted)
    }

    fn load(&mut self, path: PathBuf, force: Option<Mode>) -> Task<ViewerMessage> {
        self.generation += 1;
        let generation = self.generation;
        cosmic::task::future(async move {
            let Some(local) = crate::launch::local_paths(vec![path.clone()]).await.pop() else {
                let error = fl!("viewer-unreadable");
                return ViewerMessage::Loaded(generation, path, 0, Err(error));
            };
            let read = local.clone();
            let result = tokio::task::spawn_blocking(move || read_file(&read, force))
                .await
                .unwrap_or_else(|err| Err(err.to_string()));
            let size = std::fs::metadata(&local).map_or(0, |m| m.len());
            ViewerMessage::Loaded(generation, local, size, result)
        })
    }

    /// The original path of the file shown.
    pub fn file(&self) -> &Path {
        &self.path
    }

    pub fn update(&mut self, message: ViewerMessage) -> (Task<ViewerMessage>, Option<Event>) {
        let none = (Task::none(), None);
        match message {
            ViewerMessage::Loaded(generation, local, size, result) => {
                if generation != self.generation {
                    return none;
                }
                self.size = size;
                self.local = Some(local.clone());
                self.body = match result {
                    Err(err) => Body::Failed(err),
                    Ok(Loaded::Image { width, height }) => {
                        self.mode = Mode::Image;
                        Body::Image(widget::image::Handle::from_path(&local), width, height)
                    }
                    Ok(Loaded::Svg) => {
                        self.mode = Mode::Image;
                        Body::Svg(widget::svg::Handle::from_path(&local))
                    }
                    Ok(Loaded::Text(text)) => {
                        self.mode = Mode::Text;
                        self.binary = false;
                        Body::Text {
                            content: text_editor::Content::with_text(&text),
                            text,
                        }
                    }
                    Ok(Loaded::Paged { binary }) => {
                        self.binary = binary;
                        self.mode = match self.wanted {
                            Some(Mode::Text) => Mode::Text,
                            _ if binary => Mode::Hex,
                            Some(mode) if mode != Mode::Image => mode,
                            _ => Mode::Text,
                        };
                        let mut paged = Paged::new(local, size, self.mode == Mode::Hex);
                        paged.fill();
                        Body::Paged(paged)
                    }
                };
                none
            }
            ViewerMessage::Editor(action) => {
                if let Body::Text { content, .. } = &mut self.body {
                    // Read only: everything but editing (moving, selecting).
                    if !action.is_edit() {
                        content.perform(action);
                    }
                }
                none
            }
            ViewerMessage::Scroll(delta) => {
                let lines = match delta {
                    ScrollDelta::Lines { y, .. } => -(y * 3.0),
                    ScrollDelta::Pixels { y, .. } => -(y / ROW_HEIGHT),
                };
                if let Body::Paged(paged) = &mut self.body {
                    paged.scroll(lines.round() as i64);
                }
                none
            }
            ViewerMessage::ScrollTo(value) => {
                if let Body::Paged(paged) = &mut self.body {
                    paged.jump(1.0 - value);
                }
                none
            }
            ViewerMessage::Resized(size) => {
                if let Body::Paged(paged) = &mut self.body {
                    paged.rows = ((size.height / ROW_HEIGHT).floor() as usize).max(1);
                    paged.columns =
                        ((size.width / (FONT_SIZE as f32 * 0.6)).floor() as usize).max(8);
                }
                none
            }
            ViewerMessage::SetMode(mode) => (self.set_mode(mode), None),
            ViewerMessage::ToggleWrap => {
                self.wrap = !self.wrap;
                none
            }
            ViewerMessage::ZoomIn => {
                self.zoom = Some((self.zoom.unwrap_or(1.0) * 1.25).min(16.0));
                none
            }
            ViewerMessage::ZoomOut => {
                self.zoom = Some((self.zoom.unwrap_or(1.0) / 1.25).max(0.05));
                none
            }
            ViewerMessage::ZoomFit => {
                self.zoom = None;
                none
            }
            ViewerMessage::Next | ViewerMessage::Previous => {
                let forward = matches!(message, ViewerMessage::Next);
                match self.neighbor(forward) {
                    Some(next) => (self.show(next), None),
                    None => none,
                }
            }
            ViewerMessage::OpenExternally => {
                (Task::none(), Some(Event::OpenExternally(self.path.clone())))
            }
            ViewerMessage::Close => (Task::none(), Some(Event::Close)),
            ViewerMessage::SearchInput(text) => {
                self.search = text;
                self.not_found = false;
                none
            }
            ViewerMessage::SearchNext => (self.search(true), None),
            ViewerMessage::SearchPrevious => (self.search(false), None),
            ViewerMessage::SearchDone(generation, hit) => {
                if generation != self.search_generation {
                    return none;
                }
                self.searching = false;
                match (hit, &mut self.body) {
                    (Some(hit), Body::Paged(paged)) => {
                        paged.show_hit(hit, self.search.len() as u64)
                    }
                    _ => self.not_found = true,
                }
                none
            }
        }
    }

    fn set_mode(&mut self, mode: Mode) -> Task<ViewerMessage> {
        if mode == self.mode {
            return Task::none();
        }
        self.wanted = Some(mode);
        match (&mut self.body, mode) {
            // Between hex and text in the paged view: same file, same place.
            (Body::Paged(paged), Mode::Text | Mode::Hex) => {
                self.mode = mode;
                paged.set_hex(mode == Mode::Hex);
                Task::none()
            }
            (Body::Loading | Body::Failed(_), _) => Task::none(),
            // Image → text / hex, or a small text file → hex: read again.
            _ => {
                self.body = Body::Loading;
                self.load(self.path.clone(), Some(mode))
            }
        }
    }

    /// The panel's next (or previous) file after the one shown.
    fn neighbor(&self, forward: bool) -> Option<PathBuf> {
        let at = self.files.iter().position(|file| *file == self.path)?;
        let next = if forward { at + 1 } else { at.checked_sub(1)? };
        self.files.get(next).cloned()
    }

    fn search(&mut self, forward: bool) -> Task<ViewerMessage> {
        if self.search.is_empty() {
            return Task::none();
        }
        self.not_found = false;
        let needle = self.search.to_ascii_lowercase();
        match &mut self.body {
            Body::Text { content, text } => {
                let haystack = text.to_ascii_lowercase();
                let found = match (forward, self.text_hit) {
                    (true, Some(hit)) => haystack
                        .get(hit + 1..)
                        .and_then(|rest| rest.find(&needle))
                        .map(|at| at + hit + 1),
                    (true, None) => haystack.find(&needle),
                    (false, Some(hit)) => {
                        haystack.get(..hit).and_then(|before| before.rfind(&needle))
                    }
                    (false, None) => haystack.rfind(&needle),
                };
                match found {
                    Some(at) => {
                        self.text_hit = Some(at);
                        let (line, column) = line_and_column(text, at);
                        content.move_to(text_editor::Cursor {
                            position: text_editor::Position {
                                line,
                                column: column + needle.len(),
                            },
                            selection: Some(text_editor::Position { line, column }),
                        });
                    }
                    None => self.not_found = true,
                }
                Task::none()
            }
            Body::Paged(paged) => {
                self.search_generation += 1;
                self.searching = true;
                let generation = self.search_generation;
                let path = paged.path.clone();
                let from = match (paged.hit, forward) {
                    (Some((hit, _)), true) => hit + 1,
                    (Some((hit, _)), false) => hit,
                    (None, true) => paged.offset,
                    (None, false) => paged.offset,
                };
                cosmic::task::future(async move {
                    let hit = tokio::task::spawn_blocking(move || {
                        find_in_file(&path, needle.as_bytes(), from, forward)
                    })
                    .await
                    .ok()
                    .flatten();
                    ViewerMessage::SearchDone(generation, hit)
                })
            }
            _ => Task::none(),
        }
    }

    /// The viewer's keys; `None` for keys it doesn't use.
    pub fn key(&self, modifiers: Modifiers, key: &Key) -> Option<ViewerMessage> {
        let plain = modifiers.is_empty();
        let message = match key {
            Key::Character(c) if plain => match c.as_str() {
                "1" => ViewerMessage::SetMode(Mode::Text),
                "2" => ViewerMessage::SetMode(Mode::Hex),
                "3" if self.is_image() => ViewerMessage::SetMode(Mode::Image),
                "w" => ViewerMessage::ToggleWrap,
                "n" if !self.quick => ViewerMessage::Next,
                "p" if !self.quick => ViewerMessage::Previous,
                "+" | "=" => ViewerMessage::ZoomIn,
                "-" => ViewerMessage::ZoomOut,
                "0" => ViewerMessage::ZoomFit,
                _ => return None,
            },
            Key::Character(c) if c.as_str() == "+" => ViewerMessage::ZoomIn,
            Key::Named(Named::F3) if plain => ViewerMessage::SearchNext,
            Key::Named(Named::F3) if modifiers == Modifiers::SHIFT => ViewerMessage::SearchPrevious,
            Key::Named(Named::ArrowDown) if plain && self.paged() => {
                ViewerMessage::Scroll(lines(-1.0))
            }
            Key::Named(Named::ArrowUp) if plain && self.paged() => {
                ViewerMessage::Scroll(lines(1.0))
            }
            Key::Named(Named::PageDown) if plain && self.paged() => {
                ViewerMessage::Scroll(self.page_delta(-1.0))
            }
            Key::Named(Named::PageUp) if plain && self.paged() => {
                ViewerMessage::Scroll(self.page_delta(1.0))
            }
            Key::Named(Named::Home) if self.paged() => ViewerMessage::ScrollTo(1.0),
            Key::Named(Named::End) if self.paged() => ViewerMessage::ScrollTo(0.0),
            _ => return None,
        };
        Some(message)
    }

    fn paged(&self) -> bool {
        matches!(self.body, Body::Paged(_))
    }

    fn is_image(&self) -> bool {
        matches!(self.body, Body::Image(..) | Body::Svg(_))
            || self.wanted == Some(Mode::Image)
            || self.local.as_deref().is_some_and(is_image_file)
    }

    fn page_delta(&self, sign: f32) -> ScrollDelta {
        let rows = match &self.body {
            Body::Paged(paged) => paged.rows.saturating_sub(1).max(1),
            _ => 20,
        };
        // `Scroll` multiplies lines by 3 (the wheel's step).
        lines(sign * rows as f32 / 3.0)
    }

    pub fn view(&self) -> Element<'_, ViewerMessage> {
        let body: Element<'_, ViewerMessage> = match &self.body {
            Body::Loading => centered(widget::text::body(fl!("viewer-loading"))),
            Body::Failed(err) => centered(widget::text::body(err.clone())),
            Body::Image(handle, width, height) => match self.zoom {
                None => widget::image(handle.clone())
                    .content_fit(ContentFit::ScaleDown)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into(),
                Some(zoom) => widget::scrollable(
                    widget::image(handle.clone())
                        .width(Length::Fixed(*width as f32 * zoom))
                        .height(Length::Fixed(*height as f32 * zoom)),
                )
                .direction(cosmic::iced::widget::scrollable::Direction::Both {
                    vertical: Default::default(),
                    horizontal: Default::default(),
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            },
            Body::Svg(handle) => widget::svg(handle.clone())
                .content_fit(ContentFit::Contain)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            Body::Text { content, .. } => text_editor::text_editor(content)
                .on_action(ViewerMessage::Editor)
                // Read only: typing keys pass through to the viewer's own
                // shortcuts (N, P, 1, 2, W…) instead of editing.
                .key_binding(
                    |key_press| match text_editor::Binding::from_key_press(key_press)? {
                        text_editor::Binding::Insert(_)
                        | text_editor::Binding::Enter
                        | text_editor::Binding::Backspace
                        | text_editor::Binding::Delete
                        | text_editor::Binding::Cut
                        | text_editor::Binding::Paste
                        | text_editor::Binding::Unfocus => None,
                        binding => Some(binding),
                    },
                )
                .font(cosmic::font::mono())
                .size(FONT_SIZE)
                .height(Length::Fill)
                .padding(8)
                .wrapping(if self.wrap {
                    cosmic::iced::widget::text::Wrapping::WordOrGlyph
                } else {
                    cosmic::iced::widget::text::Wrapping::None
                })
                .into(),
            Body::Paged(paged) => paged.view(self.wrap),
        };
        widget::Column::new()
            .push(self.toolbar())
            .push(widget::divider::horizontal::default())
            .push(
                widget::container(body)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn toolbar(&self) -> Element<'_, ViewerMessage> {
        let icon = |name: &'static str, message: ViewerMessage, tip: String| {
            widget::tooltip(
                widget::button::icon(widget::icon::from_name(name)).on_press(message),
                widget::text::body(tip),
                widget::tooltip::Position::Bottom,
            )
        };
        let mode_button = |label: String, mode: Mode| {
            let button = if self.mode == mode {
                widget::button::suggested(label)
            } else {
                widget::button::standard(label)
            };
            button.on_press(ViewerMessage::SetMode(mode))
        };
        let name = crate::tab::tab_label(&self.path);
        let mut bar = widget::Row::new()
            .spacing(6)
            .padding([4, 8])
            .align_y(Alignment::Center);
        if !self.quick {
            bar = bar
                .push(icon(
                    "go-previous-symbolic",
                    ViewerMessage::Previous,
                    fl!("viewer-previous"),
                ))
                .push(icon(
                    "go-next-symbolic",
                    ViewerMessage::Next,
                    fl!("viewer-next"),
                ));
        }
        bar = bar
            .push(widget::text::heading(name).wrapping(cosmic::iced::widget::text::Wrapping::None))
            .push(widget::text::caption(crate::file_item::format_size(
                self.size,
            )))
            .push(widget::space::horizontal())
            .push(mode_button(fl!("viewer-text"), Mode::Text))
            .push(mode_button(fl!("viewer-hex"), Mode::Hex));
        if self.is_image() {
            bar = bar
                .push(mode_button(fl!("viewer-image"), Mode::Image))
                .push(icon(
                    "zoom-out-symbolic",
                    ViewerMessage::ZoomOut,
                    fl!("viewer-zoom-out"),
                ))
                .push(icon(
                    "zoom-fit-best-symbolic",
                    ViewerMessage::ZoomFit,
                    fl!("viewer-zoom-fit"),
                ))
                .push(icon(
                    "zoom-in-symbolic",
                    ViewerMessage::ZoomIn,
                    fl!("viewer-zoom-in"),
                ));
        }
        if self.mode == Mode::Text {
            bar = bar.push(
                widget::checkbox(self.wrap)
                    .label(fl!("viewer-wrap"))
                    .on_toggle(|_| ViewerMessage::ToggleWrap),
            );
        }
        if self.mode != Mode::Image {
            let status = if self.searching {
                fl!("viewer-searching")
            } else if self.not_found {
                fl!("viewer-not-found")
            } else {
                String::new()
            };
            bar = bar
                .push(
                    widget::search_input(fl!("viewer-find"), &self.search)
                        .on_input(ViewerMessage::SearchInput)
                        .on_submit(|_| ViewerMessage::SearchNext)
                        .width(Length::Fixed(200.0)),
                )
                .push(icon(
                    "go-up-symbolic",
                    ViewerMessage::SearchPrevious,
                    fl!("viewer-find-previous"),
                ))
                .push(icon(
                    "go-down-symbolic",
                    ViewerMessage::SearchNext,
                    fl!("viewer-find-next"),
                ))
                .push(widget::text::caption(status));
        }
        bar = bar.push(icon(
            "document-open-symbolic",
            ViewerMessage::OpenExternally,
            fl!("viewer-open-externally"),
        ));
        if !self.quick {
            bar = bar.push(icon(
                "window-close-symbolic",
                ViewerMessage::Close,
                fl!("viewer-close"),
            ));
        }
        bar.into()
    }
}

fn lines(y: f32) -> ScrollDelta {
    ScrollDelta::Lines { x: 0.0, y }
}

fn centered<'a>(content: impl Into<Element<'a, ViewerMessage>>) -> Element<'a, ViewerMessage> {
    widget::container(content).center(Length::Fill).into()
}

/// The 0-based line and the byte column of byte `at` in `text`.
fn line_and_column(text: &str, at: usize) -> (usize, usize) {
    let before = &text[..at];
    let line = before.matches('\n').count();
    let start = before.rfind('\n').map_or(0, |newline| newline + 1);
    (line, at - start)
}

fn is_image_file(path: &Path) -> bool {
    fs_ops::mime::mime_type(path).is_some_and(|mime| mime.starts_with("image/"))
}

/// F3: whether the file should open in its own app instead (PDFs, videos,
/// documents): a binary file that isn't an image, with a default app.
pub fn wants_own_app(path: &Path) -> bool {
    if is_image_file(path) {
        return false;
    }
    let mut head = vec![0u8; SNIFF];
    let binary = File::open(path)
        .and_then(|mut file| read_up_to(&mut file, &mut head))
        .is_ok_and(|read| head[..read].contains(&0));
    binary && crate::launch::has_default_app(path)
}

/// Reads `path` and decides how to show it (`force`: a mode the user chose).
fn read_file(path: &Path, force: Option<Mode>) -> Result<Loaded, String> {
    let metadata = std::fs::metadata(path).map_err(|err| err.to_string())?;
    if metadata.is_dir() {
        return Err(fl!("viewer-folder"));
    }
    let mime = fs_ops::mime::mime_type(path).unwrap_or_default();
    if force.is_none() || force == Some(Mode::Image) {
        if mime == "image/svg+xml" {
            return Ok(Loaded::Svg);
        }
        if mime.starts_with("image/") {
            if let Ok((width, height)) = image::image_dimensions(path) {
                return Ok(Loaded::Image { width, height });
            }
        }
    }
    let mut file = File::open(path).map_err(|err| err.to_string())?;
    let mut head = vec![0u8; SNIFF];
    let read = read_up_to(&mut file, &mut head).map_err(|err| err.to_string())?;
    let binary = head[..read].contains(&0);
    if force == Some(Mode::Hex) || binary || metadata.len() > WHOLE_TEXT_MAX {
        return Ok(Loaded::Paged {
            binary: binary || force == Some(Mode::Hex),
        });
    }
    let bytes = std::fs::read(path).map_err(|err| err.to_string())?;
    Ok(Loaded::Text(decode(&bytes).replace("\r\n", "\n")))
}

fn read_up_to(file: &mut File, buffer: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match file.read(&mut buffer[filled..])? {
            0 => break,
            read => filled += read,
        }
    }
    Ok(filled)
}

/// UTF-8, else Latin-1 (every byte is a character there, so old files with
/// accents still read).
fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => bytes.iter().map(|&byte| byte as char).collect(),
    }
}

/// Finds `needle` (lowercase ASCII-insensitive) in the file from byte
/// `from`, forward, or backward before it.
fn find_in_file(path: &Path, needle: &[u8], from: u64, forward: bool) -> Option<u64> {
    const CHUNK: u64 = 4 * 1024 * 1024;
    let mut file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let overlap = needle.len() as u64 - 1;
    let finder = memchr::memmem::Finder::new(needle);
    let finder_rev = memchr::memmem::FinderRev::new(needle);
    let mut buffer = Vec::new();
    let read_at = |file: &mut File, start: u64, end: u64, buffer: &mut Vec<u8>| -> Option<()> {
        buffer.resize((end - start) as usize, 0);
        file.seek(SeekFrom::Start(start)).ok()?;
        file.read_exact(buffer).ok()?;
        buffer.make_ascii_lowercase();
        Some(())
    };
    if forward {
        let mut start = from;
        while start < len {
            let end = (start + CHUNK + overlap).min(len);
            read_at(&mut file, start, end, &mut buffer)?;
            if let Some(at) = finder.find(&buffer) {
                return Some(start + at as u64);
            }
            start += CHUNK;
        }
    } else {
        let mut end = (from + overlap).min(len);
        while end > 0 {
            let start = end.saturating_sub(CHUNK + overlap);
            read_at(&mut file, start, end, &mut buffer)?;
            if let Some(at) = finder_rev.rfind(&buffer) {
                let hit = start + at as u64;
                if hit < from {
                    return Some(hit);
                }
            }
            end = start + overlap;
            if start == 0 {
                break;
            }
        }
    }
    None
}

/// A big file or hex view: only the part around `offset` is in memory.
struct Paged {
    path: PathBuf,
    len: u64,
    hex: bool,
    /// The first byte shown: a line's start (text) or a multiple of 16 (hex).
    offset: u64,
    window_start: u64,
    window: Vec<u8>,
    rows: usize,
    columns: usize,
    /// The search match shown (offset, length).
    hit: Option<(u64, u64)>,
}

impl Paged {
    fn new(path: PathBuf, len: u64, hex: bool) -> Self {
        Self {
            path,
            len,
            hex,
            offset: 0,
            window_start: 0,
            window: Vec::new(),
            rows: 30,
            columns: 120,
            hit: None,
        }
    }

    /// Makes sure the bytes around `offset` are in memory.
    fn fill(&mut self) {
        let have_start = self.window_start;
        let have_end = self.window_start + self.window.len() as u64;
        let need_end = (self.offset + WINDOW_AFTER / 2).min(self.len);
        let need_start = self.offset.saturating_sub(WINDOW_BEFORE / 2);
        if !self.window.is_empty() && need_start >= have_start && need_end <= have_end {
            return;
        }
        let start = self.offset.saturating_sub(WINDOW_BEFORE);
        let end = (self.offset + WINDOW_AFTER).min(self.len);
        let mut window = vec![0u8; (end - start) as usize];
        let read = File::open(&self.path).and_then(|mut file| {
            file.seek(SeekFrom::Start(start))?;
            read_up_to(&mut file, &mut window)
        });
        if let Ok(read) = read {
            window.truncate(read);
            self.window_start = start;
            self.window = window;
        }
    }

    fn set_hex(&mut self, hex: bool) {
        self.hex = hex;
        self.offset = if hex {
            self.offset - self.offset % HEX_ROW
        } else {
            self.line_start(self.offset)
        };
        self.fill();
    }

    /// Moves by `rows` rows (negative: up).
    fn scroll(&mut self, rows: i64) {
        if self.hex {
            let step = rows.unsigned_abs() * HEX_ROW;
            self.offset = if rows < 0 {
                self.offset.saturating_sub(step)
            } else {
                (self.offset + step).min(self.last_page())
            };
        } else {
            for _ in 0..rows.unsigned_abs() {
                let next = if rows < 0 {
                    self.previous_line(self.offset)
                } else {
                    self.next_line(self.offset)
                };
                match next {
                    Some(next) => self.offset = next,
                    None => break,
                }
            }
            if rows > 0 {
                self.offset = self.offset.min(self.last_page());
            }
        }
        self.fill();
    }

    /// Jumps to `fraction` of the file (0 = start, 1 = end).
    fn jump(&mut self, fraction: f64) {
        let target = (fraction.clamp(0.0, 1.0) * self.len as f64) as u64;
        self.offset = if self.hex {
            (target - target % HEX_ROW).min(self.last_page())
        } else {
            self.line_start(target.min(self.len)).min(self.last_page())
        };
        self.fill();
    }

    /// Shows the match at `hit` a few rows from the top.
    fn show_hit(&mut self, hit: u64, length: u64) {
        self.hit = Some((hit, length));
        self.offset = if self.hex {
            hit - hit % HEX_ROW
        } else {
            self.line_start(hit)
        };
        self.fill();
        self.scroll(-3);
    }

    /// Where the last full page starts.
    fn last_page(&mut self) -> u64 {
        let rows = self.rows.saturating_sub(1) as u64;
        if self.hex {
            let total = self.len.div_ceil(HEX_ROW);
            return total.saturating_sub(rows + 1) * HEX_ROW;
        }
        let mut at = self.line_start(self.len.saturating_sub(1));
        for _ in 0..rows {
            match self.previous_line(at) {
                Some(previous) => at = previous,
                None => break,
            }
        }
        at
    }

    fn byte(&mut self, at: u64) -> Option<u8> {
        if at >= self.len {
            return None;
        }
        if at < self.window_start || at >= self.window_start + self.window.len() as u64 {
            let keep = self.offset;
            self.offset = at;
            self.fill();
            self.offset = keep;
        }
        self.window.get((at - self.window_start) as usize).copied()
    }

    /// The start of the line holding byte `at`.
    fn line_start(&mut self, at: u64) -> u64 {
        let mut at = at;
        let mut steps = 0;
        while at > 0 && steps < MAX_LINE {
            if self.byte(at - 1) == Some(b'\n') {
                return at;
            }
            at -= 1;
            steps += 1;
        }
        at
    }

    /// The start of the line after the one starting at `at`.
    fn next_line(&mut self, at: u64) -> Option<u64> {
        let mut position = at;
        let mut steps = 0;
        while position < self.len && steps < MAX_LINE {
            if self.byte(position) == Some(b'\n') {
                return (position + 1 < self.len).then_some(position + 1);
            }
            position += 1;
            steps += 1;
        }
        // A very long line goes on in the next row.
        (position < self.len).then_some(position)
    }

    /// The start of the line before the one starting at `at`.
    fn previous_line(&mut self, at: u64) -> Option<u64> {
        if at == 0 {
            return None;
        }
        Some(self.line_start(at - 1))
    }

    fn view(&self, wrap: bool) -> Element<'_, ViewerMessage> {
        let rows = if self.hex {
            self.hex_rows()
        } else {
            self.text_rows(wrap)
        };
        let mut column = widget::Column::new().padding([4, 8]);
        for (start, end, line) in rows {
            let text = widget::text(line)
                .font(cosmic::font::mono())
                .size(FONT_SIZE)
                .line_height(LineHeight::Absolute(ROW_HEIGHT.into()))
                .wrapping(cosmic::iced::widget::text::Wrapping::None);
            let hit = self
                .hit
                .is_some_and(|(hit, length)| hit < end && hit + length > start);
            column = column.push(if hit {
                widget::container(text)
                    .width(Length::Fill)
                    .class(cosmic::theme::Container::custom(|theme| {
                        let mut background = theme.cosmic().accent_color();
                        background.alpha = 0.3;
                        widget::container::Style {
                            background: Some(cosmic::iced::Background::Color(background.into())),
                            ..Default::default()
                        }
                    }))
                    .into()
            } else {
                Element::from(text)
            });
        }
        let area = widget::mouse_area(
            widget::container(column)
                .width(Length::Fill)
                .height(Length::Fill)
                .clip(true),
        )
        .on_scroll(ViewerMessage::Scroll);
        let sensor = cosmic::iced::widget::sensor(area)
            .on_show(ViewerMessage::Resized)
            .on_resize(ViewerMessage::Resized);
        let fraction = if self.len == 0 {
            0.0
        } else {
            self.offset as f64 / self.len as f64
        };
        let bar = cosmic::iced::widget::vertical_slider(
            0.0..=1.0,
            1.0 - fraction,
            ViewerMessage::ScrollTo,
        )
        .step(0.0001);
        widget::Row::new()
            .push(sensor)
            .push(bar)
            .height(Length::Fill)
            .into()
    }

    /// The bytes shown as text rows: (start, end, line).
    fn text_rows(&self, wrap: bool) -> Vec<(u64, u64, String)> {
        let mut rows = Vec::with_capacity(self.rows);
        let Some(mut at) = self
            .offset
            .checked_sub(self.window_start)
            .map(|at| at as usize)
        else {
            return rows;
        };
        let width = if wrap { self.columns.max(8) } else { MAX_LINE };
        while rows.len() < self.rows && at < self.window.len() {
            let rest = &self.window[at..];
            let newline = memchr::memchr(b'\n', rest);
            let line_end = newline.unwrap_or(rest.len()).min(MAX_LINE);
            let mut line = &rest[..line_end];
            if line.last() == Some(&b'\r') {
                line = &line[..line.len() - 1];
            }
            let text = decode(line).replace('\t', "    ");
            let start = self.window_start + at as u64;
            let next = match newline {
                Some(newline) if newline <= MAX_LINE => at + newline + 1,
                _ => at + line_end,
            };
            let end = self.window_start + next as u64;
            if wrap && text.chars().count() > width {
                let chars: Vec<char> = text.chars().collect();
                for piece in chars.chunks(width) {
                    if rows.len() == self.rows {
                        break;
                    }
                    rows.push((start, end, piece.iter().collect()));
                }
            } else {
                rows.push((start, end, text));
            }
            if next == at {
                break;
            }
            at = next;
        }
        rows
    }

    /// `0000001A30  48 65 6C 6C 6F …  Hello…`
    fn hex_rows(&self) -> Vec<(u64, u64, String)> {
        let mut rows = Vec::with_capacity(self.rows);
        let Some(mut at) = self
            .offset
            .checked_sub(self.window_start)
            .map(|at| at as usize)
        else {
            return rows;
        };
        while rows.len() < self.rows && at < self.window.len() {
            let end = (at + HEX_ROW as usize).min(self.window.len());
            let bytes = &self.window[at..end];
            let mut line = format!("{:010X}  ", self.window_start + at as u64);
            for column in 0..HEX_ROW as usize {
                match bytes.get(column) {
                    Some(byte) => line.push_str(&format!("{byte:02X} ")),
                    None => line.push_str("   "),
                }
                if column == 7 {
                    line.push(' ');
                }
            }
            line.push(' ');
            line.extend(bytes.iter().map(|&byte| {
                if byte.is_ascii_graphic() || byte == b' ' {
                    byte as char
                } else {
                    '.'
                }
            }));
            let start = self.window_start + at as u64;
            rows.push((start, start + bytes.len() as u64, line));
            at = end;
        }
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paged(text: &[u8]) -> (tempdir::Dir, Paged) {
        let dir = tempdir::Dir::new();
        let path = dir.0.join("file.txt");
        std::fs::write(&path, text).unwrap();
        let mut paged = Paged::new(path, text.len() as u64, false);
        paged.rows = 3;
        paged.fill();
        (dir, paged)
    }

    /// A folder removed on drop (the app crate has no tempfile).
    mod tempdir {
        pub struct Dir(pub std::path::PathBuf);
        impl Dir {
            pub fn new() -> Self {
                use std::sync::atomic::{AtomicUsize, Ordering};
                static NEXT: AtomicUsize = AtomicUsize::new(0);
                let dir = std::env::temp_dir().join(format!(
                    "2fip-viewer-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                std::fs::create_dir_all(&dir).unwrap();
                Dir(dir)
            }
        }
        impl Drop for Dir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    #[test]
    fn pages_through_lines() {
        let (_dir, mut paged) = paged(b"one\ntwo\r\nthree\nfour\nfive\n");
        let rows: Vec<String> = paged
            .text_rows(false)
            .into_iter()
            .map(|row| row.2)
            .collect();
        assert_eq!(rows, ["one", "two", "three"]);
        paged.scroll(1);
        assert_eq!(paged.text_rows(false)[0].2, "two");
        // The last page keeps three rows on screen.
        paged.scroll(100);
        assert_eq!(paged.text_rows(false)[0].2, "three");
        paged.scroll(-100);
        assert_eq!(paged.offset, 0);
        paged.jump(1.0);
        assert_eq!(paged.text_rows(false)[0].2, "three");
    }

    #[test]
    fn hex_rows_show_offset_bytes_and_text() {
        let (_dir, mut paged) = paged(b"Hello, world! 0123456789");
        paged.set_hex(true);
        let rows = paged.hex_rows();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].2.starts_with("0000000000  48 65 6C 6C 6F"));
        assert!(rows[0].2.ends_with("Hello, world! 01"));
        assert!(rows[1].2.starts_with("0000000010  32 33"));
    }

    #[test]
    fn finds_text_both_ways_ignoring_case() {
        let (dir, _) = paged(b"");
        let path = dir.0.join("search.txt");
        std::fs::write(&path, b"abc Needle def needle ghi").unwrap();
        assert_eq!(find_in_file(&path, b"needle", 0, true), Some(4));
        assert_eq!(find_in_file(&path, b"needle", 5, true), Some(15));
        assert_eq!(find_in_file(&path, b"needle", 15, false), Some(4));
        assert_eq!(find_in_file(&path, b"missing", 0, true), None);
    }

    #[test]
    fn latin1_files_still_read() {
        assert_eq!(decode(b"caf\xe9"), "café");
        assert_eq!(decode("ñandú".as_bytes()), "ñandú");
        assert_eq!(line_and_column("ab\ncd\nef", 7), (2, 1));
    }
}
