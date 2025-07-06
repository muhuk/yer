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

use std::f32;

use bevy::math::Vec2;

pub trait Sample2D: Send + Sync {
    fn sample(&self, position: Vec2, height: f32) -> f32;
}

pub fn approx_eq(a: f32, b: f32, ratio: f32) -> bool {
    let max_difference = f32::max(f32::max(a.abs(), b.abs()) * ratio, f32::EPSILON);
    (a - b).abs() < max_difference
}
