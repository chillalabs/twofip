# 2fip — English (base language; every key must exist here)

## Menu bar
menu-file = File
menu-edit = Edit
menu-view = View
menu-help = Help
help = Help
new-tab = New Tab
new-folder = New Folder
close-tab = Close Tab
quit = Quit
cut = Cut
copy = Copy
paste = Paste
select-all = Select All
rename-ellipsis = Rename…
delete = Delete
list-view = List View
grid-view = Grid View
favorites = Favorites
settings = Settings

## File context menu
open = Open
open-with-ellipsis = Open With…
view = View
edit = Edit
calculate-size = Calculate Size
compress-ellipsis = Pack…
show-details = Show Details

## List columns
column-name = Name
column-ext = Ext
column-size = Size
column-modified = Modified

## Running operations (progress panel)
op-copying = Copying
op-moving = Moving
op-deleting = Deleting
op-compressing = Compressing
op-copy-failed = Copy failed
op-move-failed = Move failed
op-delete-failed = Delete failed
op-compress-failed = Compress failed
op-progress = { $operation } { $percent }%
op-files-done = { $done } of { $total } files
op-bytes-done = { $done } of { $total }
op-preparing = Preparing…
op-queued = { $count } waiting
op-queue-remove = Remove
close = Close
cancel = Cancel

## Panels
path-not-found = Not found: { $path }
status-items = { $count ->
    [one] 1 item
   *[other] { $count } items
}
status-items-filtered = { $count ->
    [one] 1 item matches the filter
   *[other] { $count } items match the filter
}
status-selected = { $selected } of { $total } selected ({ $size })
tooltip-back = Back (Alt+←)
tooltip-forward = Forward (Alt+→)
tooltip-up = Up one folder (Backspace)
tooltip-drive = Drive
tooltip-tree-off = Folder tree: off (click to show it beside the left panel)
tooltip-tree-left = Folder tree: left panel (click for the right panel)
tooltip-tree-right = Folder tree: right panel (click for both panels)
tooltip-tree-both = Folder tree: both panels (click to hide it)
tooltip-eject = Eject (safe to unplug)
drive-system = System
drive-home = Home
drive-removable = USB
drive-not-mounted = not mounted
drive-mount-failed = Couldn't open the drive: { $error }
drive-eject-failed = Couldn't eject the drive: { $error }
tooltip-new-tab = New tab (Ctrl+T)
tooltip-refresh = Refresh both panels (Ctrl+R)
tooltip-edit-path = Edit path (Ctrl+L)
tooltip-cancel-esc = Cancel (Esc)
no-tab-open = No tab open
filter-placeholder = Filter (e.g. report or *.txt)
path-placeholder = Type a folder path

## Settings
settings-general = General
settings-show-hidden = Show hidden files
settings-show-hidden-description = Files and folders whose name starts with a dot
settings-separate-ext = Show extension in its own column
settings-separate-ext-description = List view: separate Name and Ext columns, like Total Commander
settings-thumbnails = Show thumbnails
settings-thumbnails-description = Previews of images, PDFs, videos and fonts
settings-language = Language
language-system = System default
settings-theme = Theme
settings-color-theme = Color theme
settings-color-theme-description = Only affects 2fip; System follows COSMIC's appearance
color-theme-system = System
color-theme-light = Light
color-theme-dark = Dark
settings-icon-style = Icon style
settings-icon-style-description = Colorful matches the COSMIC Files app
icon-style-colorful = Colorful
icon-style-monochrome = Monochrome
icon-style-vivid = Vivid
icon-style-classic = Windows style
icon-style-soft = macOS style
settings-font-size = File name size
settings-font-size-description = Smaller text fits more files on screen
settings-row-lines = Lines between rows
settings-row-lines-description = In the list view, a thin line between files and folders, like COSMIC Files
font-size-medium = Medium ({ $px } px)
font-size-small = Small ({ $px } px)
font-size-smaller = Smaller ({ $px } px)
font-size-very-small = Very small ({ $px } px)
font-size-tiny = Tiny ({ $px } px)
settings-corners = Rounded corners
settings-corners-description = How round the window, buttons and panels are
corners-square = Square
corners-small = Slightly rounded
corners-medium = Rounded
corners-large = Very rounded (COSMIC)

## Favorites
favorites-saved = Saved folders
favorites-empty = No favorites saved yet.
favorites-move-up = Move up (Ctrl+↑)
favorites-move-down = Move down (Ctrl+↓)
favorites-add-current = Add current folder
favorites-add = Add
favorites-duplicate-title = Favorite already saved
favorites-duplicate-path = This folder is already saved as "{ $name }".
favorites-duplicate-name = There is already a favorite named "{ $name }", pointing to { $path }.

## "File Already Exists" dialog
conflict-title = File Already Exists
conflict-body = "{ $existing }" already exists in the destination. Replace it with "{ $new }"?
skip-all = Skip All
replace-all = Replace All
replace = Replace
skip = Skip

## Compress dialog
compress = Compress
compress-one = Compress "{ $name }" into a zip archive.
compress-many = Compress { $count } items into a zip archive.
archive-name = Archive name
pack = Pack files
pack-one = Pack "{ $name }" into an archive.
pack-many = Pack { $count } items into an archive.
pack-path = Archive (folder and name)
pack-format = Format
pack-password = Password (optional, encrypts the archive)
pack-password-unavailable = Password: only for zip and 7z
pack-move = Move to archive (delete the originals afterwards)
pack-single-file-only = gz and xz pack a single file: choose tgz, txz or another format for folders or several files.
unpack = Extract files
unpack-one = Extract "{ $name }" to { $dest }.
unpack-many = Extract { $count } archives to { $dest }.
unpack-keep-paths = Unpack path names if stored with files
unpack-overwrite = Overwrite existing files
unpack-password = Password for encrypted files
tooltip-pack = Pack files (Alt+F5)
tooltip-unpack = Extract files to the other panel (Alt+F9)
extract-ellipsis = Extract…
op-unpacking = Extracting
op-unpack-failed = Extract failed
default-archive-name = Archive

## New folder dialog
new-folder-default-name = New folder
folder-name = Folder name
create = Create

## Rename dialog
rename = Rename
renaming = Renaming "{ $name }"
new-name = New name

## Delete dialog
delete-one = Move "{ $name }" to the trash?
delete-many = Move { $count } items to the trash?
delete-permanently = Delete permanently
delete-permanently-one = Permanently delete "{ $name }"? This can't be undone.
delete-permanently-many = Permanently delete { $count } items? This can't be undone.

## Function key bar (the "F2" etc. prefix is added by the app)
fkey-rename = Rename
fkey-view = View
fkey-edit = Edit
fkey-copy = Copy
fkey-move = Move
fkey-mkdir = MkDir
fkey-delete = Delete
fkey-terminal = Terminal
fkey-exit = Exit

## Open With dialog
open-with = Open With
open-with-remember = Always use this app for { $mime } files
open-with-one = Choose an application to open "{ $name }".
open-with-many = Choose an application to open { $count } items.
loading-apps = Loading applications…
no-apps = No applications found.
recommended-apps = Recommended Applications
other-apps = Other Applications
default-app = Default

## Details dialog
details-items-title = { $count } items
details-calculating = Calculating…
details-error = Error
details-contents = { $files } files, { $folders } folders inside
details-size = { $size } ({ $bytes } bytes)
details-items = Items
details-location = Location
details-total-size = Total size
details-type = Type
details-name = Name
details-link-target = Link target
details-size-label = Size
details-modified = Modified
details-accessed = Accessed
details-created = Created
details-permissions = Permissions
details-owner = Owner
details-group = Group
details-unknown = Unknown
kind-folder = Folder
kind-file = File
kind-symlink = Symbolic link

## Find Files dialog
find = Find files
tooltip-find = Find files (Ctrl+F)
find-in = In { $path } and its subfolders
find-placeholder = Name or pattern (e.g. report or *.pdf)
find-search = Search
find-stop = Stop
find-hint = Type part of a name and press Enter.
find-searching = Searching… { $count } found
find-results = { $count ->
    [one] 1 result
   *[other] { $count } results
}
find-no-results = Nothing found.
find-truncated = Showing the first { $count } results; try a more specific name.
find-go-to = Go to file
find-listbox = View in Listbox
search-results-tab = Search: { $pattern }

## About window
about-2fip = About 2fip
about-description = A keyboard-driven, dual-pane file manager, inspired by Total Commander.
about-version = Version:
about-repository = Repository:
about-license = License:

## Connect to server dialog
connect-to-server = Connect to server…
disconnect = Disconnect
connect-title = Connect to server
connect = Connect
connect-host = Server (e.g. files.example.com)
connect-port = Port
connect-user = User name
connect-password = Password
connect-hint-sftp = Your SSH agent and the keys in ~/.ssh are tried first, so the password is optional.
connect-hint-ftp = Leave the user name empty to log in anonymously. FTP sends the password unencrypted: prefer FTPS or SFTP.
connect-hint-ftps = FTP encrypted with TLS. The server needs a valid certificate.
connect-connecting = Connecting…
connect-error-host = Type the server's name or address.
connect-error-port = The port must be a number.
connect-error-login = Login failed: check the user name and password.
connect-unknown-host-title = Unknown server
connect-unknown-host-body = 2fip hasn't connected to { $host } before. Check that this key fingerprint matches the server's, then trust it to connect. It will be saved in ~/.ssh/known_hosts.
connect-trust = Trust and connect
connect-key-changed-title = The server's key has changed
connect-key-changed-body = The key of { $host } doesn't match the one saved in ~/.ssh/known_hosts. Someone may be intercepting the connection, so 2fip won't connect. If the server was reinstalled, remove its old line from ~/.ssh/known_hosts.

## Connections panel
connections = Connections
tooltip-connections = Connections
folder-size = Folder Size
move-to-trash = Move to Trash
open-path-in-new-tab = Open Path in New Tab
folder-size-loading = Calculating folder sizes…
folder-size-scanning = { $count } files and folders, { $size } so far
folder-size-drive = Drive: { $used } used of { $total } · { $free } free
folder-size-other = Not counted: { $size } (system files and folders 2fip can't read, file system data)
folder-size-unreadable = { $count } folders couldn't be read (no permission)
folder-size-empty = This folder is empty
folder-size-parent-share = % of parent
folder-size-root-share = % of folder
folder-size-graph = Usage
folder-size-more = … { $count } more
folder-size-selected = { $count } folders selected
connection-add = Add connection
connection-edit = Edit connection
connection-save-only = Save
connection-save = Save connection
connection-open = Open
connection-reconnect = Reconnect
connection-status-busy = Connecting…
connection-status-connected = Connected
connection-status-lost = Connection lost
connection-status-closed = Not connected
connections-saved = Saved
connections-open-unsaved = Open, not saved
connections-empty = No saved connections yet.
connections-hint = Passwords you choose to remember are kept in the system keyring (GNOME Keyring, KWallet), encrypted, never in 2fip's files.
connect-name = Name (optional)
connect-password-stored = Stored in the keyring (type to change)
connect-save = Save in Connections
connect-remember-password = Remember the password in the system keyring

# F3 viewer and Quick View (Ctrl+Q)
quick-view = Quick View
viewer-loading = Loading…
viewer-unreadable = This file can't be read.
viewer-folder = This is a folder.
viewer-previous = Previous file (P)
viewer-next = Next file (N)
viewer-text = Text
viewer-hex = Hex
viewer-image = Image
viewer-zoom-out = Zoom out (−)
viewer-zoom-fit = Fit to window (0)
viewer-zoom-in = Zoom in (+)
viewer-wrap = Wrap lines
viewer-find = Find
viewer-find-previous = Find previous (Shift+F3)
viewer-find-next = Find next (F3)
viewer-searching = Searching…
viewer-not-found = Not found
viewer-open-externally = Open with default app
viewer-close = Close (Esc)

# F4: editor choice, Shift+F4, editing files on servers
new-file = New File
new-file-default-name = New file.txt
file-name = File name
settings-programs = Programs
settings-editor = Editor (F4)
settings-editor-description = The app F4 opens files in. Files on a server are uploaded again each time you save.
editor-system-default = System text editor
notice-uploaded = Saved { $name } to the server.
notice-upload-failed = Couldn't save { $name } to the server: { $error }

# Favorites menu (Ctrl+D) and Configure dialog
favorites-remove-current = Remove current folder
favorites-configure = Configure…
favorites-configure-title = Configure favorites
favorites-add-item = Add current folder
favorites-delete-item = Delete
favorites-up = Move up
favorites-down = Move down
favorites-title = Title
favorites-folder = Folder
favorites-use-current = Use the active panel's folder
ok = OK

# Folder colors (right-click → Folder color)
folder-color-ellipsis = Folder color…
folder-color-one = Color for “{ $name }”
folder-color-many = Color for { $count } folders
folder-color-default = Default
color-red = Red
color-orange = Orange
color-yellow = Yellow
color-green = Green
color-blue = Blue
color-purple = Purple
color-pink = Pink
color-grey = Grey

# Ctrl+C, Ctrl+V in the same folder: "readme - Copy.txt"
copy-suffix = Copy

# Toolbar: list / grid view
tooltip-list-view = List view (Ctrl+1)
tooltip-grid-view = Grid view (Ctrl+2)
