//! `process` / `os` ops and the static configuration object.

use crate::js::{Context, JsArgs, JsObject, JsResult, JsString, JsValue, js_string};

use super::{array, object};
use crate::engine::{OpFn, arg_f64, arg_opt_string, js_str, state};

#[cfg(unix)]
fn hostname() -> String {
    let mut buf = [0u8; 256];
    // SAFETY: buffer is valid for its length; gethostname NUL-terminates on success.
    let r = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) };
    if r != 0 {
        return "localhost".into();
    }
    let end = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

#[cfg(windows)]
fn hostname() -> String {
    use windows_sys::Win32::System::WindowsProgramming::GetComputerNameW;

    let mut buf = [0u16; 256];
    let mut len = buf.len() as u32;
    // SAFETY: buf is writable for len UTF-16 code units and len is initialized.
    if unsafe { GetComputerNameW(buf.as_mut_ptr(), &mut len) } == 0 {
        return "localhost".into();
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

struct Uname {
    sysname: String,
    release: String,
    version: String,
    machine: String,
}

#[cfg(unix)]
fn uname() -> Uname {
    // SAFETY: utsname is plain data; uname fills it.
    let mut u: libc::utsname = unsafe { std::mem::zeroed() };
    let ok = unsafe { libc::uname(&mut u) } == 0;
    let f = |s: &[libc::c_char]| {
        let bytes: Vec<u8> = s.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
        String::from_utf8_lossy(&bytes).into_owned()
    };
    if !ok {
        return Uname {
            sysname: std::env::consts::OS.into(),
            release: String::new(),
            version: String::new(),
            machine: std::env::consts::ARCH.into(),
        };
    }
    Uname {
        sysname: f(&u.sysname),
        release: f(&u.release),
        version: f(&u.version),
        machine: f(&u.machine),
    }
}

#[cfg(windows)]
fn uname() -> Uname {
    use windows_sys::Win32::System::SystemInformation::{GetVersionExW, OSVERSIONINFOW};

    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
        ..Default::default()
    };
    // SAFETY: info has the size declared in dwOSVersionInfoSize and remains writable.
    let ok = unsafe { GetVersionExW(&mut info) } != 0;
    Uname {
        sysname: "Windows_NT".into(),
        release: if ok { format!("{}.{}", info.dwMajorVersion, info.dwMinorVersion) } else { String::new() },
        version: if ok { info.dwBuildNumber.to_string() } else { String::new() },
        machine: std::env::consts::ARCH.into(),
    }
}

fn node_platform() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    }
}

fn node_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "x86" => "ia32",
        other => other,
    }
}

#[cfg(unix)]
fn parent_pid() -> u32 { unsafe { libc::getppid() as u32 } }

#[cfg(windows)]
fn parent_pid() -> u32 {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS};
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    // SAFETY: the snapshot is a kernel-owned handle, and entry is initialized with its ABI size.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() || snapshot == INVALID_HANDLE_VALUE { return 0; }
    let pid = unsafe { GetCurrentProcessId() };
    let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
    let mut parent = 0;
    unsafe {
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                if entry.th32ProcessID == pid { parent = entry.th32ParentProcessID; break; }
                if Process32NextW(snapshot, &mut entry) == 0 { break; }
            }
        }
        CloseHandle(snapshot);
    }
    parent
}

/// Static configuration passed to the glue as `ops.config`.
pub(crate) fn config_object(ctx: &mut Context) -> JsResult<JsValue> {
    let st = state(ctx);
    let shared = st.shared.clone();
    let env = JsObject::with_object_proto(ctx.intrinsics());
    let mut keys: Vec<_> = if shared.options.node_compat { shared.env.iter().collect() } else { Vec::new() };
    keys.sort();
    for (k, v) in keys {
        env.set(JsString::from(k.as_str()), js_str(v), true, ctx)?;
    }
    let argv: Vec<JsValue> = shared.options.argv.iter().map(|a| js_str(a)).collect();
    let argv = array(ctx, argv);
    let exec_argv: Vec<JsValue> = shared.options.exec_argv.iter().map(|a| js_str(a)).collect();
    let exec_argv = array(ctx, exec_argv);
    let node_exec: Vec<JsValue> = match &shared.options.node_exec {
        Some(v) => v.iter().map(|a| js_str(a)).collect(),
        None => vec![
            js_str(&std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default()),
            js_str(crate::node::NODE_MARKER),
        ],
    };
    let node_exec = array(ctx, node_exec);
    let exec_path = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
    let home = dirs::home_dir().map(|p| p.display().to_string()).unwrap_or_else(|| "/".into());
    let tmp = std::env::temp_dir().display().to_string();
    let tmp = if tmp.len() > 1 { tmp.trim_end_matches('/').to_string() } else { tmp };
    let u = uname();
    object(
        ctx,
        vec![
            ("nodeCompat", JsValue::from(shared.options.node_compat)),
            ("env", env.into()),
            ("argv", argv),
            ("execArgv", exec_argv),
            ("nodeExec", node_exec),
            ("cwd", js_str(&shared.cwd.display().to_string())),
            ("platform", js_str(node_platform())),
            ("arch", js_str(node_arch())),
            ("pid", JsValue::from(std::process::id())),
            ("ppid", JsValue::from(parent_pid())),
            ("execPath", js_str(&exec_path)),
            ("hostname", js_str(&hostname())),
            ("homedir", js_str(&home)),
            ("tmpdir", js_str(&tmp)),
            ("osType", js_str(&u.sysname)),
            ("osRelease", js_str(&u.release)),
            ("osVersion", js_str(&u.version)),
            ("machine", js_str(&u.machine)),
            ("version", js_str(env!("CARGO_PKG_VERSION"))),
            ("hasExitHandler", JsValue::from(shared.options.on_exit.is_some())),
            ("stdin", JsValue::from(shared.options.stdin)),
        ],
    )
}

fn op_exit(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let code = args.get_or_undefined(0).to_number(ctx).unwrap_or(0.0) as i32;
    let st = state(ctx);
    if let Some(h) = &st.shared.options.on_exit {
        h(code);
    }
    Ok(JsValue::undefined())
}

#[cfg(windows)]
fn windows_process_kill(pid: u32, sig: i32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE};
    if sig == 0 {
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if process.is_null() { return false; }
        let mut code = 0;
        let ok = unsafe { GetExitCodeProcess(process, &mut code) } != 0 && code == STILL_ACTIVE as u32;
        unsafe { CloseHandle(process) };
        return ok;
    }
    let process = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
    if process.is_null() { return false; }
    let ok = unsafe { windows_sys::Win32::System::Threading::TerminateProcess(process, 1) } != 0;
    unsafe { CloseHandle(process) };
    ok
}

#[cfg(windows)]
fn windows_std_handle(fd: i32) -> windows_sys::Win32::Foundation::HANDLE {
    use windows_sys::Win32::System::Console::{GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};
    let kind = match fd { 0 => STD_INPUT_HANDLE, 1 => STD_OUTPUT_HANDLE, 2 => STD_ERROR_HANDLE, _ => return std::ptr::null_mut() };
    unsafe { GetStdHandle(kind) }
}

fn op_kill(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let pid = arg_f64(args, 0, ctx)? as i32;
    let sig = args.get_or_undefined(1).to_number(ctx).unwrap_or(15.0) as i32;
    #[cfg(unix)]
    let failed = unsafe { libc::kill(pid, sig) } != 0;
    #[cfg(windows)]
    let failed = !windows_process_kill(pid as u32, sig);
    if failed {
        // Node's uvException: "kill ESRCH" with errno/code/syscall.
        let e = std::io::Error::last_os_error();
        let code = super::io_code(&e);
        return Err(super::coded_error(
            ctx,
            "Error",
            &format!("kill {code}"),
            &[("errno", JsValue::from(-e.raw_os_error().unwrap_or(0))), ("code", js_str(code)), ("syscall", js_str("kill"))],
        ));
    }
    Ok(JsValue::from(true))
}

fn op_isatty(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let fd = arg_f64(args, 0, ctx)? as i32;
    #[cfg(unix)]
    let is_tty = unsafe { libc::isatty(fd) } == 1;
    #[cfg(windows)]
    let is_tty = {
        use windows_sys::Win32::System::Console::GetConsoleMode;
        let handle = windows_std_handle(fd);
        let mut mode = 0;
        !handle.is_null() && unsafe { GetConsoleMode(handle, &mut mode) } != 0
    };
    Ok(JsValue::from(is_tty))
}

fn op_term_size(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let fd = arg_f64(args, 0, ctx)? as i32;
    #[cfg(unix)]
    {
    // SAFETY: winsize is plain data; ioctl fills it.
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let r = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) };
    if r != 0 || ws.ws_col == 0 {
        return Ok(JsValue::null());
    }
    let cols = JsValue::from(ws.ws_col as u32);
    let rows = JsValue::from(ws.ws_row as u32);
    Ok(array(ctx, vec![cols, rows]))
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{CONSOLE_SCREEN_BUFFER_INFO, GetConsoleScreenBufferInfo};
        let handle = windows_std_handle(fd);
        let mut info = CONSOLE_SCREEN_BUFFER_INFO::default();
        if handle.is_null() || unsafe { GetConsoleScreenBufferInfo(handle, &mut info) } == 0 {
            return Ok(JsValue::null());
        }
        let cols = (info.srWindow.Right - info.srWindow.Left + 1).max(0) as u32;
        let rows = (info.srWindow.Bottom - info.srWindow.Top + 1).max(0) as u32;
        if cols == 0 { return Ok(JsValue::null()); }
        Ok(array(ctx, vec![JsValue::from(cols), JsValue::from(rows)]))
    }
}

fn op_memory(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    #[cfg(unix)]
    let rss = std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1).and_then(|v| v.parse::<f64>().ok()))
        .map(|pages| pages * 4096.0)
        .unwrap_or(0.0);
    #[cfg(windows)]
    let rss = {
        use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        let mut counters = PROCESS_MEMORY_COUNTERS { cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32, ..Default::default() };
        let process = unsafe { GetCurrentProcess() };
        if unsafe { GetProcessMemoryInfo(process, &mut counters, counters.cb) } != 0 { counters.WorkingSetSize as f64 } else { 0.0 }
    };
    object(
        ctx,
        vec![
            ("rss", JsValue::from(rss)),
            ("heapTotal", JsValue::from(0)),
            ("heapUsed", JsValue::from(0)),
            ("external", JsValue::from(0)),
            ("arrayBuffers", JsValue::from(0)),
        ],
    )
}

fn meminfo(key: &str) -> f64 {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with(key))
                .and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse::<f64>().ok()))
        })
        .map(|kb| kb * 1024.0)
        .unwrap_or(0.0)
}

fn op_os_info(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    #[cfg(unix)]
    let model = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1).map(|v| v.trim().to_string()))
        })
        .unwrap_or_else(|| "unknown".into());
    #[cfg(windows)]
    let model = "unknown".to_string();
    #[cfg(unix)]
    let uptime = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next().and_then(|v| v.parse::<f64>().ok()))
        .unwrap_or(0.0);
    #[cfg(windows)]
    let uptime = unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() as f64 / 1000.0 };
    #[cfg(unix)]
    let load = std::fs::read_to_string("/proc/loadavg")
        .ok()
        .map(|s| s.split_whitespace().take(3).filter_map(|v| v.parse::<f64>().ok()).collect::<Vec<_>>())
        .unwrap_or_default();
    #[cfg(windows)]
    let load = vec![0.0; 3];
    let load: Vec<JsValue> = (0..3).map(|i| JsValue::from(load.get(i).copied().unwrap_or(0.0))).collect();
    let load = array(ctx, load);
    #[cfg(unix)]
    let (uid, gid) = unsafe { (JsValue::from(libc::getuid()), JsValue::from(libc::getgid())) };
    #[cfg(windows)]
    let (uid, gid) = (JsValue::null(), JsValue::null());
    let user = std::env::var("USER").or_else(|_| std::env::var("LOGNAME")).or_else(|_| std::env::var("USERNAME")).unwrap_or_default();
    #[cfg(unix)]
    let shell = std::env::var("SHELL").unwrap_or_default();
    #[cfg(windows)]
    let shell = std::env::var("COMSPEC").unwrap_or_default();
    #[cfg(unix)]
    let (totalmem, freemem) = (meminfo("MemTotal:"), meminfo("MemAvailable:"));
    #[cfg(windows)]
    let (totalmem, freemem) = {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        let mut status = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
        if unsafe { GlobalMemoryStatusEx(&mut status) } != 0 { (status.ullTotalPhys as f64, status.ullAvailPhys as f64) } else { (0.0, 0.0) }
    };
    object(
        ctx,
        vec![
            ("cpus", JsValue::from(cpus as u32)),
            ("cpuModel", js_str(&model)),
            ("uptime", JsValue::from(uptime)),
            ("loadavg", load),
            ("totalmem", JsValue::from(totalmem)),
            ("freemem", JsValue::from(freemem)),
            ("uid", uid),
            ("gid", gid),
            ("username", js_str(&user)),
            ("shell", js_str(&shell)),
        ],
    )
}

fn op_cpu_usage(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    #[cfg(unix)]
    {
    // SAFETY: rusage is plain data.
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut ru) };
    let user = ru.ru_utime.tv_sec as f64 * 1e6 + ru.ru_utime.tv_usec as f64;
    let system = ru.ru_stime.tv_sec as f64 * 1e6 + ru.ru_stime.tv_usec as f64;
    object(ctx, vec![("user", JsValue::from(user)), ("system", JsValue::from(system))])
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::FILETIME;
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        let ok = unsafe { GetProcessTimes(GetCurrentProcess(), &mut creation, &mut exit, &mut kernel, &mut user) } != 0;
        let micros = |f: FILETIME| ((u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime)) as f64 / 10.0;
        object(ctx, vec![("user", JsValue::from(if ok { micros(user) } else { 0.0 })), ("system", JsValue::from(if ok { micros(kernel) } else { 0.0 }))])
    }
}

/// `ops.setEnv(name, value | undefined)`: `process.env` reports every assignment and deletion.
/// `TZ` sets the runtime's timezone, as Node's does; deleting it returns to the host's.
fn op_set_env(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let name = arg_opt_string(args, 0, ctx)?;
    let value = arg_opt_string(args, 1, ctx)?;
    if name.as_deref() == Some("TZ") {
        *crate::engine::state(ctx).timezone.borrow_mut() = value.as_deref().map(crate::engine::timezone_named);
    }
    Ok(JsValue::undefined())
}

// ---------------------------------------------------------------------------
// process.stdin (Node mode): one blocking read of fd 0 per request, on a reader thread
// ---------------------------------------------------------------------------

static STDIN_READER: std::sync::Mutex<Option<std::sync::mpsc::Sender<crate::engine::Poster>>> = std::sync::Mutex::new(None);

/// `ops.stdinRead()`: read one chunk of fd 0; the result arrives as `internal.stdinEvent(kind,
/// bytes)` with kind "data", "end" or "error".
fn op_stdin_read(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let st = state(ctx);
    if !st.shared.options.stdin {
        return Ok(JsValue::from(false));
    }
    let poster = st.poster();
    let mut guard = STDIN_READER.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        let (tx, rx) = std::sync::mpsc::channel::<crate::engine::Poster>();
        let spawned = std::thread::Builder::new().name("pi-jsrt-stdin".into()).spawn(move || {
            use std::io::Read;
            let stdin = std::io::stdin();
            let mut stdin = stdin.lock();
            let mut buf = vec![0u8; 64 * 1024];
            while let Ok(poster) = rx.recv() {
                let (kind, data, errno): (&'static str, Vec<u8>, i32) = match stdin.read(&mut buf) {
                    Ok(0) => ("end", Vec::new(), 0),
                    Ok(n) => ("data", buf[..n].to_vec(), 0),
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted || e.kind() == std::io::ErrorKind::WouldBlock => ("retry", Vec::new(), 0),
                    Err(e) => ("error", Vec::new(), e.raw_os_error().unwrap_or(libc::EIO)),
                };
                poster.post(move |ctx| {
                    let Some(internal) = crate::engine::try_internal(ctx) else { return };
                    let Ok(f) = internal.get(js_string!("stdinEvent"), ctx) else { return };
                    let Some(f) = f.as_function() else { return };
                    let payload = if kind == "error" {
                        let e = std::io::Error::from_raw_os_error(errno);
                        crate::error::to_js(&super::io_error(&e, "read", None), ctx).into_opaque(ctx).unwrap_or_default()
                    } else {
                        super::bytes_to_js(&data, ctx).unwrap_or_default()
                    };
                    if let Err(e) = f.call(&JsValue::undefined(), &[js_str(kind), payload], ctx) {
                        crate::engine::report_uncaught(ctx, e);
                    }
                });
            }
        });
        if spawned.is_err() {
            return Ok(JsValue::from(false));
        }
        *guard = Some(tx);
    }
    let ok = guard.as_ref().is_some_and(|tx| tx.send(poster).is_ok());
    Ok(JsValue::from(ok))
}

/// The terminal settings before the first `setRawMode(true)`, restored at exit.
#[cfg(unix)]
static ORIGINAL_TERMIOS: std::sync::Mutex<Option<libc::termios>> = std::sync::Mutex::new(None);
#[cfg(windows)]
static ORIGINAL_CONSOLE_MODE: std::sync::Mutex<Option<u32>> = std::sync::Mutex::new(None);

/// `ops.setRawMode(fd, raw) -> errno`: libuv's UV_TTY_MODE_RAW / UV_TTY_MODE_NORMAL.
fn op_set_raw_mode(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    #[cfg(unix)]
    let fd = arg_f64(args, 0, ctx)? as i32;
    let raw = args.get_or_undefined(1).to_boolean();
    if !state(ctx).shared.options.stdin {
        return Ok(JsValue::from(0));
    }
    #[cfg(unix)]
    let mut orig = ORIGINAL_TERMIOS.lock().unwrap_or_else(|e| e.into_inner());
    // SAFETY: termios is plain data filled by tcgetattr.
    #[cfg(unix)]
    unsafe {
        let mut current: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(fd, &mut current) != 0 {
            return Ok(JsValue::from(-std::io::Error::last_os_error().raw_os_error().unwrap_or(libc::EIO)));
        }
        if orig.is_none() {
            *orig = Some(current);
        }
        let mut next = orig.unwrap_or(current);
        if raw {
            next.c_iflag &= !(libc::BRKINT | libc::ICRNL | libc::INPCK | libc::ISTRIP | libc::IXON);
            next.c_oflag |= libc::ONLCR;
            next.c_cflag |= libc::CS8;
            next.c_lflag &= !(libc::ECHO | libc::ICANON | libc::IEXTEN | libc::ISIG);
            next.c_cc[libc::VMIN] = 1;
            next.c_cc[libc::VTIME] = 0;
        }
        if libc::tcsetattr(fd, libc::TCSADRAIN, &next) != 0 {
            return Ok(JsValue::from(-std::io::Error::last_os_error().raw_os_error().unwrap_or(libc::EIO)));
        }
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{GetConsoleMode, GetStdHandle, SetConsoleMode, ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, STD_INPUT_HANDLE};
        let mut orig = ORIGINAL_CONSOLE_MODE.lock().unwrap_or_else(|e| e.into_inner());
        let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        let mut current = 0u32;
        if handle.is_null() || unsafe { GetConsoleMode(handle, &mut current) } == 0 {
            return Ok(JsValue::from(-std::io::Error::last_os_error().raw_os_error().unwrap_or(libc::EIO)));
        }
        if orig.is_none() { *orig = Some(current); }
        let next = if raw {
            orig.unwrap_or(current) & !(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT)
        } else {
            orig.unwrap_or(current)
        };
        if unsafe { SetConsoleMode(handle, next) } == 0 {
            return Ok(JsValue::from(-std::io::Error::last_os_error().raw_os_error().unwrap_or(libc::EIO)));
        }
    }
    Ok(JsValue::from(0))
}

/// Restore the terminal if `setRawMode` changed it (Node's uv_tty_reset_mode at exit).
pub(crate) fn reset_tty_mode() {
    #[cfg(unix)]
    if let Some(orig) = ORIGINAL_TERMIOS.lock().unwrap_or_else(|e| e.into_inner()).take() {
        // SAFETY: restoring settings read from the same descriptor.
        unsafe { libc::tcsetattr(0, libc::TCSADRAIN, &orig) };
    }
    #[cfg(windows)]
    if let Some(mode) = ORIGINAL_CONSOLE_MODE.lock().unwrap_or_else(|e| e.into_inner()).take() {
        use windows_sys::Win32::System::Console::{GetStdHandle, SetConsoleMode, STD_INPUT_HANDLE};
        let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        if !handle.is_null() { unsafe { SetConsoleMode(handle, mode) }; }
    }
}

pub(crate) const OPS: &[(&str, usize, OpFn)] = &[
    ("stdinRead", 0, op_stdin_read),
    ("setRawMode", 2, op_set_raw_mode),
    ("exit", 1, op_exit),
    ("kill", 2, op_kill),
    ("isatty", 1, op_isatty),
    ("termSize", 1, op_term_size),
    ("memoryUsage", 0, op_memory),
    ("osInfo", 0, op_os_info),
    ("cpuUsage", 0, op_cpu_usage),
    ("setEnv", 2, op_set_env),
];
