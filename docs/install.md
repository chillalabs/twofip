---
layout: default
title: Installation
---

2fip runs on Linux and Windows. On Linux it is made for the COSMIC desktop
(Pop!_OS 24.04 and later, Fedora COSMIC, and others) but works on other
Wayland desktops too. For Windows, see [Windows](#windows) below.

## Flatpak (recommended)

The Flatpak works on any distribution and updates itself with the rest of
your Flatpak apps:

```sh
flatpak install --user https://chillalabs.github.io/flatpak/2fip.flatpakref
```

Or open [chillalabs.github.io/flatpak](https://chillalabs.github.io/flatpak/)
and click **Install with Flatpak**. The runtime it needs is installed from
Flathub automatically.

To update: `flatpak update`. To remove: `flatpak uninstall io.github.chillalabs.TwoFip`.

> The Flatpak runs your terminal, other apps and thumbnailers on the system
> (outside the sandbox), so it behaves like a regular install.

## AppImage

Download `2fip-<version>-x86_64.AppImage` from the
[latest release](https://github.com/chillalabs/twofip/releases/latest),
make it executable and run it:

```sh
chmod +x 2fip-*-x86_64.AppImage
./2fip-*-x86_64.AppImage
```

It needs glibc 2.39 or newer (Ubuntu / Pop!_OS 24.04, Fedora 40, or later)
and doesn't update itself.

## Package for your user

The `2fip-<version>-x86_64.tar.gz` file on the releases page installs 2fip
into `~/.local` (the program, its launcher and icon):

```sh
tar xzf 2fip-*-x86_64.tar.gz
cd 2fip-*-x86_64
./install.sh
```

Remove it with `./install.sh --uninstall`. Your settings are kept.

## Windows

Windows 10 and 11 (64-bit). Download from the
[latest release](https://github.com/chillalabs/twofip/releases/latest):

- **Installer:** `2fip-<version>-windows-x64-setup.exe`. It installs for your
  user without admin rights (or for all users, if you choose so), adds 2fip
  to the Start menu and an uninstaller to *Settings → Apps*. Installing a
  newer version updates the old one.
- **Portable:** `2fip-<version>-windows-x64-portable.zip`. Unzip it anywhere
  (Documents, a USB stick…) and run `2fip.exe`: no installation, no admin
  rights. The `portable.txt` file next to it keeps settings, favorites and
  saved connections in a `data` folder beside the program, so the folder can
  be moved. Delete `portable.txt` to use your user profile instead.

2fip isn't signed yet, so Windows may say **"Windows protected your PC"** the
first time: click **More info → Run anyway**. In companies, rules such as
AppLocker may only allow programs installed in `C:\Program Files`: use the
installer and choose **Install for all users** (that needs admin rights).

On Windows, 2fip uses drive letters (`C:`, `D:`, USB sticks…), Explorer's
right-click menu and icons, and Windows Credential Manager for saved
passwords.

## Build from source

You need Rust (stable), [just](https://github.com/casey/just) and the usual
COSMIC build packages (for example `libxkbcommon-dev`):

```sh
git clone https://github.com/chillalabs/twofip
cd twofip
just run        # build and start
just install    # install into ~/.local
```

## Where 2fip keeps its data

| What | Where |
|---|---|
| Settings, favorites, saved connections, open tabs | `~/.config/2fip/` |
| Passwords you chose to remember | the system keyring (GNOME Keyring, KWallet) |
| Thumbnails (shared with other file managers) | `~/.cache/thumbnails/` |
| Local copies of files opened from servers | `~/.cache/2fip/remote/` |

On Windows: settings in `%APPDATA%\2fip\`, caches in `%LOCALAPPDATA%\2fip\`,
passwords in Credential Manager; the portable version keeps settings and
caches in its `data` folder.
