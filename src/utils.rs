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
