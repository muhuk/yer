// Copyright © 2024-2026 Atamert Ölçgen.
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
use std::fmt::Display;
use std::ops::Deref;

use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Reflect, Serialize)]
pub struct BitmapId(uuid::Uuid);

impl BitmapId {
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7())
    }
}

impl Deref for BitmapId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for BitmapId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
impl From<uuid::Uuid> for BitmapId {
    fn from(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }
}

/// A stable id for masks.
///
/// We cannot use `Entity` as a stable id because if a mask is deleted and
/// then the delete is undoed, the new entity will be a different one.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Reflect, Serialize)]
pub struct LayerId(uuid::Uuid);

impl LayerId {
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7())
    }
}

impl Deref for LayerId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for LayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
impl From<uuid::Uuid> for LayerId {
    fn from(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }
}

/// A stable id for masks.
///
/// We cannot use `Entity` as a stable id because if a mask is deleted and
/// then the delete is undoed, the new entity will be a different one.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Reflect, Serialize)]
pub struct MaskId(uuid::Uuid);

impl MaskId {
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7())
    }
}

impl Deref for MaskId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for MaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
impl From<uuid::Uuid> for MaskId {
    fn from(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }
}
