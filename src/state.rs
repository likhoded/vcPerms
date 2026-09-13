use std::sync::{Mutex, OnceLock};

use crate::store::Store;
use crate::verbose::Verbose;

static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
static VERBOSE: OnceLock<Mutex<Verbose>> = OnceLock::new();

pub fn init(store: Store) {
    let _ = STORE.set(Mutex::new(store));
    let _ = VERBOSE.set(Mutex::new(Verbose::default()));
}

pub fn with_store<T>(f: impl FnOnce(&Store) -> T) -> T {
    let lock = STORE.get().expect("store").lock().unwrap_or_else(|e| e.into_inner());
    f(&lock)
}

pub fn with_store_mut<T>(f: impl FnOnce(&mut Store) -> T) -> T {
    let mut lock = STORE.get().expect("store").lock().unwrap_or_else(|e| e.into_inner());
    f(&mut lock)
}

pub fn with_verbose<T>(f: impl FnOnce(&mut Verbose) -> T) -> T {
    let mut lock = VERBOSE.get().expect("verbose").lock().unwrap_or_else(|e| e.into_inner());
    f(&mut lock)
}
