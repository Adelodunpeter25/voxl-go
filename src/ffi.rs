//! C ABI for embedding voxl in other languages (the Go module in go/). See include/voxl.h.
//!
//! A session runs the capture -> convert -> x264 pipeline and input injection without the
//! HTTP/WebSocket server; encoded frames are handed to a callback instead of a socket. The
//! frame bytes use the same wire format the browser client receives (see video.rs).

use crate::hub::{Hub, Out};
use crate::input::InputEvent;
use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const VOXL_OK: i32 = 0;
pub const VOXL_ERR_ARG: i32 = -1;
pub const VOXL_ERR_STATE: i32 = -2;
pub const VOXL_ERR_INIT: i32 = -3;

/// Kinds passed to the callback.
pub const VOXL_FRAME_VIDEO: u32 = 1;
pub const VOXL_FRAME_TEXT: u32 = 2;

#[repr(C)]
pub struct VoxlConfig {
    /// X11 display (":0"), or the macOS display index; NULL for the default.
    pub display: *const c_char,
    /// x264 preset; NULL for "ultrafast".
    pub preset: *const c_char,
    /// 0 selects the default for each numeric field.
    pub fps: u32,
    pub max_kbps: u32,
    pub min_kbps: u32,
    pub threads: u32,
    pub crf: f32,
    /// Non-zero lets voxl_session_resize change the desktop size.
    pub enable_resize: u8,
}

pub type VoxlFrameCallback =
    extern "C" fn(data: *const u8, len: usize, kind: u32, user_data: *mut c_void);

pub struct VoxlSession {
    cfg: crate::video::VideoConfig,
    resize: bool,
    hub: Arc<Hub>,
    started: AtomicBool,
    stop: Arc<AtomicBool>,
    client: parking_lot::Mutex<Option<u64>>,
    pump: parking_lot::Mutex<Option<std::thread::JoinHandle<()>>>,
    buttons: AtomicU32,
}

struct UserData(*mut c_void);
// The caller promises user_data may be used from the callback's thread.
unsafe impl Send for UserData {}

unsafe fn opt_str(p: *const c_char) -> Option<String> {
    (!p.is_null()).then(|| CStr::from_ptr(p).to_string_lossy().into_owned())
}

/// Creates a session; nothing is captured until voxl_session_start. NULL config uses
/// defaults. Returns NULL on invalid input.
#[no_mangle]
pub unsafe extern "C" fn voxl_session_create(config: *const VoxlConfig) -> *mut VoxlSession {
    let c = config.as_ref();
    let or = |v: Option<u32>, d: u32| v.filter(|&n| n != 0).unwrap_or(d);
    let ncpu = std::thread::available_parallelism().map_or(4, |n| n.get()) as u32;
    let cfg = crate::video::VideoConfig {
        display: c.and_then(|c| opt_str(c.display)),
        #[cfg(target_os = "macos")]
        enable_resize: c.is_some_and(|c| c.enable_resize != 0),
        fps: or(c.map(|c| c.fps), 60),
        preset: c
            .and_then(|c| opt_str(c.preset))
            .unwrap_or_else(|| "ultrafast".into()),
        threads: or(c.map(|c| c.threads), (ncpu / 4).clamp(1, 4)),
        convert_threads: 4,
        crf: c.map(|c| c.crf).filter(|&v| v > 0.0).unwrap_or(24.0),
        max_kbps: or(c.map(|c| c.max_kbps), 30000),
        paintover_crf: 16.0,
        paintover_burst: 3,
        paintover_delay_ms: 150,
        min_kbps: or(c.map(|c| c.min_kbps), 2000),
        coalesce: Duration::ZERO,
        stripes: 1,
        adaptive_threads: false,
    };
    Box::into_raw(Box::new(VoxlSession {
        cfg,
        resize: c.is_some_and(|c| c.enable_resize != 0),
        hub: Hub::new(),
        started: AtomicBool::new(false),
        stop: Arc::new(AtomicBool::new(false)),
        client: parking_lot::Mutex::new(None),
        pump: parking_lot::Mutex::new(None),
        buttons: AtomicU32::new(0),
    }))
}

/// Starts capture and encoding; encoded frames are passed to `callback` on a voxl thread
/// (the data is only valid during the call). A session can be started once.
#[no_mangle]
pub unsafe extern "C" fn voxl_session_start(
    session: *mut VoxlSession,
    callback: Option<VoxlFrameCallback>,
    user_data: *mut c_void,
) -> i32 {
    let (Some(s), Some(cb)) = (session.as_ref(), callback) else {
        return VOXL_ERR_ARG;
    };
    if s.started.swap(true, Ordering::SeqCst) {
        return VOXL_ERR_STATE;
    }
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();
    let display = s.cfg.display.clone();
    let screen = if s.resize {
        crate::screen::Screen::open(display.as_deref()).ok()
    } else {
        None
    };
    if let Err(e) = crate::video::spawn(s.cfg.clone(), s.hub.clone(), screen) {
        log::error!("video init failed: {e}");
        s.started.store(false, Ordering::SeqCst);
        return VOXL_ERR_INIT;
    }
    let mode = crate::desktop::ClipboardMode {
        to_server: false,
        to_client: false,
    };
    let clipboard = crate::desktop::spawn(display.clone(), s.hub.clone(), mode).ok();
    match crate::input::spawn(display, clipboard, 180.0) {
        Ok(tx) => *s.hub.input.lock() = Some(tx),
        Err(e) => log::error!("input init failed: {e}"),
    }
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let client = s.hub.add_client(tx);
    client.video.store(true, Ordering::Relaxed);
    *s.client.lock() = Some(client.id);
    s.hub.wake();
    let stop = s.stop.clone();
    let ud = UserData(user_data);
    let mut rx = rx;
    *s.pump.lock() = std::thread::Builder::new()
        .name("voxl-ffi".into())
        .spawn(move || {
            let ud = ud;
            while !stop.load(Ordering::Relaxed) {
                match rx.try_recv() {
                    Ok(Out::Bin(b)) => cb(b.as_ptr(), b.len(), VOXL_FRAME_VIDEO, ud.0),
                    Ok(Out::Text(t)) => cb(t.as_ptr(), t.len(), VOXL_FRAME_TEXT, ud.0),
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                        std::thread::sleep(Duration::from_millis(1))
                    }
                    Err(_) => break,
                }
                // Acknowledge so the congestion controller doesn't throttle a consumer
                // that has no network round trip.
                if let Some(id) = client_ack(&client) {
                    client.on_ack(id);
                }
            }
        })
        .ok();
    VOXL_OK
}

fn client_ack(c: &crate::hub::Client) -> Option<u32> {
    c.inflight.lock().back().map(|f| f.id)
}

/// Stops frame delivery: after it returns the callback is no longer called. Safe to call
/// more than once. (The capture threads are released when the process exits; see ffi docs.)
#[no_mangle]
pub unsafe extern "C" fn voxl_session_stop(session: *mut VoxlSession) -> i32 {
    let Some(s) = session.as_ref() else {
        return VOXL_ERR_ARG;
    };
    s.stop.store(true, Ordering::SeqCst);
    if let Some(id) = s.client.lock().take() {
        s.hub.remove_client(id);
        s.hub.wake();
    }
    if let Some(h) = s.pump.lock().take() {
        let _ = h.join();
    }
    if let Some(tx) = s.hub.input.lock().as_ref() {
        let _ = tx.send(InputEvent::ReleaseAll);
    }
    VOXL_OK
}

fn send(s: &VoxlSession, ev: InputEvent) -> i32 {
    match s.hub.input.lock().as_ref() {
        Some(tx) if tx.send(ev).is_ok() => VOXL_OK,
        _ => VOXL_ERR_STATE,
    }
}

/// Pointer position in desktop pixels, `button_mask` the buttons now held (bit 0 left,
/// 1 middle, 2 right, 3 back, 4 forward; changes become press/release events) and `wheel`
/// notches (positive scrolls up, negative down).
#[no_mangle]
pub unsafe extern "C" fn voxl_session_send_mouse(
    session: *mut VoxlSession,
    x: i32,
    y: i32,
    button_mask: u32,
    wheel: i32,
) -> i32 {
    let Some(s) = session.as_ref() else {
        return VOXL_ERR_ARG;
    };
    let mut rc = send(s, InputEvent::Move { x, y });
    let old = s.buttons.swap(button_mask, Ordering::Relaxed);
    // X11 button numbers: 1 left, 2 middle, 3 right, 8 back, 9 forward.
    for (bit, button) in [(0u32, 1u8), (1, 2), (2, 3), (3, 8), (4, 9)] {
        let (was, now) = (old >> bit & 1, button_mask >> bit & 1);
        if was != now {
            rc = send(s, InputEvent::Button { button, down: now == 1 });
        }
    }
    if wheel != 0 {
        rc = send(s, InputEvent::Scroll { dx: 0, dy: -wheel });
    }
    rc
}

/// An X11 keysym press (`down` non-zero) or release.
#[no_mangle]
pub unsafe extern "C" fn voxl_session_send_key(
    session: *mut VoxlSession,
    keysym: u32,
    down: i32,
) -> i32 {
    match session.as_ref() {
        Some(s) => send(s, InputEvent::Key { keysym, down: down != 0 }),
        None => VOXL_ERR_ARG,
    }
}

/// Asks for a new desktop size (needs enable_resize); the size is clamped to what the
/// display supports and the stream restarts with a keyframe.
#[no_mangle]
pub unsafe extern "C" fn voxl_session_resize(
    session: *mut VoxlSession,
    width: u32,
    height: u32,
) -> i32 {
    let Some(s) = session.as_ref() else {
        return VOXL_ERR_ARG;
    };
    let Some(max) = *s.hub.resize_max.lock() else {
        return VOXL_ERR_STATE;
    };
    let want = crate::screen::fit(width, height, max);
    let cur = (
        s.hub.width.load(Ordering::Relaxed),
        s.hub.height.load(Ordering::Relaxed),
    );
    *s.hub.resize_req.lock() = (want != cur).then_some(want);
    s.hub.wake();
    VOXL_OK
}

/// Stops the session and frees it. The pointer is invalid afterwards.
#[no_mangle]
pub unsafe extern "C" fn voxl_session_destroy(session: *mut VoxlSession) {
    if session.is_null() {
        return;
    }
    voxl_session_stop(session);
    drop(Box::from_raw(session));
}
