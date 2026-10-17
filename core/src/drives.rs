//! The drives for each panel's drive menu: drive letters on Windows; on
//! Linux the system, the home folder, and the disks and USB sticks the
//! system knows (from `lsblk`), mounted or not. Mounting and ejecting go
//! through udisks (`udisksctl`), like in the desktop's own file manager:
//! no password for USB sticks.

use std::path::{Component, Path, PathBuf};

/// One entry of the drive menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drive {
    pub kind: DriveKind,
    /// Its name: the partition label, or e.g. `C:`; empty when it has none.
    pub label: String,
    /// Where its files are; `None` while not mounted.
    pub mount: Option<PathBuf>,
    /// The partition to mount (`/dev/sdb1`).
    pub device: Option<String>,
    /// The whole disk it is on (`/dev/sdb`), to power it off on eject.
    pub disk: Option<String>,
    /// Size in bytes (0 if unknown).
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveKind {
    /// `/`.
    System,
    Home,
    /// A USB stick, memory card or other disk that can be unplugged.
    Removable,
    /// Another internal disk or partition.
    Internal,
    /// A Windows drive letter.
    Letter,
}

impl Drive {
    /// Whether it can be ejected (unmounted and powered off).
    pub fn ejectable(&self) -> bool {
        self.kind == DriveKind::Removable && self.mount.is_some() && self.device.is_some()
    }
}

/// The drives Windows reports (also removable and network drives), in
/// letter order.
#[cfg(windows)]
pub fn list() -> Vec<Drive> {
    // SAFETY: no arguments; returns a bitmask, bit 0 = A:, bit 1 = B:, ...
    let mask = unsafe { windows_sys::Win32::Storage::FileSystem::GetLogicalDrives() };
    (0..26u8)
        .filter(|bit| mask & (1 << bit) != 0)
        .map(|bit| {
            let letter = (b'A' + bit) as char;
            Drive {
                kind: DriveKind::Letter,
                label: format!("{letter}:"),
                mount: Some(PathBuf::from(format!("{letter}:\\"))),
                device: None,
                disk: None,
                size: 0,
            }
        })
        .collect()
}

/// The system, the home folder, then the other drives by name. Blocking:
/// runs `lsblk` (on the host, in the Flatpak).
#[cfg(not(windows))]
pub fn list() -> Vec<Drive> {
    let home = crate::sandbox::home_dir();
    let mut drives = vec![
        Drive {
            kind: DriveKind::System,
            label: String::new(),
            mount: Some(PathBuf::from("/")),
            device: None,
            disk: None,
            size: 0,
        },
        Drive {
            kind: DriveKind::Home,
            label: String::new(),
            mount: Some(home.clone()),
            device: None,
            disk: None,
            size: 0,
        },
    ];
    let output = crate::sandbox::host_command("lsblk")
        .args([
            "-J",
            "-b",
            "-o",
            "NAME,PATH,LABEL,SIZE,FSTYPE,MOUNTPOINTS,HOTPLUG,RM,TYPE",
        ])
        .output();
    if let Some(output) = output.ok().filter(|output| output.status.success()) {
        let mut others = parse_lsblk(&String::from_utf8_lossy(&output.stdout), &home);
        others.sort_by_key(|drive| drive.label.to_lowercase());
        drives.extend(others);
    }
    drives
}

/// The drives in `lsblk -J -b` output worth showing: mounted file systems
/// other than the system's own (`/`, `/boot`, …), and unmounted ones on
/// removable disks.
#[cfg_attr(windows, allow(dead_code))]
fn parse_lsblk(json: &str, home: &Path) -> Vec<Drive> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let mut drives = Vec::new();
    for disk in value["blockdevices"].as_array().into_iter().flatten() {
        let disk_path = disk["path"].as_str().map(str::to_string);
        let removable = flag(&disk["hotplug"]) || flag(&disk["rm"]);
        collect(disk, disk_path.as_deref(), removable, home, &mut drives);
    }
    drives
}

/// Adds `node` (if it holds a file system) or its partitions to `drives`.
#[cfg_attr(windows, allow(dead_code))]
fn collect(
    node: &serde_json::Value,
    disk: Option<&str>,
    removable: bool,
    home: &Path,
    drives: &mut Vec<Drive>,
) {
    if let Some(children) = node["children"].as_array().filter(|list| !list.is_empty()) {
        for child in children {
            collect(child, disk, removable, home, drives);
        }
        return;
    }
    const NOT_FILES: [&str; 5] = [
        "swap",
        "crypto_LUKS",
        "LVM2_member",
        "linux_raid_member",
        "zfs_member",
    ];
    let Some(fstype) = node["fstype"].as_str() else {
        return;
    };
    if NOT_FILES.contains(&fstype) {
        return;
    }
    let mounts: Vec<&Path> = node["mountpoints"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|mount| mount.as_str())
        .filter(|mount| mount.starts_with('/'))
        .map(Path::new)
        .collect();
    // The system's own partitions (shown as System and Home already, or not
    // meant to be browsed: boot, recovery). In a container (Distrobox,
    // Toolbx) the host's are mounted in many places (/usr/…, /etc/…,
    // /run/host/…), so any such mount point marks them.
    if mounts.iter().any(|mount| is_system_mount(mount, home)) {
        return;
    }
    // The same partition can show up twice in a container: prefer the
    // container's own path (/media/…) to the host's (/run/host/media/…).
    let mount = mounts
        .iter()
        .find(|mount| !mount.starts_with("/run/host"))
        .or(mounts.first())
        .map(|mount| mount.to_path_buf());
    let size = number(&node["size"]);
    match &mount {
        Some(_) => {}
        // Unmounted: only removable ones (internal ones are usually system
        // partitions), and not tiny ones (boot data of installer sticks).
        None if !removable || size < 64 * 1024 * 1024 => return,
        None => {}
    }
    drives.push(Drive {
        kind: if removable {
            DriveKind::Removable
        } else {
            DriveKind::Internal
        },
        label: node["label"].as_str().unwrap_or_default().to_string(),
        mount,
        device: node["path"].as_str().map(str::to_string),
        disk: disk.map(str::to_string),
        size,
    });
}

/// Whether a mount point belongs to the system rather than to a disk of
/// the user's.
#[cfg_attr(windows, allow(dead_code))]
fn is_system_mount(mount: &Path, home: &Path) -> bool {
    const SYSTEM: [&str; 10] = [
        "/boot",
        "/efi",
        "/recovery",
        "/usr",
        "/etc",
        "/var",
        "/opt",
        "/srv",
        "/nix",
        "/snap",
    ];
    let host = Path::new("/run/host");
    let mount = match mount.strip_prefix(host) {
        // The host's /media and /mnt are still the user's disks.
        Ok(inner) if inner.starts_with("media") || inner.starts_with("mnt/") => return false,
        Ok(inner) => Path::new("/").join(inner),
        Err(_) => mount.to_path_buf(),
    };
    mount == Path::new("/")
        || mount == home
        || mount == Path::new("/mnt")
        || mount == Path::new("/media")
        || SYSTEM.iter().any(|dir| mount.starts_with(dir))
}

/// lsblk writes flags as `true`/`false`, older versions as `"1"`/`"0"`.
fn flag(value: &serde_json::Value) -> bool {
    value
        .as_bool()
        .unwrap_or_else(|| value.as_str() == Some("1"))
}

/// A number, or a number in a string (older lsblk).
fn number(value: &serde_json::Value) -> u64 {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
        .unwrap_or(0)
}

/// `udisksctl`, on the host: directly, through `flatpak-spawn` in the
/// Flatpak, or through `distrobox-host-exec` in a Distrobox container that
/// doesn't have it.
fn udisksctl() -> std::process::Command {
    use crate::sandbox::{host_command, host_program_exists};
    if !host_program_exists("udisksctl") && host_program_exists("distrobox-host-exec") {
        let mut command = std::process::Command::new("distrobox-host-exec");
        command.arg("udisksctl");
        return command;
    }
    host_command("udisksctl")
}

/// Mounts the partition `device` (udisks; no password for USB sticks) and
/// returns where. Blocking.
pub fn mount(device: &str) -> Result<PathBuf, String> {
    let output = udisksctl()
        .args(["mount", "--no-user-interaction", "-b", device])
        .output()
        .map_err(|err| format!("udisksctl: {err}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    // "Mounted /dev/sdb1 at /media/user/STICK" (older versions add a dot).
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .split_once(" at ")
        .map(|(_, path)| PathBuf::from(path.trim().trim_end_matches('.')))
        .ok_or_else(|| stdout.trim().to_string())
}

/// Unmounts every mounted partition of `drive`'s disk, then powers the disk
/// off, so it can be unplugged safely. Blocking.
pub fn eject(drive: &Drive, all: &[Drive]) -> Result<(), String> {
    let run = |args: &[&str]| -> Result<(), String> {
        let output = udisksctl()
            .args(args)
            .output()
            .map_err(|err| format!("udisksctl: {err}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    };
    let same_disk = all
        .iter()
        .filter(|other| other.mount.is_some() && other.disk.is_some() && other.disk == drive.disk);
    for partition in same_disk {
        if let Some(device) = &partition.device {
            run(&["unmount", "--no-user-interaction", "-b", device])?;
        }
    }
    if let Some(disk) = &drive.disk {
        run(&["power-off", "--no-user-interaction", "-b", disk])?;
    }
    Ok(())
}

/// The drive `path` is on, e.g. `C:\` for `C:\Users` (`None` on Linux and
/// for server paths).
pub fn drive_of(path: &Path) -> Option<PathBuf> {
    match path.components().next()? {
        // Upper case, so a typed `c:\x` still matches the menu's `C:\`.
        Component::Prefix(prefix) => Some(PathBuf::from(format!(
            "{}\\",
            prefix.as_os_str().to_string_lossy().to_uppercase()
        ))),
        _ => None,
    }
}

/// The drive among `drives` that holds `path`: the one with the longest
/// mount folder containing it.
pub fn containing<'a>(drives: &'a [Drive], path: &Path) -> Option<&'a Drive> {
    // Windows: by drive letter (any case).
    if let Some(letter) = drive_of(path) {
        return drives
            .iter()
            .find(|drive| drive.mount.as_deref() == Some(letter.as_path()));
    }
    drives
        .iter()
        .filter(|drive| {
            drive
                .mount
                .as_ref()
                .is_some_and(|mount| path.starts_with(mount))
        })
        .max_by_key(|drive| {
            drive
                .mount
                .as_ref()
                .map_or(0, |mount| mount.as_os_str().len())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LSBLK: &str = r#"{"blockdevices": [
        {"name":"sda","path":"/dev/sda","label":"Pop_OS","size":62914560000,"fstype":"iso9660",
         "mountpoints":[null],"hotplug":true,"rm":true,"type":"disk","children":[
            {"name":"sda1","path":"/dev/sda1","label":"Pop_OS","size":3519774720,"fstype":"iso9660",
             "mountpoints":["/media/me/Pop_OS"],"hotplug":true,"rm":true,"type":"part"},
            {"name":"sda2","path":"/dev/sda2","label":null,"size":4194304,"fstype":"vfat",
             "mountpoints":[null],"hotplug":true,"rm":true,"type":"part"},
            {"name":"sda3","path":"/dev/sda3","label":"writable","size":59377057792,"fstype":"ext4",
             "mountpoints":[null],"hotplug":true,"rm":true,"type":"part"}]},
        {"name":"nvme0n1","path":"/dev/nvme0n1","label":null,"size":1000204886016,"fstype":null,
         "mountpoints":[null],"hotplug":false,"rm":false,"type":"disk","children":[
            {"name":"p1","path":"/dev/nvme0n1p1","label":null,"size":1071644672,"fstype":"vfat",
             "mountpoints":["/boot/efi"],"hotplug":false,"rm":false,"type":"part"},
            {"name":"p3","path":"/dev/nvme0n1p3","label":null,"size":990000000000,"fstype":"ext4",
             "mountpoints":["/"],"hotplug":false,"rm":false,"type":"part"},
            {"name":"p4","path":"/dev/nvme0n1p4","label":null,"size":4294967296,"fstype":"swap",
             "mountpoints":["[SWAP]"],"hotplug":false,"rm":false,"type":"part"},
            {"name":"p5","path":"/dev/nvme0n1p5","label":"Data","size":500000000000,"fstype":"ext4",
             "mountpoints":["/mnt/data"],"hotplug":false,"rm":false,"type":"part"}]}
    ]}"#;

    #[test]
    fn lists_usb_partitions_and_other_disks_but_not_the_system() {
        let drives = parse_lsblk(LSBLK, Path::new("/home/me"));
        let names: Vec<(&str, bool)> = drives
            .iter()
            .map(|drive| (drive.label.as_str(), drive.mount.is_some()))
            .collect();
        // sda2 is too small, /boot/efi, / and swap are the system's.
        assert_eq!(
            names,
            vec![("Pop_OS", true), ("writable", false), ("Data", true)]
        );
        assert_eq!(drives[1].kind, DriveKind::Removable);
        assert_eq!(drives[1].device.as_deref(), Some("/dev/sda3"));
        assert_eq!(drives[1].disk.as_deref(), Some("/dev/sda"));
        assert!(drives[0].ejectable());
        assert!(!drives[1].ejectable());
        assert_eq!(drives[2].kind, DriveKind::Internal);
    }

    #[test]
    fn in_a_container_the_hosts_system_partitions_stay_hidden() {
        let json = r#"{"blockdevices": [
            {"name":"sda1","path":"/dev/sda1","label":"STICK","size":3519774720,"fstype":"vfat",
             "mountpoints":["/media/me/STICK","/run/host/media/me/STICK"],
             "hotplug":true,"rm":true,"type":"part"},
            {"name":"p1","path":"/dev/nvme0n1p1","label":null,"size":1071644672,"fstype":"vfat",
             "mountpoints":["/run/host/boot/efi"],"hotplug":false,"rm":false,"type":"part"},
            {"name":"p3","path":"/dev/nvme0n1p3","label":null,"size":990000000000,"fstype":"ext4",
             "mountpoints":["/usr/local/share/fonts","/mnt","/media","/etc/machine-id"],
             "hotplug":false,"rm":false,"type":"part"}
        ]}"#;
        let drives = parse_lsblk(json, Path::new("/home/me"));
        assert_eq!(drives.len(), 1);
        assert_eq!(
            drives[0].mount.as_deref(),
            Some(Path::new("/media/me/STICK"))
        );
    }

    #[test]
    fn finds_the_drive_holding_a_folder() {
        let mut drives = parse_lsblk(LSBLK, Path::new("/home/me"));
        drives.insert(
            0,
            Drive {
                kind: DriveKind::System,
                label: String::new(),
                mount: Some(PathBuf::from("/")),
                device: None,
                disk: None,
                size: 0,
            },
        );
        let found = containing(&drives, Path::new("/media/me/Pop_OS/casper")).unwrap();
        assert_eq!(found.label, "Pop_OS");
        let found = containing(&drives, Path::new("/etc")).unwrap();
        assert_eq!(found.kind, DriveKind::System);
    }

    #[test]
    fn windows_drive_of_a_path() {
        assert_eq!(drive_of(Path::new("/home/user")), None);
    }
}
