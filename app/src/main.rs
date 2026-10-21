// Windows release builds have no console window (debug builds keep it for
// the log messages).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod about;
mod app;
mod connect;
mod connections_panel;
mod context_menu;
mod dnd;
mod drag_start;
mod fallback_icons;
mod favorites_menu;
mod file_item;
mod find;
mod folder_size;
mod folder_tree;
mod icon_sets;
mod keybinds;
mod launch;
mod localize;
mod marquee;
mod menu_bar;
mod operation;
mod pane;
#[cfg(windows)]
mod shell_drag;
#[cfg(windows)]
mod shell_menu;
mod tab;
mod themes;
mod viewer;
mod watcher;
#[cfg(windows)]
mod windows;

fn main() -> cosmic::iced::Result {
    // Before any icon is looked up.
    fallback_icons::install();
    cosmic::app::run::<app::App>(cosmic::app::Settings::default(), ())
}
