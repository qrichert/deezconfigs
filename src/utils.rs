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

use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Read to pre-allocated `String` buffer.
///
/// Same as [`std::fs::read_to_string()`], but doesn't allocate if it
/// doesn't need to increase the buffer size (and bypasses the metadata
/// check for size-hint, since the buffer is supposed to be big enough).
///
/// # Errors
///
/// Errors if file cannot be read.
pub fn read_to_string_buffer(buffer: &mut String, path: &Path) -> io::Result<usize> {
    let mut file = File::open(path)?;
    buffer.clear();
    file.read_to_string(buffer)
}

/// Read to pre-allocated `Vec<u8>` buffer.
///
/// Same as [`std::fs::read()`], but doesn't allocate if it doesn't need
/// to increase the buffer size (and bypasses the metadata check for
/// size-hint, since the buffer is supposed to be big enough).
///
/// # Errors
///
/// Errors if file cannot be read.
pub fn read_to_bytes_buffer(buffer: &mut Vec<u8>, path: &Path) -> io::Result<usize> {
    let mut file = File::open(path)?;
    buffer.clear();
    file.read(buffer)
}

// TODO: Use `Path::file_prefix()` once it lands in stable.
//  This is taken straight from the standard library.
pub fn file_prefix(path: &Path) -> Option<&OsStr> {
    fn split_file_at_dot(file: &OsStr) -> (&OsStr, Option<&OsStr>) {
        let slice = file.as_encoded_bytes();
        if slice == b".." {
            return (file, None);
        }

        // The unsafety here stems from converting between &OsStr and &[u8]
        // and back. This is safe to do because (1) we only look at ASCII
        // contents of the encoding and (2) new &OsStr values are produced
        // only from ASCII-bounded slices of existing &OsStr values.
        let i = match slice[1..].iter().position(|b| *b == b'.') {
            Some(i) => i + 1,
            None => return (file, None),
        };
        let before = &slice[..i];
        let after = &slice[i + 1..];
        unsafe {
            (
                OsStr::from_encoded_bytes_unchecked(before),
                Some(OsStr::from_encoded_bytes_unchecked(after)),
            )
        }
    }

    path.file_name()
        .map(split_file_at_dot)
        .map(|(before, _after)| before)
}
