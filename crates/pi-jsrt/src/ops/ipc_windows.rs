//! Full duplex Node child-process IPC over native Windows named pipes.
//!
//! Windows has no Unix descriptor passing, so the channel itself is a named pipe whose name is
//! handed to the child in `NODE_CHANNEL_FD`. Socket transfer uses a separate `WSADuplicateSocket`
//! path and returns an explicit feature error until that native handle protocol is implemented.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::js::{Context, JsArgs, JsResult, JsValue, js_string};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions};
use tokio::sync::mpsc;

use super::{bytes_arg, bytes_to_js};
use crate::engine::{self, OpFn, arg_f64, arg_string, js_str, state};

enum Out { Data(Vec<u8>), Close }
struct Channel { tx: mpsc::UnboundedSender<Out>, pending: std::sync::Arc<std::sync::atomic::AtomicUsize> }

thread_local! { static CHANNELS: RefCell<HashMap<u64, Channel>> = RefCell::new(HashMap::new()); }

fn dispatch(ctx: &mut Context, id: u64, args: Vec<JsValue>) {
    let Some(internal) = engine::try_internal(ctx) else { return };
    let Ok(f) = internal.get(js_string!("ipcEvent"), ctx) else { return };
    let Some(f) = f.as_function() else { return };
    let mut all = vec![JsValue::from(id as f64)];
    all.extend(args);
    if let Err(e) = f.call(&JsValue::undefined(), &all, ctx) { engine::report_uncaught(ctx, e); }
}

/// Create the listening end before the child is spawned so its startup connection cannot race
/// listener creation. The name is private to this process and unique across concurrent forks.
pub(crate) fn prepare_pipe() -> std::io::Result<(String, NamedPipeServer)> {
    let name = format!(r"\\.\pipe\pi-jsrt-{}-{}", std::process::id(), uuid::Uuid::new_v4().simple());
    let server = ServerOptions::new().first_pipe_instance(true).create(&name)?;
    Ok((name, server))
}

fn register<S>(ctx: &mut Context, stream: S) -> u64
where S: AsyncRead + AsyncWrite + Unpin + Send + 'static {
    let st = state(ctx);
    let id = st.next_id();
    let poster = st.poster();
    let tokio = st.tokio.clone();
    let (tx, rx) = mpsc::unbounded_channel::<Out>();
    let pending = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    CHANNELS.with(|c| c.borrow_mut().insert(id, Channel { tx, pending: pending.clone() }));
    start_tasks(&tokio, poster, id, stream, rx, pending);
    id
}

fn start_tasks<S>(tokio: &tokio::runtime::Handle, poster: crate::engine::Poster, id: u64, stream: S,
                  mut rx: mpsc::UnboundedReceiver<Out>, pending: std::sync::Arc<std::sync::atomic::AtomicUsize>)
where S: AsyncRead + AsyncWrite + Unpin + Send + 'static {
    let (mut reader, mut writer) = tokio::io::split(stream);
    let read_poster = poster.clone();
    tokio.spawn(async move {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let chunk = buf[..n].to_vec();
                    read_poster.post(move |ctx| if let Ok(b) = bytes_to_js(&chunk, ctx) { dispatch(ctx, id, vec![js_str("data"), b]); });
                }
            }
        }
        read_poster.post(move |ctx| {
            CHANNELS.with(|c| c.borrow_mut().remove(&id));
            dispatch(ctx, id, vec![js_str("close")]);
        });
    });
    tokio.spawn(async move {
        while let Some(out) = rx.recv().await {
            match out {
                Out::Data(data) => {
                    if writer.write_all(&data).await.is_err() { return; }
                    pending.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                }
                Out::Close => { let _ = writer.shutdown().await; return; }
            }
        }
    });
}

/// Start the parent side of a child IPC channel after `CreateProcess` succeeds.
pub(crate) fn start_server(ctx: &mut Context, server: NamedPipeServer) -> u64 {
    let st = state(ctx);
    let id = st.next_id();
    let poster = st.poster();
    let tokio = st.tokio.clone();
    let io = tokio.clone();
    let (tx, rx) = mpsc::unbounded_channel::<Out>();
    let pending = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    CHANNELS.with(|c| c.borrow_mut().insert(id, Channel { tx, pending: pending.clone() }));
    tokio.spawn(async move {
        match tokio::time::timeout(std::time::Duration::from_secs(30), server.connect()).await {
            Ok(Ok(())) => start_tasks(&io, poster, id, server, rx, pending),
            _ => { let _ = poster.post(move |ctx| {
                CHANNELS.with(|c| c.borrow_mut().remove(&id));
                dispatch(ctx, id, vec![js_str("close")]);
            }); }
        }
    });
    id
}

fn op_ipc_open(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let name = arg_string(args, 0, ctx)?;
    let client = ClientOptions::new().open(&name).map_err(|e| super::throw_io(ctx, &e, "open", None))?;
    Ok(JsValue::from(register(ctx, client) as f64))
}

fn op_ipc_write(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = arg_f64(args, 0, ctx)? as u64;
    let data = bytes_arg(args.get_or_undefined(1), ctx)?;
    if !args.get_or_undefined(2).is_null_or_undefined() {
        return Err(super::coded_error(ctx, "Error", "Windows IPC socket transfer is not yet available", &[("code", js_str("ERR_FEATURE_UNAVAILABLE")), ("syscall", js_str("sendHandle"))]));
    }
    let ok = CHANNELS.with(|c| c.borrow().get(&id).is_some_and(|ch| {
        ch.pending.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if ch.tx.send(Out::Data(data)).is_ok() { true } else {
            ch.pending.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            false
        }
    }));
    Ok(JsValue::from(ok))
}

fn op_ipc_close(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = arg_f64(args, 0, ctx)? as u64;
    CHANNELS.with(|c| if let Some(ch) = c.borrow_mut().remove(&id) { let _ = ch.tx.send(Out::Close); });
    Ok(JsValue::undefined())
}

fn op_close_fd(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> { Ok(JsValue::undefined()) }
pub(crate) fn teardown(_ctx: &mut Context) {
    CHANNELS.with(|c| for (_, ch) in c.borrow_mut().drain() { let _ = ch.tx.send(Out::Close); });
}

pub(crate) const OPS: &[(&str, usize, OpFn)] = &[
    ("ipcOpen", 1, op_ipc_open), ("ipcWrite", 3, op_ipc_write),
    ("ipcClose", 1, op_ipc_close), ("closeFd", 1, op_close_fd),
];
