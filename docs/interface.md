---
layout: default
title: The main window
---

The window has a **menu bar** at the top, **two panels** side by side, and the
**function key bar** at the bottom.

## Menus and header buttons

| Menu | Contains |
|---|---|
| **File** | New tab, New folder, Connect to server…, Connections, Disconnect, Close tab, Quit |
| **Edit** | Cut, Copy, Paste, Select all, Rename…, Delete, Delete permanently |
| **View** | List view, Grid view, Favorites, Settings |
| **Help** | Help (<kbd>F1</kbd>), About 2fip |

Next to the menus are the tool buttons, in groups:

| Button | Does |
|---|---|
| **Refresh** (<kbd>Ctrl</kbd>+<kbd>R</kbd>) | re-reads the folders of both panels |
| **List view** (<kbd>Ctrl</kbd>+<kbd>1</kbd>) / **Grid view** (<kbd>Ctrl</kbd>+<kbd>2</kbd>) | the active panel's view; the current one is greyed out |
| **Folder tree** | shows the folder tree beside the left panel, the right, both, or none, in turn |
| **Back** (<kbd>Alt</kbd>+<kbd>←</kbd>) / **Forward** (<kbd>Alt</kbd>+<kbd>→</kbd>) | through the active panel's history |
| **Up** (<kbd>Backspace</kbd>) | the active panel's parent folder |
| **Find files** (<kbd>Ctrl</kbd>+<kbd>F</kbd>) | searches by name (see [Finding files](find.html)) |
| **Folder Size** | what fills a folder or a drive (see [Folder Size](folder-size.html)) |
| **Connections** | saved and open server connections |
| **Pack files** (<kbd>Alt</kbd>+<kbd>F5</kbd>) | packs the selection into an archive in the other panel's folder |
| **Extract files** (<kbd>Alt</kbd>+<kbd>F9</kbd>) | unpacks the selected archives into the other panel's folder |

A button that can't act right now is greyed out: for example **Extract files**
only works when an archive is selected, and **Back** when there's no history. See
[Working with files](files.html#archives) for packing and extracting.

## The two panels

One panel is **active**: its frame is drawn in the accent color, and the
keyboard and most commands act on it. Click a panel, or press <kbd>Tab</kbd>,
to make the other one active. Copy (<kbd>F5</kbd>) and Move (<kbd>F6</kbd>)
send the active panel's selection **to the other panel's folder**.

Each panel has, from top to bottom:

1. **Tabs.** See [Tabs](#tabs) below.
2. **Navigation**, on a shaded band: **★** for the
   [favorites](favorites.html), the **drive menu** (see below), **+** for a
   new tab, and the **path bar**. (Back, Forward and Up are in the toolbar at the top and
   act on the active panel.)
3. **The file list**, in list or grid view.
4. **The status line:** how many items the folder has, or how many are
   selected and their total size.

### The drive menu

At the left of each panel, the drive menu lists **System** (`/`), **Home**,
and your other disks and **USB sticks**, each with its icon, name and size.
Pick one to open it in that panel. A drive that isn't mounted yet ("not
mounted") is mounted first, without a password for USB sticks. A stick you
plug in shows up within a few seconds.

While a panel shows a USB stick, an **eject** button (⏏) appears next to the
menu: it unmounts the stick and powers it off, so you can unplug it safely.
On Windows the menu lists the drive letters.

### The folder tree

The **Folder tree** button shows the folders as a tree on the left side of a
panel; each click moves on: left panel, right panel, both, none. Its icon
shows which panels have the tree. Click a folder to open it in the panel;
click its arrow (or double-click it) to open or close it in the tree. The
tree follows the panel: it opens down to the current folder and highlights
it.

### The path bar

The path bar shows the folder as buttons, one per level: click one to go
there. For a server it starts with the server's name, for example
`me@example.com › home › me`.

To type a path, click the pencil, click the empty space of the bar, or press
<kbd>Ctrl</kbd>+<kbd>L</kbd>. You can type an absolute path (`/etc`), one
relative to the current folder (`docs/../src`), your home (`~` or
`~/Downloads`), or a server address (`sftp://me@example.com/home/me`). Typing
a file's path opens its folder with the file selected. <kbd>Esc</kbd> cancels.

### Tabs

- **New tab:** <kbd>Ctrl</kbd>+<kbd>T</kbd>, the **+** button, or a
  **double click on the empty space** beside the tabs. It opens on the current
  folder.
- **Switch tabs:** click one, or <kbd>Ctrl</kbd>+<kbd>Tab</kbd> /
  <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Tab</kbd>.
- **Close a tab:** its **×**, a middle click, or <kbd>Ctrl</kbd>+<kbd>W</kbd>.
  The last tab of a panel stays open.
- **Copy a tab to the other panel:** drag it onto the other panel. The copy
  opens with the same folder and view; the original stays.

2fip remembers the open tabs, their folders and the active panel, and
restores them the next time it starts. (Tabs on a server come back at your
home folder: connections are opened again by hand.)

## List view and grid view

- **List view** (<kbd>Ctrl</kbd>+<kbd>1</kbd>): columns **Name**, **Ext**
  (optional, see [Settings](settings.html)), **Size** and **Modified**. Click a
  column title to sort by it; click again to reverse the order. Folders are
  always listed first.
- **Grid view** (<kbd>Ctrl</kbd>+<kbd>2</kbd>): large icons or thumbnails.

In list view, drag the thin line between two column titles to make them wider
or narrower; each tab keeps its widths. **Settings → Lines between rows**
draws a thin line between the rows, like COSMIC Files.

Each tab keeps its own view.

## The function key bar

The buttons at the bottom do the same as the keys:
<kbd>F2</kbd> Rename, <kbd>F3</kbd> View, <kbd>F4</kbd> Edit, <kbd>F5</kbd> Copy,
<kbd>F6</kbd> Move, <kbd>F7</kbd> New folder, <kbd>F8</kbd> Delete,
<kbd>F9</kbd> Terminal, <kbd>Alt</kbd>+<kbd>F4</kbd> Exit.

## Side panels

**Settings** (<kbd>Ctrl</kbd>+<kbd>,</kbd>), **Connections** and **About**
open on the right. ([Favorites](favorites.html), <kbd>Ctrl</kbd>+<kbd>D</kbd>,
are a small menu over the active panel instead.) While one is open it has the keyboard; <kbd>Esc</kbd> or its **×**
closes it.
