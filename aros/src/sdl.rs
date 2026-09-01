//! Just enough SDL2 to open a window and push pixels at it.
//!
//! Only opaque handles are used, so no SDL struct layout is duplicated here —
//! the one exception is `SDL_Event`, which is read as raw bytes and never
//! interpreted beyond its leading event type.
//!
//! AROS renders SDL in software, so the texture path (upload a whole frame,
//! let SDL scale it) is both the simplest and the fastest option.
use core::ffi::c_void;

// AROS' SDL2 reaches gl.library for its renderer, so GL has to be linked too.
#[link(name = "SDL2")]
#[link(name = "GL")]
extern "C" {
    fn SDL_Init(flags: u32) -> i32;
    fn SDL_Quit();
    fn SDL_GetError() -> *const u8;
    fn SDL_CreateWindow(title: *const u8, x: i32, y: i32, w: i32, h: i32, flags: u32) -> *mut c_void;
    fn SDL_DestroyWindow(w: *mut c_void);
    fn SDL_CreateRenderer(window: *mut c_void, index: i32, flags: u32) -> *mut c_void;
    fn SDL_DestroyRenderer(r: *mut c_void);
    fn SDL_CreateTexture(r: *mut c_void, format: u32, access: i32, w: i32, h: i32) -> *mut c_void;
    fn SDL_DestroyTexture(t: *mut c_void);
    fn SDL_UpdateTexture(t: *mut c_void, rect: *const c_void, pixels: *const c_void, pitch: i32) -> i32;
    fn SDL_RenderCopy(r: *mut c_void, t: *mut c_void, src: *const c_void, dst: *const c_void) -> i32;
    fn SDL_RenderPresent(r: *mut c_void);
    fn SDL_PollEvent(event: *mut u8) -> i32;
    fn SDL_GetTicks() -> u32;
}

pub const INIT_VIDEO: u32 = 0x0000_0020;
pub const WINDOWPOS_CENTERED: i32 = 0x2FFF_0000u32 as i32;
pub const WINDOW_SHOWN: u32 = 0x0000_0004;
pub const PIXELFORMAT_ARGB8888: u32 = 0x1636_2004;
pub const TEXTUREACCESS_STREAMING: i32 = 1;

pub const EVENT_QUIT: u32 = 0x100;
pub const EVENT_KEYDOWN: u32 = 0x300;

/// Milliseconds since SDL started.
pub fn ticks() -> u32 {
    unsafe { SDL_GetTicks() }
}

/// The last SDL error, as a best-effort string.
pub fn error() -> &'static str {
    unsafe {
        let p = SDL_GetError();
        if p.is_null() {
            return "(no error)";
        }
        let mut n = 0;
        while *p.add(n) != 0 && n < 256 {
            n += 1;
        }
        core::str::from_utf8(core::slice::from_raw_parts(p, n)).unwrap_or("(bad utf8)")
    }
}

/// A window with a streaming texture behind it: write pixels, call
/// [`Screen::present`], repeat. Everything is released on drop, in order.
pub struct Screen {
    window: *mut c_void,
    renderer: *mut c_void,
    texture: *mut c_void,
    pub width: usize,
    pub height: usize,
}

impl Screen {
    /// Open a `win_w` x `win_h` window drawing a `w` x `h` pixel buffer,
    /// scaled by SDL. `title` must be NUL-terminated.
    pub fn open(title: &[u8], w: usize, h: usize, win_w: i32, win_h: i32) -> Result<Screen, &'static str> {
        unsafe {
            if SDL_Init(INIT_VIDEO) != 0 {
                return Err("SDL_Init failed");
            }
            let window = SDL_CreateWindow(
                title.as_ptr(), WINDOWPOS_CENTERED, WINDOWPOS_CENTERED, win_w, win_h, WINDOW_SHOWN);
            if window.is_null() {
                SDL_Quit();
                return Err("SDL_CreateWindow failed");
            }
            let renderer = SDL_CreateRenderer(window, -1, 0);
            if renderer.is_null() {
                SDL_DestroyWindow(window);
                SDL_Quit();
                return Err("SDL_CreateRenderer failed");
            }
            let texture = SDL_CreateTexture(
                renderer, PIXELFORMAT_ARGB8888, TEXTUREACCESS_STREAMING, w as i32, h as i32);
            if texture.is_null() {
                SDL_DestroyRenderer(renderer);
                SDL_DestroyWindow(window);
                SDL_Quit();
                return Err("SDL_CreateTexture failed");
            }
            Ok(Screen { window, renderer, texture, width: w, height: h })
        }
    }

    /// Upload one frame of `width * height` ARGB pixels and show it.
    pub fn present(&mut self, pixels: &[u32]) {
        debug_assert_eq!(pixels.len(), self.width * self.height);
        unsafe {
            SDL_UpdateTexture(
                self.texture,
                core::ptr::null(),
                pixels.as_ptr() as *const c_void,
                (self.width * 4) as i32,
            );
            SDL_RenderCopy(self.renderer, self.texture, core::ptr::null(), core::ptr::null());
            SDL_RenderPresent(self.renderer);
        }
    }

    /// True once the user closes the window or presses a key.
    pub fn should_quit(&mut self) -> bool {
        // SDL_Event is a union of at most 56 bytes; we only read its type.
        let mut ev = [0u8; 64];
        let mut quit = false;
        unsafe {
            while SDL_PollEvent(ev.as_mut_ptr()) != 0 {
                let kind = u32::from_ne_bytes([ev[0], ev[1], ev[2], ev[3]]);
                if kind == EVENT_QUIT || kind == EVENT_KEYDOWN {
                    quit = true;
                }
            }
        }
        quit
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        unsafe {
            SDL_DestroyTexture(self.texture);
            SDL_DestroyRenderer(self.renderer);
            SDL_DestroyWindow(self.window);
            SDL_Quit();
        }
    }
}
