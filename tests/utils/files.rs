use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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

pub fn make_permissions_differ(source: &Path, destination: &Path) {
    #[cfg(unix)]
    set_modes(source, destination, 0o755, 0o644);

    #[cfg(windows)]
    {
        let mut source_permissions = fs::metadata(source).unwrap().permissions();
        source_permissions.set_readonly(true);
        fs::set_permissions(source, source_permissions).unwrap();

        assert!(!fs::metadata(destination).unwrap().permissions().readonly());
    }
}

pub fn have_equal_permissions(a: &Path, b: &Path) -> bool {
    let a_permissions = fs::metadata(a).unwrap().permissions();
    let b_permissions = fs::metadata(b).unwrap().permissions();

    if a_permissions.readonly() != b_permissions.readonly() {
        return false;
    }

    #[cfg(unix)]
    if a_permissions.mode() & 0o7777 != b_permissions.mode() & 0o7777 {
        return false;
    }

    true
}

#[cfg(unix)]
pub fn set_modes(a: &Path, b: &Path, a_mode: u32, b_mode: u32) {
    fs::set_permissions(a, fs::Permissions::from_mode(a_mode)).unwrap();
    fs::set_permissions(b, fs::Permissions::from_mode(b_mode)).unwrap();
}

#[cfg(unix)]
pub fn mode(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o7777
}

#[cfg(windows)]
#[allow(clippy::permissions_set_readonly_false)] // This function only compiles on Windows.
pub fn make_writable(paths: &[&Path]) {
    for path in paths {
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions).unwrap();
    }
}
