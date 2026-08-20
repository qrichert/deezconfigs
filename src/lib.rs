//! Dotfiles management library behind the `deez` CLI.
//!
//! Copy, sync, or symlink configuration files into the user's home
//! directory.

pub mod hooks;
pub mod pathspec;
pub mod ui;
pub mod utils;
pub mod walk;
