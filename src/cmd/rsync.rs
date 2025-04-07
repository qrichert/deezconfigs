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

pub fn rsync(_root: Option<String>, _verbose: bool) -> Result<(), i32> {
    todo!("update files _from_ destination")

    // TODO:
    //  1. Collect all files in `configs`
    //  2. Find matching files in `/home`
    //  3. Replace files in `configs` with files in `/home`.
}
