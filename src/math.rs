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

use std::f32;

use bevy::math::{Rot2, Vec2};
use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

const ONE_IN_TEN_THOUSAND: f32 = 0.0001f32;

pub trait ApproxEq: Sized {
    type Ratio;

    const DEFAULT_RATIO: Self::Ratio;

    fn approx_eq(&self, other: Self, ratio: Option<Self::Ratio>) -> bool;
}

impl ApproxEq for f32 {
    type Ratio = f32;

    const DEFAULT_RATIO: Self::Ratio = ONE_IN_TEN_THOUSAND;

    fn approx_eq(&self, other: Self, ratio: Option<f32>) -> bool {
        let ratio = ratio.unwrap_or(Self::DEFAULT_RATIO);
        let max_difference = f32::max(f32::max(self.abs(), other.abs()) * ratio, f32::EPSILON);
        (self - other).abs() < max_difference
    }
}

impl ApproxEq for Vec2 {
    type Ratio = f32;

    const DEFAULT_RATIO: Self::Ratio = ONE_IN_TEN_THOUSAND;

    fn approx_eq(&self, other: Self, ratio: Option<Self::Ratio>) -> bool {
        self.x.approx_eq(other.x, ratio) && self.y.approx_eq(other.y, ratio)
    }
}

// We cannot just return a single f32.
//
// Minimum; we need an alpha value.
//
// Design this in a way we can return additional channels.
pub trait Sampler2D: Send + Sync {
    type Context;

    fn sample(&self, position: Vec2, base_sample: &Sample, context: &Self::Context) -> Sample;
}

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
        if factor.approx_eq(1.0, None) {
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

    /// Mix `other` above `self`.
    ///
    /// See [reference](https://en.wikipedia.org/wiki/Alpha_compositing#Description).
    pub fn mix_in_place(&mut self, other: &Self) {
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

#[derive(Clone, Debug, Deserialize, PartialEq, Reflect, Serialize)]
pub struct Transform2D {
    pub scale: f32,
    pub translation: Vec2,
    pub rotation: f32,
}

impl Transform2D {
    pub fn apply(&self, position: Vec2) -> Vec2 {
        //self.translate(self.rotate(position))
        self.rotate(self.translate(self.scale(position)))
    }

    #[inline]
    fn rotate(&self, position: Vec2) -> Vec2 {
        Rot2::degrees(-self.rotation) * position
    }

    #[inline]
    fn scale(&self, position: Vec2) -> Vec2 {
        position / self.scale
    }

    #[inline]
    fn translate(&self, position: Vec2) -> Vec2 {
        position - self.translation
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            scale: 1.0,
            translation: Vec2::ZERO,
            rotation: 0.0,
        }
    }
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
    fn apply_transform_2d() {
        let transform = Transform2D {
            scale: 2.0,
            translation: Vec2::new(2.0, 5.0),
            rotation: 90.0, // degrees
        };
        assert!(transform
            .apply(Vec2::X)
            .approx_eq(Vec2::new(-5.0, 1.5), None));
        assert!(transform
            .apply(Vec2::NEG_Y)
            .approx_eq(Vec2::new(-5.5, 2.0), None));
    }

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
        {
            let mut mixed = a.clone();
            mixed.mix_in_place(&b);
            assert!(mixed.height().approx_eq(b.height(), None));
            assert!(mixed.alpha().is_opaque());
        }

        {
            let mut mixed = c.clone();
            mixed.mix_in_place(&b);
            assert!(mixed.height().approx_eq(b.height(), None));
            assert!(mixed.alpha().is_opaque());
        }

        // If the 2nd operand is not opaque, but the 1st operand is opaque
        // then the result is mixed.  Final alpha is not changed.
        {
            let mut mixed = a.clone();
            mixed.mix_in_place(&c);
            assert!(mixed
                .height()
                .approx_eq((a.height() + c.height()) / 2.0, None));
            assert!(mixed.alpha().is_opaque());
        }

        // If the 1st operand is not opaque, 2nd operand's values is mixed but
        // the final alpha still equals to the 1st operand's.
        {
            let mut mixed = d.clone();
            mixed.mix_in_place(&c);
            assert!(mixed.height().approx_eq(6.15, None));
            assert_eq!(mixed.alpha(), Alpha::from_factor(0.7));
        }
    }
}
