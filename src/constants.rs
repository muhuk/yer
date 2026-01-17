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
use std::{num::NonZeroUsize, ops::RangeInclusive};

use bevy::color::Color;
use bevy::math::Vec3;

pub const APPLICATION_TITLE: &str = concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"));

pub const PREVIEW_DEFAULT_FACE_COLOR: Color = Color::hsl(0.0, 0.0, 0.5);
pub const PREVIEW_DEFAULT_WIREFRAME_COLOR: Color = Color::hsl(0.0, 0.0, 0.85);
pub const PREVIEW_DEFAULT_FACE_ALPHA: f32 = 0.65f32;

pub const UNDO_STACK_SIZE_DEFAULT: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(20) };
pub const UNDO_STACK_SIZE_RANGE: RangeInclusive<usize> = 1..=100;

pub const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION_MAJOR"),
    ".",
    env!("CARGO_PKG_VERSION_MINOR")
);

pub const VIEWPORT_CAMERA_INITIAL_TARGET: Vec3 = Vec3::ZERO;
pub const VIEWPORT_CAMERA_INITIAL_TRANSLATION: Vec3 = Vec3::new(-50.0, 300.0, 200.0);
pub const VIEWPORT_LIGHT_POSITION: Vec3 = Vec3::new(-3.0, 5.0, -4.0);
pub const VIEWPORT_LIGHT_LOOK_AT_TARGET: Vec3 = Vec3::ZERO;
