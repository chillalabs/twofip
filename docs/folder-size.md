---
layout: default
title: Folder Size
---

**Folder Size** shows where the space inside a folder goes: which subfolders
and files are the largest, as a list and as a ring chart.

## Open it

1. In the active panel, select **one folder** (on your computer, not on a
   server), or select nothing to analyze the folder the panel shows.
2. Click the **Folder Size** button at the top of the window, next to
   Refresh and Find.

The analysis takes the place of the **other panel**, so the folder you're in
stays visible. While 2fip scans, it shows how many files and folders it has
counted so far; **Cancel** stops a long scan. Click the button again, or
**Close**, to bring the panel back.

## A whole drive

To see what fills a drive, go to its top folder (`/` on Linux, `C:\` on
Windows: use the drive menu at the left of the panel), select nothing and
click **Folder Size**.

- A bar at the top shows the drive's **used, total and free** space.
- 2fip stays on that drive: it doesn't go into other disks mounted inside
  it, network shares, or system folders like `/proc`.
- **Not counted** is the space the drive uses that 2fip couldn't see:
  system files and folders you aren't allowed to read, and the file system's
  own data. The number of folders it couldn't read is shown too.
- In the Flatpak version, the computer's system folders that the sandbox
  can't see (such as `/var`) are part of "Not counted".

## The list

The top half lists the folder's contents, **largest first**:

| Column | Shows |
|---|---|
| **Name** | the file or folder; ▸ expands a folder in place |
| **Usage** | a bar with its share of the whole analyzed folder |
| **Size** | its total size, everything inside included |
| **% of parent** | its share of the folder that contains it |
| **% of folder** | its share of the folder you analyze |

The colors go from red (the largest entry among its neighbors) to green (the
smallest), so the big ones stand out.

- **Click a folder** to analyze it: the list and the chart move into it. Click
  the same folder again to go back up.
- A very full folder shows its **500 largest** entries and one
  **"… N more"** row with the size of the rest.

## The ring chart

The bottom half draws the same folder as rings: the center is the folder,
the first ring its contents, the next rings what is inside those, up to five
levels deep. The wider a piece, the more space it takes.

- **Point** at a piece to see its name and size.
- **Click** a piece to analyze that folder; **click the center** to go back
  up.

## Clean up

**Right-click** an entry, in the list or in the chart:

- **Move to Trash** asks first, like <kbd>Delete</kbd> in the panels.
- **Open Path in New Tab** opens it in a new tab of the panel the analysis
  covers, so you can look inside before you decide.

After a copy, move or delete, the analysis updates by itself.

## Good to know

- Sizes are the **space on the disk**, like `du`: a file that shares its
  data with others (hard links, common in Flatpak's storage) is counted
  once, and it shows 0 the other times.
- Symbolic links count as small files; 2fip doesn't follow them.
- Folders it isn't allowed to read are left out (and counted).
- Text and rows follow **Settings → File name size**.
- For a quick total without the analysis, select folders and press
  <kbd>Space</kbd>: their size appears in the Size column (see
  [Working with files](files.html)).
