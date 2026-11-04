//! AROS socket selector. All operations except notify stay on the owning task.
use std::{collections::BTreeMap, ffi::c_void, io, os::fd::{AsRawFd, BorrowedFd}, sync::{Mutex, atomic::{AtomicUsize, Ordering}}, time::Instant};
use crate::{Event, PollMode};

unsafe extern "C" {
    fn rs_selector_new() -> *mut c_void;
    fn rs_selector_check(context: *mut c_void) -> i32;
    fn rs_selector_free(context: *mut c_void);
    fn rs_selector_notify(context: *mut c_void) -> i32;
    fn rs_selector_validate(context: *mut c_void, fd: i32) -> i32;
    fn rs_selector_wait(context: *mut c_void, fds: *mut libc::pollfd, n: usize, micros: i64) -> i32;
}
fn cvt(value: i32) -> io::Result<()> {
    if value < 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}
#[derive(Debug)]
struct Interest { event: Event, oneshot: bool }
#[derive(Debug)]
pub struct Poller { context: usize, cursor: AtomicUsize, interests: Mutex<BTreeMap<i32, Interest>> }
impl Poller {
    fn context(&self) -> *mut c_void { self.context as *mut c_void }
    pub(super) fn owner(&self) -> io::Result<()> { cvt(unsafe { rs_selector_check(self.context()) }) }
    fn mode(mode: PollMode) -> io::Result<bool> {
        match mode {
            PollMode::Oneshot => Ok(true), PollMode::Level => Ok(false),
            _ => Err(io::ErrorKind::Unsupported.into()),
        }
    }
    fn validate(event: Event) -> io::Result<()> {
        if event.extra.hup || event.extra.pri { Err(io::ErrorKind::Unsupported.into()) } else { Ok(()) }
    }
    pub fn new() -> io::Result<Self> {
        let context = unsafe { rs_selector_new() };
        if context.is_null() { return Err(io::Error::last_os_error()); }
        Ok(Self { context: context as usize, cursor: AtomicUsize::new(0), interests: Mutex::new(BTreeMap::new()) })
    }
    pub fn supports_level(&self) -> bool { true }
    pub fn supports_edge(&self) -> bool { false }
    pub fn add(&self, fd: i32, event: Event, mode: PollMode) -> io::Result<()> {
        self.owner()?; Self::validate(event)?; let oneshot = Self::mode(mode)?;
        cvt(unsafe { rs_selector_validate(self.context(), fd) })?;
        let mut entries = self.interests.lock().unwrap();
        if entries.contains_key(&fd) { return Err(io::ErrorKind::AlreadyExists.into()); }
        entries.insert(fd, Interest { event, oneshot }); Ok(())
    }
    pub fn modify(&self, fd: BorrowedFd<'_>, event: Event, mode: PollMode) -> io::Result<()> {
        self.owner()?; Self::validate(event)?; let oneshot = Self::mode(mode)?;
        let mut entries = self.interests.lock().unwrap();
        let interest = entries.get_mut(&fd.as_raw_fd()).ok_or(io::ErrorKind::NotFound)?;
        *interest = Interest { event, oneshot }; Ok(())
    }
    pub fn delete(&self, fd: BorrowedFd<'_>) -> io::Result<()> {
        self.owner()?;
        self.interests.lock().unwrap().remove(&fd.as_raw_fd()).ok_or(io::ErrorKind::NotFound)?;
        Ok(())
    }
    pub fn notify(&self) -> io::Result<()> { cvt(unsafe { rs_selector_notify(self.context()) }) }
    pub fn wait_deadline(&self, events: &mut Events, deadline: Option<Instant>) -> io::Result<()> {
        self.owner()?;
        if events.inner.len() >= events.capacity { return Ok(()); }
        let mut entries = self.interests.lock().unwrap();
        let mut fds: Vec<_> = entries.iter().filter_map(|(&fd, interest)| {
            let flags = (if interest.event.readable { 1 } else { 0 }) | (if interest.event.writable { 4 } else { 0 });
            (flags != 0).then_some(libc::pollfd { fd, events: flags, revents: 0 })
        }).collect();
        let micros = deadline.map(|d| d.saturating_duration_since(Instant::now()).as_micros().min(i64::MAX as u128) as i64).unwrap_or(-1);
        cvt(unsafe { rs_selector_wait(self.context(), fds.as_mut_ptr(), fds.len(), micros) })?;
        let length = fds.len();
        let start = if length == 0 { 0 } else { self.cursor.load(Ordering::Relaxed) % length };
        for offset in 0..length {
            let index = (start + offset) % length;
            let fd = &fds[index];
            if fd.revents == 0 { continue; }
            if events.inner.len() == events.capacity { break; }
            let interest = entries.get_mut(&fd.fd).unwrap();
            let mut event = interest.event;
            event.readable = fd.revents & 1 != 0; event.writable = fd.revents & 4 != 0;
            events.inner.push(event);
            self.cursor.store((index + 1) % length, Ordering::Relaxed);
            if interest.oneshot { interest.event.readable = false; interest.event.writable = false; }
        }
        Ok(())
    }
}
impl Drop for Poller { fn drop(&mut self) { unsafe { rs_selector_free(self.context()) } } }
pub struct Events { inner: Vec<Event>, capacity: usize }
impl Events {
    pub fn with_capacity(capacity: usize) -> Self { Self { inner: Vec::with_capacity(capacity), capacity } }
    pub fn iter(&self) -> impl Iterator<Item=Event> + '_ { self.inner.iter().copied() }
    pub fn clear(&mut self) { self.inner.clear(); }
    pub fn capacity(&self) -> usize { self.capacity }
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct EventExtra { hup: bool, pri: bool }
impl EventExtra {
    pub const fn empty() -> Self { Self { hup: false, pri: false } }
    pub fn set_hup(&mut self, value: bool) { self.hup = value; }
    pub fn set_pri(&mut self, value: bool) { self.pri = value; }
    pub fn is_hup(&self) -> bool { false }
    pub fn is_pri(&self) -> bool { false }
    pub fn is_connect_failed(&self) -> Option<bool> { None }
    pub fn is_err(&self) -> Option<bool> { None }
}
