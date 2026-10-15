//! Folder Size: how much disk space a folder, or a whole drive, takes, as a
//! size-sorted tree. Sizes are counted like `du`: the space on the disk,
//! each hard-linked file once, without going into other drives or file
//! systems (`/proc`, network shares, other disks mounted inside).

use std::cmp::Reverse;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    /// Space on the disk; for a folder, everything inside it.
    pub size: u64,
    pub is_dir: bool,
    pub children: Vec<Entry>,
}

/// The result of a scan.
#[derive(Debug, Clone)]
pub struct Scan {
    pub tree: Entry,
    /// Folders inside that couldn't be read (no permission), so not counted.
    pub unreadable: u64,
    /// The space of the drive the folder is on, if the system tells.
    pub drive: Option<DriveSpace>,
    /// Whether the folder is the top of its drive (`/`, `C:\`, a mounted
    /// disk): then the drive's used space minus the counted size is what
    /// 2fip couldn't see (system files, unreadable folders, file system
    /// data).
    pub is_drive_root: bool,
}

/// A drive's size, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriveSpace {
    pub total: u64,
    pub used: u64,
    /// Free space the user can use.
    pub available: u64,
}

/// Follows a running scan from another thread: how far it got, and a way to
/// stop it.
#[derive(Debug, Default)]
pub struct ScanProgress {
    entries: AtomicU64,
    bytes: AtomicU64,
    cancelled: AtomicBool,
}

impl ScanProgress {
    pub fn new() -> Self {
        Self::default()
    }

    /// Files and folders seen so far, and their size.
    pub fn counted(&self) -> (u64, u64) {
        (
            self.entries.load(Ordering::Relaxed),
            self.bytes.load(Ordering::Relaxed),
        )
    }

    /// Stops the scan (it then returns an error).
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

/// What every thread of a scan shares.
struct Context<'a> {
    spare_threads: AtomicUsize,
    progress: &'a ScanProgress,
    /// The file system to stay on (Unix); `None` to go everywhere.
    device: Option<u64>,
    /// Hard-linked files already counted, by (device, inode).
    seen_links: Mutex<HashSet<(u64, u64)>>,
    unreadable: AtomicU64,
}

/// Builds the size-sorted tree of a local folder. In the Flatpak, `/` means
/// the computer's root (its system folders are under `/run/host`).
pub fn scan(root: &Path, progress: &ScanProgress) -> Result<Scan, String> {
    if crate::vfs::is_remote(root) {
        return Err("folder size is not available for server folders".to_string());
    }
    let metadata = fs::symlink_metadata(root)
        .map_err(|err| format!("failed to read {}: {err}", root.display()))?;
    if !metadata.is_dir() {
        return Err(format!("{} is not a directory", root.display()));
    }

    let flatpak_root = crate::sandbox::in_flatpak() && root == Path::new("/");
    // The drive is the one the folder is on; for the Flatpak's `/`, the
    // computer's own root (where its /usr is).
    let drive_path = if flatpak_root {
        PathBuf::from("/run/host/usr")
    } else {
        root.to_path_buf()
    };
    let context = Context {
        // Extra threads for subfolders, on top of the calling one.
        spare_threads: AtomicUsize::new(
            std::thread::available_parallelism().map_or(1, |n| n.get()) - 1,
        ),
        progress,
        device: device_of(&drive_path),
        seen_links: Mutex::new(HashSet::new()),
        unreadable: AtomicU64::new(0),
    };

    let name = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());
    let tree = if flatpak_root {
        scan_flatpak_root(&context)?
    } else {
        scan_directory(root, name, &context)?
    };
    Ok(Scan {
        tree,
        unreadable: context.unreadable.load(Ordering::Relaxed),
        drive: drive_space(&drive_path),
        is_drive_root: flatpak_root || is_drive_root(root),
    })
}

/// The Flatpak's `/` is its own (a runtime, a few folders of the
/// computer's): the computer's folders that are visible, plus its `/usr`
/// and `/etc` from `/run/host`. The rest (e.g. its `/var`) shows up as the
/// space 2fip couldn't see.
fn scan_flatpak_root(context: &Context) -> Result<Entry, String> {
    const SANDBOX_ONLY: [&str; 15] = [
        ".flatpak-info",
        "app",
        "bin",
        "dev",
        "etc",
        "lib",
        "lib32",
        "lib64",
        "proc",
        "run",
        "sbin",
        "sys",
        "tmp",
        "usr",
        "var",
    ];
    let read_dir = fs::read_dir("/").map_err(|err| format!("failed to read /: {err}"))?;
    let mut folders: Vec<(PathBuf, String)> = read_dir
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| {
            (
                entry.path(),
                entry.file_name().to_string_lossy().into_owned(),
            )
        })
        .filter(|(_, name)| !SANDBOX_ONLY.contains(&name.as_str()))
        .collect();
    for name in ["usr", "etc"] {
        let path = Path::new("/run/host").join(name);
        if path.is_dir() {
            folders.push((path, name.to_string()));
        }
    }
    let mut children: Vec<Entry> = folders
        .into_iter()
        .filter(|(path, _)| context.device.is_none() || device_of(path) == context.device)
        .filter_map(|(path, name)| scan_directory(&path, name, context).ok())
        .collect();
    if context.progress.is_cancelled() {
        return Err("cancelled".to_string());
    }
    sort_largest_first(&mut children);
    Ok(Entry {
        name: "/".to_string(),
        path: PathBuf::from("/"),
        size: children.iter().map(|child| child.size).sum(),
        is_dir: true,
        children,
    })
}

/// Takes one of the spare threads, if any is left.
fn take_thread(spare: &AtomicUsize) -> bool {
    spare
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_sub(1))
        .is_ok()
}

/// Scans `path` recursively. Subfolders go to spare threads while there are
/// any (the rest are scanned on this one), so a big tree uses every core.
fn scan_directory(path: &Path, name: String, context: &Context) -> Result<Entry, String> {
    if context.progress.is_cancelled() {
        return Err("cancelled".to_string());
    }
    let read_dir =
        fs::read_dir(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    // The folder's own space (its list of names), as `du` counts it.
    let own_size = fs::symlink_metadata(path)
        .map(|metadata| disk_usage(&metadata))
        .unwrap_or(0);
    let mut children = Vec::new();
    let mut subfolders = Vec::new();
    let mut counted = 0u64;

    for entry in read_dir.flatten() {
        // The type comes with the listing, and the size is read relative to
        // the open folder: no extra lookup of the full path. Neither follows
        // symbolic links.
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let child_name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_dir() {
            // Another file system mounted here (another disk, /proc, a
            // network share): not part of this drive.
            if context.device.is_some() && device(&metadata) != context.device {
                continue;
            }
            subfolders.push((entry.path(), child_name));
            continue;
        }
        let size = if context.first_link(&metadata) {
            disk_usage(&metadata)
        } else {
            0
        };
        counted += size;
        children.push(Entry {
            name: child_name,
            path: entry.path(),
            size,
            is_dir: false,
            children: Vec::new(),
        });
    }
    context
        .progress
        .entries
        .fetch_add(children.len() as u64 + 1, Ordering::Relaxed);
    context
        .progress
        .bytes
        .fetch_add(counted + own_size, Ordering::Relaxed);

    let scan_subfolder = |child_path: &Path, child_name: String| {
        let result = scan_directory(child_path, child_name, context);
        if result.is_err() && !context.progress.is_cancelled() {
            context.unreadable.fetch_add(1, Ordering::Relaxed);
        }
        result
    };
    std::thread::scope(|scope| {
        let mut threads = Vec::new();
        for (child_path, child_name) in subfolders {
            if take_thread(&context.spare_threads) {
                threads.push(scope.spawn(move || {
                    let result = scan_subfolder(&child_path, child_name);
                    context.spare_threads.fetch_add(1, Ordering::AcqRel);
                    result
                }));
            } else if let Ok(child) = scan_subfolder(&child_path, child_name) {
                children.push(child);
            }
        }
        for thread in threads {
            if let Ok(Ok(child)) = thread.join() {
                children.push(child);
            }
        }
    });
    if context.progress.is_cancelled() {
        return Err("cancelled".to_string());
    }

    sort_largest_first(&mut children);
    let size = own_size + children.iter().map(|child| child.size).sum::<u64>();
    Ok(Entry {
        name,
        path: path.to_path_buf(),
        size,
        is_dir: true,
        children,
    })
}

/// Largest first; the lower-case names are made once per entry, not on
/// every comparison.
fn sort_largest_first(children: &mut [Entry]) {
    children.sort_by_cached_key(|child| (Reverse(child.size), child.name.to_lowercase()));
}

impl Context<'_> {
    /// Whether this is the first time the scan meets this file: a file with
    /// several hard links (common in Flatpak's storage) is counted once.
    fn first_link(&self, metadata: &fs::Metadata) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.nlink() > 1 {
                let key = (metadata.dev(), metadata.ino());
                return self
                    .seen_links
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .insert(key);
            }
        }
        #[cfg(not(unix))]
        let _ = (metadata, &self.seen_links);
        true
    }
}

/// The space a file takes on the disk (blocks used), like `du`: less than
/// its size for sparse files, rounded up to whole blocks otherwise.
#[cfg(unix)]
fn disk_usage(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    metadata.blocks() * 512
}

#[cfg(not(unix))]
fn disk_usage(metadata: &fs::Metadata) -> u64 {
    if metadata.is_dir() {
        0
    } else {
        metadata.len()
    }
}

#[cfg(unix)]
fn device(metadata: &fs::Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    Some(metadata.dev())
}

#[cfg(not(unix))]
fn device(_metadata: &fs::Metadata) -> Option<u64> {
    None
}

fn device_of(path: &Path) -> Option<u64> {
    fs::symlink_metadata(path).ok().as_ref().and_then(device)
}

/// Whether `path` is the top of its drive: `/`, a mount point, `C:\`.
fn is_drive_root(path: &Path) -> bool {
    match path.parent() {
        None => true,
        Some(parent) => device_of(path).is_some() && device_of(path) != device_of(parent),
    }
}

/// The size, used and free space of the drive holding `path`.
#[cfg(unix)]
pub fn drive_space(path: &Path) -> Option<DriveSpace> {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: `path` is NUL-terminated and `stats` is a valid statvfs.
    if unsafe { libc::statvfs(path.as_ptr(), &mut stats) } != 0 {
        return None;
    }
    let block = stats.f_frsize as u64;
    let total = stats.f_blocks as u64 * block;
    let free = stats.f_bfree as u64 * block;
    Some(DriveSpace {
        total,
        used: total.saturating_sub(free),
        available: stats.f_bavail as u64 * block,
    })
}

/// The size, used and free space of the drive holding `path`.
#[cfg(windows)]
pub fn drive_space(path: &Path) -> Option<DriveSpace> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let (mut available, mut total, mut free) = (0u64, 0u64, 0u64);
    // SAFETY: `wide` is NUL-terminated and the outputs are valid u64s.
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, &mut total, &mut free) };
    (ok != 0).then(|| DriveSpace {
        total,
        used: total.saturating_sub(free),
        available,
    })
}

#[cfg(not(any(unix, windows)))]
pub fn drive_space(_path: &Path) -> Option<DriveSpace> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregates_directory_sizes_and_sorts_children() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("small.txt"), b"123").unwrap();
        fs::write(root.join("nested").join("large.bin"), vec![7u8; 200_000]).unwrap();
        fs::write(root.join("nested").join("tiny.txt"), b"1").unwrap();

        let progress = ScanProgress::new();
        let scan = scan(&root, &progress).unwrap();
        let tree = scan.tree;

        assert!(tree.is_dir);
        assert_eq!(tree.children[0].name, "nested");
        assert_eq!(tree.children[0].children[0].name, "large.bin");
        assert_eq!(tree.children[1].name, "small.txt");
        assert!(!tree.children[1].is_dir);
        // A folder holds at least what is inside it.
        let inside: u64 = tree.children.iter().map(|child| child.size).sum();
        assert!(tree.size >= inside);
        assert!(tree.children[0].size >= 200_000);
        assert_eq!(scan.unreadable, 0);
        assert!(!scan.is_drive_root);
        assert!(progress.counted().0 >= 5);
    }

    #[cfg(unix)]
    #[test]
    fn hard_links_count_once() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("a.bin"), vec![1u8; 100_000]).unwrap();
        fs::hard_link(root.join("a.bin"), root.join("b.bin")).unwrap();

        let tree = scan(&root, &ScanProgress::new()).unwrap().tree;

        let sizes: Vec<u64> = tree.children.iter().map(|child| child.size).collect();
        assert_eq!(sizes.len(), 2);
        assert!(sizes[0] >= 100_000);
        assert_eq!(sizes[1], 0);
    }

    #[test]
    fn a_cancelled_scan_stops() {
        let temp = tempfile::tempdir().unwrap();
        let progress = ScanProgress::new();
        progress.cancel();
        assert!(scan(temp.path(), &progress).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn the_root_is_a_drive_root_with_its_space() {
        assert!(is_drive_root(Path::new("/")));
        let space = drive_space(Path::new("/")).unwrap();
        assert!(space.total > 0 && space.used <= space.total);
    }
}
