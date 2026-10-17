//! Browsing inside archives, like Total Commander: an archive opens as a
//! folder, and the files in it have paths that continue the archive's own,
//! e.g. `/home/me/photos.zip/2024/beach.jpg`. Listing reads the archive's
//! table of contents (kept while the archive doesn't change); copying,
//! moving and deleting are in [`crate::ops::archive`].

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::BufReader;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use crate::entry::{DirEntry, EntryKind};
use crate::ops::ArchiveFormat;

/// Whether `path` is an archive 2fip can open as a folder (gz and xz hold
/// just one file, so they are only extracted).
pub fn browsable(path: &Path) -> bool {
    ArchiveFormat::of(path).is_some_and(|format| !format.single_file())
}

/// The archive and the path inside it, for a path inside an archive
/// (`/a/b.zip/c` → `/a/b.zip`, `c`; the archive itself → `/a/b.zip`, ``).
pub fn split(path: &Path) -> Option<(PathBuf, PathBuf)> {
    if crate::vfs::is_remote(path) {
        return None;
    }
    for ancestor in path.ancestors() {
        match std::fs::metadata(ancestor) {
            Ok(metadata) if metadata.is_dir() => return None,
            Ok(metadata) if metadata.is_file() && browsable(ancestor) => {
                let inner = path.strip_prefix(ancestor).unwrap_or(Path::new(""));
                return Some((ancestor.to_path_buf(), inner.to_path_buf()));
            }
            Ok(_) => return None,
            // Not on disk: maybe inside an archive further up.
            Err(_) => {}
        }
    }
    None
}

/// Whether `path` is a file or folder *inside* an archive (not the archive
/// file itself, which is copied, moved and deleted like any file).
pub fn is_inside(path: &Path) -> bool {
    split(path).is_some_and(|(_, inner)| !inner.as_os_str().is_empty())
}

/// Whether `dir`, a panel's folder, is an archive opened as a folder or a
/// folder inside one.
pub fn in_archive(dir: &Path) -> bool {
    split(dir).is_some()
}

/// One file or folder of an archive.
#[derive(Debug, Clone)]
struct IndexEntry {
    path: PathBuf,
    is_dir: bool,
    size: u64,
    modified: Option<SystemTime>,
}

type Index = Arc<Vec<IndexEntry>>;

/// Archive → (modified time, size, table of contents) when it was read.
type IndexCache = Mutex<HashMap<PathBuf, (SystemTime, u64, Index)>>;

/// The archive's table of contents, read again when it changes.
fn index(archive: &Path) -> Result<Index, String> {
    static CACHE: OnceLock<IndexCache> = OnceLock::new();
    let metadata = std::fs::metadata(archive)
        .map_err(|err| format!("failed to read {}: {err}", archive.display()))?;
    let stamp = (
        metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        metadata.len(),
    );
    let cache = CACHE.get_or_init(Default::default);
    if let Some((modified, len, index)) = cache.lock().unwrap().get(archive) {
        if (*modified, *len) == stamp {
            return Ok(index.clone());
        }
    }
    let index = Arc::new(read_index(archive)?);
    cache
        .lock()
        .unwrap()
        .insert(archive.to_path_buf(), (stamp.0, stamp.1, index.clone()));
    Ok(index)
}

fn read_index(archive: &Path) -> Result<Vec<IndexEntry>, String> {
    let error = |err: &dyn std::fmt::Display| format!("{}: {err}", archive.display());
    let open = || {
        File::open(archive)
            .map(BufReader::new)
            .map_err(|err| error(&err))
    };
    let format = ArchiveFormat::of(archive).ok_or_else(|| error(&"not an archive"))?;
    let mut entries = Vec::new();
    match format {
        ArchiveFormat::Zip => {
            let mut zip = zip::ZipArchive::new(open()?).map_err(|err| error(&err))?;
            for index in 0..zip.len() {
                let file = zip.by_index_raw(index).map_err(|err| error(&err))?;
                let Some(path) = file.enclosed_name() else {
                    continue;
                };
                entries.push(IndexEntry {
                    path,
                    is_dir: file.is_dir(),
                    size: file.size(),
                    modified: file.last_modified().and_then(zip_time),
                });
            }
        }
        ArchiveFormat::Tar => tar_index(open()?, &mut entries).map_err(|err| error(&err))?,
        ArchiveFormat::Tgz => tar_index(flate2::read::GzDecoder::new(open()?), &mut entries)
            .map_err(|err| error(&err))?,
        ArchiveFormat::Txz => tar_index(lzma_rust2::XzReader::new(open()?, true), &mut entries)
            .map_err(|err| error(&err))?,
        ArchiveFormat::SevenZ => {
            let seven = sevenz_rust2::ArchiveReader::open(archive, sevenz_rust2::Password::empty())
                .map_err(|err| {
                    if err.to_string().to_lowercase().contains("password") {
                        format!(
                            "{} is encrypted: use Extract with its password",
                            archive.display()
                        )
                    } else {
                        error(&err)
                    }
                })?;
            for file in &seven.archive().files {
                entries.push(IndexEntry {
                    path: safe(Path::new(file.name())),
                    is_dir: file.is_directory(),
                    size: file.size(),
                    modified: file
                        .has_last_modified_date
                        .then(|| file.last_modified_date().into()),
                });
            }
        }
        ArchiveFormat::Gz | ArchiveFormat::Xz => {}
    }
    entries.retain(|entry| !entry.path.as_os_str().is_empty());
    Ok(entries)
}

fn tar_index(reader: impl std::io::Read, entries: &mut Vec<IndexEntry>) -> std::io::Result<()> {
    let mut tar = tar::Archive::new(reader);
    for entry in tar.entries()? {
        let entry = entry?;
        let header = entry.header();
        let kind = header.entry_type();
        if !(kind.is_dir() || kind.is_file() || kind.is_symlink()) {
            continue;
        }
        entries.push(IndexEntry {
            path: safe(&entry.path()?),
            is_dir: kind.is_dir(),
            size: header.size().unwrap_or(0),
            modified: header
                .mtime()
                .ok()
                .map(|secs| SystemTime::UNIX_EPOCH + Duration::from_secs(secs)),
        });
    }
    Ok(())
}

/// A name from an archive as a relative path; anything that would leave the
/// archive (`..`, `/`) is dropped.
fn safe(name: &Path) -> PathBuf {
    name.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part),
            _ => None,
        })
        .collect()
}

/// A zip timestamp (local time, no zone) as a system time, taken as UTC.
fn zip_time(time: zip::DateTime) -> Option<SystemTime> {
    let (year, month, day) = (
        i64::from(time.year()),
        i64::from(time.month()),
        i64::from(time.day()),
    );
    // Days from 1970-01-01 to the date (proleptic Gregorian calendar).
    let shifted = if month <= 2 { year - 1 } else { year };
    let era = shifted.div_euclid(400);
    let year_of_era = shifted - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    let secs = days * 86_400
        + i64::from(time.hour()) * 3600
        + i64::from(time.minute()) * 60
        + i64::from(time.second());
    u64::try_from(secs)
        .ok()
        .map(|secs| SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
}

/// The files and folders in `path`, a folder inside an archive (or the
/// archive itself). Folders only implied by their files' names are listed
/// too.
pub async fn list(path: &Path) -> Result<Vec<DirEntry>, String> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || list_blocking(&path))
        .await
        .map_err(|err| err.to_string())?
}

fn list_blocking(path: &Path) -> Result<Vec<DirEntry>, String> {
    let (archive, inner) =
        split(path).ok_or_else(|| format!("{} is not inside an archive", path.display()))?;
    let index = index(&archive)?;
    // Name → entry; a folder wins over a file of the same name.
    let mut children: BTreeMap<String, DirEntry> = BTreeMap::new();
    for entry in index.iter() {
        let Ok(rest) = entry.path.strip_prefix(&inner) else {
            continue;
        };
        let mut parts = rest.components();
        let Some(first) = parts.next() else {
            continue;
        };
        let name = first.as_os_str().to_string_lossy().into_owned();
        let deeper = parts.next().is_some();
        let is_dir = deeper || entry.is_dir;
        if children
            .get(&name)
            .is_some_and(|existing| existing.kind == EntryKind::Dir)
        {
            continue;
        }
        children.insert(
            name.clone(),
            DirEntry {
                path: archive.join(&inner).join(&name),
                name,
                kind: if is_dir {
                    EntryKind::Dir
                } else {
                    EntryKind::File
                },
                size: if is_dir { 0 } else { entry.size },
                modified: if deeper { None } else { entry.modified },
            },
        );
    }
    Ok(children.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn sample_zip(dir: &Path) -> PathBuf {
        let path = dir.join("pack.zip");
        let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("top.txt", options).unwrap();
        zip.write_all(b"top").unwrap();
        // "docs/" is only implied by its file.
        zip.start_file("docs/inner/deep.txt", options).unwrap();
        zip.write_all(b"deep!").unwrap();
        zip.finish().unwrap();
        path
    }

    #[test]
    fn paths_inside_an_archive_are_recognized() {
        let dir = tempfile::tempdir().unwrap();
        let archive = sample_zip(dir.path());
        assert_eq!(
            split(&archive.join("docs/inner")),
            Some((archive.clone(), PathBuf::from("docs/inner")))
        );
        assert_eq!(split(&archive), Some((archive.clone(), PathBuf::new())));
        // The archive file itself is an ordinary file to copy or delete…
        assert!(!is_inside(&archive));
        assert!(is_inside(&archive.join("top.txt")));
        // …and, opened, a folder view.
        assert!(in_archive(&archive));
        assert_eq!(split(dir.path()), None);
        assert_eq!(split(&dir.path().join("missing/x")), None);
    }

    #[tokio::test]
    async fn lists_folders_implied_by_file_names() {
        let dir = tempfile::tempdir().unwrap();
        let archive = sample_zip(dir.path());
        let top = list(&archive).await.unwrap();
        let names: Vec<(&str, EntryKind)> = top
            .iter()
            .map(|entry| (entry.name.as_str(), entry.kind))
            .collect();
        assert_eq!(
            names,
            vec![("docs", EntryKind::Dir), ("top.txt", EntryKind::File)]
        );
        assert_eq!(top[0].path, archive.join("docs"));

        let deep = list(&archive.join("docs/inner")).await.unwrap();
        assert_eq!(deep.len(), 1);
        assert_eq!(deep[0].name, "deep.txt");
        assert_eq!(deep[0].size, 5);
    }

    #[test]
    fn zip_times_convert() {
        let time = zip::DateTime::from_date_and_time(2024, 3, 1, 12, 30, 0).unwrap();
        let secs = zip_time(time)
            .unwrap()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(secs, 1_709_296_200);
    }
}
