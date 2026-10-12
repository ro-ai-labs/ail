//! Shared, allocator-free Linux x86-64 process lifecycle engine.
//! The caller validates and marshals all language values before this boundary.
//! This engine owns descriptors and the direct child until cleanup completes;
//! it never calls back into AIL or transfers control to a recovery handler.

#[repr(C)]
pub struct ProcessRequest {
    pub executable: *const u8,
    pub argv: *const *const u8,
    pub input: *const u8,
    pub input_len: u64,
    pub timeout_ms: u64,
    pub stdout: *mut u8,
    pub stderr: *mut u8,
    pub capacity: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct ProcessResult {
    pub stdout_len: u64,
    pub stderr_len: u64,
    pub status: i64,
}

/// # Safety
/// Request and result point to live, disjoint objects. The executable and argv
/// are bounded, validated NUL-terminated strings and a terminated pointer array.
/// Input is readable for input_len; each output is writable for capacity bytes
/// and disjoint from all other buffers, objects and input strings. All referenced
/// storage remains live through the call. No caller handler/thread may reap this
/// call's child or change SIGCHLD. The caller must support synchronous kill/reap
/// cleanup, which may outlast the deadline during an uninterruptible kernel wait.
pub unsafe extern "C" fn ail_process_run(
    request: *const ProcessRequest,
    result: *mut ProcessResult,
) -> u64 {
    if request.is_null() || result.is_null() {
        return PROCESS;
    }
    let request = unsafe { &*request };
    if request.executable.is_null()
        || request.argv.is_null()
        || request.input.is_null()
        || request.stdout.is_null()
        || request.stderr.is_null()
        || request.capacity != CAPACITY as u64
        || request.input_len > 1_048_576
        || !(1..=60_000).contains(&request.timeout_ms)
    {
        return PROCESS;
    }
    let mut engine = Engine {
        fds: [-1; 8],
        pid: 0,
        old_mask: None,
    };
    let outcome = unsafe { engine.execute(request) };
    if let Err(error) = engine.cleanup() {
        return error;
    }
    match outcome {
        Ok(value) => {
            unsafe {
                result.write(value);
            }
            0
        }
        Err(error) => error,
    }
}

const PROCESS: u64 = 1;
const TIMEOUT: u64 = 2;
const IO: u64 = 3;
const TEXT_LIMIT: u64 = 4;
const CAPACITY: usize = 1_048_577;
const EINTR: isize = -4;
const EAGAIN: isize = -11;

#[inline(always)]
fn syscall(number: usize, a: usize, b: usize, c: usize, d: usize, e: usize, f: usize) -> isize {
    let result: isize;
    unsafe {
        core::arch::asm!("syscall", inlateout("rax") number => result,
            in("rdi") a, in("rsi") b, in("rdx") c, in("r10") d, in("r8") e, in("r9") f,
            lateout("rcx") _, lateout("r11") _, options(nostack));
    }
    result
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

fn monotonic() -> Result<u64, u64> {
    let mut time = [0_u64; 2];
    if syscall(228, 1, time.as_mut_ptr() as usize, 0, 0, 0, 0) < 0 {
        return Err(IO);
    }
    time[0]
        .checked_mul(1_000_000_000)
        .and_then(|n| n.checked_add(time[1]))
        .ok_or(IO)
}

struct Engine {
    // Parent/child pairs: stdin socket, stdout pipe, stderr pipe, exec-error pipe.
    fds: [i32; 8],
    pid: i32,
    old_mask: Option<u64>,
}
impl Engine {
    fn close(&mut self, index: usize) {
        let fd = core::mem::replace(&mut self.fds[index], -1);
        if fd >= 0 {
            // On Linux even an interrupted close releases the descriptor: never retry.
            syscall(3, fd as usize, 0, 0, 0, 0, 0);
        }
    }
    fn pair(&mut self, index: usize, socket: bool) -> Result<(), u64> {
        let mut pair = [-1_i32; 2];
        let result = if socket {
            syscall(53, 1, 1 | 0x80000, 0, pair.as_mut_ptr() as usize, 0, 0)
        } else {
            syscall(293, pair.as_mut_ptr() as usize, 0x80000, 0, 0, 0, 0)
        };
        if result < 0 {
            return Err(IO);
        }
        self.fds[index] = pair[0];
        self.fds[index + 1] = pair[1];
        for offset in index..index + 2 {
            if self.fds[offset] < 4 {
                let normalized = syscall(72, self.fds[offset] as usize, 1030, 4, 0, 0, 0);
                if normalized < 0 {
                    return Err(IO);
                }
                self.close(offset);
                self.fds[offset] = normalized as i32;
            }
        }
        let flags = syscall(72, self.fds[index] as usize, 3, 0, 0, 0, 0);
        if flags < 0
            || syscall(
                72,
                self.fds[index] as usize,
                4,
                flags as usize | 0x800,
                0,
                0,
                0,
            ) < 0
        {
            return Err(IO);
        }
        Ok(())
    }
    fn restore_mask(&mut self) -> Result<(), u64> {
        if let Some(mask) = self.old_mask {
            if syscall(14, 2, &mask as *const u64 as usize, 0, 8, 0, 0) < 0 {
                return Err(IO);
            }
            self.old_mask = None;
        }
        Ok(())
    }
    fn observe(&mut self) -> Result<Option<i64>, u64> {
        let mut info = [0_u64; 16];
        let result = syscall(
            247,
            1,
            self.pid as usize,
            info.as_mut_ptr() as usize,
            0x1000005,
            0,
            0,
        );
        if result == EINTR {
            return Ok(None);
        }
        if result < 0 {
            if result == -10 {
                self.pid = 0;
            } // External reaper: PID is no longer ours to signal.
            return Err(IO);
        }
        let words = info.as_ptr().cast::<i32>();
        let (pid, code, status) = unsafe {
            (
                words.add(4).read(),
                words.add(2).read(),
                words.add(6).read(),
            )
        };
        if pid == 0 {
            return Ok(None);
        }
        match code {
            1 => Ok(Some(i64::from(status))),      // CLD_EXITED
            2 | 3 => Ok(Some(-i64::from(status))), // CLD_KILLED/DUMPED
            _ => Err(IO),
        }
    }
    fn cleanup(&mut self) -> Result<(), u64> {
        let mut result = self.restore_mask();
        for index in 0..self.fds.len() {
            self.close(index);
        }
        if self.pid > 0 {
            // Recheck ownership immediately before signaling. WNOWAIT leaves a
            // dead child as a zombie, pinning its PID until our final wait4.
            if self.observe().is_err() {
                result = Err(IO);
            }
            if self.pid > 0 {
                let group = syscall(62, (-(self.pid as isize)) as usize, 9, 0, 0, 0, 0);
                if group < 0 && group != -3 {
                    result = Err(IO);
                }
                let direct = syscall(62, self.pid as usize, 9, 0, 0, 0, 0);
                if direct < 0 && direct != -3 {
                    result = Err(IO);
                }
                loop {
                    let waited = syscall(61, self.pid as usize, 0, 0, 0, 0, 0);
                    if waited == EINTR {
                        continue;
                    }
                    if waited != self.pid as isize {
                        result = Err(IO);
                    }
                    self.pid = 0;
                    break;
                }
            }
        }
        result
    }
    unsafe fn execute(&mut self, request: &ProcessRequest) -> Result<ProcessResult, u64> {
        let start = monotonic()?;
        let deadline = start
            .checked_add(request.timeout_ms * 1_000_000)
            .ok_or(IO)?;
        let mut action = [0_u64; 4];
        if syscall(13, 17, 0, action.as_mut_ptr() as usize, 8, 0, 0) < 0
            || action[0] == 1
            || action[1] & 2 != 0
        {
            return Err(IO);
        }
        for index in [0, 2, 4, 6] {
            self.pair(index, index == 0)?;
        }
        if monotonic()? >= deadline {
            return Err(TIMEOUT);
        }
        let blocked = u64::MAX;
        let mut old_mask = 0_u64;
        if syscall(
            14,
            0,
            &blocked as *const u64 as usize,
            &mut old_mask as *mut u64 as usize,
            8,
            0,
            0,
        ) < 0
        {
            return Err(IO);
        }
        self.old_mask = Some(old_mask);
        let pid = syscall(57, 0, 0, 0, 0, 0, 0);
        if pid == 0 {
            unsafe { child(request, &self.fds) }
        }
        if pid < 0 {
            return Err(IO);
        }
        self.pid = pid as i32;
        self.restore_mask()?;
        // Both sides set the group, closing the fork/exec race. EACCES means
        // the child has already exec'd after successfully setting its group.
        let grouped = syscall(109, pid as usize, pid as usize, 0, 0, 0, 0);
        if grouped < 0 && grouped != -13 && grouped != -3 {
            return Err(IO);
        }
        for index in [1, 3, 5, 7] {
            self.close(index);
        }
        let mut sent = 0_usize;
        let mut lengths = [0_usize; 2];
        let mut status = None;
        loop {
            if monotonic()? >= deadline {
                return Err(TIMEOUT);
            }
            let mut progressed = false;
            if self.fds[0] >= 0 {
                if sent == request.input_len as usize {
                    self.close(0);
                } else {
                    let remaining = (request.input_len as usize - sent).min(65_536);
                    let pointer = unsafe { request.input.add(sent) };
                    let written = syscall(
                        44,
                        self.fds[0] as usize,
                        pointer as usize,
                        remaining,
                        0x4040,
                        0,
                        0,
                    );
                    if written > 0 {
                        sent += written as usize;
                        progressed = true;
                    } else if written == -32 || written == -104 {
                        self.close(0);
                    } else if written != EINTR && written != EAGAIN {
                        return Err(IO);
                    }
                }
            }
            for (stream, index) in [(0, 2), (1, 4)] {
                if self.fds[index] < 0 {
                    continue;
                }
                let pointer = if stream == 0 {
                    request.stdout
                } else {
                    request.stderr
                };
                let remaining = (CAPACITY - lengths[stream]).min(65_536);
                let read = syscall(
                    0,
                    self.fds[index] as usize,
                    unsafe { pointer.add(lengths[stream]) } as usize,
                    remaining,
                    0,
                    0,
                    0,
                );
                if read > 0 {
                    lengths[stream] += read as usize;
                    if lengths[stream] == CAPACITY {
                        return Err(TEXT_LIMIT);
                    }
                    progressed = true;
                } else if read == 0 {
                    self.close(index);
                } else if read != EINTR && read != EAGAIN {
                    return Err(IO);
                }
            }
            if self.fds[6] >= 0 {
                let mut byte = 0_u8;
                let read = syscall(
                    0,
                    self.fds[6] as usize,
                    &mut byte as *mut u8 as usize,
                    1,
                    0,
                    0,
                    0,
                );
                if read > 0 {
                    return Err(PROCESS);
                }
                if read == 0 {
                    self.close(6);
                } else if read != EINTR && read != EAGAIN {
                    return Err(IO);
                }
            }
            if status.is_none() {
                status = self.observe()?;
            }
            if let Some(status) = status
                && self.fds[2] < 0
                && self.fds[4] < 0
                && self.fds[6] < 0
            {
                return Ok(ProcessResult {
                    stdout_len: lengths[0] as u64,
                    stderr_len: lengths[1] as u64,
                    status,
                });
            }
            let now = monotonic()?;
            if now >= deadline {
                return Err(TIMEOUT);
            }
            let timeout = if progressed {
                0
            } else {
                ((deadline - now).div_ceil(1_000_000)).min(10)
            };
            let mut poll = [
                PollFd {
                    fd: self.fds[0],
                    events: 4,
                    revents: 0,
                },
                PollFd {
                    fd: self.fds[2],
                    events: 1,
                    revents: 0,
                },
                PollFd {
                    fd: self.fds[4],
                    events: 1,
                    revents: 0,
                },
                PollFd {
                    fd: self.fds[6],
                    events: 1,
                    revents: 0,
                },
            ];
            let polled = syscall(
                7,
                poll.as_mut_ptr() as usize,
                poll.len(),
                timeout as usize,
                0,
                0,
                0,
            );
            if polled < 0 && polled != EINTR {
                return Err(IO);
            }
            if poll.iter().any(|p| p.revents & 0x20 != 0) {
                return Err(IO);
            }
        }
    }
}

// This post-fork path uses only raw syscalls and fixed-size stack data. In
// particular it never touches Rust allocation, libc/TLS locks, or formatting.
unsafe fn child(request: &ProcessRequest, fds: &[i32; 8]) -> ! {
    let mut error_fd = fds[7];
    if syscall(292, error_fd as usize, 3, 0x80000, 0, 0, 0) < 0 {
        child_error(error_fd);
    }
    error_fd = 3;
    if syscall(109, 0, 0, 0, 0, 0, 0) < 0 {
        child_error(error_fd);
    }
    for (source, destination) in [(fds[1], 0), (fds[3], 1), (fds[5], 2)] {
        if syscall(292, source as usize, destination, 0, 0, 0, 0) < 0 {
            child_error(error_fd);
        }
    }
    if syscall(436, 4, u32::MAX as usize, 0, 0, 0, 0) < 0 {
        child_error(error_fd);
    }
    let default = [0_u64; 4];
    for signal in 1..=64 {
        if signal != 9
            && signal != 19
            && syscall(13, signal, default.as_ptr() as usize, 0, 8, 0, 0) < 0
        {
            child_error(error_fd);
        }
    }
    let empty = 0_u64;
    if syscall(14, 2, &empty as *const u64 as usize, 0, 8, 0, 0) < 0 {
        child_error(error_fd);
    }
    let environment = [core::ptr::null::<u8>()];
    syscall(
        59,
        request.executable as usize,
        request.argv as usize,
        environment.as_ptr() as usize,
        0,
        0,
        0,
    );
    child_error(error_fd)
}
fn child_error(fd: i32) -> ! {
    let byte = 1_u8;
    loop {
        if syscall(1, fd as usize, &byte as *const u8 as usize, 1, 0, 0, 0) != EINTR {
            break;
        }
    }
    unsafe {
        core::arch::asm!("syscall", in("rax") 60_usize, in("rdi") 127_usize, options(noreturn));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    // Signal dispositions and wait(-1) are process-wide. Keep the tests that
    // inspect those outside the cargo harness's other concurrently running tests.
    fn isolated() -> bool {
        let thread = std::thread::current();
        let name = thread.name().expect("named test thread");
        if std::env::var("AIL_PROCESS_HOST_TEST").as_deref() == Ok(name) {
            return false;
        }
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name, "--nocapture"])
            .env("AIL_PROCESS_HOST_TEST", name)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        true
    }

    fn run(
        path: &str,
        args: &[&str],
        input: &[u8],
        timeout: u64,
    ) -> (u64, ProcessResult, Vec<u8>, Vec<u8>) {
        let path = CString::new(path).unwrap();
        let strings: Vec<_> = args.iter().map(|s| CString::new(*s).unwrap()).collect();
        let mut argv = vec![path.as_ptr().cast::<u8>()];
        argv.extend(strings.iter().map(|s| s.as_ptr().cast::<u8>()));
        argv.push(core::ptr::null());
        let mut stdout = vec![0; 1_048_577];
        let mut stderr = vec![0; 1_048_577];
        let request = ProcessRequest {
            executable: path.as_ptr().cast(),
            argv: argv.as_ptr(),
            input: input.as_ptr(),
            input_len: input.len() as u64,
            timeout_ms: timeout,
            stdout: stdout.as_mut_ptr(),
            stderr: stderr.as_mut_ptr(),
            capacity: stdout.len() as u64,
        };
        let mut result = ProcessResult {
            stdout_len: 71,
            stderr_len: 72,
            status: 73,
        };
        let status = unsafe { ail_process_run(&request, &mut result) };
        if status == 0 {
            stdout.truncate(result.stdout_len as usize);
            stderr.truncate(result.stderr_len as usize);
        }
        (status, result, stdout, stderr)
    }

    #[test]
    fn host_process_streams_status_and_abi() {
        assert_eq!(core::mem::size_of::<ProcessRequest>(), 64);
        assert_eq!(core::mem::size_of::<ProcessResult>(), 24);
        let (code, result, out, err) = run(
            "/usr/bin/python3",
            &[
                "-c",
                "import os; os.write(1,os.read(0,100)); os.write(2,b'error'); raise SystemExit(7)",
            ],
            b"hello\0\n",
            2000,
        );
        assert_eq!(code, 0);
        assert_eq!(result.status, 7);
        assert_eq!(out, b"hello\0\n");
        assert_eq!(err, b"error");
    }

    #[test]
    fn host_process_failure_result_is_unchanged() {
        let (code, result, _, _) = run("/does/not/exist", &[], b"", 2000);
        assert_eq!(code, 1);
        assert_eq!(
            result,
            ProcessResult {
                stdout_len: 71,
                stderr_len: 72,
                status: 73
            }
        );
        assert_eq!(
            run(
                "/usr/bin/python3",
                &["-c", "import time; time.sleep(5)"],
                b"",
                20
            )
            .0,
            2
        );
        assert_eq!(
            run(
                "/usr/bin/python3",
                &["-c", "raise SystemExit(127)"],
                b"",
                2000
            )
            .1
            .status,
            127
        );
    }

    #[test]
    fn host_process_concurrent_streams_and_early_close() {
        let input = vec![b'i'; 350_000];
        let script = "import os; os.write(1,b'o'*200000); os.write(2,b'e'*200000); data=b''\nwhile True:\n b=os.read(0,65536)\n if not b: break\n data+=b\nos.write(1,data)";
        let (code, result, out, err) = run("/usr/bin/python3", &["-c", script], &input, 2000);
        assert_eq!(code, 0);
        assert_eq!(result.status, 0);
        assert_eq!(out, [vec![b'o'; 200_000], input.clone()].concat());
        assert_eq!(err, vec![b'e'; 200_000]);
        let (code, result, out, _) = run(
            "/bin/sh",
            &["-c", "exec 0<&-; printf closed; exit 17"],
            &vec![b'x'; 1_048_576],
            2000,
        );
        assert_eq!((code, result.status, out), (0, 17, b"closed".to_vec()));
    }

    #[test]
    fn host_process_capture_bound_environment_signals_and_arguments() {
        for (bytes, expected) in [(1_048_576, 0), (1_048_577, 4)] {
            let script = format!("import os; os.write(1,b'x'*{bytes})");
            let (code, result, out, _) = run("/usr/bin/python3", &["-c", &script], b"", 2000);
            assert_eq!(code, expected);
            if code == 0 {
                assert_eq!(out.len(), bytes);
            } else {
                assert_eq!(result.status, 73);
            }
        }
        assert!(run("/usr/bin/env", &[], b"", 2000).2.is_empty());
        assert_eq!(
            run("/bin/sh", &["-c", "kill -PIPE $$"], b"", 2000).1.status,
            -13
        );
        let (_, _, out, _) = run(
            "/usr/bin/printf",
            &["[%s][%s]", "", "$(no shell);*"],
            b"",
            2000,
        );
        assert_eq!(out, b"[][$(no shell);*]");
    }

    #[test]
    fn host_process_descendant_timeout_and_no_fd_leaks() {
        if isolated() {
            return;
        }
        let before = std::fs::read_dir("/proc/self/fd").unwrap().count();
        let started = std::time::Instant::now();
        // The leader exits while a descendant retains both capture descriptors.
        assert_eq!(run("/bin/sh", &["-c", "sleep 10 & exit 0"], b"", 30).0, 2);
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        for _ in 0..20 {
            assert_eq!(run("/bin/true", &[], b"", 2000).0, 0);
            assert_eq!(run("/does/not/exist", &[], b"", 2000).0, 1);
        }
        assert_eq!(std::fs::read_dir("/proc/self/fd").unwrap().count(), before);
        // There are no direct children waiting for our library caller to reap.
        assert_eq!(syscall(61, usize::MAX, 0, 1, 0, 0, 0), -10);
    }

    #[test]
    fn host_process_child_has_no_inherited_descriptors_or_blocked_signals() {
        let inherited = syscall(72, 1, 0, 100, 0, 0, 0); // F_DUPFD, deliberately inheritable.
        assert!(inherited >= 100);
        let script = format!(
            "import os\ntry: os.fstat({inherited}); os.write(1,b'leaked')\nexcept OSError: os.write(1,b'closed')"
        );
        let outcome = run("/usr/bin/python3", &["-c", &script], b"", 2000);
        syscall(3, inherited as usize, 0, 0, 0, 0, 0);
        assert_eq!(outcome.0, 0);
        assert_eq!(outcome.2, b"closed");

        let blocked = 1_u64 << 9; // SIGUSR1 is blocked in the calling thread only.
        let mut original = 0_u64;
        assert_eq!(
            syscall(
                14,
                0,
                &blocked as *const u64 as usize,
                &mut original as *mut u64 as usize,
                8,
                0,
                0
            ),
            0
        );
        let outcome = run("/bin/sh", &["-c", "kill -USR1 $$"], b"", 2000);
        let mut retained = 0_u64;
        let queried = syscall(14, 0, 0, &mut retained as *mut u64 as usize, 8, 0, 0);
        let restored = syscall(14, 2, &original as *const u64 as usize, 0, 8, 0, 0);
        assert_eq!((queried, restored), (0, 0));
        assert_eq!(retained, original | blocked);
        assert_eq!((outcome.0, outcome.1.status), (0, -10));
    }

    #[test]
    fn host_process_rejects_sigchld_auto_reaping_without_modifying_disposition() {
        if isolated() {
            return;
        }
        for action in [[1_u64, 0, 0, 0], [0, 2, 0, 0]] {
            let mut original = [0_u64; 4];
            assert_eq!(
                syscall(
                    13,
                    17,
                    action.as_ptr() as usize,
                    original.as_mut_ptr() as usize,
                    8,
                    0,
                    0
                ),
                0
            );
            let outcome = run("/bin/true", &[], b"", 2000);
            let mut retained = [0_u64; 4];
            let queried = syscall(13, 17, 0, retained.as_mut_ptr() as usize, 8, 0, 0);
            let restored = syscall(13, 17, original.as_ptr() as usize, 0, 8, 0, 0);
            assert_eq!((queried, restored), (0, 0));
            assert_eq!(retained, action);
            assert_eq!(outcome.0, IO);
            assert_eq!(outcome.1.status, 73);
        }
    }
}
