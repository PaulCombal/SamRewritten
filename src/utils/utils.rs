use std::env;
use std::path::PathBuf;

pub fn get_executable_path() -> PathBuf {
    env::current_exe()
        .expect("Failed to get current executable path")
        .canonicalize() // Resolves symlinks to absolute path
        .expect("Failed to canonicalize path")
}