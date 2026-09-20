//! Stable C ABI for linking Gromnie's Rust client into the iOS app.
//!
//! The ABI intentionally exposes commands and a JSON event queue, not protocol
//! objects or Rust callbacks. Swift owns an opaque session handle and drains
//! events from its dedicated serial queue.

mod event;
mod session;

use std::{
    ffi::{CStr, c_char},
    ptr,
    sync::Mutex,
    time::Duration,
};

use session::{Command, RunningSession};

// Deliberately not `#[repr(C)]`: the session is always used behind a pointer,
// and cbindgen emits only a forward declaration for a type without a
// guaranteed layout, keeping the Rust internals out of the C header.
#[allow(non_camel_case_types)]
pub struct gromnie_session_t {
    inner: Mutex<SessionState>,
}

enum SessionState {
    Idle,
    Running(RunningSession),
    Closed,
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultCode {
    Ok = 0,
    NoEvent = 1,
    InvalidArgument = 2,
    InvalidState = 3,
    QueueFull = 4,
    TimedOut = 5,
    InternalError = 6,
}

pub type SessionError = ResultCode;

const MAX_HOST_BYTES: usize = 255;
const MAX_ACCOUNT_BYTES: usize = 255;
const MAX_PASSWORD_BYTES: usize = 512;
const MAX_CHAT_BYTES: usize = 512;

/// Allocate a session handle. The returned handle must be destroyed exactly once.
#[unsafe(no_mangle)]
pub extern "C" fn gromnie_session_create() -> *mut gromnie_session_t {
    Box::into_raw(Box::new(gromnie_session_t {
        inner: Mutex::new(SessionState::Idle),
    }))
}

/// Start a direct-UDP Gromnie login. Network outcomes arrive through events.
///
/// # Safety
///
/// `session` must be a live handle returned by `gromnie_session_create`; all
/// strings must point to valid, NUL-terminated UTF-8 for this call's duration.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gromnie_session_connect(
    session: *mut gromnie_session_t,
    host_utf8: *const c_char,
    port: u16,
    username_utf8: *const c_char,
    password_utf8: *const c_char,
) -> i32 {
    catch_code(|| {
        let Some(session) = (unsafe { session.as_ref() }) else {
            return ResultCode::InvalidArgument;
        };
        if port == 0 {
            return ResultCode::InvalidArgument;
        }
        let Ok(host) = (unsafe { required_utf8(host_utf8, MAX_HOST_BYTES) }) else {
            return ResultCode::InvalidArgument;
        };
        let Ok(username) = (unsafe { required_utf8(username_utf8, MAX_ACCOUNT_BYTES) }) else {
            return ResultCode::InvalidArgument;
        };
        let Ok(password) = (unsafe { required_utf8(password_utf8, MAX_PASSWORD_BYTES) }) else {
            return ResultCode::InvalidArgument;
        };
        if !valid_host(&host) || username.is_empty() || password.is_empty() {
            return ResultCode::InvalidArgument;
        }

        let Ok(mut state) = session.inner.lock() else {
            return ResultCode::InternalError;
        };
        if !matches!(*state, SessionState::Idle | SessionState::Closed) {
            return ResultCode::InvalidState;
        }
        *state = SessionState::Running(session::start(host, port, username, password));
        ResultCode::Ok
    }) as i32
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `session` must be a live handle returned by `gromnie_session_create`.
pub unsafe extern "C" fn gromnie_session_select_character(
    session: *mut gromnie_session_t,
    character_id: u32,
) -> i32 {
    enqueue(session, Command::SelectCharacter(character_id)) as i32
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `session` must be live and `message_utf8` must point to NUL-terminated UTF-8
/// for this call's duration.
pub unsafe extern "C" fn gromnie_session_send_chat(
    session: *mut gromnie_session_t,
    message_utf8: *const c_char,
) -> i32 {
    catch_code(|| {
        let Ok(message) = (unsafe { required_utf8(message_utf8, MAX_CHAT_BYTES) }) else {
            return ResultCode::InvalidArgument;
        };
        if message.trim().is_empty() {
            return ResultCode::InvalidArgument;
        }
        enqueue(session, Command::SendChat(message))
    }) as i32
}

/// Wait for one event. A successful event is returned as an owned UTF-8 JSON buffer.
///
/// # Safety
///
/// `session` must be live and `json_utf8`/`json_len` must be writable pointers.
/// A successful buffer must be passed exactly once to `gromnie_buffer_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gromnie_session_next_event(
    session: *mut gromnie_session_t,
    timeout_ms: u32,
    json_utf8: *mut *mut u8,
    json_len: *mut usize,
) -> i32 {
    catch_code(|| {
        if json_utf8.is_null() || json_len.is_null() {
            return ResultCode::InvalidArgument;
        }
        unsafe {
            *json_utf8 = ptr::null_mut();
            *json_len = 0;
        }
        let Some(session) = (unsafe { session.as_ref() }) else {
            return ResultCode::InvalidArgument;
        };
        let Ok(state) = session.inner.lock() else {
            return ResultCode::InternalError;
        };
        let SessionState::Running(running) = &*state else {
            return ResultCode::InvalidState;
        };
        let event = match running
            .event_rx
            .recv_timeout(Duration::from_millis(u64::from(timeout_ms)))
        {
            Ok(event) => event,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => return ResultCode::NoEvent,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return ResultCode::NoEvent,
        };
        let Ok(bytes) = serde_json::to_vec(&event) else {
            return ResultCode::InternalError;
        };
        let len = bytes.len();
        let bytes = bytes.into_boxed_slice();
        let data = Box::into_raw(bytes) as *mut u8;
        unsafe {
            *json_utf8 = data;
            *json_len = len;
        }
        ResultCode::Ok
    }) as i32
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `json_utf8` and `json_len` must be the unmodified pair returned by a
/// successful `gromnie_session_next_event` call, and must not have been freed.
pub unsafe extern "C" fn gromnie_buffer_free(json_utf8: *mut u8, json_len: usize) {
    if json_utf8.is_null() {
        return;
    }
    // SAFETY: buffers are only allocated by gromnie_session_next_event with this exact length.
    unsafe {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            json_utf8, json_len,
        )))
    };
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `session` must be a live handle and must not be used concurrently.
pub unsafe extern "C" fn gromnie_session_disconnect(session: *mut gromnie_session_t) -> i32 {
    catch_code(|| disconnect(session)) as i32
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `session` must be the single matching pointer returned by `create`, with no
/// concurrent or future users. It is invalid after this call.
pub unsafe extern "C" fn gromnie_session_destroy(session: *mut gromnie_session_t) {
    if session.is_null() {
        return;
    }
    let _ = disconnect(session);
    // SAFETY: caller promises this is the one matching create call and has no further users.
    unsafe { drop(Box::from_raw(session)) };
}

#[unsafe(no_mangle)]
pub extern "C" fn gromnie_result_message(code: i32) -> *const c_char {
    match code {
        0 => c"ok".as_ptr(),
        1 => c"no event available".as_ptr(),
        2 => c"invalid argument".as_ptr(),
        3 => c"invalid session state".as_ptr(),
        4 => c"command queue is full".as_ptr(),
        5 => c"operation timed out".as_ptr(),
        _ => c"internal error".as_ptr(),
    }
}

fn enqueue(session: *mut gromnie_session_t, command: Command) -> ResultCode {
    catch_code(|| {
        let Some(session) = (unsafe { session.as_ref() }) else {
            return ResultCode::InvalidArgument;
        };
        let Ok(state) = session.inner.lock() else {
            return ResultCode::InternalError;
        };
        let SessionState::Running(running) = &*state else {
            return ResultCode::InvalidState;
        };
        match running.command_tx.try_send(command) {
            Ok(()) => ResultCode::Ok,
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => ResultCode::QueueFull,
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => ResultCode::InvalidState,
        }
    })
}

fn disconnect(session: *mut gromnie_session_t) -> ResultCode {
    let Some(session) = (unsafe { session.as_ref() }) else {
        return ResultCode::InvalidArgument;
    };
    let Ok(mut state) = session.inner.lock() else {
        return ResultCode::InternalError;
    };
    let SessionState::Running(running) = std::mem::replace(&mut *state, SessionState::Closed)
    else {
        return ResultCode::Ok;
    };
    let _ = running.command_tx.try_send(Command::Disconnect);
    drop(state);
    if running.worker.join().is_err() {
        ResultCode::InternalError
    } else {
        ResultCode::Ok
    }
}

unsafe fn required_utf8(value: *const c_char, max_len: usize) -> Result<String, ResultCode> {
    if value.is_null() {
        return Err(ResultCode::InvalidArgument);
    }
    let bytes = unsafe { CStr::from_ptr(value) }.to_bytes();
    if bytes.len() > max_len {
        return Err(ResultCode::InvalidArgument);
    }
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| ResultCode::InvalidArgument)
}

fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && !host.contains([':', '/', '[', ']', ' '])
        && host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
}

fn catch_code(f: impl FnOnce() -> ResultCode + std::panic::UnwindSafe) -> ResultCode {
    std::panic::catch_unwind(f).unwrap_or(ResultCode::InternalError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn host_validation_allows_dns_and_ipv4_only() {
        assert!(valid_host("play.example.com"));
        assert!(valid_host("192.0.2.42"));
        assert!(!valid_host("https://play.example.com"));
        assert!(!valid_host("[2001:db8::1]"));
        assert!(!valid_host("bad host"));
    }

    #[test]
    fn ffi_rejects_invalid_connect_arguments_without_starting_a_worker() {
        let session = gromnie_session_create();
        let account = CString::new("account").unwrap();
        let password = CString::new("password").unwrap();

        assert_eq!(
            unsafe {
                gromnie_session_connect(
                    session,
                    c"[2001:db8::1]".as_ptr(),
                    9000,
                    account.as_ptr(),
                    password.as_ptr(),
                )
            },
            ResultCode::InvalidArgument as i32
        );
        assert_eq!(
            unsafe { gromnie_session_send_chat(session, c"hello".as_ptr()) },
            ResultCode::InvalidState as i32
        );

        unsafe { gromnie_session_destroy(session) };
    }
}
