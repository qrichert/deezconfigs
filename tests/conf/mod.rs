// deezconfigs — Manage deez config files.
// Copyright (C) 2025  Quentin Richert
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const TMP_DIR: &str = env!("CARGO_TARGET_TMPDIR");
pub const HOME: &str = concat!(env!("CARGO_TARGET_TMPDIR"), "/home");
const CONFIGS: &str = concat!(env!("CARGO_TARGET_TMPDIR"), "/configs");

pub fn root() -> String {
    PathBuf::from(CONFIGS).display().to_string()
}

pub fn init() {
    // Clean new $HOME.
    if Path::new(HOME).exists() {
        fs::remove_dir_all(HOME).unwrap();
    }
    fs::create_dir_all(HOME).unwrap();
    unsafe {
        // This patches `HOME` with the same value every time, so it
        // doesn't matter who writes when.
        env::set_var("HOME", HOME);
    }

    // Clean new config dir.
    let dir = PathBuf::from(TMP_DIR).join("configs");
    println!("init: '{}'.", dir.display());
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir(&dir).unwrap();
}

pub fn create_file(file_path: &str) {
    let file = PathBuf::from(CONFIGS).join(file_path);
    println!("create file: '{}'.", file.display());

    // Create parent directories (if any).
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    fs::File::create(file).unwrap();
}

pub fn create_symlink(symlink_path: &str) {
    let symlink = PathBuf::from(CONFIGS).join(symlink_path);
    println!("create symlink: '{}'.", symlink.display());

    // Create parent directories (if any).
    if let Some(parent) = symlink.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    // Create target file.
    let target = PathBuf::from(TMP_DIR).join("symlink_target");
    fs::File::create(&target).unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, symlink).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&target, symlink).unwrap();
}
