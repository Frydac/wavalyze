use std::{path, sync::atomic::AtomicU64};

#[allow(dead_code)]
pub fn get_project_dir() -> path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    path::PathBuf::from(manifest_dir)
}

#[allow(dead_code)]
pub fn get_input_data_dir() -> path::PathBuf {
    let project_dir = get_project_dir();
    project_dir.join("data")
}

pub type Id = u64;

// Threadsafe app wide unique id generator
// NOTE: if we want to presist over sessions, maybe use uuid crate in stead?
pub fn unique_id() -> Id {
    static mut COUNTER: AtomicU64 = AtomicU64::new(0);
    #[allow(static_mut_refs)]
    unsafe {
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}
