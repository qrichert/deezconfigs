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

use std::fs;
use std::path::{Path, PathBuf};

use super::conf::{CONFIGS, HOME};

pub fn file_exists_in_configs(file_path: &str) -> bool {
    let file = PathBuf::from(CONFIGS).join(file_path);
    file.is_file()
}

pub fn file_exists_in_home(file_path: &str) -> bool {
    let file = PathBuf::from(HOME).join(file_path);
    file.is_file()
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

pub fn read_in_home(file_path: &str) -> String {
    let file = PathBuf::from(HOME).join(file_path);
    fs::read_to_string(file).unwrap()
}

pub fn read_symlink_in_home(symlink_path: &str) -> PathBuf {
    let file = PathBuf::from(HOME).join(symlink_path);
    fs::read_link(file).unwrap()
}
