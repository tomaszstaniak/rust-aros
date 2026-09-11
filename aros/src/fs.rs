//! Files, on top of dos.library.
//!
//! Thin, but not a straight pass-through, because three of dos.library's
//! habits will bite anything that treats it like POSIX:
//!
//! * **`Read` and `Write` are allowed to do less than you asked.** Both
//!   return a count, and a count short of the length is neither an error nor
//!   the end of the file. Everything here loops.
//! * **`Rename` refuses an existing target.** Measured on AROS One's `AROS:`
//!   volume (SFS-BE, 2026-09-10): `Can't rename :t/a as :t/b because object
//!   already exists`, and the failed rename changed nothing. There is no
//!   atomic replace to build on, so [`replace`] does the dance by hand.
//! * **A handle is a `BPTR`**, opaque, and zero means failure rather than
//!   being a usable handle.

use core::fmt;

use crate::sys;

/// What went wrong, with dos.library's own code where it has one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Error {
    pub what: What,
    /// `IoErr()` at the time. Zero when there was nothing to ask.
    pub code: i64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum What {
    Open,
    Read,
    Write,
    Close,
    Rename,
    Delete,
    CreateDir,
    /// The path is longer than [`MAX_PATH`], or holds a zero byte.
    BadPath,
    /// The file is larger than the caller allowed for.
    TooLarge,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} failed", self.what)?;
        if self.code != 0 {
            write!(f, " (dos error {})", self.code)?;
        }
        Ok(())
    }
}

fn err(what: What) -> Error {
    Error { what, code: unsafe { sys::IoErr() } }
}

/// Long enough for `Volume:dir/dir/name` and then some. Paths are built on
/// the stack so that this module needs no allocator.
pub const MAX_PATH: usize = 512;

/// A path with the zero byte dos.library expects.
struct CPath {
    bytes: [u8; MAX_PATH],
}

impl CPath {
    fn new(path: &str) -> Result<CPath, Error> {
        let raw = path.as_bytes();
        if raw.is_empty() || raw.len() >= MAX_PATH || raw.contains(&0) {
            return Err(Error { what: What::BadPath, code: 0 });
        }
        let mut bytes = [0u8; MAX_PATH];
        bytes[..raw.len()].copy_from_slice(raw);
        Ok(CPath { bytes })
    }

    fn as_ptr(&self) -> *const u8 {
        self.bytes.as_ptr()
    }
}

/// An open file that closes itself, and says so when closing fails.
///
/// **Closing is not a formality for a file being written.** It is where a
/// handler can still report that the data did not land, which is why
/// [`write_new`] checks it rather than dropping the handle.
pub struct File(i64);

impl File {
    fn open(path: &CPath, mode: i32) -> Result<File, Error> {
        let handle = unsafe { sys::Open(path.as_ptr(), mode) };
        if handle == 0 {
            return Err(err(What::Open));
        }
        Ok(File(handle))
    }

    fn close(self) -> Result<(), Error> {
        let handle = self.0;
        core::mem::forget(self);
        if unsafe { sys::Close(handle) } == 0 {
            return Err(err(What::Close));
        }
        Ok(())
    }
}

impl Drop for File {
    fn drop(&mut self) {
        // Only reached on an error path, where the close result has nowhere
        // useful to go — the error that got us here is the one to report.
        unsafe { sys::Close(self.0) };
    }
}

/// Read a whole file into `buffer`, returning how much of it was used.
///
/// A file that does not fit is [`What::TooLarge`] rather than a truncated
/// read, because a truncated save is indistinguishable from a corrupt one
/// and the caller would have no way to tell.
pub fn read_into(path: &str, buffer: &mut [u8]) -> Result<usize, Error> {
    let path = CPath::new(path)?;
    let file = File::open(&path, sys::MODE_OLDFILE)?;
    let mut used = 0;
    loop {
        if used == buffer.len() {
            // One more byte would mean the file is larger than we allow.
            let mut probe = [0u8; 1];
            let n = unsafe { sys::Read(file.0, probe.as_mut_ptr(), 1) };
            if n < 0 {
                return Err(err(What::Read));
            }
            if n > 0 {
                return Err(Error { what: What::TooLarge, code: 0 });
            }
            break;
        }
        let want = (buffer.len() - used) as i64;
        let n = unsafe { sys::Read(file.0, buffer[used..].as_mut_ptr(), want) };
        if n < 0 {
            return Err(err(What::Read));
        }
        if n == 0 {
            break; // end of file
        }
        used += n as usize;
    }
    file.close()?;
    Ok(used)
}

/// Write a file from nothing, replacing whatever was there.
///
/// Every partial write is looped on and the close is checked. Use it for a
/// temporary; [`replace`] is what puts a finished file in place.
pub fn write_new(path: &str, data: &[u8]) -> Result<(), Error> {
    let path = CPath::new(path)?;
    let file = File::open(&path, sys::MODE_NEWFILE)?;
    let mut done = 0;
    while done < data.len() {
        let n =
            unsafe { sys::Write(file.0, data[done..].as_ptr(), (data.len() - done) as i64) };
        if n <= 0 {
            return Err(err(What::Write));
        }
        done += n as usize;
    }
    file.close()
}

pub fn exists(path: &str) -> bool {
    let Ok(path) = CPath::new(path) else { return false };
    match File::open(&path, sys::MODE_OLDFILE) {
        Ok(file) => {
            let _ = file.close();
            true
        }
        Err(_) => false,
    }
}

pub fn delete(path: &str) -> Result<(), Error> {
    let path = CPath::new(path)?;
    if unsafe { sys::DeleteFile(path.as_ptr()) } == 0 {
        return Err(err(What::Delete));
    }
    Ok(())
}

pub fn rename(from: &str, to: &str) -> Result<(), Error> {
    let from = CPath::new(from)?;
    let to = CPath::new(to)?;
    if unsafe { sys::Rename(from.as_ptr(), to.as_ptr()) } == 0 {
        return Err(err(What::Rename));
    }
    Ok(())
}

/// Make a directory. An existing one is not an error.
pub fn create_dir(path: &str) -> Result<(), Error> {
    if exists_dir(path) {
        return Ok(());
    }
    let cpath = CPath::new(path)?;
    let lock = unsafe { sys::CreateDir(cpath.as_ptr()) };
    if lock == 0 {
        // It may have appeared between the two calls, which is not a failure.
        if exists_dir(path) {
            return Ok(());
        }
        return Err(err(What::CreateDir));
    }
    // `CreateDir` hands back a lock, and dropping it on the floor would leak
    // it. `UnLock` is the pair; it is reached through the same glue as the
    // rest of dos.library.
    unsafe { sys::UnLock(lock) };
    Ok(())
}

/// Make a directory and every directory above it.
///
/// **`CreateDir` makes one level only.** Handing it `AROS:Burntime/saves`
/// when `AROS:Burntime` does not exist fails with dos error 205,
/// `ERROR_OBJECT_NOT_FOUND` — which reads like a missing file and is
/// really a missing parent.
pub fn create_dir_all(path: &str) -> Result<(), Error> {
    if path.is_empty() || path.len() >= MAX_PATH {
        return Err(Error { what: What::BadPath, code: 0 });
    }
    // Everything up to and including the colon is the volume, and volumes
    // are not made here.
    let after_volume = path.find(':').map_or(0, |i| i + 1);
    let bytes = path.as_bytes();

    let mut at = after_volume;
    while at < bytes.len() {
        let Some(offset) = bytes[at..].iter().position(|&b| b == b'/') else { break };
        let cut = at + offset;
        // `AROS:/x` and `a//b` have empty components; there is nothing to
        // make for those.
        if cut > after_volume {
            create_dir(&path[..cut])?;
        }
        at = cut + 1;
    }
    // A path ending in a separator has already been made by the loop.
    if at < bytes.len() {
        create_dir(path)?;
    }
    Ok(())
}

fn exists_dir(path: &str) -> bool {
    let Ok(cpath) = CPath::new(path) else { return false };
    let lock = unsafe { sys::Lock(cpath.as_ptr(), sys::SHARED_LOCK) };
    if lock == 0 {
        return false;
    }
    unsafe { sys::UnLock(lock) };
    true
}

/// Put `data` at `path` without destroying what is already there until the
/// new copy is complete.
///
/// **There is no atomic replace on AROS.** `Rename` refuses a target that
/// exists, so this is the dance instead: write a temporary beside the real
/// file — same directory, therefore the same volume, because `Rename`
/// requires that — close it, move the old file aside, move the new one in,
/// and only then drop the old one.
///
/// **What this does and does not buy, narrowly.** Against the failures it
/// can see — a full disk, a refused open, a write or close that reports an
/// error — the previous save survives, as either the real file or the
/// `.bak` beside it, and the error says so. It also survives the process
/// going away between steps, because each step is a whole operation.
///
/// It is **not** crash safety. Nothing here orders writes against the
/// filesystem's own caching or asks it to commit, so a power cut or a
/// kernel that dies mid-operation can leave any of these files short or
/// absent. Getting further than this would need guarantees from the
/// filesystem that we have not established for SFS, and claiming them
/// without would be worse than not having them.
///
/// The one thing a caller must do: **when the real name is missing and a
/// `.bak` is beside it, say so.** That is a save interrupted between the
/// two moves, and the previous game is sitting right there under a name
/// nobody will look for on their own.
pub fn replace(path: &str, data: &[u8]) -> Result<(), Error> {
    let mut tmp = PathBuf::new(path)?;
    tmp.push(".new")?;
    let mut bak = PathBuf::new(path)?;
    bak.push(".bak")?;

    write_new(tmp.as_str(), data)?;

    if exists(path) {
        // A leftover from an interrupted earlier attempt would make the
        // rename below fail, so it goes first.
        if exists(bak.as_str()) {
            delete(bak.as_str())?;
        }
        if let Err(e) = rename(path, bak.as_str()) {
            let _ = delete(tmp.as_str());
            return Err(e);
        }
    }

    if let Err(e) = rename(tmp.as_str(), path) {
        // Put the old one back, so the failure costs nothing.
        if exists(bak.as_str()) {
            let _ = rename(bak.as_str(), path);
        }
        let _ = delete(tmp.as_str());
        return Err(e);
    }

    let _ = delete(bak.as_str());
    Ok(())
}

/// A path with room to append, on the stack.
pub struct PathBuf {
    bytes: [u8; MAX_PATH],
    len: usize,
}

impl PathBuf {
    pub fn new(base: &str) -> Result<PathBuf, Error> {
        let raw = base.as_bytes();
        if raw.is_empty() || raw.len() >= MAX_PATH || raw.contains(&0) {
            return Err(Error { what: What::BadPath, code: 0 });
        }
        let mut bytes = [0u8; MAX_PATH];
        bytes[..raw.len()].copy_from_slice(raw);
        Ok(PathBuf { bytes, len: raw.len() })
    }

    pub fn push(&mut self, more: &str) -> Result<(), Error> {
        let raw = more.as_bytes();
        if self.len + raw.len() >= MAX_PATH || raw.contains(&0) {
            return Err(Error { what: What::BadPath, code: 0 });
        }
        self.bytes[self.len..self.len + raw.len()].copy_from_slice(raw);
        self.len += raw.len();
        Ok(())
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }

    /// The directory part, or `None` when the path names no directory.
    /// AROS accepts both `Volume:name` and `dir/name`, so both separators
    /// count.
    pub fn parent(&self) -> Option<&str> {
        let s = self.as_str();
        let cut = s.rfind('/').or_else(|| s.rfind(':'))?;
        // `Volume:` keeps its colon; `dir/` loses its slash.
        let end = if s.as_bytes()[cut] == b':' { cut + 1 } else { cut };
        (end > 0).then(|| &s[..end])
    }
}
