//! Bounded owner-task TLS transport. Drop frees memory and never performs I/O.
use std::{
    ffi::{c_char, c_void, CString},
    io,
    marker::PhantomData,
    net::{SocketAddr, TcpStream},
    os::fd::AsRawFd,
    ptr::NonNull,
    rc::Rc,
};
#[repr(C)]
#[derive(Clone, Copy)]
struct RawError {
    domain: i32,
    code: i32,
    verify: u32,
    os_error: i32,
    message: [c_char; 192],
}
impl Default for RawError {
    fn default() -> Self {
        Self {
            domain: 0,
            code: 0,
            verify: 0,
            os_error: 0,
            message: [0; 192],
        }
    }
}
#[derive(Debug)]
pub struct Error {
    pub domain: i32,
    pub code: i32,
    pub verify: u32,
    pub os_error: i32,
    pub message: String,
}
impl From<RawError> for Error {
    fn from(e: RawError) -> Self {
        let bytes: Vec<u8> = e
            .message
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as u8)
            .collect();
        Self {
            domain: e.domain,
            code: e.code,
            verify: e.verify,
            os_error: e.os_error,
            message: String::from_utf8_lossy(&bytes).into_owned(),
        }
    }
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self {
            domain: 5,
            code: 0,
            verify: 0,
            os_error: e.raw_os_error().unwrap_or(0),
            message: e.to_string(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TLS domain={} code={} verify={:#x} OS={}: {}",
            self.domain, self.code, self.verify, self.os_error, self.message
        )
    }
}
impl std::error::Error for Error {}
type Bio = unsafe extern "C" fn(*mut c_void, *mut u8, usize, i32) -> i32;
extern "C" {
    fn rs_tls_new(
        ca: *const u8,
        len: usize,
        name: *const c_char,
        bio: Bio,
        user: *mut c_void,
        error: *mut RawError,
    ) -> *mut c_void;
    fn rs_tls_handshake(ctx: *mut c_void) -> i32;
    fn rs_tls_read(ctx: *mut c_void, b: *mut u8, n: usize, got: *mut usize) -> i32;
    fn rs_tls_write(ctx: *mut c_void, b: *const u8, n: usize, sent: *mut usize) -> i32;
    fn rs_tls_close(ctx: *mut c_void) -> i32;
    fn rs_tls_error(ctx: *mut c_void, error: *mut RawError) -> i32;
    fn rs_tls_free(ctx: *mut c_void) -> i32;
}
struct Transport {
    fd: i32,
}
unsafe extern "C" fn bio(user: *mut c_void, b: *mut u8, n: usize, writing: i32) -> i32 {
    let fd = (*(user as *mut Transport)).fd;
    if fd < 0 {
        return -libc::ENOTCONN;
    }
    let count = if writing != 0 {
        libc::send(fd, b as *const c_void, n, 0)
    } else {
        libc::recv(fd, b as *mut c_void, n, 0)
    };
    if count < 0 {
        -io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EIO)
    } else {
        count as i32
    }
}
pub struct Tls {
    ctx: NonNull<c_void>,
    transport: Box<Transport>,
    socket: Option<async_io::Async<TcpStream>>,
    _owner: PhantomData<Rc<()>>,
}
impl Tls {
    pub async fn connect(endpoint: SocketAddr, ca: &[u8], name: &str) -> Result<Self, Error> {
        let mut tls = Self::begin(endpoint, ca, name).await?;
        tls.handshake().await?;
        Ok(tls)
    }
    pub(crate) async fn begin(endpoint: SocketAddr, ca: &[u8], name: &str) -> Result<Self, Error> {
        let name = CString::new(name)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL hostname"))?;
        let mut ca = ca.to_vec();
        if ca.last() != Some(&0) {
            ca.push(0);
        }
        let mut transport = Box::new(Transport { fd: -1 });
        let mut error = RawError::default();
        // Crypto construction precedes socket connect, including entropy health tests.
        let raw = unsafe {
            rs_tls_new(
                ca.as_ptr(),
                ca.len(),
                name.as_ptr(),
                bio,
                (&mut *transport as *mut Transport).cast(),
                &mut error,
            )
        };
        let ctx = NonNull::new(raw).ok_or_else(|| Error::from(error))?;
        let mut tls = Self {
            ctx,
            transport,
            socket: None,
            _owner: PhantomData,
        };
        let socket = async_io::Async::<TcpStream>::connect(endpoint).await?;
        tls.transport.fd = socket.as_raw_fd();
        tls.socket = Some(socket);
        Ok(tls)
    }
    pub(crate) async fn handshake(&mut self) -> Result<(), Error> {
        loop {
            let status = unsafe { rs_tls_handshake(self.ctx.as_ptr()) };
            if status == 0 {
                break;
            }
            self.wait(status).await?;
        }
        Ok(())
    }
    fn error(&self) -> Error {
        let mut e = RawError::default();
        unsafe { rs_tls_error(self.ctx.as_ptr(), &mut e) };
        e.into()
    }
    async fn wait(&self, status: i32) -> Result<(), Error> {
        match status {
            1 => self.socket.as_ref().unwrap().readable().await?,
            2 => self.socket.as_ref().unwrap().writable().await?,
            3 => futures_lite::future::yield_now().await,
            _ => return Err(self.error()),
        }
        Ok(())
    }
    pub async fn write_all(&mut self, mut bytes: &[u8]) -> Result<(), Error> {
        while !bytes.is_empty() {
            let mut sent = 0;
            // The identical buffer and length survive every WANT retry.
            let status =
                unsafe { rs_tls_write(self.ctx.as_ptr(), bytes.as_ptr(), bytes.len(), &mut sent) };
            if status == 0 {
                if sent == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "TLS write made no progress",
                    )
                    .into());
                }
                bytes = &bytes[sent..];
            } else {
                self.wait(status).await?;
            }
        }
        Ok(())
    }
    pub async fn read(&mut self, bytes: &mut [u8]) -> Result<usize, Error> {
        if bytes.is_empty() {
            return Ok(0);
        }
        loop {
            let mut got = 0;
            let status = unsafe {
                rs_tls_read(self.ctx.as_ptr(), bytes.as_mut_ptr(), bytes.len(), &mut got)
            };
            if status == 4 {
                return Ok(0);
            }
            if status == 0 {
                return Ok(got);
            }
            self.wait(status).await?;
        }
    }
    pub async fn close(&mut self) -> Result<(), Error> {
        loop {
            let status = unsafe { rs_tls_close(self.ctx.as_ptr()) };
            if status == 0 {
                return Ok(());
            }
            self.wait(status).await?;
        }
    }
}
impl Drop for Tls {
    fn drop(&mut self) {
        let rc = unsafe { rs_tls_free(self.ctx.as_ptr()) };
        debug_assert_eq!(rc, 0, "TLS destroyed outside owner task");
    }
}
