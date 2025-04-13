use std::path;

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
