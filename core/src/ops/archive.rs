//! Pack files into an archive and unpack archives (Alt+F5 / Extract), in
//! the formats Total Commander's packer offers: zip, tar, gz, tgz, xz, txz
//! and 7z. All pure Rust, so they behave the same everywhere. Zip and 7z
//! can be encrypted with a password (AES-256).

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Read, Write};
use std::num::NonZeroU64;
use std::path::{Component, Path, PathBuf};

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;

use super::compress::{plan, ArchiveEntry, ArchiveEntryKind};
use super::{CancelHandle, ConflictHandle, ConflictResolution, OpEvent, OpProgress};

/// An archive format, in the order the Pack dialog offers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    Zip,
    Tar,
    /// One file, gzip-compressed.
    Gz,
    /// tar + gzip.
    Tgz,
    /// One file, xz-compressed.
    Xz,
    /// tar + xz.
    Txz,
    SevenZ,
}

impl ArchiveFormat {
    pub const ALL: [ArchiveFormat; 7] = [
        ArchiveFormat::Zip,
        ArchiveFormat::Tar,
        ArchiveFormat::Gz,
        ArchiveFormat::Tgz,
        ArchiveFormat::Xz,
        ArchiveFormat::Txz,
        ArchiveFormat::SevenZ,
    ];

    /// The file extension, also its name in the Pack dialog.
    pub fn extension(self) -> &'static str {
        match self {
            ArchiveFormat::Zip => "zip",
            ArchiveFormat::Tar => "tar",
            ArchiveFormat::Gz => "gz",
            ArchiveFormat::Tgz => "tgz",
            ArchiveFormat::Xz => "xz",
            ArchiveFormat::Txz => "txz",
            ArchiveFormat::SevenZ => "7z",
        }
    }

    /// Whether it can be protected with a password.
    pub fn encrypts(self) -> bool {
        matches!(self, ArchiveFormat::Zip | ArchiveFormat::SevenZ)
    }

    /// Whether it holds a single file only (gz, xz).
    pub fn single_file(self) -> bool {
        matches!(self, ArchiveFormat::Gz | ArchiveFormat::Xz)
    }

    /// The format of an archive, from its name (`.tar.gz` too).
    pub fn of(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().to_lowercase();
        let ends = |suffix: &str| name.ends_with(suffix);
        Some(if ends(".tar.gz") || ends(".tgz") {
            ArchiveFormat::Tgz
        } else if ends(".tar.xz") || ends(".txz") {
            ArchiveFormat::Txz
        } else if ends(".zip") {
            ArchiveFormat::Zip
        } else if ends(".tar") {
            ArchiveFormat::Tar
        } else if ends(".gz") {
            ArchiveFormat::Gz
        } else if ends(".xz") {
            ArchiveFormat::Xz
        } else if ends(".7z") {
            ArchiveFormat::SevenZ
        } else {
            return None;
        })
    }
}

/// How to pack.
#[derive(Debug, Clone)]
pub struct PackOptions {
    pub format: ArchiveFormat,
    /// Encrypts zip and 7z archives; ignored for the other formats.
    pub password: Option<String>,
    /// "Move to archive": delete the originals once the archive is complete.
    pub move_files: bool,
}

/// How to unpack.
#[derive(Debug, Clone)]
pub struct UnpackOptions {
    /// Recreate the folders stored in the archive (else everything goes
    /// straight into the destination).
    pub keep_paths: bool,
    /// Replace existing files without asking.
    pub overwrite: bool,
    pub password: Option<String>,
}

/// Packs `sources` (files and folders) into the new archive `archive`. It
/// never replaces an existing file; a cancelled or failed run removes the
/// partial archive. With `move_files`, the originals are deleted afterwards.
pub fn pack(
    sources: Vec<PathBuf>,
    archive: PathBuf,
    options: PackOptions,
    cancel: CancelHandle,
) -> impl Stream<Item = OpEvent> {
    let (tx, rx) = mpsc::channel(16);
    if sources
        .iter()
        .chain([&archive])
        .any(|path| crate::vfs::is_remote(path))
    {
        let _ = tx.try_send(OpEvent::Error(
            "packing files on a server isn't supported yet; copy them here first".to_string(),
        ));
        return ReceiverStream::new(rx);
    }
    tokio::task::spawn_blocking(move || {
        let event = match pack_blocking(&sources, &archive, &options, &cancel, &tx) {
            Ok(()) => OpEvent::Done,
            Err(stop) => stop,
        };
        let _ = tx.blocking_send(event);
    });
    ReceiverStream::new(rx)
}

fn pack_blocking(
    sources: &[PathBuf],
    archive: &Path,
    options: &PackOptions,
    cancel: &CancelHandle,
    tx: &mpsc::Sender<OpEvent>,
) -> Result<(), OpEvent> {
    if options.format.single_file() && !(sources.len() == 1 && sources[0].is_file()) {
        return Err(OpEvent::Error(format!(
            "{} packs a single file; use tgz, txz or another format for folders and several files",
            options.format.extension()
        )));
    }
    let entries = plan(sources).map_err(OpEvent::Error)?;
    let file = File::create_new(archive)
        .map_err(|err| OpEvent::Error(format!("failed to create {}: {err}", archive.display())))?;
    // From here on the archive is ours, so a partial one is removed on failure.
    let mut progress = Progress::new(&entries, tx);
    let written = match options.format {
        ArchiveFormat::Zip => write_zip(file, &entries, options, cancel, &mut progress),
        ArchiveFormat::Tar => write_tar(BufWriter::new(file), &entries, cancel, &mut progress)
            .and_then(|writer| finish(writer.into_inner().map_err(|err| err.into_error()))),
        ArchiveFormat::Tgz => {
            let gz = ParallelGzWriter::new(BufWriter::new(file))
                .map_err(|err| io_error(archive, err))?;
            write_tar(gz, &entries, cancel, &mut progress).and_then(|gz| finish(gz.finish()))
        }
        ArchiveFormat::Txz => {
            let xz = xz_writer(BufWriter::new(file))?;
            write_tar(xz, &entries, cancel, &mut progress).and_then(|xz| finish(xz.finish()))
        }
        ArchiveFormat::Gz => {
            let gz = ParallelGzWriter::new(BufWriter::new(file))
                .map_err(|err| io_error(archive, err))?;
            write_single(gz, &entries[0], cancel, &mut progress).and_then(|gz| finish(gz.finish()))
        }
        ArchiveFormat::Xz => {
            let xz = xz_writer(BufWriter::new(file))?;
            write_single(xz, &entries[0], cancel, &mut progress).and_then(|xz| finish(xz.finish()))
        }
        ArchiveFormat::SevenZ => write_7z(file, &entries, options, cancel, &mut progress),
    };
    if let Err(stop) = written {
        let _ = std::fs::remove_file(archive);
        return Err(stop);
    }
    if options.move_files {
        for source in sources {
            let removed = if source.is_dir() && !source.is_symlink() {
                std::fs::remove_dir_all(source)
            } else {
                std::fs::remove_file(source)
            };
            removed.map_err(|err| {
                OpEvent::Error(format!(
                    "the archive was created, but {} couldn't be deleted: {err}",
                    source.display()
                ))
            })?;
        }
    }
    Ok(())
}

/// Reports packing progress, entry by entry.
struct Progress<'a> {
    tx: &'a mpsc::Sender<OpEvent>,
    files_total: usize,
    bytes_total: u64,
    files_done: usize,
    bytes_done: u64,
}

impl<'a> Progress<'a> {
    fn new(entries: &[ArchiveEntry], tx: &'a mpsc::Sender<OpEvent>) -> Self {
        Self {
            tx,
            files_total: entries.len(),
            bytes_total: entries.iter().map(|entry| entry.size).sum(),
            files_done: 0,
            bytes_done: 0,
        }
    }

    fn done(&mut self, current: &Path, bytes: u64) {
        self.files_done += 1;
        self.bytes_done += bytes;
        let _ = self.tx.blocking_send(OpEvent::Progress(OpProgress {
            current_file: current.to_path_buf(),
            files_done: self.files_done,
            files_total: self.files_total,
            bytes_done: self.bytes_done,
            bytes_total: self.bytes_total,
        }));
    }
}

fn io_error(what: &Path, err: impl std::fmt::Display) -> OpEvent {
    OpEvent::Error(format!("failed to pack {}: {err}", what.display()))
}

/// Flushes a finished writer.
fn finish<W: Write>(writer: std::io::Result<W>) -> Result<(), OpEvent> {
    writer
        .and_then(|mut writer| writer.flush())
        .map_err(|err| OpEvent::Error(format!("failed to finish the archive: {err}")))
}

/// How many cores packing uses.
fn threads() -> usize {
    std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(16)
}

/// xz and 7z cut the data in blocks this big and compress them on all
/// cores at once (a little less compression than one long block).
const LZMA_BLOCK: u64 = 16 * 1024 * 1024;

fn xz_writer<W: Write>(inner: W) -> Result<lzma_rust2::XzWriterMt<W>, OpEvent> {
    let mut options = lzma_rust2::XzOptions::with_preset(6);
    options.set_block_size(NonZeroU64::new(LZMA_BLOCK));
    lzma_rust2::XzWriterMt::new(inner, options, threads() as u32)
        .map_err(|err| OpEvent::Error(format!("failed to start xz compression: {err}")))
}

/// gzip piece size: each piece is compressed on its own core.
const GZ_PIECE: usize = 1024 * 1024;

/// gzip on all cores, the way pigz does it: the data is cut in 1 MiB
/// pieces, each deflated on its own thread and ended with a sync flush, so
/// the pieces join into one ordinary gzip stream any tool can read.
struct ParallelGzWriter<W: Write> {
    inner: W,
    piece: Vec<u8>,
    crc: flate2::Crc,
    /// Pieces being compressed, oldest first: written in this order.
    pending: VecDeque<std::thread::JoinHandle<std::io::Result<Vec<u8>>>>,
    threads: usize,
}

impl<W: Write> ParallelGzWriter<W> {
    fn new(mut inner: W) -> std::io::Result<Self> {
        // Magic, deflate, no flags, no time, no extra flags, unknown OS.
        inner.write_all(&[0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 255])?;
        Ok(Self {
            inner,
            piece: Vec::with_capacity(GZ_PIECE),
            crc: flate2::Crc::new(),
            pending: VecDeque::new(),
            threads: threads(),
        })
    }

    fn send(&mut self, last: bool) -> std::io::Result<()> {
        while self.pending.len() >= self.threads {
            self.write_oldest()?;
        }
        let piece = std::mem::replace(&mut self.piece, Vec::with_capacity(GZ_PIECE));
        self.pending
            .push_back(std::thread::spawn(move || deflate_piece(&piece, last)));
        Ok(())
    }

    fn write_oldest(&mut self) -> std::io::Result<()> {
        if let Some(piece) = self.pending.pop_front() {
            let compressed = piece
                .join()
                .map_err(|_| std::io::Error::other("gzip compression stopped"))??;
            self.inner.write_all(&compressed)?;
        }
        Ok(())
    }

    fn finish(mut self) -> std::io::Result<W> {
        self.send(true)?;
        while !self.pending.is_empty() {
            self.write_oldest()?;
        }
        self.inner.write_all(&self.crc.sum().to_le_bytes())?;
        self.inner.write_all(&self.crc.amount().to_le_bytes())?;
        Ok(self.inner)
    }
}

impl<W: Write> Write for ParallelGzWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let take = buf.len().min(GZ_PIECE - self.piece.len());
        self.piece.extend_from_slice(&buf[..take]);
        self.crc.update(&buf[..take]);
        if self.piece.len() == GZ_PIECE {
            self.send(false)?;
        }
        Ok(take)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// One gzip piece as raw deflate: the last one ends the stream, the others
/// end on a byte boundary (sync flush) so the next piece can follow.
fn deflate_piece(input: &[u8], last: bool) -> std::io::Result<Vec<u8>> {
    use flate2::{Compress, Compression, FlushCompress, Status};
    let mut deflate = Compress::new(Compression::default(), false);
    let flush = if last {
        FlushCompress::Finish
    } else {
        FlushCompress::Sync
    };
    let mut out = Vec::with_capacity(input.len() / 2 + 4096);
    loop {
        let read = deflate.total_in() as usize;
        let status = deflate
            .compress_vec(&input[read..], &mut out, flush)
            .map_err(std::io::Error::other)?;
        let done = if last {
            status == Status::StreamEnd
        } else {
            deflate.total_in() as usize == input.len() && out.len() < out.capacity()
        };
        if done {
            return Ok(out);
        }
        out.reserve(out.capacity().max(4096));
    }
}

/// Zip files up to this size are compressed on all cores at once, each in
/// memory; bigger ones are streamed into the archive as before.
const ZIP_IN_MEMORY: u64 = 32 * 1024 * 1024;

/// A zip entry on its way into the archive, in archive order.
enum ZipJob<'a, 'scope> {
    /// Written by the main thread: folders, links, big files.
    Direct(&'a ArchiveEntry),
    /// Compressed on its own thread into a one-entry zip in memory.
    Packed(
        &'a ArchiveEntry,
        std::thread::ScopedJoinHandle<'scope, zip::result::ZipResult<Vec<u8>>>,
    ),
}

/// One file as a one-entry zip in memory, to be copied into the archive.
fn zip_in_memory(
    entry: &ArchiveEntry,
    options: zip::write::FileOptions<'_, ()>,
) -> zip::result::ZipResult<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file(entry.name.as_str(), options)?;
    std::io::copy(&mut File::open(&entry.src)?, &mut zip)?;
    Ok(zip.finish()?.into_inner())
}

fn write_zip(
    file: File,
    entries: &[ArchiveEntry],
    options: &PackOptions,
    cancel: &CancelHandle,
    progress: &mut Progress,
) -> Result<(), OpEvent> {
    let password = options.password.as_deref().filter(|p| !p.is_empty());
    let file_options = |entry: &ArchiveEntry| {
        let options = zip::write::FileOptions::<()>::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(entry.mode & 0o7777)
            .large_file(entry.size >= u32::MAX as u64);
        match password {
            Some(password) => options.with_aes_encryption(zip::AesMode::Aes256, password),
            None => options,
        }
    };
    let mut zip = zip::ZipWriter::new(BufWriter::new(file));
    let threads = threads();
    std::thread::scope(|scope| {
        let mut write = |job: ZipJob| -> Result<(), OpEvent> {
            let entry = match job {
                ZipJob::Direct(entry) => {
                    let options = file_options(entry);
                    match &entry.kind {
                        ArchiveEntryKind::Dir => zip
                            .add_directory(entry.name.as_str(), options)
                            .map_err(|err| io_error(&entry.src, err))?,
                        ArchiveEntryKind::Symlink(target) => zip
                            .add_symlink(entry.name.as_str(), target.to_string_lossy(), options)
                            .map_err(|err| io_error(&entry.src, err))?,
                        ArchiveEntryKind::File => {
                            zip.start_file(entry.name.as_str(), options)
                                .map_err(|err| io_error(&entry.src, err))?;
                            let mut src =
                                File::open(&entry.src).map_err(|err| io_error(&entry.src, err))?;
                            copy_cancellable(&mut src, &mut zip, cancel).map_err(
                                |stop| match stop {
                                    CopyStop::Cancelled => OpEvent::Cancelled,
                                    CopyStop::Io(err) => io_error(&entry.src, err),
                                },
                            )?;
                        }
                    }
                    entry
                }
                ZipJob::Packed(entry, packing) => {
                    let packed = packing
                        .join()
                        .map_err(|_| io_error(&entry.src, "compression stopped"))?
                        .map_err(|err| io_error(&entry.src, err))?;
                    let mut one = zip::ZipArchive::new(Cursor::new(packed))
                        .map_err(|err| io_error(&entry.src, err))?;
                    let raw = one
                        .by_index_raw(0)
                        .map_err(|err| io_error(&entry.src, err))?;
                    zip.raw_copy_file(raw)
                        .map_err(|err| io_error(&entry.src, err))?;
                    entry
                }
            };
            progress.done(&entry.src, entry.size);
            Ok(())
        };
        // Up to `threads` files are compressed at once while the oldest
        // one is written.
        let mut queue = VecDeque::new();
        for entry in entries {
            if cancel.is_cancelled() {
                return Err(OpEvent::Cancelled);
            }
            let job = if matches!(entry.kind, ArchiveEntryKind::File) && entry.size <= ZIP_IN_MEMORY
            {
                let options = file_options(entry);
                ZipJob::Packed(entry, scope.spawn(move || zip_in_memory(entry, options)))
            } else {
                ZipJob::Direct(entry)
            };
            queue.push_back(job);
            while queue.len() > threads {
                if let Some(job) = queue.pop_front() {
                    write(job)?;
                }
            }
        }
        while let Some(job) = queue.pop_front() {
            if cancel.is_cancelled() {
                return Err(OpEvent::Cancelled);
            }
            write(job)?;
        }
        Ok(())
    })?;
    let writer = zip
        .finish()
        .map_err(|err| OpEvent::Error(format!("failed to finish the archive: {err}")))?;
    finish(Ok(writer))
}

/// Writes the entries as a tar stream into `writer`, and returns it.
fn write_tar<W: Write>(
    writer: W,
    entries: &[ArchiveEntry],
    cancel: &CancelHandle,
    progress: &mut Progress,
) -> Result<W, OpEvent> {
    let mut tar = tar::Builder::new(writer);
    // Links are stored as links.
    tar.follow_symlinks(false);
    for entry in entries {
        if cancel.is_cancelled() {
            return Err(OpEvent::Cancelled);
        }
        tar.append_path_with_name(&entry.src, &entry.name)
            .map_err(|err| io_error(&entry.src, err))?;
        progress.done(&entry.src, entry.size);
    }
    tar.into_inner()
        .map_err(|err| OpEvent::Error(format!("failed to finish the archive: {err}")))
}

/// gz / xz: the one file's contents, compressed.
fn write_single<W: Write>(
    mut writer: W,
    entry: &ArchiveEntry,
    cancel: &CancelHandle,
    progress: &mut Progress,
) -> Result<W, OpEvent> {
    let mut src = File::open(&entry.src).map_err(|err| io_error(&entry.src, err))?;
    copy_cancellable(&mut src, &mut writer, cancel).map_err(|stop| match stop {
        CopyStop::Cancelled => OpEvent::Cancelled,
        CopyStop::Io(err) => io_error(&entry.src, err),
    })?;
    progress.done(&entry.src, entry.size);
    Ok(writer)
}

fn write_7z(
    file: File,
    entries: &[ArchiveEntry],
    options: &PackOptions,
    cancel: &CancelHandle,
    progress: &mut Progress,
) -> Result<(), OpEvent> {
    use sevenz_rust2::encoder_options::{AesEncoderOptions, Lzma2Options};
    use sevenz_rust2::{ArchiveEntry as SevenEntry, ArchiveWriter, EncoderConfiguration, Password};
    let error = |err: sevenz_rust2::Error| OpEvent::Error(format!("failed to pack: {err}"));
    let mut seven = ArchiveWriter::new(file).map_err(error)?;
    // LZMA2 on all cores (for files bigger than one block).
    let lzma2: EncoderConfiguration =
        Lzma2Options::from_level_mt(6, threads() as u32, LZMA_BLOCK).into();
    match options.password.as_deref().filter(|p| !p.is_empty()) {
        Some(password) => seven.set_content_methods(vec![
            AesEncoderOptions::new(Password::new(password)).into(),
            lzma2,
        ]),
        None => seven.set_content_methods(vec![lzma2]),
    };
    for entry in entries {
        if cancel.is_cancelled() {
            return Err(OpEvent::Cancelled);
        }
        match &entry.kind {
            ArchiveEntryKind::Dir => {
                seven
                    .push_archive_entry::<&[u8]>(SevenEntry::new_directory(&entry.name), None)
                    .map_err(error)?;
            }
            ArchiveEntryKind::File => {
                let src = File::open(&entry.src).map_err(|err| io_error(&entry.src, err))?;
                seven
                    .push_archive_entry(
                        SevenEntry::from_path(&entry.src, entry.name.clone()),
                        Some(BufReader::new(src)),
                    )
                    .map_err(error)?;
            }
            // 7z has no portable way to store a link: left out.
            ArchiveEntryKind::Symlink(_) => {}
        }
        progress.done(&entry.src, entry.size);
    }
    seven
        .finish()
        .map_err(|err| OpEvent::Error(format!("failed to finish the archive: {err}")))?;
    Ok(())
}

enum CopyStop {
    Cancelled,
    Io(std::io::Error),
}

/// Copies in blocks, stopping when the operation is cancelled.
fn copy_cancellable(
    reader: &mut dyn Read,
    writer: &mut dyn Write,
    cancel: &CancelHandle,
) -> Result<u64, CopyStop> {
    let mut buffer = vec![0u8; 256 * 1024];
    let mut total = 0u64;
    loop {
        if cancel.is_cancelled() {
            return Err(CopyStop::Cancelled);
        }
        let read = reader.read(&mut buffer).map_err(CopyStop::Io)?;
        if read == 0 {
            return Ok(total);
        }
        writer.write_all(&buffer[..read]).map_err(CopyStop::Io)?;
        total += read as u64;
    }
}

/// Unpacks each of `archives` into `dest`. Existing files are replaced with
/// `overwrite`; otherwise the UI is asked (Replace, Skip, … all, Rename).
pub fn unpack(
    archives: Vec<PathBuf>,
    dest: PathBuf,
    options: UnpackOptions,
    cancel: CancelHandle,
    conflict: ConflictHandle,
) -> impl Stream<Item = OpEvent> {
    let (tx, rx) = mpsc::channel(16);
    if archives
        .iter()
        .chain([&dest])
        .any(|path| crate::vfs::is_remote(path))
    {
        let _ = tx.try_send(OpEvent::Error(
            "unpacking on a server isn't supported yet; copy the archive here first".to_string(),
        ));
        return ReceiverStream::new(rx);
    }
    tokio::task::spawn_blocking(move || {
        let mut unpacker = Unpacker {
            dest,
            options,
            cancel,
            conflict,
            tx: tx.clone(),
            replace_all: false,
            skip_all: false,
            files_done: 0,
            bytes_done: 0,
            files_total: archives.len(),
            bytes_total: archives
                .iter()
                .filter_map(|archive| archive.metadata().ok())
                .map(|metadata| metadata.len())
                .sum(),
            select: None,
        };
        let mut result = Ok(());
        for archive in &archives {
            result = unpacker.unpack(archive);
            if result.is_err() {
                break;
            }
            unpacker.files_done += 1;
            unpacker.bytes_done += archive.metadata().map_or(0, |metadata| metadata.len());
            unpacker.report(archive);
        }
        let event = match result {
            Ok(()) => OpEvent::Done,
            Err(stop) => stop,
        };
        let _ = tx.blocking_send(event);
    });
    ReceiverStream::new(rx)
}

struct Unpacker {
    dest: PathBuf,
    options: UnpackOptions,
    cancel: CancelHandle,
    conflict: ConflictHandle,
    tx: mpsc::Sender<OpEvent>,
    replace_all: bool,
    skip_all: bool,
    files_done: usize,
    bytes_done: u64,
    files_total: usize,
    bytes_total: u64,
    /// Only these paths inside the archive (and what is in them), placed
    /// relative to their own folder; `None` for everything.
    select: Option<Vec<PathBuf>>,
}

/// What to do with one entry of an archive.
enum Place {
    Write(PathBuf),
    Skip,
}

impl Unpacker {
    fn report(&self, current: &Path) {
        let _ = self.tx.blocking_send(OpEvent::Progress(OpProgress {
            current_file: current.to_path_buf(),
            files_done: self.files_done,
            files_total: self.files_total,
            bytes_done: self.bytes_done,
            bytes_total: self.bytes_total,
        }));
    }

    fn unpack(&mut self, archive: &Path) -> Result<(), OpEvent> {
        let format = ArchiveFormat::of(archive).ok_or_else(|| {
            OpEvent::Error(format!(
                "{} isn't an archive 2fip can unpack",
                archive.display()
            ))
        })?;
        let open = || {
            File::open(archive).map(BufReader::new).map_err(|err| {
                OpEvent::Error(format!("failed to open {}: {err}", archive.display()))
            })
        };
        match format {
            ArchiveFormat::Zip => self.unpack_zip(archive),
            ArchiveFormat::Tar => self.unpack_tar(archive, open()?),
            ArchiveFormat::Tgz => self.unpack_tar(archive, flate2::read::GzDecoder::new(open()?)),
            ArchiveFormat::Txz => {
                self.unpack_tar(archive, lzma_rust2::XzReader::new(open()?, true))
            }
            ArchiveFormat::Gz => self.unpack_single(archive, flate2::read::GzDecoder::new(open()?)),
            ArchiveFormat::Xz => {
                self.unpack_single(archive, lzma_rust2::XzReader::new(open()?, true))
            }
            ArchiveFormat::SevenZ => self.unpack_7z(archive),
        }
    }

    /// Where an entry named `name` (a path inside the archive) goes, asking
    /// about existing files. `None` for names that would leave the
    /// destination (`../x`, `/etc/x`).
    fn place(&mut self, archive: &Path, name: &Path, is_dir: bool) -> Result<Place, OpEvent> {
        if self.cancel.is_cancelled() {
            return Err(OpEvent::Cancelled);
        }
        let mut relative = PathBuf::new();
        for component in name.components() {
            match component {
                Component::Normal(part) => relative.push(part),
                Component::CurDir => {}
                // Unsafe names are left out.
                _ => return Ok(Place::Skip),
            }
        }
        if relative.as_os_str().is_empty() {
            return Ok(Place::Skip);
        }
        // Copying out of an archive: only the chosen entries, placed like
        // files copied from that folder.
        if let Some(select) = &self.select {
            let Some(chosen) = select.iter().find(|chosen| relative.starts_with(chosen)) else {
                return Ok(Place::Skip);
            };
            let base = chosen.parent().unwrap_or(Path::new(""));
            relative = relative
                .strip_prefix(base)
                .unwrap_or(&relative)
                .to_path_buf();
        }
        if !self.options.keep_paths {
            if is_dir {
                return Ok(Place::Skip);
            }
            relative = PathBuf::from(relative.file_name().unwrap_or_default());
        }
        let mut target = self.dest.join(&relative);
        if is_dir || !target.exists() || self.options.overwrite || self.replace_all {
            return Ok(Place::Write(target));
        }
        if self.skip_all {
            return Ok(Place::Skip);
        }
        let _ = self.tx.blocking_send(OpEvent::Conflict {
            src: archive.join(&relative),
            dest: target.clone(),
        });
        match self.conflict.ask_blocking() {
            ConflictResolution::Replace => {}
            ConflictResolution::ReplaceAll => self.replace_all = true,
            ConflictResolution::Skip => return Ok(Place::Skip),
            ConflictResolution::SkipAll => {
                self.skip_all = true;
                return Ok(Place::Skip);
            }
            ConflictResolution::Rename(name) => target.set_file_name(name),
        }
        if self.cancel.is_cancelled() {
            return Err(OpEvent::Cancelled);
        }
        Ok(Place::Write(target))
    }

    /// Writes one file from `reader` to `target` (folders created as needed).
    fn write_file(
        &self,
        target: &Path,
        reader: &mut dyn Read,
        mode: Option<u32>,
    ) -> Result<(), OpEvent> {
        let error = |err: &dyn std::fmt::Display| {
            OpEvent::Error(format!("failed to unpack {}: {err}", target.display()))
        };
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|err| error(&err))?;
        }
        let mut out = BufWriter::new(File::create(target).map_err(|err| error(&err))?);
        copy_cancellable(reader, &mut out, &self.cancel).map_err(|stop| match stop {
            CopyStop::Cancelled => OpEvent::Cancelled,
            CopyStop::Io(err) => error(&err),
        })?;
        out.flush().map_err(|err| error(&err))?;
        set_mode(target, mode);
        Ok(())
    }

    fn unpack_zip(&mut self, archive: &Path) -> Result<(), OpEvent> {
        let file = File::open(archive).map_err(|err| {
            OpEvent::Error(format!("failed to open {}: {err}", archive.display()))
        })?;
        let mut zip = zip::ZipArchive::new(BufReader::new(file))
            .map_err(|err| OpEvent::Error(format!("{}: {err}", archive.display())))?;
        for index in 0..zip.len() {
            let password = self.options.password.clone().filter(|p| !p.is_empty());
            let mut entry = match &password {
                Some(password) => zip.by_index_decrypt(index, password.as_bytes()),
                None => zip.by_index(index),
            }
            .map_err(|err| password_error(archive, &err.to_string(), password.is_some()))?;
            let Some(name) = entry.enclosed_name() else {
                continue;
            };
            let is_dir = entry.is_dir();
            let mode = entry.unix_mode();
            match self.place(archive, &name, is_dir)? {
                Place::Skip => {}
                Place::Write(target) if is_dir => {
                    std::fs::create_dir_all(&target).map_err(|err| {
                        OpEvent::Error(format!("failed to create {}: {err}", target.display()))
                    })?;
                }
                Place::Write(target) => self.write_file(&target, &mut entry, mode)?,
            }
            self.report(&name);
        }
        Ok(())
    }

    fn unpack_tar(&mut self, archive: &Path, reader: impl Read) -> Result<(), OpEvent> {
        let error = |err: std::io::Error| OpEvent::Error(format!("{}: {err}", archive.display()));
        let mut tar = tar::Archive::new(reader);
        for entry in tar.entries().map_err(error)? {
            let mut entry = entry.map_err(error)?;
            let name = entry.path().map_err(error)?.into_owned();
            let kind = entry.header().entry_type();
            let mode = entry.header().mode().ok();
            match self.place(archive, &name, kind.is_dir())? {
                Place::Skip => {}
                Place::Write(target) if kind.is_dir() => {
                    std::fs::create_dir_all(&target).map_err(error)?;
                }
                Place::Write(target) if kind.is_symlink() => {
                    if let Some(link) = entry.link_name().map_err(error)? {
                        if let Some(parent) = target.parent() {
                            std::fs::create_dir_all(parent).map_err(error)?;
                        }
                        let _ = std::fs::remove_file(&target);
                        make_symlink(&link, &target).map_err(error)?;
                    }
                }
                Place::Write(target) if kind.is_file() => {
                    self.write_file(&target, &mut entry, mode)?;
                }
                // Hard links, devices, …: left out.
                Place::Write(_) => {}
            }
            self.report(&name);
        }
        Ok(())
    }

    /// gz / xz: one file, named like the archive without its extension.
    fn unpack_single(&mut self, archive: &Path, mut reader: impl Read) -> Result<(), OpEvent> {
        let name = archive
            .file_stem()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("unpacked"));
        if let Place::Write(target) = self.place(archive, &name, false)? {
            self.write_file(&target, &mut reader, None)?;
        }
        Ok(())
    }

    fn unpack_7z(&mut self, archive: &Path) -> Result<(), OpEvent> {
        use sevenz_rust2::{ArchiveReader, Password};
        let password = self.options.password.clone().filter(|p| !p.is_empty());
        let mut seven = ArchiveReader::open(
            archive,
            password
                .as_deref()
                .map_or_else(Password::empty, Password::new),
        )
        .map_err(|err| password_error(archive, &err.to_string(), password.is_some()))?;
        // The callback can't return our errors: keep the first one.
        let mut stop: Option<OpEvent> = None;
        let result = seven.for_each_entries(|entry, reader| {
            let name = PathBuf::from(entry.name());
            let is_dir = entry.is_directory();
            let outcome = self
                .place(archive, &name, is_dir)
                .and_then(|place| match place {
                    Place::Skip => Ok(()),
                    Place::Write(target) if is_dir => {
                        std::fs::create_dir_all(&target).map_err(|err| {
                            OpEvent::Error(format!("failed to create {}: {err}", target.display()))
                        })
                    }
                    Place::Write(target) => self.write_file(&target, reader, None),
                });
            self.report(&name);
            match outcome {
                Ok(()) => Ok(true),
                Err(event) => {
                    stop = Some(event);
                    Ok(false)
                }
            }
        });
        if let Some(stop) = stop {
            return Err(stop);
        }
        result.map_err(|err| password_error(archive, &err.to_string(), password.is_some()))
    }
}

/// Copies (or, with `move_files`, moves) files and folders from inside
/// archives (`/a/b.zip/docs/x.txt`) into `dest`. Moving removes them from
/// the archive by writing it again.
pub fn copy_out(
    sources: Vec<PathBuf>,
    dest: PathBuf,
    move_files: bool,
    cancel: CancelHandle,
    conflict: ConflictHandle,
) -> ReceiverStream<OpEvent> {
    let (tx, rx) = mpsc::channel(16);
    tokio::task::spawn_blocking(move || {
        let result = copy_out_blocking(&sources, &dest, move_files, &cancel, &conflict, &tx);
        let _ = tx.blocking_send(result.err().unwrap_or(OpEvent::Done));
    });
    ReceiverStream::new(rx)
}

/// The sources grouped by archive: archive → paths inside it.
fn by_archive(sources: &[PathBuf]) -> Result<Vec<(PathBuf, Vec<PathBuf>)>, OpEvent> {
    let mut groups: Vec<(PathBuf, Vec<PathBuf>)> = Vec::new();
    for source in sources {
        let (archive, inner) = crate::archive_fs::split(source).ok_or_else(|| {
            OpEvent::Error(format!("{} is not inside an archive", source.display()))
        })?;
        match groups.iter_mut().find(|(known, _)| *known == archive) {
            Some((_, inners)) => inners.push(inner),
            None => groups.push((archive, vec![inner])),
        }
    }
    Ok(groups)
}

fn copy_out_blocking(
    sources: &[PathBuf],
    dest: &Path,
    move_files: bool,
    cancel: &CancelHandle,
    conflict: &ConflictHandle,
    tx: &mpsc::Sender<OpEvent>,
) -> Result<(), OpEvent> {
    let groups = by_archive(sources)?;
    for (archive, inners) in &groups {
        let mut unpacker = Unpacker {
            dest: dest.to_path_buf(),
            options: UnpackOptions {
                keep_paths: true,
                overwrite: false,
                password: None,
            },
            cancel: cancel.clone(),
            conflict: conflict.clone(),
            tx: tx.clone(),
            replace_all: false,
            skip_all: false,
            files_done: 0,
            bytes_done: 0,
            files_total: inners.len(),
            bytes_total: 0,
            select: Some(inners.clone()),
        };
        unpacker.unpack(archive)?;
        if move_files {
            remove_from_archive(archive, inners, cancel, tx)?;
        }
    }
    Ok(())
}

/// Deletes files and folders inside archives (writing each archive again
/// without them).
pub fn delete_in_archive(sources: Vec<PathBuf>, cancel: CancelHandle) -> ReceiverStream<OpEvent> {
    let (tx, rx) = mpsc::channel(16);
    tokio::task::spawn_blocking(move || {
        let result = by_archive(&sources).and_then(|groups| {
            groups.iter().try_for_each(|(archive, inners)| {
                remove_from_archive(archive, inners, &cancel, &tx)
            })
        });
        let _ = tx.blocking_send(result.err().unwrap_or(OpEvent::Done));
    });
    ReceiverStream::new(rx)
}

/// Writes `archive` again without `inners` (paths inside it): unpacks it
/// next to itself, deletes them, packs the rest in the same format and puts
/// the new archive in place of the old one. Encrypted archives can't be
/// changed this way.
fn remove_from_archive(
    archive: &Path,
    inners: &[PathBuf],
    cancel: &CancelHandle,
    tx: &mpsc::Sender<OpEvent>,
) -> Result<(), OpEvent> {
    let format = ArchiveFormat::of(archive)
        .ok_or_else(|| OpEvent::Error(format!("{} is not an archive", archive.display())))?;
    let folder = archive.parent().unwrap_or(Path::new("."));
    let work = folder.join(format!(
        ".2fip-rewrite-{}-{}",
        std::process::id(),
        archive.file_name().unwrap_or_default().to_string_lossy()
    ));
    let result = (|| {
        let content = work.join("content");
        std::fs::create_dir_all(&content).map_err(|err| {
            OpEvent::Error(format!("failed to create {}: {err}", content.display()))
        })?;
        let mut unpacker = Unpacker {
            dest: content.clone(),
            options: UnpackOptions {
                keep_paths: true,
                overwrite: true,
                password: None,
            },
            cancel: cancel.clone(),
            conflict: ConflictHandle::new(),
            tx: tx.clone(),
            replace_all: true,
            skip_all: false,
            files_done: 0,
            bytes_done: 0,
            files_total: 1,
            bytes_total: 0,
            select: None,
        };
        unpacker.unpack(archive).map_err(|stop| match stop {
            OpEvent::Error(err) => OpEvent::Error(format!(
                "{} couldn't be changed (encrypted archives can't be): {err}",
                archive.display()
            )),
            other => other,
        })?;
        for inner in inners {
            let path = content.join(inner);
            let removed = if path.is_dir() {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };
            removed.map_err(|err| {
                OpEvent::Error(format!("failed to remove {}: {err}", inner.display()))
            })?;
        }
        let rest: Vec<PathBuf> = std::fs::read_dir(&content)
            .map_err(|err| OpEvent::Error(format!("failed to read {}: {err}", content.display())))?
            .flatten()
            .map(|entry| entry.path())
            .collect();
        let rewritten = work.join(format!("new.{}", format.extension()));
        let options = PackOptions {
            format,
            password: None,
            move_files: false,
        };
        pack_blocking(&rest, &rewritten, &options, cancel, tx)?;
        std::fs::rename(&rewritten, archive).map_err(|err| {
            OpEvent::Error(format!("failed to replace {}: {err}", archive.display()))
        })
    })();
    let _ = std::fs::remove_dir_all(&work);
    result
}

/// Extracts one file from inside an archive into 2fip's cache, to open it
/// with an app, and returns where. Blocking.
pub fn extract_to_cache(path: &Path) -> Result<PathBuf, String> {
    let (archive, inner) = crate::archive_fs::split(path)
        .ok_or_else(|| format!("{} is not inside an archive", path.display()))?;
    let key = format!("{:x}", md5::compute(archive.to_string_lossy().as_bytes()));
    let folder = crate::sandbox::cache_home()
        .join("2fip/archives")
        .join(key)
        .join(inner.parent().unwrap_or(Path::new("")));
    // Nobody listens to its progress: a closed channel drops it.
    let (tx, _) = mpsc::channel(1);
    let mut unpacker = Unpacker {
        dest: folder.clone(),
        options: UnpackOptions {
            keep_paths: true,
            overwrite: true,
            password: None,
        },
        cancel: CancelHandle::new(),
        conflict: ConflictHandle::new(),
        tx,
        replace_all: true,
        skip_all: false,
        files_done: 0,
        bytes_done: 0,
        files_total: 1,
        bytes_total: 0,
        select: Some(vec![inner.clone()]),
    };
    unpacker.unpack(&archive).map_err(|stop| match stop {
        OpEvent::Error(err) => err,
        other => format!("{other:?}"),
    })?;
    Ok(folder.join(inner.file_name().unwrap_or_default()))
}

/// An error opening or reading an archive, explaining password problems.
fn password_error(archive: &Path, err: &str, had_password: bool) -> OpEvent {
    let lower = err.to_lowercase();
    if lower.contains("password") || lower.contains("encrypt") || lower.contains("decrypt") {
        return OpEvent::Error(if had_password {
            format!("{}: wrong password", archive.display())
        } else {
            format!("{} is encrypted: enter its password", archive.display())
        });
    }
    OpEvent::Error(format!("{}: {err}", archive.display()))
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: Option<u32>) {
    use std::os::unix::fs::PermissionsExt;
    if let Some(mode) = mode.map(|mode| mode & 0o7777).filter(|mode| *mode != 0) {
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
    }
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: Option<u32>) {}

#[cfg(unix)]
fn make_symlink(link: &Path, target: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(link, target)
}

#[cfg(not(unix))]
fn make_symlink(_link: &Path, _target: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::StreamExt;

    async fn collect(stream: impl Stream<Item = OpEvent>) -> Vec<OpEvent> {
        let mut stream = std::pin::pin!(stream);
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }
        events
    }

    fn sample(dir: &Path) -> Vec<PathBuf> {
        std::fs::write(dir.join("a.txt"), b"alpha").unwrap();
        let project = dir.join("project");
        std::fs::create_dir_all(project.join("sub")).unwrap();
        std::fs::write(project.join("sub").join("b.txt"), b"beta").unwrap();
        vec![dir.join("a.txt"), project]
    }

    async fn round_trip(format: ArchiveFormat, password: Option<&str>) {
        let dir = tempfile::tempdir().unwrap();
        let sources = sample(dir.path());
        let archive = dir.path().join(format!("out.{}", format.extension()));
        let options = PackOptions {
            format,
            password: password.map(str::to_string),
            move_files: false,
        };
        let events = collect(pack(sources, archive.clone(), options, CancelHandle::new())).await;
        assert!(
            matches!(events.last(), Some(OpEvent::Done)),
            "{format:?}: {events:?}"
        );

        let dest = dir.path().join("dest");
        let unpack_options = UnpackOptions {
            keep_paths: true,
            overwrite: false,
            password: password.map(str::to_string),
        };
        let events = collect(unpack(
            vec![archive],
            dest.clone(),
            unpack_options,
            CancelHandle::new(),
            ConflictHandle::new(),
        ))
        .await;
        assert!(
            matches!(events.last(), Some(OpEvent::Done)),
            "{format:?}: {events:?}"
        );
        assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"alpha");
        assert_eq!(
            std::fs::read(dest.join("project/sub/b.txt")).unwrap(),
            b"beta"
        );
    }

    #[tokio::test]
    async fn every_multi_file_format_round_trips() {
        for format in [
            ArchiveFormat::Zip,
            ArchiveFormat::Tar,
            ArchiveFormat::Tgz,
            ArchiveFormat::Txz,
            ArchiveFormat::SevenZ,
        ] {
            round_trip(format, None).await;
        }
    }

    #[tokio::test]
    async fn encrypted_zip_and_7z_round_trip() {
        round_trip(ArchiveFormat::Zip, Some("secret")).await;
        round_trip(ArchiveFormat::SevenZ, Some("secret")).await;
    }

    #[tokio::test]
    async fn single_file_formats_round_trip_and_refuse_folders() {
        for format in [ArchiveFormat::Gz, ArchiveFormat::Xz] {
            let dir = tempfile::tempdir().unwrap();
            let sources = sample(dir.path());
            let archive = dir.path().join(format!("a.txt.{}", format.extension()));
            let options = PackOptions {
                format,
                password: None,
                move_files: false,
            };
            // A folder can't go into gz / xz.
            let events = collect(pack(
                sources.clone(),
                archive.clone(),
                options.clone(),
                CancelHandle::new(),
            ))
            .await;
            assert!(matches!(events.last(), Some(OpEvent::Error(_))));
            let events = collect(pack(
                vec![sources[0].clone()],
                archive.clone(),
                options,
                CancelHandle::new(),
            ))
            .await;
            assert!(matches!(events.last(), Some(OpEvent::Done)), "{events:?}");

            let dest = dir.path().join("dest");
            let unpack_options = UnpackOptions {
                keep_paths: true,
                overwrite: false,
                password: None,
            };
            let events = collect(unpack(
                vec![archive],
                dest.clone(),
                unpack_options,
                CancelHandle::new(),
                ConflictHandle::new(),
            ))
            .await;
            assert!(matches!(events.last(), Some(OpEvent::Done)), "{events:?}");
            assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"alpha");
        }
    }

    #[tokio::test]
    async fn move_to_archive_deletes_the_originals() {
        let dir = tempfile::tempdir().unwrap();
        let sources = sample(dir.path());
        let archive = dir.path().join("out.zip");
        let options = PackOptions {
            format: ArchiveFormat::Zip,
            password: None,
            move_files: true,
        };
        let events = collect(pack(
            sources.clone(),
            archive.clone(),
            options,
            CancelHandle::new(),
        ))
        .await;
        assert!(matches!(events.last(), Some(OpEvent::Done)), "{events:?}");
        assert!(archive.exists());
        assert!(sources.iter().all(|source| !source.exists()));
    }

    #[test]
    fn parallel_gzip_is_one_ordinary_gzip_stream() {
        // Several pieces, the last one partial, of mixed compressible data.
        let data: Vec<u8> = (0..(GZ_PIECE * 3 + 12345) as u32)
            .map(|i| {
                (i.wrapping_mul(2654435761) >> 24) as u8 & if i % 7 == 0 { 0xff } else { 0x0f }
            })
            .collect();
        let mut gz = ParallelGzWriter::new(Vec::new()).unwrap();
        for part in data.chunks(100_000) {
            gz.write_all(part).unwrap();
        }
        let compressed = gz.finish().unwrap();
        // A single-member decoder reads it all.
        let mut back = Vec::new();
        flate2::read::GzDecoder::new(compressed.as_slice())
            .read_to_end(&mut back)
            .unwrap();
        assert_eq!(back, data);
    }

    #[tokio::test]
    async fn without_paths_everything_lands_in_the_destination() {
        let dir = tempfile::tempdir().unwrap();
        let sources = sample(dir.path());
        let archive = dir.path().join("out.tgz");
        let options = PackOptions {
            format: ArchiveFormat::Tgz,
            password: None,
            move_files: false,
        };
        collect(pack(sources, archive.clone(), options, CancelHandle::new())).await;
        let dest = dir.path().join("flat");
        let unpack_options = UnpackOptions {
            keep_paths: false,
            overwrite: false,
            password: None,
        };
        let events = collect(unpack(
            vec![archive],
            dest.clone(),
            unpack_options,
            CancelHandle::new(),
            ConflictHandle::new(),
        ))
        .await;
        assert!(matches!(events.last(), Some(OpEvent::Done)), "{events:?}");
        assert!(dest.join("b.txt").exists());
        assert!(!dest.join("project").exists());
    }

    #[tokio::test]
    async fn copies_and_moves_out_of_an_archive() {
        let dir = tempfile::tempdir().unwrap();
        let sources = sample(dir.path());
        let archive = dir.path().join("box.zip");
        let options = PackOptions {
            format: ArchiveFormat::Zip,
            password: None,
            move_files: false,
        };
        collect(pack(sources, archive.clone(), options, CancelHandle::new())).await;
        let dest = dir.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        // Copy a folder from inside: it lands with its own name.
        let events = collect(copy_out(
            vec![archive.join("project/sub")],
            dest.clone(),
            false,
            CancelHandle::new(),
            ConflictHandle::new(),
        ))
        .await;
        assert!(matches!(events.last(), Some(OpEvent::Done)), "{events:?}");
        assert_eq!(std::fs::read(dest.join("sub/b.txt")).unwrap(), b"beta");

        // Move a file out: it leaves the archive.
        let events = collect(copy_out(
            vec![archive.join("a.txt")],
            dest.clone(),
            true,
            CancelHandle::new(),
            ConflictHandle::new(),
        ))
        .await;
        assert!(matches!(events.last(), Some(OpEvent::Done)), "{events:?}");
        assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"alpha");
        let left = crate::archive_fs::list(&archive).await.unwrap();
        let names: Vec<&str> = left.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, vec!["project"]);

        // Opening a file inside extracts it to the cache.
        // (Blocking: off the async runtime, as the app does it.)
        let inside = archive.join("project/sub/b.txt");
        let opened = tokio::task::spawn_blocking(move || extract_to_cache(&inside))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(std::fs::read(opened).unwrap(), b"beta");
    }

    #[tokio::test]
    async fn deletes_inside_an_archive() {
        let dir = tempfile::tempdir().unwrap();
        let sources = sample(dir.path());
        let archive = dir.path().join("box.tgz");
        let options = PackOptions {
            format: ArchiveFormat::Tgz,
            password: None,
            move_files: false,
        };
        collect(pack(sources, archive.clone(), options, CancelHandle::new())).await;
        let events = collect(delete_in_archive(
            vec![archive.join("project")],
            CancelHandle::new(),
        ))
        .await;
        assert!(matches!(events.last(), Some(OpEvent::Done)), "{events:?}");
        let left = crate::archive_fs::list(&archive).await.unwrap();
        let names: Vec<&str> = left.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, vec!["a.txt"]);
    }

    #[test]
    fn formats_come_from_file_names() {
        assert_eq!(
            ArchiveFormat::of(Path::new("x.tar.gz")),
            Some(ArchiveFormat::Tgz)
        );
        assert_eq!(
            ArchiveFormat::of(Path::new("X.TXZ")),
            Some(ArchiveFormat::Txz)
        );
        assert_eq!(
            ArchiveFormat::of(Path::new("notes.gz")),
            Some(ArchiveFormat::Gz)
        );
        assert_eq!(
            ArchiveFormat::of(Path::new("a.7z")),
            Some(ArchiveFormat::SevenZ)
        );
        assert_eq!(ArchiveFormat::of(Path::new("a.txt")), None);
    }
}
