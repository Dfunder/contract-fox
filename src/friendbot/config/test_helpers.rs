/// Single process-wide mutex that every test module must hold before
/// touching the `STELLAR_NETWORK` environment variable.
///
/// Each test module used to define its own local `ENV_MUTEX`, but separate
/// `Mutex` instances provide no mutual exclusion between modules — a thread
/// in `config::tests` and a thread in `utils::tests` could hold their own
/// respective locks simultaneously and still corrupt each other's env state.
///
/// By sharing this one static, all env-mutating tests across the whole binary
/// are serialized through the same lock.
use std::env;
use std::sync::{Mutex, OnceLock};

pub static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Run `f` with `STELLAR_NETWORK` set to `value`, then restore the variable
/// to unset — even if `f` panics.
pub fn with_network<F: FnOnce()>(value: &str, f: F) {
    let _guard = ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap();
    unsafe { env::set_var("STELLAR_NETWORK", value) };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    unsafe { env::remove_var("STELLAR_NETWORK") };
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

/// Run `f` with `STELLAR_NETWORK` unset, then ensure it stays unset —
/// even if `f` panics.
pub fn without_network<F: FnOnce()>(f: F) {
    let _guard = ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap();
    unsafe { env::remove_var("STELLAR_NETWORK") };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    unsafe { env::remove_var("STELLAR_NETWORK") };
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}
