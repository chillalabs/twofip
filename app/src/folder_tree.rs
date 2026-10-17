//! The folder tree beside a panel's file list ("Show hierarchy"): the
//! folders from the top of the disk down, opened on demand. It follows the
//! panel's folder (its parents open, it is highlighted) and a click on a
//! folder opens it in the panel.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

use crate::file_item::ListingOptions;
use crate::pane::PaneMessage;

/// Width of the tree beside the file list.
pub const TREE_WIDTH: f32 = 230.0;
/// Indentation per level, in pixels.
const INDENT: f32 = 14.0;

#[derive(Debug, Default)]
pub struct FolderTree {
    /// Folders shown open.
    expanded: HashSet<PathBuf>,
    /// The subfolders of each folder read so far, sorted by name.
    children: HashMap<PathBuf, Vec<PathBuf>>,
    /// Folders being read.
    loading: HashSet<PathBuf>,
}

impl FolderTree {
    /// Opens the parents of `dir` (so it can be seen) and returns the
    /// folders whose subfolders must be read for that.
    pub fn reveal(&mut self, dir: &Path) -> Vec<PathBuf> {
        let mut to_load = Vec::new();
        let ancestors: Vec<PathBuf> = dir.ancestors().skip(1).map(Path::to_path_buf).collect();
        for ancestor in ancestors.into_iter().rev() {
            self.expanded.insert(ancestor.clone());
            if self.needs_loading(&ancestor) {
                self.loading.insert(ancestor.clone());
                to_load.push(ancestor);
            }
        }
        to_load
    }

    /// Opens or closes `dir`; returns it if its subfolders must be read.
    pub fn toggle(&mut self, dir: PathBuf) -> Option<PathBuf> {
        if self.expanded.remove(&dir) {
            return None;
        }
        self.expanded.insert(dir.clone());
        self.needs_loading(&dir).then(|| {
            self.loading.insert(dir.clone());
            dir
        })
    }

    /// The subfolders of `dir`, as read.
    pub fn set_children(&mut self, dir: PathBuf, mut children: Vec<PathBuf>) {
        children.sort_by_cached_key(|child| {
            child
                .file_name()
                .map(|name| name.to_string_lossy().to_lowercase())
                .unwrap_or_default()
        });
        self.loading.remove(&dir);
        self.children.insert(dir, children);
    }

    /// Forgets what was read (e.g. hidden folders were switched on), keeping
    /// which folders are open; returns the open ones to read again.
    pub fn reload(&mut self) -> Vec<PathBuf> {
        self.children.clear();
        self.loading = self.expanded.clone();
        self.expanded.iter().cloned().collect()
    }

    fn needs_loading(&self, dir: &Path) -> bool {
        !self.children.contains_key(dir) && !self.loading.contains(dir)
    }

    /// The tree under `roots` (`/`, or the drive letters), with `current`
    /// highlighted.
    pub fn view(
        &self,
        roots: &[PathBuf],
        current: &Path,
        options: &ListingOptions,
    ) -> Element<'static, PaneMessage> {
        let mut rows = Vec::new();
        for root in roots {
            self.push_rows(&mut rows, root, 0, current, options);
        }
        let rows = widget::Column::with_children(rows)
            .spacing(1)
            .width(Length::Fill);
        widget::container(widget::scrollable(rows).height(Length::Fill))
            .width(Length::Fixed(TREE_WIDTH))
            .height(Length::Fill)
            .into()
    }

    /// Adds the row of `dir` and, if it is open, those of its subfolders.
    fn push_rows(
        &self,
        rows: &mut Vec<Element<'static, PaneMessage>>,
        dir: &Path,
        depth: usize,
        current: &Path,
        options: &ListingOptions,
    ) {
        let open = self.expanded.contains(dir);
        let children = self.children.get(dir);
        // Unknown until read: show the arrow, it may have subfolders.
        let has_children = children.is_none_or(|children| !children.is_empty());
        let icon_size = crate::pane::list_icon_size(options.font_size);
        let arrow: Element<'static, PaneMessage> = if has_children {
            widget::button::icon(widget::icon::from_name(if open {
                "pan-down-symbolic"
            } else {
                "pan-end-symbolic"
            }))
            .icon_size(icon_size.saturating_sub(4).max(10))
            .padding(0)
            .on_press(PaneMessage::TreeToggle(dir.to_path_buf()))
            .into()
        } else {
            widget::Space::new()
                .width(Length::Fixed(f32::from(
                    icon_size.saturating_sub(4).max(10),
                )))
                .into()
        };
        let name = dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| dir.display().to_string());
        let selected = dir == current;
        let row = widget::Row::new()
            .spacing(4)
            .align_y(Alignment::Center)
            .push(widget::Space::new().width(Length::Fixed(depth as f32 * INDENT)))
            .push(arrow)
            .push(
                widget::icon(crate::file_item::icon_for_path(dir, true, options))
                    .size(icon_size.saturating_sub(2)),
            )
            .push(crate::pane::tree_text(name, options.font_size, selected));
        let row = widget::container(row)
            .padding([1, 4])
            .width(Length::Fill)
            .class(crate::pane::selection_class(selected, false));
        rows.push(
            widget::mouse_area(row)
                .on_press(PaneMessage::Navigate(dir.to_path_buf()))
                .on_double_click(PaneMessage::TreeToggle(dir.to_path_buf()))
                .into(),
        );
        if open {
            for child in children.into_iter().flatten() {
                self.push_rows(rows, child, depth + 1, current, options);
            }
        }
    }
}

/// The subfolders of `dir` (not following links), without hidden ones when
/// `hide_hidden`. Blocking-free: reads through Tokio.
pub async fn read_subfolders(dir: PathBuf, hide_hidden: bool) -> Vec<PathBuf> {
    fs_ops::list_dir(&dir)
        .await
        .map(|entries| {
            entries
                .into_iter()
                .filter(|entry| entry.kind == fs_ops::EntryKind::Dir)
                .filter(|entry| !hide_hidden || !fs_ops::settings::is_hidden(&entry.name))
                .map(|entry| entry.path)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revealing_a_folder_opens_and_reads_its_parents_once() {
        let mut tree = FolderTree::default();
        let to_load = tree.reveal(Path::new("/home/me/docs"));
        assert_eq!(
            to_load,
            vec![
                PathBuf::from("/"),
                PathBuf::from("/home"),
                PathBuf::from("/home/me")
            ]
        );
        // Already being read: not asked for again.
        assert!(tree.reveal(Path::new("/home/me/docs")).is_empty());
        tree.set_children(PathBuf::from("/home"), vec![PathBuf::from("/home/me")]);
        // Closing and opening a read folder needs no reading.
        assert_eq!(tree.toggle(PathBuf::from("/home")), None);
        assert_eq!(tree.toggle(PathBuf::from("/home")), None);
        assert_eq!(
            tree.toggle(PathBuf::from("/home/me/docs")),
            Some(PathBuf::from("/home/me/docs"))
        );
    }
}
