use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use cosmic::iced::widget::canvas::{self, Program};
use cosmic::iced::{mouse, Alignment, Background, Border, Color, Length, Point, Rectangle};
use cosmic::widget::menu::action::MenuAction;
use cosmic::widget::menu::{self, Item as MenuItem};
use cosmic::widget::{self, icon};
use cosmic::Element;
use fs_ops::folder_size::{DriveSpace, Entry, Scan, ScanProgress};

use crate::app::{Message, PaneId};
use crate::file_item::{format_size, ListingOptions};
use crate::fl;
use crate::pane::name_text;
use crate::themes;

pub struct Panel {
    pub target: PaneId,
    pub root: PathBuf,
    tree: Option<Entry>,
    loading: bool,
    /// The running scan (to follow and stop it), and which one it is, so a
    /// late result of a replaced scan is ignored.
    scan: Option<std::sync::Arc<ScanProgress>>,
    generation: u64,
    /// Files and folders counted, and their size, while scanning.
    counted: (u64, u64),
    /// What the scan found besides the tree.
    unreadable: u64,
    drive: Option<DriveSpace>,
    is_drive_root: bool,
    error: Option<String>,
    expanded: HashSet<PathBuf>,
    focused: PathBuf,
    context_path: Option<PathBuf>,
    hovered: Option<(PathBuf, String, String)>,
    options: ListingOptions,
}

const MAX_OUTER_RINGS: usize = 5;
/// Rows shown per expanded folder (its largest entries); the rest are
/// summed up in one "… N more" row, so huge folders stay quick to show.
const MAX_ROWS_PER_FOLDER: usize = 500;
const TAU: f32 = std::f32::consts::PI * 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FolderSizeMenuAction {
    MoveToTrash(PaneId),
    OpenInNewTab(PaneId),
}

impl MenuAction for FolderSizeMenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            Self::MoveToTrash(target) => Message::FolderSizeMoveToTrash(*target),
            Self::OpenInNewTab(target) => Message::FolderSizeOpenInNewTab(*target),
        }
    }
}

const COLOR_STEPS: [(f32, f32, f32); 10] = [
    (0.83, 0.15, 0.18),
    (0.93, 0.28, 0.2),
    (0.98, 0.46, 0.23),
    (0.98, 0.63, 0.15),
    (0.95, 0.78, 0.2),
    (0.83, 0.8, 0.21),
    (0.58, 0.8, 0.21),
    (0.33, 0.72, 0.31),
    (0.18, 0.64, 0.46),
    (0.12, 0.66, 0.52),
];

impl Drop for Panel {
    /// Closing the panel stops its scan.
    fn drop(&mut self) {
        if let Some(scan) = &self.scan {
            scan.cancel();
        }
    }
}

impl Panel {
    pub fn loading(target: PaneId, root: PathBuf, options: ListingOptions) -> Self {
        Self {
            target,
            expanded: HashSet::from([root.clone()]),
            focused: root.clone(),
            root,
            tree: None,
            loading: true,
            scan: None,
            generation: 0,
            counted: (0, 0),
            unreadable: 0,
            drive: None,
            is_drive_root: false,
            error: None,
            context_path: None,
            hovered: None,
            options,
        }
    }

    /// New listing options (File name size, icon style) from Settings.
    pub fn set_options(&mut self, options: ListingOptions) {
        self.options = options;
    }

    /// Starts following a new scan (stopping the previous one): its
    /// generation, and the progress it will report to.
    pub fn start_scan(&mut self) -> (u64, std::sync::Arc<ScanProgress>) {
        if let Some(previous) = self.scan.take() {
            previous.cancel();
        }
        let progress = std::sync::Arc::new(ScanProgress::new());
        self.scan = Some(progress.clone());
        self.generation += 1;
        self.loading = true;
        self.error = None;
        self.counted = (0, 0);
        (self.generation, progress)
    }

    /// Progress of scan `generation`.
    pub fn set_progress(&mut self, generation: u64, entries: u64, bytes: u64) {
        if generation == self.generation && self.loading {
            self.counted = (entries, bytes);
        }
    }

    pub fn set_result(&mut self, generation: u64, result: Result<Scan, String>) {
        if generation != self.generation {
            return;
        }
        self.loading = false;
        self.scan = None;
        self.error = None;
        match result {
            Ok(scan) => {
                if find_entry(&scan.tree, &self.focused).is_none() {
                    self.focused = scan.tree.path.clone();
                }
                self.unreadable = scan.unreadable;
                self.drive = scan.drive;
                self.is_drive_root = scan.is_drive_root;
                self.tree = Some(scan.tree);
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub fn toggle(&mut self, path: PathBuf) {
        if !self.expanded.remove(&path) {
            self.expanded.insert(path);
        }
    }

    pub fn select(&mut self, path: PathBuf) {
        let Some(tree) = self.tree.as_ref() else {
            return;
        };

        let previous_focus = self.focused.clone();
        if path == self.focused {
            if let Some(parent) = find_parent_entry(tree, &path) {
                self.focused = parent.path.clone();
                self.expanded.insert(self.focused.clone());
            }
        } else if let Some(entry) = find_entry(tree, &path).filter(|entry| entry.is_dir) {
            self.focused = entry.path.clone();
            self.expanded.insert(self.focused.clone());
        }
        if self.focused != previous_focus {
            self.hovered = None;
        }
    }

    pub fn set_context_path(&mut self, path: PathBuf) {
        self.context_path = Some(path);
    }

    pub fn take_context_path(&mut self) -> Option<PathBuf> {
        self.context_path.take()
    }

    pub fn set_hovered(&mut self, hovered: Option<(PathBuf, String, String)>) {
        self.hovered = hovered;
    }

    pub fn view(&self) -> Element<'_, Message> {
        let path = self.focused.display().to_string();
        let header = widget::Row::new()
            .spacing(10)
            .align_y(Alignment::Center)
            .push(
                widget::Column::new()
                    .spacing(2)
                    .push(widget::text::title4(fl!("folder-size")))
                    .push(widget::text::caption(path).width(Length::Fill)),
            )
            .push(widget::Space::new().width(Length::Fill))
            .push(themes::tooltip(
                widget::button::icon(icon::from_name("window-close-symbolic"))
                    .on_press(Message::FolderSizeClose),
                widget::text(fl!("close")),
                widget::tooltip::Position::Bottom,
            ));

        let content: Element<'_, Message> = if self.loading {
            self.progress_view()
        } else if let Some(error) = &self.error {
            widget::text::body(error.clone()).into()
        } else if let Some(tree) = &self.tree {
            self.split_view(tree)
        } else {
            widget::text::body(fl!("folder-size-loading")).into()
        };

        widget::Column::new()
            .spacing(8)
            .push(header)
            .push_maybe(self.drive_summary())
            .push(widget::divider::horizontal::default())
            .push(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// While scanning: what has been counted so far, and a Cancel button.
    fn progress_view(&self) -> Element<'_, Message> {
        let (entries, bytes) = self.counted;
        widget::Column::new()
            .spacing(12)
            .push(widget::text::body(fl!("folder-size-loading")))
            .push(widget::text::caption(fl!(
                "folder-size-scanning",
                count = entries,
                size = format_size(bytes)
            )))
            .push(widget::button::standard(fl!("cancel")).on_press(Message::FolderSizeClose))
            .into()
    }

    /// The drive's bar (used, total, free) and what the scan couldn't count.
    fn drive_summary(&self) -> Option<Element<'_, Message>> {
        let drive = self.drive.filter(|drive| drive.total > 0)?;
        let mut summary = widget::Column::new()
            .spacing(4)
            .push(widget::text::caption(fl!(
                "folder-size-drive",
                used = format_size(drive.used),
                total = format_size(drive.total),
                free = format_size(drive.available)
            )))
            .push(
                cosmic::iced::widget::progress_bar(0.0..=drive.total as f32, drive.used as f32)
                    .girth(Length::Fixed(8.0)),
            );
        if !self.loading {
            // At the top of a drive, what the drive uses beyond what was
            // counted: system folders 2fip can't see, file system data.
            let counted = self.tree.as_ref().map_or(0, |tree| tree.size);
            let other = drive.used.saturating_sub(counted);
            if self.is_drive_root && other > 0 {
                summary = summary.push(widget::text::caption(fl!(
                    "folder-size-other",
                    size = format_size(other)
                )));
            }
            if self.unreadable > 0 {
                summary = summary.push(widget::text::caption(fl!(
                    "folder-size-unreadable",
                    count = self.unreadable
                )));
            }
        }
        Some(summary.into())
    }

    fn split_view(&self, tree: &Entry) -> Element<'_, Message> {
        let top = widget::container(self.tree_view(tree)).height(Length::FillPortion(1));
        let bottom = widget::container(self.chart_view(tree)).height(Length::FillPortion(1));

        widget::Column::new()
            .spacing(8)
            .push(top)
            .push(widget::divider::horizontal::default())
            .push(bottom)
            .height(Length::Fill)
            .into()
    }

    fn tree_view(&self, tree: &Entry) -> Element<'_, Message> {
        let focus = find_entry(tree, &self.focused).unwrap_or(tree);
        let headers = widget::Row::new()
            .spacing(8)
            .push(widget::text::caption_heading(fl!("column-name")).width(Length::Fill))
            .push(
                widget::text::caption_heading(fl!("folder-size-graph")).width(Length::Fixed(128.0)),
            )
            .push(
                widget::text::caption_heading(fl!("column-size"))
                    .width(Length::Fixed(88.0))
                    .align_x(cosmic::iced::alignment::Horizontal::Right),
            )
            .push(
                widget::text::caption_heading(fl!("folder-size-parent-share"))
                    .width(Length::Fixed(88.0))
                    .align_x(cosmic::iced::alignment::Horizontal::Right),
            )
            .push(
                widget::text::caption_heading(fl!("folder-size-root-share"))
                    .width(Length::Fixed(88.0))
                    .align_x(cosmic::iced::alignment::Horizontal::Right),
            );

        let mut visible = Vec::new();
        collect_visible(
            focus,
            0,
            SizeContext {
                parent_size: focus.size,
                root_size: focus.size,
                sibling_max_size: focus.size,
            },
            &self.expanded,
            &self.focused,
            &mut visible,
        );
        let mut rows = widget::Column::new().spacing(1);
        for entry in visible {
            rows = rows.push(entry_row(entry, self.target, &self.options));
        }

        widget::Column::new()
            .spacing(4)
            .push(headers)
            .push(widget::scrollable(rows).height(Length::Fill))
            .width(Length::Fill)
            .into()
    }

    fn chart_view(&self, tree: &Entry) -> Element<'_, Message> {
        let focus = find_entry(tree, &self.focused).unwrap_or(tree);
        let parent_path = find_parent_entry(tree, &focus.path).map(|parent| parent.path.clone());
        let canvas = widget::container(
            widget::canvas::Canvas::new(RingChart::new(focus, parent_path, self.target))
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill);

        if let Some((path, name, size)) = &self.hovered {
            let context_menu = widget::context_menu(canvas, entry_context_menu(self.target))
                .on_open(Message::FolderSizeContextOpen(self.target, path.clone()));
            themes::tooltip(
                context_menu,
                widget::text::body(format!("{}  {size}", short_name(name))),
                widget::tooltip::Position::FollowCursor,
            )
            .into()
        } else {
            canvas.into()
        }
    }
}

struct VisibleEntry {
    /// A "… N more" row standing for the entries left out.
    more: Option<usize>,
    name: String,
    path: PathBuf,
    size: u64,
    depth: usize,
    is_dir: bool,
    has_children: bool,
    expanded: bool,
    selected: bool,
    color: Color,
    parent_percent: String,
    root_percent: String,
    root_percent_value: f32,
}

#[derive(Clone, Copy)]
struct SizeContext {
    parent_size: u64,
    root_size: u64,
    sibling_max_size: u64,
}

fn collect_visible(
    entry: &Entry,
    depth: usize,
    size_context: SizeContext,
    expanded: &HashSet<PathBuf>,
    focused: &PathBuf,
    visible: &mut Vec<VisibleEntry>,
) {
    let is_expanded = expanded.contains(&entry.path);
    visible.push(VisibleEntry {
        more: None,
        name: entry.name.clone(),
        path: entry.path.clone(),
        size: entry.size,
        depth,
        is_dir: entry.is_dir,
        has_children: !entry.children.is_empty(),
        expanded: is_expanded,
        selected: &entry.path == focused,
        color: color_for_ratio(entry.size, size_context.sibling_max_size),
        parent_percent: percent(entry.size, size_context.parent_size),
        root_percent: percent(entry.size, size_context.root_size),
        root_percent_value: percent_value(entry.size, size_context.root_size),
    });

    if entry.is_dir && is_expanded {
        let child_max_size = entry
            .children
            .iter()
            .map(|child| child.size)
            .max()
            .unwrap_or_default();
        for child in entry.children.iter().take(MAX_ROWS_PER_FOLDER) {
            collect_visible(
                child,
                depth + 1,
                SizeContext {
                    parent_size: entry.size,
                    root_size: size_context.root_size,
                    sibling_max_size: child_max_size,
                },
                expanded,
                focused,
                visible,
            );
        }
        let hidden = &entry.children[entry.children.len().min(MAX_ROWS_PER_FOLDER)..];
        if !hidden.is_empty() {
            let size: u64 = hidden.iter().map(|child| child.size).sum();
            visible.push(VisibleEntry {
                more: Some(hidden.len()),
                name: String::new(),
                path: entry.path.clone(),
                size,
                depth: depth + 1,
                is_dir: false,
                has_children: false,
                expanded: false,
                selected: false,
                color: color_for_ratio(size, child_max_size),
                parent_percent: percent(size, entry.size),
                root_percent: percent(size, size_context.root_size),
                root_percent_value: percent_value(size, size_context.root_size),
            });
        }
    }
}

/// The "… N more" row: the left-out entries' count and total size.
fn more_row(
    entry: &VisibleEntry,
    count: usize,
    options: &ListingOptions,
) -> Element<'static, Message> {
    let indent = entry.depth as f32 * 18.0
        + f32::from(crate::pane::list_icon_size(options.font_size)) * 2.0
        + 12.0;
    widget::container(
        widget::Row::new()
            .spacing(8)
            .align_y(Alignment::Center)
            .push(widget::Space::new().width(Length::Fixed(indent)))
            .push(
                name_text(fl!("folder-size-more", count = count), options.font_size)
                    .width(Length::Fill),
            )
            .push(widget::Space::new().width(Length::Fixed(128.0)))
            .push(
                name_text(format_size(entry.size), options.font_size)
                    .width(Length::Fixed(88.0))
                    .align_x(cosmic::iced::alignment::Horizontal::Right),
            )
            .push(
                name_text(entry.parent_percent.clone(), options.font_size)
                    .width(Length::Fixed(88.0))
                    .align_x(cosmic::iced::alignment::Horizontal::Right),
            )
            .push(
                name_text(entry.root_percent.clone(), options.font_size)
                    .width(Length::Fixed(88.0))
                    .align_x(cosmic::iced::alignment::Horizontal::Right),
            ),
    )
    .padding([
        crate::pane::row_padding_y(options.font_size).saturating_sub(1),
        6,
    ])
    .width(Length::Fill)
    .into()
}

fn entry_row(
    entry: VisibleEntry,
    target: PaneId,
    options: &ListingOptions,
) -> Element<'static, Message> {
    if let Some(count) = entry.more {
        return more_row(&entry, count, options);
    }
    let disclosure = if entry.is_dir && entry.has_children {
        Some(if entry.expanded {
            "pan-down-symbolic"
        } else {
            "pan-end-symbolic"
        })
    } else {
        None
    };
    let disclosure_icon: Element<'static, Message> = if let Some(disclosure) = disclosure {
        // Small and unpadded, so it doesn't make the rows taller than the text.
        widget::button::icon(icon::from_name(disclosure))
            .icon_size(crate::pane::list_icon_size(options.font_size))
            .padding(0)
            .on_press(Message::FolderSizeToggle(target, entry.path.clone()))
            .into()
    } else {
        widget::Space::new()
            .width(Length::Fixed(f32::from(crate::pane::list_icon_size(
                options.font_size,
            ))))
            .into()
    };
    let mut name = widget::Row::new()
        .spacing(6)
        .align_y(Alignment::Center)
        .push(widget::Space::new().width(Length::Fixed(entry.depth as f32 * 18.0)))
        .push(disclosure_icon);
    name = name
        .push(
            widget::icon(crate::file_item::icon_for_path(
                &entry.path,
                entry.is_dir,
                options,
            ))
            .size(crate::pane::list_icon_size(options.font_size)),
        )
        .push(
            name_text(entry.name, options.font_size)
                .wrapping(cosmic::iced::core::text::Wrapping::None),
        )
        .width(Length::Fill);

    let row = widget::Row::new()
        .spacing(8)
        .align_y(Alignment::Center)
        .push(name)
        .push(
            cosmic::iced::widget::progress_bar(0.0..=100.0, entry.root_percent_value)
                .length(Length::Fixed(128.0))
                .girth(Length::Fixed(12.0)),
        )
        .push(
            name_text(format_size(entry.size), options.font_size)
                .width(Length::Fixed(88.0))
                .align_x(cosmic::iced::alignment::Horizontal::Right),
        )
        .push(
            name_text(entry.parent_percent, options.font_size)
                .width(Length::Fixed(88.0))
                .align_x(cosmic::iced::alignment::Horizontal::Right),
        )
        .push(
            name_text(entry.root_percent, options.font_size)
                .width(Length::Fixed(88.0))
                .align_x(cosmic::iced::alignment::Horizontal::Right),
        );
    let row_color = if entry.selected {
        Color::from_rgb(0.18, 0.48, 0.92)
    } else {
        entry.color
    };
    let row = widget::container(row)
        .padding([
            crate::pane::row_padding_y(options.font_size).saturating_sub(1),
            6,
        ])
        .width(Length::Fill)
        .class(cosmic::theme::Container::custom(move |_| {
            widget::container::Style {
                background: Some(Background::Color(color_with_alpha(row_color, 0.2))),
                border: Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }));

    let row = if entry.selected {
        widget::container(row)
            .padding([1, 2])
            .class(cosmic::theme::Container::custom(move |_| {
                widget::container::Style {
                    border: Border {
                        color: color_with_alpha(row_color, 0.8),
                        width: 1.0,
                        radius: 7.0.into(),
                    },
                    ..Default::default()
                }
            }))
    } else {
        row
    };

    let area = widget::mouse_area(row);
    let area = if entry.is_dir {
        area.on_press(Message::FolderSizeSelect(target, entry.path.clone()))
    } else {
        area
    };
    widget::context_menu(area, entry_context_menu(target))
        .on_open(Message::FolderSizeContextOpen(target, entry.path))
        .into()
}

fn entry_context_menu(target: PaneId) -> Option<Vec<menu::Tree<Message>>> {
    let key_binds = HashMap::new();
    let items = vec![
        MenuItem::Button(
            fl!("move-to-trash"),
            Some(icon::from_name("user-trash-symbolic").handle()),
            FolderSizeMenuAction::MoveToTrash(target),
        ),
        MenuItem::Button(
            fl!("open-path-in-new-tab"),
            Some(icon::from_name("tab-new-symbolic").handle()),
            FolderSizeMenuAction::OpenInNewTab(target),
        ),
    ];
    Some(menu::items(&key_binds, items))
}

fn percent(size: u64, total: u64) -> String {
    format!("{:.1}%", percent_value(size, total))
}

fn percent_value(size: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        (size as f64 * 100.0 / total as f64) as f32
    }
}

fn color_for_ratio(size: u64, largest_sibling: u64) -> Color {
    let ratio = if largest_sibling == 0 {
        1.0
    } else {
        (size as f64 / largest_sibling as f64) as f32
    }
    .clamp(0.0, 1.0);
    let index = ((1.0 - ratio) * COLOR_STEPS.len() as f32).floor() as usize;
    let step = COLOR_STEPS[index.min(COLOR_STEPS.len() - 1)];
    Color::from_rgb(step.0, step.1, step.2)
}

fn color_with_alpha(color: Color, alpha: f32) -> Color {
    Color::from_rgba(color.r, color.g, color.b, alpha)
}

/// The entry at `path`. Goes straight down the folders that contain it
/// (one level per path component) instead of visiting the whole tree, which
/// takes a third of a second for a home folder of a million files.
fn find_entry<'a>(entry: &'a Entry, path: &Path) -> Option<&'a Entry> {
    let mut current = entry;
    loop {
        if current.path == *path {
            return Some(current);
        }
        current = current
            .children
            .iter()
            .find(|child| path.starts_with(&child.path))?;
    }
}

/// The folder that directly contains `path` (same quick descent).
fn find_parent_entry<'a>(entry: &'a Entry, path: &Path) -> Option<&'a Entry> {
    // Walks down rather than looking up `path.parent()`: in the Flatpak, the
    // drive's `usr` folder is `/run/host/usr`, directly under `/`.
    let mut parent = entry;
    loop {
        let child = parent
            .children
            .iter()
            .find(|child| path.starts_with(&child.path))?;
        if child.path == path {
            return Some(parent);
        }
        parent = child;
    }
}

#[derive(Clone)]
struct RingSegment {
    path: PathBuf,
    name: String,
    size: u64,
    depth: usize,
    start: f32,
    end: f32,
    color: Color,
}

struct RingChart {
    target: PaneId,
    parent_path: Option<PathBuf>,
    center_path: PathBuf,
    center_name: String,
    center_size: String,
    segments: Vec<RingSegment>,
}

impl RingChart {
    fn new(focus: &Entry, parent_path: Option<PathBuf>, target: PaneId) -> Self {
        let mut chart = Self {
            target,
            parent_path,
            center_path: focus.path.clone(),
            center_name: focus.name.clone(),
            center_size: format_size(focus.size),
            segments: Vec::new(),
        };
        chart.add_children(focus, 1, 0.0, TAU);
        chart
    }

    fn add_children(&mut self, parent: &Entry, depth: usize, start: f32, end: f32) {
        if depth > MAX_OUTER_RINGS {
            return;
        }

        let largest_sibling = parent
            .children
            .iter()
            .map(|child| child.size)
            .max()
            .unwrap_or_default();
        if largest_sibling == 0 {
            return;
        }
        let total: u64 = parent.children.iter().map(|child| child.size).sum();

        let mut angle = start;
        for child in &parent.children {
            let sweep = (end - start) * (child.size as f64 / total as f64) as f32;
            let child_end = angle + sweep;
            if sweep > 0.0001 {
                self.segments.push(RingSegment {
                    path: child.path.clone(),
                    name: child.name.clone(),
                    size: child.size,
                    depth,
                    start: angle,
                    end: child_end,
                    color: color_for_ratio(child.size, largest_sibling),
                });
                if child.is_dir {
                    self.add_children(child, depth + 1, angle, child_end);
                }
            }
            angle = child_end;
        }
    }

    fn ring_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.depth)
            .max()
            .unwrap_or_default()
    }

    fn ring_dimensions(&self, max_radius: f32) -> (f32, f32) {
        let ring_count = self.ring_count();
        if ring_count == 0 {
            return (max_radius, 0.0);
        }

        let band = max_radius / (ring_count + 1) as f32;
        (band, band)
    }
}

/// The chart's per-widget state: what the mouse is over, and the drawn
/// rings, kept until another folder is shown (moving the mouse no longer
/// redraws every segment).
#[derive(Default)]
struct ChartState {
    hovered: Option<(PathBuf, String, String)>,
    cache: canvas::Cache,
    /// Which chart `cache` holds: its center folder and segment count.
    drawn: std::cell::RefCell<Option<(PathBuf, usize)>>,
}

impl Program<Message, cosmic::Theme> for RingChart {
    type State = ChartState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if matches!(
            event,
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. })
        ) {
            let hovered = cursor
                .position_in(bounds)
                .and_then(|position| self.hovered_item(position, bounds));
            if state.hovered != hovered {
                state.hovered = hovered.clone();
                return Some(canvas::Action::publish(Message::FolderSizeHover(
                    self.target,
                    hovered,
                )));
            }
            // Nothing in the chart depends on the mouse: no redraw needed.
            return None;
        }
        let canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event else {
            return None;
        };
        let position = cursor.position_in(bounds)?;
        if let Some(path) = self.parent_at(position, bounds) {
            return Some(canvas::Action::publish(Message::FolderSizeSelect(
                self.target,
                path,
            )));
        }
        self.segment_at(position, bounds).map(|segment| {
            canvas::Action::publish(Message::FolderSizeSelect(self.target, segment.path.clone()))
        })
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &cosmic::Renderer,
        _theme: &cosmic::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let key = (self.center_path.clone(), self.segments.len());
        if state.drawn.borrow().as_ref() != Some(&key) {
            state.cache.clear();
            *state.drawn.borrow_mut() = Some(key);
        }
        // The cache also redraws by itself when the size changes.
        vec![state.cache.draw(renderer, bounds.size(), |frame| {
            self.draw_rings(frame, bounds)
        })]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        mouse::Interaction::Pointer
    }
}

impl RingChart {
    fn draw_rings(&self, frame: &mut canvas::Frame, bounds: Rectangle) {
        let center = frame.center();
        let max_radius = bounds.width.min(bounds.height) / 2.0 - 8.0;
        if max_radius <= 0.0 {
            return;
        }
        let (center_radius, band) = self.ring_dimensions(max_radius);

        frame.fill(
            &canvas::Path::circle(center, center_radius),
            Color::from_rgb(0.18, 0.48, 0.92),
        );
        frame.fill_text(canvas::Text {
            content: self.center_size.clone(),
            position: center,
            max_width: center_radius * 1.8,
            color: Color::WHITE,
            size: 12.0.into(),
            line_height: Default::default(),
            font: cosmic::iced::Font::default(),
            align_x: cosmic::iced::core::text::Alignment::Center,
            align_y: cosmic::iced::alignment::Vertical::Center,
            shaping: cosmic::iced::core::text::Shaping::default(),
        });
        for segment in &self.segments {
            let inner = center_radius + band * (segment.depth - 1) as f32;
            let outer = inner + band;
            let path = annular_sector(center, inner, outer, segment.start, segment.end);
            frame.fill(&path, segment.color);
            frame.stroke(
                &path,
                canvas::Stroke::default()
                    .with_color(Color::WHITE)
                    .with_width(1.0),
            );
        }
    }
}

impl RingChart {
    fn parent_at(&self, position: Point, bounds: Rectangle) -> Option<PathBuf> {
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let distance = (position.x - center.x).hypot(position.y - center.y);
        let max_radius = bounds.width.min(bounds.height) / 2.0 - 8.0;
        let (center_radius, _) = self.ring_dimensions(max_radius);
        (distance <= center_radius)
            .then(|| self.parent_path.clone())
            .flatten()
    }

    fn segment_at(&self, position: Point, bounds: Rectangle) -> Option<&RingSegment> {
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let delta_x = position.x - center.x;
        let delta_y = position.y - center.y;
        let distance = delta_x.hypot(delta_y);
        let max_radius = bounds.width.min(bounds.height) / 2.0 - 8.0;
        let ring_count = self.ring_count();
        let (center_radius, band) = self.ring_dimensions(max_radius);
        if distance <= center_radius
            || distance > max_radius
            || max_radius <= 0.0
            || ring_count == 0
        {
            return None;
        }

        let angle = delta_y.atan2(delta_x).rem_euclid(TAU);
        let depth = (((distance - center_radius) / band).floor() as usize + 1).min(ring_count);
        self.segments.iter().find(|segment| {
            segment.depth == depth && angle >= segment.start && angle <= segment.end
        })
    }

    fn hovered_item(
        &self,
        position: Point,
        bounds: Rectangle,
    ) -> Option<(PathBuf, String, String)> {
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let distance = (position.x - center.x).hypot(position.y - center.y);
        let max_radius = bounds.width.min(bounds.height) / 2.0 - 8.0;
        let (center_radius, _) = self.ring_dimensions(max_radius);
        if max_radius > 0.0 && distance <= center_radius {
            return Some((
                self.center_path.clone(),
                self.center_name.clone(),
                self.center_size.clone(),
            ));
        }
        self.segment_at(position, bounds).map(|segment| {
            (
                segment.path.clone(),
                segment.name.clone(),
                format_size(segment.size),
            )
        })
    }
}

fn short_name(name: &str) -> String {
    let mut chars = name.chars();
    let shortened: String = chars.by_ref().take(30).collect();
    if chars.next().is_some() {
        format!("{shortened}...")
    } else {
        shortened
    }
}

fn annular_sector(center: Point, inner: f32, outer: f32, start: f32, end: f32) -> canvas::Path {
    let steps = (((end - start) * outer / 5.0).ceil() as usize).max(2);
    canvas::Path::new(|path| {
        for step in 0..=steps {
            let angle = start + (end - start) * step as f32 / steps as f32;
            let point = polar_point(center, outer, angle);
            if step == 0 {
                path.move_to(point);
            } else {
                path.line_to(point);
            }
        }
        for step in (0..=steps).rev() {
            let angle = start + (end - start) * step as f32 / steps as f32;
            path.line_to(polar_point(center, inner, angle));
        }
        path.close();
    })
}

fn polar_point(center: Point, radius: f32, angle: f32) -> Point {
    Point::new(
        center.x + radius * angle.cos(),
        center.y + radius * angle.sin(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, size: u64) -> Entry {
        Entry {
            name: Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            path: PathBuf::from(path),
            size,
            is_dir: false,
            children: Vec::new(),
        }
    }

    #[test]
    fn finds_entries_and_parents_by_going_down_the_tree() {
        let mut inner = file("/r/a/b", 0);
        inner.is_dir = true;
        inner.children = vec![file("/r/a/b/c.txt", 5)];
        let mut a = file("/r/a", 0);
        a.is_dir = true;
        a.children = vec![file("/r/a/x.txt", 1), inner];
        let mut root = file("/r", 0);
        root.is_dir = true;
        root.children = vec![file("/r/ab.txt", 2), a];

        let found = find_entry(&root, Path::new("/r/a/b/c.txt")).unwrap();
        assert_eq!(found.size, 5);
        // "/r/ab.txt" must not be taken for a folder containing "/r/a/...".
        assert!(find_entry(&root, Path::new("/r/a/x.txt")).is_some());
        assert_eq!(
            find_parent_entry(&root, Path::new("/r/a/b")).unwrap().path,
            PathBuf::from("/r/a")
        );
        assert!(find_parent_entry(&root, Path::new("/r")).is_none());
        assert!(find_entry(&root, Path::new("/r/missing")).is_none());
    }

    #[test]
    fn big_folders_show_their_largest_entries_and_one_more_row() {
        let mut root = file("/r", 0);
        root.is_dir = true;
        root.children = (0..MAX_ROWS_PER_FOLDER + 20)
            .map(|index| file(&format!("/r/f{index}"), 1))
            .collect();
        root.size = root.children.len() as u64;
        let expanded = HashSet::from([root.path.clone()]);
        let context = SizeContext {
            parent_size: root.size,
            root_size: root.size,
            sibling_max_size: root.size,
        };
        let mut visible = Vec::new();
        collect_visible(&root, 0, context, &expanded, &root.path, &mut visible);

        // The folder itself, its 500 largest entries, and "… 20 more".
        assert_eq!(visible.len(), 1 + MAX_ROWS_PER_FOLDER + 1);
        let more = visible.last().unwrap();
        assert_eq!(more.more, Some(20));
        assert_eq!(more.size, 20);
    }

    #[test]
    fn clicking_the_focused_folder_moves_up_one_level() {
        let root_path = PathBuf::from("/analysis");
        let child_path = root_path.join("subfolder");
        let tree = Entry {
            name: "analysis".to_string(),
            path: root_path.clone(),
            size: 10,
            is_dir: true,
            children: vec![Entry {
                name: "subfolder".to_string(),
                path: child_path.clone(),
                size: 10,
                is_dir: true,
                children: vec![Entry {
                    name: "file.txt".to_string(),
                    path: child_path.join("file.txt"),
                    size: 10,
                    is_dir: false,
                    children: Vec::new(),
                }],
            }],
        };
        let mut panel = Panel::loading(
            PaneId::Right,
            root_path.clone(),
            ListingOptions {
                hide_hidden: false,
                icon_style: fs_ops::settings::IconStyle::Colorful,
                user_dirs: std::sync::Arc::new(HashMap::new()),
                show_thumbnails: false,
                row_lines: false,
                font_size: fs_ops::settings::FontSize::Medium,
                separate_extension: true,
            },
        );
        panel.set_result(
            0,
            Ok(Scan {
                tree,
                unreadable: 0,
                drive: None,
                is_drive_root: false,
            }),
        );

        panel.select(child_path.clone());
        assert_eq!(panel.focused, child_path);

        panel.select(panel.focused.clone());
        assert_eq!(panel.focused, root_path);
    }

    #[test]
    fn ring_chart_stops_after_five_outer_rings() {
        let mut entry = Entry {
            name: "level-0".to_string(),
            path: PathBuf::from("/analysis"),
            size: 1,
            is_dir: true,
            children: Vec::new(),
        };
        let mut current = &mut entry;
        for level in 1..=7 {
            current.children.push(Entry {
                name: format!("level-{level}"),
                path: PathBuf::from(format!("/analysis/{}", "sub/".repeat(level))),
                size: 1,
                is_dir: true,
                children: Vec::new(),
            });
            current = current.children.last_mut().unwrap();
        }

        let chart = RingChart::new(&entry, None, PaneId::Right);
        assert_eq!(
            chart.segments.iter().map(|segment| segment.depth).max(),
            Some(5)
        );
        assert_eq!(chart.ring_dimensions(120.0), (20.0, 20.0));
    }

    #[test]
    fn ring_chart_scales_to_one_available_level() {
        let root = Entry {
            name: "analysis".to_string(),
            path: PathBuf::from("/analysis"),
            size: 10,
            is_dir: true,
            children: vec![Entry {
                name: "file.txt".to_string(),
                path: PathBuf::from("/analysis/file.txt"),
                size: 10,
                is_dir: false,
                children: Vec::new(),
            }],
        };
        let chart = RingChart::new(&root, None, PaneId::Right);

        assert_eq!(chart.ring_count(), 1);
        assert_eq!(chart.ring_dimensions(120.0), (60.0, 60.0));
    }

    #[test]
    fn colors_are_relative_to_the_largest_sibling() {
        let largest = color_for_ratio(4, 4);
        let three_quarters = color_for_ratio(3, 4);
        let quarter = color_for_ratio(1, 4);

        assert!(largest.r > largest.g);
        assert!(three_quarters.g > largest.g);
        assert!(quarter.g > three_quarters.g);
    }

    #[test]
    fn ring_tooltip_reports_only_the_item_under_the_pointer() {
        let root_path = PathBuf::from("/analysis");
        let folder_path = root_path.join("folder");
        let nested_path = folder_path.join("nested");
        let tree = Entry {
            name: "analysis".to_string(),
            path: root_path.clone(),
            size: 10,
            is_dir: true,
            children: vec![Entry {
                name: "folder".to_string(),
                path: folder_path.clone(),
                size: 10,
                is_dir: true,
                children: vec![Entry {
                    name: "nested".to_string(),
                    path: nested_path.clone(),
                    size: 10,
                    is_dir: true,
                    children: Vec::new(),
                }],
            }],
        };
        let chart = RingChart::new(&tree, None, PaneId::Right);
        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0,
        };
        let center = Point::new(100.0, 100.0);
        let max_radius = 92.0;
        let (center_radius, band) = chart.ring_dimensions(max_radius);
        let angle = 1.0;
        let first_ring = polar_point(center, center_radius + band * 0.5, angle);
        let second_ring = polar_point(center, center_radius + band * 1.5, angle);

        let first_hover = chart.hovered_item(first_ring, bounds).unwrap();
        let second_hover = chart.hovered_item(second_ring, bounds).unwrap();
        assert_eq!(first_hover.0, folder_path);
        assert_eq!(first_hover.1, "folder");
        assert_eq!(second_hover.0, nested_path);
        assert_eq!(second_hover.1, "nested");
    }
}
