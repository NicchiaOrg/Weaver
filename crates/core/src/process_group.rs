//! Children that never outlive Weaver, ended as a group (`requirements/tech#job`, "Process
//! lifetime").
//!
//! * Unix: the child runs in a new process group (`setpgid`), so that the group is signalled as a
//!   whole (`SIGTERM`, then `SIGKILL`); on Linux `PR_SET_PDEATHSIG(SIGTERM)` ends it when Weaver
//!   dies. That signal is tied to the *thread* that spawned the child: spawn from a thread that
//!   lives as long as the child.
//! * Windows: the child is assigned to a Job Object with `KILL_ON_JOB_CLOSE`; terminating the group
//!   is `TerminateJobObject`.
//!
//! A pid alone may be reused, so what is kept to end a process left by a crash is the pid together
//! with the process start time ([`start_time`]).

use std::io;
use std::process::{Child, Command};

/// The group of a spawned child.
#[derive(Debug)]
pub struct ProcessGroup {
    pid: u32,
    #[cfg(windows)]
    job: windows::JobHandle,
}

/// Spawns `command` as the leader of a new group that dies with Weaver.
pub fn spawn(command: &mut Command) -> io::Result<(Child, ProcessGroup)> {
    imp::spawn(command)
}

impl ProcessGroup {
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// The polite end: `SIGTERM` to the group (Windows: `TerminateJobObject`).
    pub fn terminate(&self) {
        imp::terminate(self);
    }

    /// The hard end: `SIGKILL` to the group (Windows: `TerminateJobObject`).
    pub fn kill(&self) {
        imp::kill(self);
    }
}

/// When the process `pid` started, as a text that is the same for the same process and differs
/// for a process that reuses the pid. `None` when there is no such process.
pub fn start_time(pid: u32) -> Option<String> {
    imp::start_time(pid)
}

/// Ends the group of a process a crash left behind: only when `pid` is alive and started at
/// `started`, so that a reused pid is never touched. Returns whether something was ended.
pub fn kill_leftover(pid: u32, started: &str) -> bool {
    if start_time(pid).as_deref() != Some(started) {
        return false;
    }
    imp::kill_pid(pid);
    true
}

#[cfg(unix)]
mod imp {
    use std::io;
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command};

    use super::ProcessGroup;

    #[cfg(target_os = "linux")]
    fn die_with_parent(command: &mut Command) {
        // SAFETY: between fork and exec only `prctl` runs, which is async-signal-safe.
        unsafe {
            command.pre_exec(|| {
                if libc::prctl(
                    libc::PR_SET_PDEATHSIG,
                    libc::SIGTERM as libc::c_ulong,
                    0,
                    0,
                    0,
                ) != 0
                {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn die_with_parent(_command: &mut Command) {}

    pub fn spawn(command: &mut Command) -> io::Result<(Child, ProcessGroup)> {
        command.process_group(0);
        die_with_parent(command);
        let child = command.spawn()?;
        let pid = child.id();
        Ok((child, ProcessGroup { pid }))
    }

    fn signal_group(pid: u32, signal: libc::c_int) {
        let Ok(pid) = libc::pid_t::try_from(pid) else {
            return;
        };
        // SAFETY: `kill` with a negative pid signals the process group; a group that is gone is
        // ESRCH, which is not an error here.
        unsafe {
            libc::kill(-pid, signal);
        }
    }

    pub fn terminate(group: &ProcessGroup) {
        signal_group(group.pid, libc::SIGTERM);
    }

    pub fn kill(group: &ProcessGroup) {
        signal_group(group.pid, libc::SIGKILL);
    }

    pub fn kill_pid(pid: u32) {
        signal_group(pid, libc::SIGKILL);
        if let Ok(pid) = libc::pid_t::try_from(pid) {
            // SAFETY: a plain `kill` of a process this function verified by its start time.
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
    }

    pub fn start_time(pid: u32) -> Option<String> {
        let output = crate::process::command("ps", ["-o", "lstart=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        (!text.is_empty()).then_some(text)
    }
}

#[cfg(windows)]
mod windows {
    /// A Job Object handle, closed on drop (which ends the whole group: `KILL_ON_JOB_CLOSE`).
    #[derive(Debug)]
    pub struct JobHandle(pub windows_sys::Win32::Foundation::HANDLE);

    // SAFETY: a kernel handle may be used from any thread.
    unsafe impl Send for JobHandle {}
    unsafe impl Sync for JobHandle {}

    impl Drop for JobHandle {
        fn drop(&mut self) {
            // SAFETY: the handle was returned by `CreateJobObjectW` and is closed once.
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.0);
            }
        }
    }
}

#[cfg(windows)]
mod imp {
    use std::io;
    use std::os::windows::io::AsRawHandle;
    use std::process::{Child, Command};

    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_TERMINATE,
    };

    use super::windows::JobHandle;
    use super::ProcessGroup;

    pub fn spawn(command: &mut Command) -> io::Result<(Child, ProcessGroup)> {
        // SAFETY: plain Win32 calls with valid arguments; the handle is owned by `JobHandle`.
        let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job.is_null() {
            return Err(io::Error::last_os_error());
        }
        let job = JobHandle(job);
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: `info` is a valid, initialised structure of the size passed.
        let set = unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if set == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut child = command.spawn()?;
        // SAFETY: the raw handle of a live child.
        let assigned = unsafe { AssignProcessToJobObject(job.0, child.as_raw_handle() as _) };
        if assigned == 0 {
            let error = io::Error::last_os_error();
            let _ = child.kill();
            return Err(error);
        }
        let pid = child.id();
        Ok((child, ProcessGroup { pid, job }))
    }

    pub fn terminate(group: &ProcessGroup) {
        // SAFETY: the job handle is alive as long as `group` is.
        unsafe {
            TerminateJobObject(group.job.0, 1);
        }
    }

    pub fn kill(group: &ProcessGroup) {
        terminate(group);
    }

    pub fn kill_pid(pid: u32) {
        // SAFETY: opens the process with the right to end it, ends it and closes the handle.
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if !handle.is_null() {
                TerminateProcess(handle, 1);
                CloseHandle(handle);
            }
        }
    }

    pub fn start_time(pid: u32) -> Option<String> {
        // SAFETY: queries the creation time of a process opened with the limited right.
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return None;
            }
            let zero = FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            };
            let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
            let ok = GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user);
            CloseHandle(handle);
            (ok != 0).then(|| {
                format!(
                    "{}",
                    (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime)
                )
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn sleeper() -> Command {
        let mut command = crate::process::command("/bin/sh", ["-c", "sleep 30 & sleep 30; wait"]);
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        command
    }

    #[cfg(unix)]
    #[test]
    fn terminate_ends_the_whole_group_and_the_pid_has_a_start_time_while_it_lives() {
        let (mut child, group) = spawn(&mut sleeper()).unwrap();
        let started = start_time(group.pid()).expect("a live process has a start time");
        assert!(!started.is_empty());
        assert_eq!(
            start_time(group.pid()).as_deref(),
            Some(started.as_str()),
            "stable for one process"
        );
        group.terminate();
        let status = child.wait().unwrap();
        assert!(!status.success());
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(status.signal(), Some(libc::SIGTERM));
        // The group leader is gone and so is its group: the pid answers nothing any more.
        assert_eq!(start_time(group.pid()), None);
    }

    #[cfg(unix)]
    #[test]
    fn kill_ends_a_child_that_ignores_sigterm() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("trap-installed");
        let mut command = crate::process::command(
            "/bin/sh",
            [
                "-c",
                "trap '' TERM; : > \"$0\"; while true; do sleep 1; done",
            ],
        );
        command.arg(&marker);
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let (mut child, group) = spawn(&mut command).unwrap();
        // The signal must come after the shell installed its trap.
        while !marker.exists() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        group.terminate();
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(
            child.try_wait().unwrap().is_none(),
            "it ignored the polite signal"
        );
        group.kill();
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(child.wait().unwrap().signal(), Some(libc::SIGKILL));
    }

    #[cfg(unix)]
    #[test]
    fn a_leftover_is_ended_only_when_its_start_time_matches() {
        let (mut child, group) = spawn(&mut sleeper()).unwrap();
        let started = start_time(group.pid()).unwrap();
        assert!(
            !kill_leftover(group.pid(), "Thu Jan  1 00:00:00 1970"),
            "another process with a reused pid is never touched"
        );
        assert!(child.try_wait().unwrap().is_none());
        assert!(kill_leftover(group.pid(), &started));
        child.wait().unwrap();
        assert!(!kill_leftover(group.pid(), &started), "nothing left");
    }
}
