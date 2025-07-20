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

const ONE_IN_TEN_THOUSAND: f32 = 0.0001f32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Alpha {
    Opaque,
    Transparent(f32),
}

impl Alpha {
    pub fn factor(&self) -> f32 {
        match self {
            Self::Opaque => 1.0,
            Self::Transparent(factor) => *factor,
        }
    }

    pub fn is_opaque(&self) -> bool {
        *self == Self::Opaque
    }

    pub fn from_factor(factor: f32) -> Self {
        assert!(
            factor >= 0.0 && factor <= 1.0,
            "factor must be between 0.0 and 1.0."
        );
        if approx_eq(factor, 1.0, ONE_IN_TEN_THOUSAND) {
            Self::Opaque
        } else {
            Self::Transparent(factor)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Sample {
    height: f32,
    alpha: Alpha,
}

impl Sample {
    pub fn new(height: f32, alpha: Alpha) -> Self {
        Self { height, alpha }
    }

    pub fn alpha(&self) -> Alpha {
        self.alpha
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn mix(&self, other: &Self) -> Self {
        let mut result = self.clone();
        result.mix_mut(other);
        result
    }

    /// Mix `other` above `self`.
    ///
    /// See [reference](https://en.wikipedia.org/wiki/Alpha_compositing#Description).
    pub fn mix_mut(&mut self, other: &Self) {
        let mix_factor = other.alpha.factor();
        let new_alpha = Alpha::from_factor(mix_factor + self.alpha().factor() * (1.0 - mix_factor));
        self.height =
            other.height * mix_factor + self.height * self.alpha().factor() * (1.0 - mix_factor);
        self.alpha = new_alpha;
    }

    pub fn multiply_alpha_mut(&mut self, factor: f32) {
        assert!(
            factor >= 0.0 && factor <= 1.0,
            "factor must be between 0.0 and 1.0."
        );
        self.alpha = Alpha::from_factor(self.alpha.factor() * factor);
    }
}

impl Default for Sample {
    fn default() -> Self {
        Self {
            height: 0.0,
            alpha: Alpha::Opaque,
        }
    }
}

// We cannot just return a single f32.
//
// Minimum; we need an alpha value.
//
// Design this in a way we can return additional channels.
pub trait Sampler2D: Send + Sync {
    fn sample(&self, position: Vec2, base_sample: &Sample) -> Sample;
}

pub fn approx_eq(a: f32, b: f32, ratio: f32) -> bool {
    let max_difference = f32::max(f32::max(a.abs(), b.abs()) * ratio, f32::EPSILON);
    (a - b).abs() < max_difference
}

pub fn clamp(x: f32, min: f32, max: f32) -> f32 {
    debug_assert!(min < max);
    // Poetry
    max.min(min.max(x))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_samples() {
        let a = Sample {
            height: 8.5,
            alpha: Alpha::Opaque,
        };
        let b = Sample {
            height: 7.5,
            alpha: Alpha::Opaque,
        };
        let c = Sample {
            height: 10.5,
            alpha: Alpha::from_factor(0.5),
        };
        let d = Sample {
            height: 4.5,
            alpha: Alpha::from_factor(0.4),
        };

        // If the 2nd operand is opaque, then the result is equal to 2nd
        // operand's value.  Final alpha is mixed.
        assert!(approx_eq(
            a.mix(&b).height(),
            b.height(),
            ONE_IN_TEN_THOUSAND
        ));
        assert!(a.mix(&b).alpha().is_opaque());
        assert!(approx_eq(
            c.mix(&b).height(),
            b.height(),
            ONE_IN_TEN_THOUSAND
        ));
        assert!(c.mix(&b).alpha().is_opaque());

        // If the 2nd operand is not opaque, but the 1st operand is opaque
        // then the result is mixed.  Final alpha is not changed.
        assert!(approx_eq(
            a.mix(&c).height(),
            (a.height() + c.height()) / 2.0,
            ONE_IN_TEN_THOUSAND
        ));
        assert!(a.mix(&c).alpha().is_opaque());

        // If the 1st operand is not opaque, 2nd operand's values is mixed but
        // the final alpha still equals to the 1st operand's.
        assert!(approx_eq(d.mix(&c).height(), 6.15, ONE_IN_TEN_THOUSAND));
        assert_eq!(d.mix(&c).alpha(), Alpha::from_factor(0.7));
    }
}
