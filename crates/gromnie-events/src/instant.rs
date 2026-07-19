use std::time::Duration;

/// Cross-platform instant that works on both native and WASM.
///
/// On native, wraps `std::time::Instant`. On WASM, uses `js_sys::Date::now()`
/// to get millisecond timestamps since epoch (stored as u64 for Eq/Ord/Hash).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant {
    #[cfg(not(target_arch = "wasm32"))]
    inner: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    millis: u64,
}

impl Instant {
    pub fn now() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                inner: std::time::Instant::now(),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self {
                millis: js_sys::Date::now() as u64,
            }
        }
    }

    pub fn elapsed(&self) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.elapsed()
        }
        #[cfg(target_arch = "wasm32")]
        {
            let now = js_sys::Date::now() as u64;
            let diff_ms = now.saturating_sub(self.millis);
            Duration::from_millis(diff_ms)
        }
    }
}
