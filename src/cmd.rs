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

pub mod clean;
pub mod cli;
pub mod common;
pub mod diff;
pub mod link;
pub mod rsync;
pub mod run;
pub mod status;
pub mod sync;

pub use clean::clean;
pub use diff::diff;
pub use link::link;
pub use rsync::rsync;
pub use run::run;
pub use status::status;
pub use sync::sync;
