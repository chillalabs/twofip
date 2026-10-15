---
layout: default
title: Folder Size
---

**Folder Size** shows where the space inside a folder goes: which subfolders
and files are the largest, as a list and as a ring chart.

## Open it

1. In the active panel, select **one folder** (on your computer, not on a
   server).
2. Click the **Folder Size** button at the top of the window, next to
   Refresh and Find.

The analysis takes the place of the **other panel**, so the folder you're in
stays visible. While 2fip scans the folder you see *Calculating folder
sizes…*; a big folder can take a few seconds. Click the button again, or
**Close**, to bring the panel back.

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

- Sizes are the files' own sizes (like the Size column), not the space
  they take on the disk, so the total can differ a little from `du`.
- Symbolic links count as small files; 2fip doesn't follow them.
- Folders it isn't allowed to read are left out.
- Text and rows follow **Settings → File name size**.
- For a quick total without the analysis, select folders and press
  <kbd>Space</kbd>: their size appears in the Size column (see
  [Working with files](files.html)).
