//! Terminal handling: raw mode via libc, non-blocking key reads, ANSI drawing, and
//! a best-effort terminal-size query that also works over a serial console.

use std::io::{self, Write};
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

// Stashed on raw-mode entry so the signal handler can restore the console.
static mut SAVED: MaybeUninit<libc::termios> = MaybeUninit::uninit();
static HAVE_SAVED: AtomicBool = AtomicBool::new(false);

pub fn is_tty() -> bool {
    unsafe { libc::isatty(libc::STDIN_FILENO) == 1 }
}

/// Raw mode, set up for a game loop: input is fully non-blocking (VMIN=0, VTIME=0)
/// so we can animate and poll for keys in the same loop. Restored on drop.
pub struct RawMode;

impl RawMode {
    pub fn enable() -> io::Result<RawMode> {
        unsafe {
            let mut t = MaybeUninit::<libc::termios>::uninit();
            if libc::tcgetattr(libc::STDIN_FILENO, t.as_mut_ptr()) != 0 {
                return Err(io::Error::last_os_error());
            }
            let orig = t.assume_init();
            (*std::ptr::addr_of_mut!(SAVED)).write(orig);
            HAVE_SAVED.store(true, Ordering::SeqCst);

            let mut raw = orig;
            raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG | libc::IEXTEN);
            raw.c_iflag &= !(libc::IXON | libc::ICRNL | libc::BRKINT | libc::INPCK | libc::ISTRIP);
            raw.c_oflag &= !libc::OPOST;
            raw.c_cc[libc::VMIN] = 0;
            raw.c_cc[libc::VTIME] = 0;

            if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &raw) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        install_signal_handlers();
        Ok(RawMode)
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        restore();
    }
}

fn restore() {
    unsafe {
        if HAVE_SAVED.load(Ordering::SeqCst) {
            let saved = (*std::ptr::addr_of!(SAVED)).as_ptr();
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, saved);
        }
    }
}

extern "C" fn on_signal(_sig: libc::c_int) {
    restore();
    // Raw write, not io::stdout(): the latter takes a lock the main thread may already
    // hold, and a signal handler must not block on it.
    let msg = b"\x1b[?25h\r\n";
    unsafe {
        libc::write(
            libc::STDOUT_FILENO,
            msg.as_ptr() as *const libc::c_void,
            msg.len(),
        );
        libc::_exit(0)
    }
}

fn install_signal_handlers() {
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = on_signal as *const () as usize;
        sa.sa_flags = 0;
        libc::sigemptyset(&mut sa.sa_mask);
        libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut());
    }
}

fn read_fd(buf: &mut [u8]) -> isize {
    unsafe {
        libc::read(
            libc::STDIN_FILENO,
            buf.as_mut_ptr() as *mut libc::c_void,
            buf.len(),
        )
    }
}

/// Everything available right now, without blocking. Empty = nothing pending.
pub fn read_pending() -> Vec<u8> {
    let mut out = Vec::new();
    let mut b = [0u8; 64];
    loop {
        let n = read_fd(&mut b);
        if n > 0 {
            out.extend_from_slice(&b[..n as usize]);
            if (n as usize) < b.len() {
                break;
            }
        } else {
            break;
        }
    }
    out
}

/// Blocking single-byte read for the scripted (non-TTY) path. None means EOF.
pub fn read_byte_blocking() -> Option<u8> {
    let mut b = [0u8; 1];
    loop {
        let n = read_fd(&mut b);
        if n == 1 {
            return Some(b[0]);
        }
        if n == 0 {
            return None;
        }
        if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
            continue;
        }
        return None;
    }
}

/// Terminal size as (cols, rows). Tries an ioctl, then asks the terminal directly
/// (which is how we get a real answer over a serial line), then falls back to 80x24.
pub fn query_size() -> (u16, u16) {
    if let Some(s) = ioctl_size() {
        return s;
    }
    if let Some(s) = dsr_size() {
        return s;
    }
    (80, 24)
}

fn ioctl_size() -> Option<(u16, u16)> {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0
            && ws.ws_col > 0
            && ws.ws_row > 0
        {
            Some((ws.ws_col, ws.ws_row))
        } else {
            None
        }
    }
}

/// Park the cursor at the far corner and ask where it ended up. Needs raw mode (so
/// the reply isn't echoed or line-buffered), which is always on by the time we ask.
fn dsr_size() -> Option<(u16, u16)> {
    {
        let mut out = io::stdout().lock();
        out.write_all(b"\x1b[s\x1b[999;999H\x1b[6n\x1b[u").ok()?;
        out.flush().ok()?;
    }
    let mut buf = Vec::new();
    for _ in 0..20 {
        buf.extend(read_pending());
        if buf.contains(&b'R') {
            break;
        }
        sleep_ms(8);
    }
    parse_dsr(&buf)
}

// Reply looks like ESC [ rows ; cols R
fn parse_dsr(buf: &[u8]) -> Option<(u16, u16)> {
    let s = String::from_utf8_lossy(buf);
    let tail = &s[s.rfind('\x1b')?..];
    let body = tail.strip_prefix("\x1b[")?;
    let body = &body[..body.find('R')?];
    let (rows, cols) = body.split_once(';')?;
    let rows: u16 = rows.trim().parse().ok()?;
    let cols: u16 = cols.trim().parse().ok()?;
    (cols > 0 && rows > 0).then_some((cols, rows))
}

/// One full clear, done once before the first frame.
pub fn clear() {
    let mut out = io::stdout().lock();
    let _ = out.write_all(b"\x1b[2J\x1b[H");
    let _ = out.flush();
}

/// Paint a frame in place: home the cursor and overwrite. The frame erases to the end
/// of each line and to the end of the screen as it goes, so we never blank the whole
/// screen first - that's what kills the flicker on a slow console.
pub fn draw(frame: &str) {
    let mut out = io::stdout().lock();
    let _ = out.write_all(b"\x1b[H");
    let _ = out.write_all(frame.as_bytes());
    let _ = out.flush();
}

pub fn hide_cursor() {
    let mut out = io::stdout().lock();
    let _ = out.write_all(b"\x1b[?25l");
    let _ = out.flush();
}

pub fn show_cursor() {
    let mut out = io::stdout().lock();
    let _ = out.write_all(b"\x1b[?25h");
    let _ = out.flush();
}

pub fn sleep_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}
