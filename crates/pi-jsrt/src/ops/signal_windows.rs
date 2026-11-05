//! Windows console-control events lowered to Node-compatible SIGINT and SIGBREAK.
//! The console callback only enqueues an event; all JS dispatch happens on the worker thread.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Once, OnceLock, mpsc};

use crate::engine::{self, OpFn, Poster, arg_f64, js_str, state};
use crate::js::{Context, JsResult, JsValue, js_string};
use parking_lot::Mutex;
use super::coded_error;

const SIGINT: i32 = 2;
const SIGBREAK: i32 = 21;
static ACTIVE_INT: AtomicUsize = AtomicUsize::new(0);
static ACTIVE_BREAK: AtomicUsize = AtomicUsize::new(0);
static IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
static SENDER: OnceLock<mpsc::SyncSender<i32>> = OnceLock::new();
static START: Once = Once::new();
static STARTED: AtomicBool = AtomicBool::new(false);
static SUBSCRIBERS: Mutex<Vec<Subscriber>> = Mutex::new(Vec::new());

struct Subscriber { signo: i32, runtime: usize, poster: Poster }

pub(crate) fn in_flight() -> bool { IN_FLIGHT.load(Ordering::SeqCst) > 0 }

unsafe extern "system" fn console_handler(event: u32) -> i32 {
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT};
    let sig = match event { CTRL_C_EVENT => SIGINT, CTRL_BREAK_EVENT => SIGBREAK, _ => return 0 };
    let active = if sig == SIGINT { &ACTIVE_INT } else { &ACTIVE_BREAK };
    if active.load(Ordering::Acquire) == 0 { return 0; }
    let Some(tx) = SENDER.get() else { return 0 };
    IN_FLIGHT.fetch_add(1, Ordering::SeqCst);
    if tx.try_send(sig).is_err() { IN_FLIGHT.fetch_sub(1, Ordering::SeqCst); }
    1
}

fn start_dispatcher() -> bool {
    START.call_once(|| {
        let (tx, rx) = mpsc::sync_channel::<i32>(64);
        if unsafe { windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(console_handler), 1) } == 0 { return; }
        let _ = SENDER.set(tx);
        if std::thread::Builder::new().name("pi-jsrt-console-events".into()).spawn(move || {
            while let Ok(sig) = rx.recv() {
                let posts: Vec<_> = SUBSCRIBERS.lock().iter().filter(|s| s.signo == sig).map(|s| s.poster.clone()).collect();
                for poster in posts {
                    poster.post(move |ctx| {
                        let Some(internal) = engine::try_internal(ctx) else { return };
                        let Ok(f) = internal.get(js_string!("signalEvent"), ctx) else { return };
                        let Some(f) = f.as_function() else { return };
                        if let Err(e) = f.call(&JsValue::undefined(), &[JsValue::from(sig)], ctx) { engine::report_uncaught(ctx, e); }
                    });
                }
                IN_FLIGHT.fetch_sub(1, Ordering::SeqCst);
            }
        }).is_ok() {
            STARTED.store(true, Ordering::Release);
        } else {
            unsafe { windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(console_handler), 0); }
        }
    });
    STARTED.load(Ordering::Acquire)
}

fn runtime_key(ctx: &Context) -> usize { std::sync::Arc::as_ptr(&state(ctx).shared) as usize }

fn op_signal_subscribe(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let sig = arg_f64(args, 0, ctx)? as i32;
    if sig != SIGINT && sig != SIGBREAK {
        return Err(coded_error(ctx, "Error", "uv_signal_start EINVAL", &[("errno", JsValue::from(-libc::EINVAL)), ("code", js_str("EINVAL")), ("syscall", js_str("uv_signal_start"))]));
    }
    if !start_dispatcher() {
        return Err(coded_error(ctx, "Error", "uv_signal_start EIO", &[("errno", JsValue::from(-libc::EIO)), ("code", js_str("EIO")), ("syscall", js_str("uv_signal_start"))]));
    }
    SUBSCRIBERS.lock().push(Subscriber { signo: sig, runtime: runtime_key(ctx), poster: state(ctx).poster() });
    if sig == SIGINT { ACTIVE_INT.fetch_add(1, Ordering::SeqCst); } else { ACTIVE_BREAK.fetch_add(1, Ordering::SeqCst); }
    Ok(JsValue::undefined())
}

fn op_signal_unsubscribe(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let sig = arg_f64(args, 0, ctx)? as i32;
    let me = runtime_key(ctx);
    let mut subs = SUBSCRIBERS.lock();
    if let Some(i) = subs.iter().position(|s| s.signo == sig && s.runtime == me) {
        subs.remove(i);
        if sig == SIGINT { ACTIVE_INT.fetch_sub(1, Ordering::SeqCst); } else if sig == SIGBREAK { ACTIVE_BREAK.fetch_sub(1, Ordering::SeqCst); }
    }
    Ok(JsValue::undefined())
}

pub(crate) fn teardown(ctx: &mut Context) {
    let me = runtime_key(ctx);
    SUBSCRIBERS.lock().retain(|s| {
        if s.runtime == me {
            if s.signo == SIGINT { ACTIVE_INT.fetch_sub(1, Ordering::SeqCst); } else { ACTIVE_BREAK.fetch_sub(1, Ordering::SeqCst); }
            false
        } else { true }
    });
}

pub(crate) const OPS: &[(&str, usize, OpFn)] = &[
    ("signalSubscribe", 1, op_signal_subscribe),
    ("signalUnsubscribe", 1, op_signal_unsubscribe),
];
