// Copyright © 2024-2025 Atamert Ölçgen.
// This file is part of Yer.
//
// Yer is free software: you can redistribute it and/or modify it under the
// terms of the GNU General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later
// version.
//
// Yer is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more
// details.
//
// You should have received a copy of the GNU General Public License along
// with Yer.  If not, see <https://www.gnu.org/licenses/>.

/// A stable id for masks.
///
/// We cannot use `Entity` as a stable id because if a mask is deleted and
/// then the delete is undoed, the new entity will be a different one.
pub type LayerId = uuid::Uuid;

/// A stable id for masks.
///
/// We cannot use `Entity` as a stable id because if a mask is deleted and
/// then the delete is undoed, the new entity will be a different one.
pub type MaskId = uuid::Uuid;
