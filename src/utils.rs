use std::cell::RefCell;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

/// Determine whether two files have identical contents and permissions.
///
/// Compares Unix permission and special mode bits and file sizes first,
/// then reads and compares the full contents only when those match. On
/// other platforms, this compares sizes and contents only.
///
/// # Errors
///
/// Errors if either file's metadata or contents cannot be read.
pub fn are_files_equal(a: &Path, b: &Path) -> io::Result<bool> {
    // Possible improvements if this is a bottleneck:
    //  - Compare _streaming_ bytes, to cater for big files.
    //  - Compare hashes (e.g., xxHashes) if we do the streaming.
    //    Streaming is slower because you have to jump back-and-forth
    //    between files.

    let a_metadata = fs::metadata(a)?;
    let b_metadata = fs::metadata(b)?;

    // 1. Compare Unix permission and special mode bits (quick).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        const PERMISSION_BITS: u32 = 0o7777;

        let a_mode = a_metadata.permissions().mode() & PERMISSION_BITS;
        let b_mode = b_metadata.permissions().mode() & PERMISSION_BITS;
        if a_mode != b_mode {
            return Ok(false);
        }
    }

    // 2. Compare by file size (quick).
    if a_metadata.len() != b_metadata.len() {
        return Ok(false);
    }

    // 3. Compare contents (slow; as raw bytes to avoid UTF-8 overhead).
    thread_local! {
        static BUFFERS: RefCell<(Vec<u8>, Vec<u8>)> = RefCell::new(
            // 64 Kb should be plenty for the majority of config files.
            (Vec::with_capacity(65_536), Vec::with_capacity(65_536))
        );
    }

    BUFFERS.with_borrow_mut(|(a_buf, b_buf)| {
        read_to_bytes_buffer(a_buf, a)?;
        read_to_bytes_buffer(b_buf, b)?;

        Ok(a_buf == b_buf)
    })
}

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
    file.read_to_end(buffer)
}
