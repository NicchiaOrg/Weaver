//! What stops a command that waits with no time limit (`weaver question new`): SIGINT, SIGTERM or
//! a closed standard output (on Windows: Ctrl+C, Ctrl+Break, a closing console or a closed pipe).
//!
//! The command asks [`Interrupts`] to call a function when that happens and withdraws its round by
//! returning, which ends the process and closes the connection to the app. The real signals are
//! kept out of in-process tests through this trait ([`NoInterrupts`] never calls the function).

/// A source of "stop waiting".
pub trait Interrupts {
    /// Calls `wake` once, on another thread, when the process is asked to stop or its standard
    /// output is closed. Called at most once per process.
    fn watch(&self, wake: Box<dyn Fn() + Send>);
}

/// Never interrupts: for tests that run a command in process.
pub struct NoInterrupts;

impl Interrupts for NoInterrupts {
    fn watch(&self, _wake: Box<dyn Fn() + Send>) {}
}

/// The signals and the standard output of this process.
pub struct ProcessInterrupts;

impl Interrupts for ProcessInterrupts {
    fn watch(&self, wake: Box<dyn Fn() + Send>) {
        imp::watch(wake);
    }
}

#[cfg(unix)]
mod imp {
    use std::os::raw::c_int;
    use std::sync::atomic::{AtomicI32, Ordering};

    /// The write end of the self-pipe: the signal handler writes one byte to it.
    static WAKE_FD: AtomicI32 = AtomicI32::new(-1);

    extern "C" fn on_signal(_signal: c_int) {
        let fd = WAKE_FD.load(Ordering::Relaxed);
        if fd >= 0 {
            let byte = 1u8;
            // SAFETY: `write` is async-signal-safe; the byte lives on this stack frame.
            unsafe {
                libc::write(fd, (&byte as *const u8).cast(), 1);
            }
        }
    }

    /// Whether standard output is a pipe or a socket: only then does its reader go away.
    fn stdout_has_reader() -> bool {
        // SAFETY: `fstat` fills a zeroed struct for a descriptor number.
        unsafe {
            let mut stat: libc::stat = std::mem::zeroed();
            if libc::fstat(1, &mut stat) != 0 {
                return false;
            }
            matches!(stat.st_mode & libc::S_IFMT, libc::S_IFIFO | libc::S_IFSOCK)
        }
    }

    fn set_flags(fd: c_int, nonblocking: bool) {
        // SAFETY: plain `fcntl` calls on descriptors this function owns.
        unsafe {
            libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
            if nonblocking {
                let flags = libc::fcntl(fd, libc::F_GETFL);
                libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }
    }

    /// Blocks until a signal wrote to `signals` or standard output lost its reader; false when
    /// waiting itself failed. Standard output is a pipe or a socket: Linux reports a reader that
    /// went away to `poll` as an error on the write end.
    #[cfg(not(target_os = "macos"))]
    fn wait(signals: c_int, watch_stdout: bool) -> bool {
        loop {
            let mut polled = [
                libc::pollfd {
                    fd: signals,
                    events: libc::POLLIN,
                    revents: 0,
                },
                // No events asked: only an error or a hang-up of the reader is reported.
                libc::pollfd {
                    fd: if watch_stdout { 1 } else { -1 },
                    events: 0,
                    revents: 0,
                },
            ];
            // SAFETY: `polled` is a valid array of two `pollfd`.
            let ready = unsafe { libc::poll(polled.as_mut_ptr(), 2, -1) };
            if ready < 0 {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return false;
            }
            if polled[0].revents != 0 || polled[1].revents & (libc::POLLERR | libc::POLLHUP) != 0 {
                return true;
            }
        }
    }

    /// The same on macOS, where `poll` does not report a pipe whose reader went away: `kqueue`
    /// sets `EV_EOF` on the write filter. `EV_CLEAR` makes it report changes, not the pipe being
    /// writable, which it always is.
    #[cfg(target_os = "macos")]
    fn wait(signals: c_int, watch_stdout: bool) -> bool {
        // SAFETY: plain `kqueue` calls; the arrays live on this stack frame.
        unsafe {
            let queue = libc::kqueue();
            if queue < 0 {
                return false;
            }
            let change = |ident: c_int, filter: i16, flags: u16| {
                let mut event: libc::kevent = std::mem::zeroed();
                event.ident = ident as libc::uintptr_t;
                event.filter = filter;
                event.flags = flags;
                event
            };
            let mut changes = vec![change(signals, libc::EVFILT_READ, libc::EV_ADD)];
            if watch_stdout {
                changes.push(change(1, libc::EVFILT_WRITE, libc::EV_ADD | libc::EV_CLEAR));
            }
            let mut count = changes.len() as c_int;
            loop {
                let mut events: [libc::kevent; 2] = std::mem::zeroed();
                let ready = libc::kevent(
                    queue,
                    changes.as_ptr(),
                    count,
                    events.as_mut_ptr(),
                    2,
                    std::ptr::null(),
                );
                count = 0;
                if ready < 0 {
                    if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    libc::close(queue);
                    return false;
                }
                for event in &events[..ready as usize] {
                    if event.ident == signals as libc::uintptr_t || event.flags & libc::EV_EOF != 0
                    {
                        libc::close(queue);
                        return true;
                    }
                }
            }
        }
    }

    pub fn watch(wake: Box<dyn Fn() + Send>) {
        let mut fds = [0 as c_int; 2];
        // SAFETY: `pipe` fills two descriptors; `sigaction` gets a zeroed, then initialised struct.
        unsafe {
            if libc::pipe(fds.as_mut_ptr()) != 0 {
                return;
            }
            set_flags(fds[0], false);
            set_flags(fds[1], true);
            WAKE_FD.store(fds[1], Ordering::Relaxed);
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = on_signal as extern "C" fn(c_int) as libc::sighandler_t;
            libc::sigemptyset(&mut action.sa_mask);
            action.sa_flags = 0;
            for signal in [libc::SIGINT, libc::SIGTERM] {
                libc::sigaction(signal, &action, std::ptr::null_mut());
            }
        }
        let read_end = fds[0];
        let watch_stdout = stdout_has_reader();
        std::thread::spawn(move || {
            if wait(read_end, watch_stdout) {
                wake();
            }
        });
    }
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::sync::Mutex;
    use std::time::Duration;

    #[link(name = "kernel32")]
    extern "system" {
        fn SetConsoleCtrlHandler(
            handler: Option<unsafe extern "system" fn(u32) -> i32>,
            add: i32,
        ) -> i32;
        fn GetStdHandle(which: u32) -> *mut c_void;
        fn GetFileType(handle: *mut c_void) -> u32;
        fn WriteFile(
            handle: *mut c_void,
            buffer: *const u8,
            length: u32,
            written: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
    }

    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const FILE_TYPE_PIPE: u32 = 3;
    const ERROR_BROKEN_PIPE: i32 = 109;
    const ERROR_NO_DATA: i32 = 232;

    static WAKE: Mutex<Option<Box<dyn Fn() + Send>>> = Mutex::new(None);

    fn wake_once() {
        let wake = WAKE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(wake) = wake {
            wake();
        }
    }

    /// Ctrl+C, Ctrl+Break and a closing console stop the wait; the event is handled, so the
    /// process ends through the command and not by the default handler.
    unsafe extern "system" fn on_control(kind: u32) -> i32 {
        // CTRL_C_EVENT 0, CTRL_BREAK_EVENT 1, CTRL_CLOSE_EVENT 2.
        if kind <= 2 {
            wake_once();
            1
        } else {
            0
        }
    }

    pub fn watch(wake: Box<dyn Fn() + Send>) {
        *WAKE.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(wake);
        // SAFETY: registers a handler with the signature the system expects.
        unsafe {
            SetConsoleCtrlHandler(Some(on_control), 1);
        }
        // A pipe has no event for its reader going away: an empty write fails once it has.
        std::thread::spawn(|| {
            // SAFETY: handles of the process; the empty write reads no memory.
            let handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
            if handle.is_null() || unsafe { GetFileType(handle) } != FILE_TYPE_PIPE {
                return;
            }
            loop {
                std::thread::sleep(Duration::from_millis(250));
                let mut written = 0u32;
                let done = unsafe {
                    WriteFile(
                        handle,
                        std::ptr::null(),
                        0,
                        &mut written,
                        std::ptr::null_mut(),
                    )
                };
                if done == 0
                    && matches!(
                        std::io::Error::last_os_error().raw_os_error(),
                        Some(ERROR_BROKEN_PIPE | ERROR_NO_DATA)
                    )
                {
                    wake_once();
                    return;
                }
            }
        });
    }
}
