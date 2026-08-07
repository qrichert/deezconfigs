use std::fs;
use std::path::{Path, PathBuf};

use super::conf::{CONFIGS, HOME};

pub fn file_exists_in_configs(file_path: &str) -> bool {
    let file = PathBuf::from(CONFIGS).join(file_path);
    file.is_file() && !file.is_symlink()
}

pub fn file_exists_in_home(file_path: &str) -> bool {
    let file = PathBuf::from(HOME).join(file_path);
    file.is_file() && !file.is_symlink()
}

pub fn symlink_exists_in_home(symlink_path: &str) -> bool {
    let symlink = PathBuf::from(HOME).join(symlink_path);
    symlink.is_symlink()
}

pub fn dir_exists_in_configs(dir_path: &str) -> bool {
    let dir = PathBuf::from(CONFIGS).join(dir_path);
    dir.is_dir()
}

pub fn dir_exists_in_home(dir_path: &str) -> bool {
    let dir = PathBuf::from(HOME).join(dir_path);
    dir.is_dir()
}

pub fn read(file_path: &Path) -> String {
    fs::read_to_string(file_path).unwrap()
}

pub fn read_in_configs(file_path: &str) -> String {
    let file = PathBuf::from(CONFIGS).join(file_path);
    fs::read_to_string(file).unwrap()
}

pub fn read_in_home(file_path: &str) -> String {
    let file = PathBuf::from(HOME).join(file_path);
    fs::read_to_string(file).unwrap()
}

pub fn read_symlink_in_configs(symlink_path: &str) -> PathBuf {
    let file = PathBuf::from(CONFIGS).join(symlink_path);
    fs::read_link(file).unwrap()
}

pub fn read_symlink_in_home(symlink_path: &str) -> PathBuf {
    let file = PathBuf::from(HOME).join(symlink_path);
    fs::read_link(file).unwrap()
}
