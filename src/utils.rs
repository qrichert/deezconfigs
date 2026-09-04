use std::cell::RefCell;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

/// Copy a file's contents and permissions.
///
/// On Unix, permissions are applied again after writing because writing
/// the contents can clear set-ID bits applied by [`fs::copy()`].
///
/// # Errors
///
/// Errors if the file cannot be copied or its permissions cannot be read
/// or applied.
pub fn copy_file(source: &Path, destination: &Path) -> io::Result<u64> {
    let bytes_copied = fs::copy(source, destination)?;

    #[cfg(unix)]
    fs::set_permissions(destination, fs::metadata(source)?.permissions())?;

    Ok(bytes_copied)
}

/// Determine whether two files have identical contents and permissions.
///
/// Compares the portable read-only permission, Unix permission and
/// special mode bits, and file sizes first, then reads and compares the
/// full contents only when those match.
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
    let a_permissions = a_metadata.permissions();
    let b_permissions = b_metadata.permissions();

    // 1. Compare the portable read-only permission (quick).
    if a_permissions.readonly() != b_permissions.readonly() {
        return Ok(false);
    }

    // 2. Compare Unix permission and special mode bits (quick).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        const PERMISSION_BITS: u32 = 0o7777;

        let a_mode = a_permissions.mode() & PERMISSION_BITS;
        let b_mode = b_permissions.mode() & PERMISSION_BITS;
        if a_mode != b_mode {
            return Ok(false);
        }
    }

    // 3. Compare by file size (quick).
    if a_metadata.len() != b_metadata.len() {
        return Ok(false);
    }

    // 4. Compare contents (slow; as raw bytes to avoid UTF-8 overhead).
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
