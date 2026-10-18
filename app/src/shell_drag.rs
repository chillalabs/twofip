//! Windows: dragging files out of 2fip with Windows' own drag and drop, the
//! way Explorer does it, so every program that takes files from Explorer
//! takes them from 2fip too: Explorer, browsers, WhatsApp, Notepad++, …
//! A drop back onto 2fip arrives like a drop from Explorer.

use std::path::PathBuf;

use windows::Win32::System::Com::{CoInitializeEx, IDataObject, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Ole::{
    OleInitialize, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE,
};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{BHID_DataObject, SHCreateShellItemArrayFromIDLists, SHDoDragDrop};
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

use crate::shell_menu::Pidls;

/// Drags `paths` (local files) until the mouse button is released.
/// Windows runs the drag, so this returns only once it's over.
pub fn drag(paths: &[PathBuf]) {
    // SAFETY: plain shell/COM calls on the UI thread; the item ids are
    // freed by `Pidls`, the COM objects by their wrappers.
    if let Err(err) = unsafe { drag_files(paths) } {
        eprintln!("drag and drop: {err}");
    }
}

unsafe fn drag_files(paths: &[PathBuf]) -> windows::core::Result<()> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        // The window already did this for drops; dragging needs it too.
        let _ = OleInitialize(None);
        let pidls = Pidls::parse(paths)?;
        let items: Vec<*const ITEMIDLIST> = pidls
            .0
            .iter()
            .map(|pidl| *pidl as *const ITEMIDLIST)
            .collect();
        // The same data Explorer offers for its files: the file list most
        // programs read, plus the shell's own formats.
        let data: IDataObject =
            SHCreateShellItemArrayFromIDLists(&items)?.BindToHandler(None, &BHID_DataObject)?;
        let effects = DROPEFFECT(DROPEFFECT_COPY.0 | DROPEFFECT_MOVE.0 | DROPEFFECT_LINK.0);
        // A move into Explorer is done by Explorer itself.
        SHDoDragDrop(Some(GetForegroundWindow()), &data, None, effects)?;
        Ok(())
    }
}
