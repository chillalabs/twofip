---
layout: default
title: Viewer and Quick View
---

Press <kbd>F3</kbd> to look at a file **inside 2fip**, without starting another
app, like Total Commander's Lister. <kbd>Esc</kbd> goes back to the panels.

## What F3 shows

- **Text files** (code, logs, settings, CSV…) as text. Small files can be
  selected and copied (<kbd>Ctrl</kbd>+<kbd>C</kbd>).
- **Images** (PNG, JPEG, GIF, WebP, BMP, SVG), fitted to the window.
- **Big files** open at once, even several GB: 2fip only reads the part on
  the screen.
- **PDFs, videos, documents** and other files with an app of their own open
  in **that app**, as before. Files that no app opens are shown as **hex**.

<kbd>Alt</kbd>+<kbd>F3</kbd> always uses the built-in viewer, also for those
files (as hex).

Files on a server or inside an archive work too: 2fip downloads or extracts
them first.

## Keys in the viewer

| Key | Action |
|---|---|
| <kbd>Esc</kbd> | Close the viewer |
| <kbd>N</kbd> / <kbd>P</kbd> | Next / previous file of the panel |
| <kbd>1</kbd> / <kbd>2</kbd> / <kbd>3</kbd> | Text / hex / image |
| <kbd>W</kbd> | Wrap long lines on / off |
| <kbd>F3</kbd> / <kbd>Shift</kbd>+<kbd>F3</kbd> | Find next / previous (type the text in the Find box first) |
| <kbd>↑</kbd> <kbd>↓</kbd> <kbd>Page Up</kbd> <kbd>Page Down</kbd> <kbd>Home</kbd> <kbd>End</kbd> | Scroll (big files and hex) |
| <kbd>+</kbd> / <kbd>-</kbd> / <kbd>0</kbd> | Zoom in / out / fit (images) |

The toolbar has the same actions, plus **Open with default app**.

Search ignores upper and lower case. In big files it runs in the background
and jumps to the match, highlighted.

## Quick View

<kbd>Ctrl</kbd>+<kbd>Q</kbd> (or **View → Quick View**) turns the **other
panel** into a viewer for the file under the cursor: move through the files
and the preview follows. Press <kbd>Ctrl</kbd>+<kbd>Q</kbd> again to get the
panel back.

> In 2fip 0.2.11 and earlier, <kbd>Ctrl</kbd>+<kbd>Q</kbd> closed 2fip. Now it
> is Quick View, as in Total Commander; quit with <kbd>Alt</kbd>+<kbd>F4</kbd>
> or **File → Quit**.

## Editing (F4)

- <kbd>F4</kbd> opens the file in your **text editor**. Choose which one in
  **Settings → Programs → Editor (F4)**; by default it's the system's.
- <kbd>Shift</kbd>+<kbd>F4</kbd> (or **File → New File**) asks for a name,
  creates an empty file in the panel's folder and opens it in the editor.
- **Files on a server:** 2fip downloads a copy and opens it. **Each time you
  save** in the editor, 2fip uploads it back to the server and says so at the
  bottom of the window. This lasts while the connection is open.
- Files **inside an archive** open as a copy: changes stay in the copy, not in
  the archive.
