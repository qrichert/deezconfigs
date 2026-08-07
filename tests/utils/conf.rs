use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const TMP_DIR: &str = env!("CARGO_TARGET_TMPDIR");
pub const CONFIGS: &str = concat!(env!("CARGO_TARGET_TMPDIR"), "/configs");
pub const HOME: &str = concat!(env!("CARGO_TARGET_TMPDIR"), "/home");

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
        #[cfg(unix)]
        env::set_var("HOME", HOME);
        #[cfg(windows)]
        env::set_var("USERPROFILE", HOME);

        // Can't test defaults if `DEEZ_ROOT` is set.
        env::remove_var("DEEZ_ROOT");
    }

    // Clean new config dir.
    let dir = PathBuf::from(CONFIGS);
    println!("init: '{}'.", dir.display());
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir(&dir).unwrap();

    create_file_in_configs(".deez", None);
}

pub fn create_file_in_configs(file_path: &str, content: Option<&str>) -> PathBuf {
    create_file(CONFIGS, file_path, content)
}

pub fn create_executable_file_in_configs(file_path: &str, content: Option<&str>) -> PathBuf {
    let f = create_file(CONFIGS, file_path, content);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let file = fs::File::open(&f).unwrap();
        let mut perms = file.metadata().unwrap().permissions();
        perms.set_mode(perms.mode() | 0o100); // u+x
        file.set_permissions(perms).unwrap();
    }
    f
}

pub fn create_file_in_home(file_path: &str, content: Option<&str>) -> PathBuf {
    create_file(HOME, file_path, content)
}

fn create_file(root: &str, file_path: &str, content: Option<&str>) -> PathBuf {
    let file = PathBuf::from(root).join(file_path);
    println!("create file: '{}'.", file.display());

    // Create parent directories (if any).
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    if let Some(content) = content {
        _ = fs::write(&file, content);
    } else {
        fs::File::create(&file).unwrap();
    }

    file
}

pub fn create_symlink_in_configs(symlink_path: &str, target: Option<&str>) -> (PathBuf, PathBuf) {
    create_symlink(CONFIGS, symlink_path, target)
}

pub fn create_symlink_in_home(symlink_path: &str, target: Option<&str>) -> (PathBuf, PathBuf) {
    create_symlink(HOME, symlink_path, target)
}

fn create_symlink(root: &str, symlink_path: &str, target: Option<&str>) -> (PathBuf, PathBuf) {
    let symlink = PathBuf::from(root).join(symlink_path);
    println!("create symlink: '{}'.", symlink.display());

    // Create parent directories (if any).
    if let Some(parent) = symlink.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    // Create target file.
    let target = if let Some(target) = target {
        let target = PathBuf::from(root).join(target);
        assert!(target.exists(), "target must exist before creating symlink");
        target
    } else {
        let target = PathBuf::from(TMP_DIR).join("symlink_target");
        fs::File::create(&target).unwrap();
        target
    };

    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &symlink).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&target, &symlink).unwrap();

    (symlink, target)
}

pub fn create_dir_in_configs(dir_path: &str) -> PathBuf {
    create_dir(CONFIGS, dir_path)
}

pub fn create_dir_in_home(dir_path: &str) -> PathBuf {
    create_dir(HOME, dir_path)
}

fn create_dir(root: &str, dir_path: &str) -> PathBuf {
    let dir = PathBuf::from(root).join(dir_path);
    println!("create dir: '{}'.", dir.display());

    fs::create_dir_all(&dir).unwrap();

    dir
}
