//! Native Windows console implementation of the process terminal.

use super::*;
use crate::keys::set_kitty_protocol_active;
use crate::program_status::{
    PROGRAM_STATUS_QUERY, ProgramState, format_program_status, is_program_status_reply,
};
use crate::stdin_buffer::{StdinBuffer, StdinBufferOptions, StdinEvent};
use std::io::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Instant;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows_sys::Win32::System::Console::{
    CONSOLE_SCREEN_BUFFER_INFO, ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT,
    ENABLE_PROCESSED_OUTPUT, ENABLE_VIRTUAL_TERMINAL_INPUT, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
    GetConsoleMode, GetConsoleScreenBufferInfo, GetStdHandle, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    SetConsoleMode,
};
use windows_sys::Win32::System::Threading::{CreateEventW, SetEvent, WaitForMultipleObjects};

const KITTY_QUERY: &str = "\x1b[>7u\x1b[?u";
const DEVICE_ATTRIBUTES_QUERY: &str = "\x1b[c";
const MODIFY_OTHER_KEYS_ON: &str = "\x1b[>4;2m";
const MODIFY_OTHER_KEYS_OFF: &str = "\x1b[>4;0m";
const PROGRESS_ON: &str = "\x1b]9;4;3\x07";
const PROGRESS_OFF: &str = "\x1b]9;4;0\x07";

#[derive(Default)]
struct State {
    input_mode: Option<u32>,
    output_mode: Option<u32>,
    input_handler: Option<InputHandler>,
    resize_handler: Option<ResizeHandler>,
    stdin_buffer: Option<StdinBuffer>,
    kitty_active: bool,
    modify_other_keys_active: bool,
    keyboard_protocol_pushed: bool,
    pending_device_attributes: u32,
    negotiation_buffer: String,
    negotiation_deadline: Option<Instant>,
    program_status: Option<ProgramStatus>,
    program_status_supported: bool,
    program_status_query_pending: bool,
    progress: Option<(Arc<AtomicBool>, JoinHandle<()>)>,
    reader: Option<(isize, JoinHandle<()>)>,
}

struct Inner {
    state: std::sync::Mutex<State>,
    input: isize,
    output: isize,
    console_input: bool,
    console_output: bool,
    write_log_path: Option<std::path::PathBuf>,
    epoch: Instant,
    last_data_ms: std::sync::atomic::AtomicU64,
}

#[derive(Clone)]
pub struct ProcessTerminal {
    inner: Arc<Inner>,
}

impl Default for ProcessTerminal {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessTerminal {
    pub fn new() -> Self {
        let input = unsafe { GetStdHandle(STD_INPUT_HANDLE) } as isize;
        let output = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) } as isize;
        let mut mode = 0;
        let console_input =
            input != 0 && unsafe { GetConsoleMode(input as HANDLE, &mut mode) } != 0;
        let console_output =
            output != 0 && unsafe { GetConsoleMode(output as HANDLE, &mut mode) } != 0;
        let write_log_path = std::env::var_os("PI_TUI_WRITE_LOG")
            .map(std::path::PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty());
        Self {
            inner: Arc::new(Inner {
                state: std::sync::Mutex::new(State::default()),
                input,
                output,
                console_input,
                console_output,
                write_log_path,
                epoch: Instant::now(),
                last_data_ms: std::sync::atomic::AtomicU64::new(0),
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.inner.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn raw_write(&self, data: &str) {
        write_handle(self.inner.output as HANDLE, data.as_bytes());
    }

    pub fn modify_other_keys_active(&self) -> bool {
        self.lock().modify_other_keys_active
    }

    fn enable_modify_other_keys(&self, state: &mut State) {
        if !state.kitty_active && !state.modify_other_keys_active {
            self.raw_write(MODIFY_OTHER_KEYS_ON);
            state.modify_other_keys_active = true;
        }
    }

    fn disable_modify_other_keys(&self, state: &mut State) {
        if state.modify_other_keys_active {
            self.raw_write(MODIFY_OTHER_KEYS_OFF);
            state.modify_other_keys_active = false;
        }
    }

    fn write_program_status(&self, state: &State) {
        if state.program_status_supported
            && let Some(status) = &state.program_status
        {
            self.raw_write(&format_program_status(status));
        }
    }

    fn handle_events(&self, state: &mut State, events: Vec<StdinEvent>) -> Vec<String> {
        let mut forward = Vec::new();
        for event in events {
            match event {
                StdinEvent::Paste(content) => forward.push(format!("\x1b[200~{content}\x1b[201~")),
                StdinEvent::Data(sequence) => {
                    if is_program_status_reply(&sequence) {
                        if state.program_status_query_pending {
                            state.program_status_query_pending = false;
                            state.program_status_supported = true;
                            self.write_program_status(state);
                        }
                        continue;
                    }
                    let buffered = if state.negotiation_buffer.is_empty() {
                        sequence.clone()
                    } else {
                        format!("{}{}", state.negotiation_buffer, sequence)
                    };
                    if let Some(parsed) = parse_keyboard_protocol_negotiation_sequence(&buffered) {
                        state.negotiation_buffer.clear();
                        state.negotiation_deadline = None;
                        match parsed {
                            KeyboardProtocolNegotiationSequence::KittyFlags(flags) => {
                                if flags == 0 {
                                    self.enable_modify_other_keys(state);
                                } else {
                                    self.disable_modify_other_keys(state);
                                    state.kitty_active = true;
                                    set_kitty_protocol_active(true);
                                }
                            }
                            KeyboardProtocolNegotiationSequence::DeviceAttributes => {
                                state.pending_device_attributes =
                                    state.pending_device_attributes.saturating_sub(1);
                                if state.pending_device_attributes == 0 {
                                    state.program_status_query_pending = false;
                                }
                                if !state.kitty_active {
                                    self.enable_modify_other_keys(state);
                                }
                            }
                        }
                    } else if is_keyboard_protocol_negotiation_sequence_prefix(&buffered) {
                        state.negotiation_buffer = buffered;
                        state.negotiation_deadline =
                            Some(Instant::now() + Duration::from_millis(150));
                    } else {
                        if !state.negotiation_buffer.is_empty() {
                            forward.push(std::mem::take(&mut state.negotiation_buffer));
                        }
                        state.negotiation_deadline = None;
                        forward.push(sequence);
                    }
                }
            }
        }
        forward
    }

    fn forward(&self, sequences: Vec<String>) {
        let handler = self.lock().input_handler.clone();
        if let Some(handler) = handler {
            for sequence in sequences {
                let detect = sequence == "\r";
                let shift = detect
                    && crate::native_platform::is_native_modifier_pressed(
                        crate::native_platform::ModifierKey::Shift,
                    );
                handler(normalize_native_shift_enter_input(&sequence, detect, shift));
            }
        }
    }

    fn reader_loop(self, stop_event_token: isize) {
        let stop_event = stop_event_token as HANDLE;
        let mut carry = Vec::new();
        let mut bytes = [0u8; 8192];
        let mut previous_size = window_size(self.inner.output as HANDLE);
        loop {
            let (timeout, input_handler_present, parser_deadline) = {
                let state = self.lock();
                let mut deadline = state.stdin_buffer.as_ref().and_then(StdinBuffer::deadline);
                if let Some(d) = state.negotiation_deadline {
                    deadline = Some(deadline.map_or(d, |p| p.min(d)));
                }
                let timeout = deadline
                    .map(|d| {
                        d.saturating_duration_since(Instant::now())
                            .as_millis()
                            .min(50) as u32
                    })
                    .unwrap_or(50);
                (timeout, state.input_handler.is_some(), deadline)
            };
            let handles = [stop_event, self.inner.input as HANDLE];
            let count = if self.inner.console_input { 2 } else { 1 };
            let result = unsafe { WaitForMultipleObjects(count, handles.as_ptr(), 0, timeout) };
            if result == WAIT_OBJECT_0 {
                break;
            }

            let size = window_size(self.inner.output as HANDLE);
            if size != previous_size {
                previous_size = size;
                if let Some(handler) = self.lock().resize_handler.clone() {
                    handler();
                }
            }

            let now = Instant::now();
            let mut forward = Vec::new();
            {
                let mut state = self.lock();
                if parser_deadline.is_some_and(|d| now >= d) && !state.negotiation_buffer.is_empty()
                {
                    state.negotiation_deadline = None;
                    forward.push(std::mem::take(&mut state.negotiation_buffer));
                }
                let events = state
                    .stdin_buffer
                    .as_mut()
                    .map(|b| b.poll_timeout(now))
                    .unwrap_or_default();
                forward.extend(self.handle_events(&mut state, events));
            }
            self.forward(forward);

            if self.inner.console_input
                && count == 2
                && result == WAIT_OBJECT_0 + 1
                && input_handler_present
            {
                let mut read = 0u32;
                let ok = unsafe {
                    ReadFile(
                        self.inner.input as HANDLE,
                        bytes.as_mut_ptr(),
                        bytes.len() as u32,
                        &mut read,
                        std::ptr::null_mut(),
                    )
                };
                if ok == 0 {
                    let error = std::io::Error::last_os_error();
                    notify_stdin_error(&error);
                    break;
                }
                if read == 0 {
                    notify_stdin_end();
                    break;
                }
                self.inner.last_data_ms.store(
                    self.inner.epoch.elapsed().as_millis() as u64,
                    Ordering::SeqCst,
                );
                carry.extend_from_slice(&bytes[..read as usize]);
                let valid = match std::str::from_utf8(&carry) {
                    Ok(_) => carry.len(),
                    Err(e) if e.error_len().is_none() => e.valid_up_to(),
                    Err(_) => carry.len(),
                };
                let chunk: Vec<_> = carry.drain(..valid).collect();
                if !chunk.is_empty() {
                    let outputs = {
                        let mut state = self.lock();
                        if state.stdin_buffer.is_none() {
                            Vec::new()
                        } else {
                            let events = state
                                .stdin_buffer
                                .as_mut()
                                .map(|b| b.process(&String::from_utf8_lossy(&chunk)))
                                .unwrap_or_default();
                            self.handle_events(&mut state, events)
                        }
                    };
                    self.forward(outputs);
                }
            }
        }
    }

    fn clear_progress(state: &mut State) {
        if let Some((stop, handle)) = state.progress.take() {
            stop.store(true, Ordering::SeqCst);
            handle.thread().unpark();
        }
    }
}

impl Terminal for ProcessTerminal {
    fn start(&self, on_input: InputHandler, on_resize: ResizeHandler) {
        let mut state = self.lock();
        state.input_handler = Some(on_input);
        state.resize_handler = Some(on_resize);
        if self.inner.console_input {
            let mut old = 0;
            if unsafe { GetConsoleMode(self.inner.input as HANDLE, &mut old) } != 0 {
                state.input_mode.get_or_insert(old);
                let raw = (old & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT))
                    | ENABLE_VIRTUAL_TERMINAL_INPUT;
                unsafe {
                    SetConsoleMode(self.inner.input as HANDLE, raw);
                }
            }
        }
        if self.inner.console_output {
            let mut old = 0;
            if unsafe { GetConsoleMode(self.inner.output as HANDLE, &mut old) } != 0 {
                state.output_mode.get_or_insert(old);
                unsafe {
                    SetConsoleMode(
                        self.inner.output as HANDLE,
                        old | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
                    );
                }
            }
        }
        self.raw_write("\x1b[?2004h");
        state.stdin_buffer = Some(StdinBuffer::new(StdinBufferOptions {
            timeout: None,
            escape_timeout: Some(Duration::from_secs_f64(
                resolve_escape_timeout_ms(&|k| std::env::var(k).ok()) / 1000.0,
            )),
        }));
        state.keyboard_protocol_pushed = true;
        state.pending_device_attributes = 1;
        state.negotiation_buffer.clear();
        state.program_status = None;
        let override_ = std::env::var("PI_PROGRAM_STATUS").ok();
        state.program_status_supported = override_.as_deref() == Some("1");
        state.program_status_query_pending = !matches!(override_.as_deref(), Some("1") | Some("0"));
        let status_query = if state.program_status_query_pending {
            PROGRAM_STATUS_QUERY
        } else {
            ""
        };
        self.raw_write(&format!(
            "{KITTY_QUERY}{status_query}{DEVICE_ATTRIBUTES_QUERY}"
        ));
        if state.reader.is_none() && self.inner.console_input {
            let stop = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
            if !stop.is_null() {
                let me = self.clone();
                let stop_token = stop as isize;
                let thread = std::thread::Builder::new()
                    .name("pi-tui-windows-stdin".into())
                    .spawn(move || me.reader_loop(stop_token));
                if let Ok(thread) = thread {
                    state.reader = Some((stop as isize, thread));
                } else {
                    unsafe {
                        CloseHandle(stop);
                    }
                }
            }
        }
    }

    fn stop(&self) {
        let reader = {
            let mut state = self.lock();
            Self::clear_progress(&mut state);
            if state.program_status_supported && state.program_status.is_some() {
                self.raw_write(&format_program_status(&ProgramStatus::new(
                    ProgramState::Clear,
                )));
            }
            state.program_status_supported = false;
            state.program_status_query_pending = false;
            self.raw_write("\x1b[?2004l");
            if state.keyboard_protocol_pushed || state.kitty_active {
                self.raw_write("\x1b[<u");
            }
            state.keyboard_protocol_pushed = false;
            state.kitty_active = false;
            set_kitty_protocol_active(false);
            self.disable_modify_other_keys(&mut state);
            state.stdin_buffer = None;
            state.input_handler = None;
            state.resize_handler = None;
            state.reader.take()
        };
        if let Some((event, thread)) = reader {
            unsafe {
                SetEvent(event as HANDLE);
            }
            let _ = thread.join();
            unsafe {
                CloseHandle(event as HANDLE);
            }
        }
        let mut state = self.lock();
        if let Some(mode) = state.input_mode.take() {
            unsafe {
                SetConsoleMode(self.inner.input as HANDLE, mode);
            }
        }
        if let Some(mode) = state.output_mode.take() {
            unsafe {
                SetConsoleMode(self.inner.output as HANDLE, mode);
            }
        }
    }

    fn restore_on_exit(&self, before: &str, after: &str) {
        let state = self.lock();
        let mut sequence = before.to_string();
        if state.progress.is_some() {
            sequence.push_str(PROGRESS_OFF);
        }
        sequence.push_str("\x1b[?2004l");
        if state.keyboard_protocol_pushed || state.kitty_active {
            sequence.push_str("\x1b[<u");
        }
        if state.modify_other_keys_active {
            sequence.push_str(MODIFY_OTHER_KEYS_OFF);
        }
        sequence.push_str(after);
        write_handle(self.inner.output as HANDLE, sequence.as_bytes());
        if let Some(mode) = state.input_mode {
            unsafe {
                SetConsoleMode(self.inner.input as HANDLE, mode);
            }
        }
        if let Some(mode) = state.output_mode {
            unsafe {
                SetConsoleMode(self.inner.output as HANDLE, mode);
            }
        }
    }

    fn drain_input(&self, max: Duration, idle: Duration) {
        let previous = self.lock().input_handler.take();
        let start = Instant::now();
        let mut last = start;
        let mut prev_ms = self.inner.last_data_ms.load(Ordering::SeqCst);
        while start.elapsed() < max {
            std::thread::sleep(Duration::from_millis(10).min(max.saturating_sub(start.elapsed())));
            let next = self.inner.last_data_ms.load(Ordering::SeqCst);
            if next != prev_ms {
                prev_ms = next;
                last = Instant::now();
            }
            if last.elapsed() >= idle {
                break;
            }
        }
        self.lock().input_handler = previous;
    }

    fn write(&self, data: &str) {
        self.raw_write(data);
        if let Some(path) = &self.inner.write_log_path
            && let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
        {
            let _ = file.write_all(data.as_bytes());
        }
    }

    fn columns(&self) -> usize {
        let (cols, _) = window_size(self.inner.output as HANDLE);
        if cols > 0 {
            cols
        } else {
            env_number("COLUMNS").unwrap_or(80)
        }
    }
    fn rows(&self) -> usize {
        let (_, rows) = window_size(self.inner.output as HANDLE);
        if rows > 0 {
            rows
        } else {
            env_number("LINES").unwrap_or(24)
        }
    }
    fn kitty_protocol_active(&self) -> bool {
        self.lock().kitty_active
    }
    fn set_program_status(&self, status: &ProgramStatus) {
        let mut state = self.lock();
        state.program_status = if status.state == ProgramState::Clear {
            None
        } else {
            Some(status.clone())
        };
        if state.program_status_supported {
            self.raw_write(&format_program_status(status));
        }
    }
    fn set_progress(&self, active: bool) {
        let mut state = self.lock();
        if active {
            self.raw_write(PROGRESS_ON);
            if state.progress.is_none() {
                let stop = Arc::new(AtomicBool::new(false));
                let stop2 = stop.clone();
                let output = self.inner.output;
                let handle = std::thread::spawn(move || {
                    loop {
                        std::thread::park_timeout(Duration::from_secs(1));
                        if stop2.load(Ordering::SeqCst) {
                            break;
                        }
                        write_handle(output as HANDLE, PROGRESS_ON.as_bytes());
                    }
                });
                state.progress = Some((stop, handle));
            }
        } else {
            Self::clear_progress(&mut state);
            self.raw_write(PROGRESS_OFF);
        }
    }
}

fn write_handle(handle: HANDLE, mut data: &[u8]) {
    while !data.is_empty() {
        let mut written = 0;
        let ok = unsafe {
            WriteFile(
                handle,
                data.as_ptr(),
                data.len().min(u32::MAX as usize) as u32,
                &mut written,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 || written == 0 {
            notify_output_error(&std::io::Error::last_os_error());
            return;
        }
        data = &data[written as usize..];
    }
}

fn window_size(output: HANDLE) -> (usize, usize) {
    let mut info: CONSOLE_SCREEN_BUFFER_INFO = unsafe { std::mem::zeroed() };
    if unsafe { GetConsoleScreenBufferInfo(output, &mut info) } != 0 {
        let w = (info.srWindow.Right - info.srWindow.Left + 1).max(0) as usize;
        let h = (info.srWindow.Bottom - info.srWindow.Top + 1).max(0) as usize;
        return (w, h);
    }
    (0, 0)
}

fn env_number(name: &str) -> Option<usize> {
    let value: f64 = std::env::var(name).ok()?.trim().parse().ok()?;
    if value.is_finite() && value >= 1.0 {
        Some(value as usize)
    } else {
        None
    }
}
