---
layout: default
title: Working with files
---

## Selecting

| To select | Mouse | Keyboard |
|---|---|---|
| One item | Click | <kbd>↑</kbd> / <kbd>↓</kbd>, <kbd>Home</kbd> / <kbd>End</kbd> |
| Add or remove one | <kbd>Ctrl</kbd>+click | — |
| A range | <kbd>Shift</kbd>+click | <kbd>Shift</kbd>+<kbd>↑</kbd> / <kbd>↓</kbd> / <kbd>Home</kbd> / <kbd>End</kbd> |
| Everything | — | <kbd>Ctrl</kbd>+<kbd>A</kbd> |
| Several at once | Drag a rectangle from **empty space** (<kbd>Ctrl</kbd>: add to the selection) | — |

A click on empty space clears the selection.

Right-click an item for its menu: Open, Open with…, View, Edit, Calculate
size, Cut, Copy, Paste, Rename…, Pack…, Extract… (archives), Delete and Show
details.

## Opening

- **Folders:** double-click, <kbd>Enter</kbd> or <kbd>→</kbd>. Go back up
  with <kbd>Backspace</kbd> or <kbd>←</kbd>; 2fip selects the folder you came
  from.
- **Files:** double-click or <kbd>Enter</kbd> opens a file with its default
  app. If its type has **no default app yet**, 2fip shows **Open with…**
  instead, with **Always use this app** ticked: choose the app once and that
  type opens with it from then on (in other apps too).
- **Programs:** an executable program, such as an **AppImage**, runs.
- **Archives** (zip, tar, tgz, txz, 7z): double-click or <kbd>Enter</kbd>
  opens them **like a folder** (see [Archives](#archives)).
- **<kbd>F3</kbd> View** opens the file's default app (images: the image
  viewer).
- **<kbd>F4</kbd> Edit** always opens the text editor (images: the image
  viewer), even for files whose default app is something else.
- **Open with…** (right-click) lists the apps for the file's type first, then
  all the others. Tick **Always use this app** to make the choice the default
  for that file type.

## Copying and moving

- **<kbd>F5</kbd> Copy** and **<kbd>F6</kbd> Move** send the selection to the
  **other panel's** folder.
- **Clipboard:** <kbd>Ctrl</kbd>+<kbd>C</kbd> (copy) or
  <kbd>Ctrl</kbd>+<kbd>X</kbd> (cut), then <kbd>Ctrl</kbd>+<kbd>V</kbd> in any
  folder.
- **Drag and drop** between the panels, into a folder, or from and to other
  apps. Dropping copies; hold <kbd>Shift</kbd> to move.

Operations run in the background, with a progress panel and **Cancel**. If a
file already exists, 2fip asks: **Replace**, **Skip**, the same for all,
**Rename** (keep both), or **Cancel** (stop the whole operation).

## Renaming and new folders

- **<kbd>F2</kbd>** (or <kbd>Shift</kbd>+<kbd>F6</kbd>, as in Total Commander)
  renames the selected item.
- **<kbd>F7</kbd>** creates a new folder.

## Deleting

- **<kbd>F8</kbd>** or **<kbd>Delete</kbd>** moves the selection **to the
  trash**, after asking.
- **<kbd>Shift</kbd>+<kbd>Delete</kbd>** or **<kbd>Shift</kbd>+<kbd>F8</kbd>**
  deletes **permanently**, without the trash. This can't be undone, so 2fip
  asks first, with a red warning.
- On a server, deleting is always permanent (servers have no trash).

In the question, <kbd>←</kbd> / <kbd>→</kbd> or <kbd>Tab</kbd> choose the
button and <kbd>Enter</kbd> confirms; <kbd>Esc</kbd> cancels.

## Archives

### Pack files

Select files and folders, then:

- **Right-click → Pack…** creates the archive **in the same folder**;
- the **Pack files** button or <kbd>Alt</kbd>+<kbd>F5</kbd> creates it **in
  the other panel's folder**.

In the window you choose:

- **Archive (folder and name):** where the archive goes; you can type another
  folder or name.
- **Format:** zip, tar, gz, tgz, xz, txz or 7z. gz and xz pack a **single
  file**; use tgz or txz for folders or several files.
- **Password:** for zip and 7z, encrypts the archive (AES-256).
- **Move to archive:** deletes the originals once the archive is complete.

### Extract files

Select one or more archives, then:

- **Right-click → Extract…** unpacks them **in the same folder** as the
  archive;
- the **Extract files** button or <kbd>Alt</kbd>+<kbd>F9</kbd> unpacks them
  **into the other panel's folder**.

Options: **Unpack path names if stored with files** (on: recreates the
archive's folders; off: everything goes straight into one folder),
**Overwrite existing files** (off: 2fip asks for each one), and a
**Password** for encrypted archives.

### Inside an archive

Double-click an archive (or press <kbd>Enter</kbd>) to open it **like a
folder**, with all its folders and files. From there:

- <kbd>F5</kbd> copies (and drag and drop copies) files and folders out, to
  the other panel;
- <kbd>F6</kbd> moves them out: they are removed from the archive;
- <kbd>F8</kbd> deletes them from the archive;
- <kbd>Enter</kbd>, <kbd>F3</kbd> and <kbd>F4</kbd> open a file (2fip
  extracts it to its cache first).

An archive opened this way is read-only otherwise: nothing can be pasted or
dropped into it, and renaming or creating folders there isn't possible.
Moving or deleting inside a big archive takes a while, because the whole
archive is written again. Encrypted 7z archives with hidden file names can
only be extracted (with their password).

## More

- **Folder sizes:** select folders and press <kbd>Space</kbd>; their total size
  appears in the Size column. To see what takes the space inside a folder, use
  [Folder Size](folder-size.html).
- **Details:** <kbd>Alt</kbd>+<kbd>Enter</kbd> shows type, size, dates,
  permissions and owner.
- **Compress…** (right-click) packs the selection into a ZIP archive.
- **Terminal:** <kbd>F9</kbd> opens your terminal in the current folder. 2fip
  uses the terminal chosen in COSMIC Settings, then `$TERMINAL`, then the
  usual terminals of other desktops.
- **Hidden files:** <kbd>Ctrl</kbd>+<kbd>H</kbd> shows or hides files whose
  name starts with a dot.
- **Refresh:** <kbd>Ctrl</kbd>+<kbd>R</kbd> re-reads both panels.
