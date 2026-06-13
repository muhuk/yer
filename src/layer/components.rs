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

use std::fmt::{self, Display};
use std::ops::RangeInclusive;

use bevy::ecs::{lifecycle::HookContext, world::DeferredWorld};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::bitmap::BitmapHandle;
use crate::id::LayerId;
use crate::math::{Alpha, ApproxEq, Sample, Sampler2D, Transform2D};

use super::context::LayerSamplerContext;

pub const HEIGHT_RANGE: RangeInclusive<f32> = -16000.0..=64000.0;
pub const LAYER_SPACING: u32 = 100;

const DEFAULT_LAYER_NAME: &str = "<unnamed>";

// PLUGIN

pub struct LayerComponentsPlugin;

impl Plugin for LayerComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HeightMap>()
            .register_type::<Layer>()
            .register_type::<LayerOrder>()
            .register_type::<LayerId>();
        app.register_type::<NeedsLayerOrderNormalization>();
    }
}

// BUNDLES

#[derive(Bundle, Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct LayerBundle {
    pub layer: Layer,
    pub name: Name,
    pub height_map: HeightMap,
}

impl LayerBundle {
    pub fn new_bitmap(bitmap_handle: BitmapHandle) -> Self {
        let layer = Layer::default();
        Self {
            name: layer.name_component(),
            layer,
            height_map: HeightMap::Bitmap {
                bitmap_handle,
                transform: Transform2D::default(),
                repeat_mode: BitmapRepeatMode::default(),
            },
        }
    }

    pub fn new_constant() -> Self {
        let layer = Layer::default();
        Self {
            name: layer.name_component(),
            layer,
            height_map: HeightMap::default(),
        }
    }

    pub fn extract_all(world: &mut World) -> Vec<Self> {
        let mut layer_bundles = vec![];
        for (layer, name, height_map) in world
            .query::<(&Layer, &LayerOrder, &Name, &HeightMap)>()
            .iter(world)
            .sort::<&LayerOrder>()
            .map(|(l, _, n, h)| (l, n, h))
        {
            layer_bundles.push(Self {
                layer: layer.clone(),
                name: name.clone(),
                height_map: height_map.clone(),
            });
        }
        layer_bundles
    }

    pub fn insert_all(world: &mut World, layer_bundles: Vec<Self>) {
        layer_bundles
            .into_iter()
            .enumerate()
            .for_each(|(idx, layer_bundle)| {
                world.spawn((layer_bundle, LayerOrder(idx as u32 * LAYER_SPACING)));
            });
    }
}

// COMPONENTS

#[derive(Component, Clone, Debug, Deserialize, PartialEq, Reflect, Serialize)]
#[require(Layer)]
pub enum HeightMap {
    Bitmap {
        bitmap_handle: BitmapHandle,
        transform: Transform2D,
        repeat_mode: BitmapRepeatMode,
    },
    Constant(f32),
}

impl Default for HeightMap {
    fn default() -> Self {
        Self::Constant(0.0)
    }
}

impl Sampler2D for HeightMap {
    type Context = LayerSamplerContext;

    fn sample(&self, position: Vec2, _base: &Sample, context: &Self::Context) -> Sample {
        match self {
            Self::Bitmap {
                bitmap_handle,
                transform,
                repeat_mode,
            } => {
                let image = context.bitmap_server.get(bitmap_handle).unwrap();
                let image_size = image.size();
                let offset = image_size.as_vec2() * 0.5;
                let transformed_position = transform.apply(position) + offset;

                let value: f32 = {
                    let repeat_applied_position =
                        repeat_mode.apply(transformed_position, image_size);
                    // Flip Y to convert from Z-up world coordinates to Y-down image
                    // coordinates.
                    let image_position = UVec2::new(
                        repeat_applied_position.x.floor() as u32,
                        image_size.y - 1 - repeat_applied_position.y.floor() as u32,
                    );

                    // FIXME: Remove 20.0 multiplier, use height param.
                    image.get_pixel_luma(image_position) * 20.0
                };

                Sample::new(value, Alpha::Opaque)
            }
            Self::Constant(value) => Sample::new(*value, Alpha::Opaque),
        }
    }
}

#[derive(Component, Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct Layer {
    pub name: String,
    pub enable_baking: bool,
    pub enable_preview: bool,
    id: LayerId,
}

impl Layer {
    pub fn id(&self) -> LayerId {
        self.id
    }

    pub fn name_component(&self) -> Name {
        Name::new(format!("Layer 0x{}", &self.id.simple().to_string()[25..32]))
    }

    pub(super) fn new(id: LayerId) -> Self {
        Self {
            name: DEFAULT_LAYER_NAME.to_owned(),
            enable_baking: true,
            enable_preview: true,
            id,
        }
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self::new(LayerId::new())
    }
}

impl Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            // Last 7 digits of uuid.
            &self.id.simple().to_string()[25..32]
        )
    }
}

#[derive(Component, Copy, Clone, Debug, Deref, Eq, Ord, PartialEq, PartialOrd, Reflect)]
#[component(on_remove = layer_order_on_remove_hook)]
#[require(Layer)]
pub struct LayerOrder(#[deref] pub(super) u32);

/// When a layer is despawned, and its [LayerOrder] is removed, this component
/// signals a layer reordering.
#[derive(Clone, Component, Debug, Reflect)]
pub struct NeedsLayerOrderNormalization;

// LIB

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Reflect, Serialize)]
pub enum BitmapRepeatMode {
    Extend,
    Fade,
    #[default]
    Repeat,
}

impl BitmapRepeatMode {
    pub const VALUES: [Self; 3] = [Self::Extend, Self::Fade, Self::Repeat];

    pub fn apply(&self, position: Vec2, size: UVec2) -> Vec2 {
        match self {
            Self::Extend => Vec2::new(
                position.x.max(0.0).min((size.x - 1) as f32),
                position.y.max(0.0).min((size.y - 1) as f32),
            ),
            Self::Fade => unimplemented!(),
            Self::Repeat => Vec2::new(
                Self::normalize(position.x, size.x as f32),
                Self::normalize(position.y, size.y as f32),
            ),
        }
    }

    fn normalize(x: f32, boundary: f32) -> f32 {
        let mut n = x;
        while n > boundary || n.approx_eq(boundary, None) {
            n -= boundary;
        }
        while n < 0.0 {
            n += boundary;
        }
        n
    }
}

impl Display for BitmapRepeatMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

fn layer_order_on_remove_hook(mut world: DeferredWorld, HookContext { .. }: HookContext) {
    world.commands().spawn(NeedsLayerOrderNormalization);
}

#[cfg(test)]
mod tests {
    use crate::math::ApproxEq;

    use super::*;

    #[test]
    fn bitmap_repeat_mode_repeating() {
        // (input,  output)
        const CASES: &[(Vec2, Vec2)] = &[
            // Within boundaries, unchanged.
            (Vec2::new(24.0, 18.0), Vec2::new(24.0, 18.0)),
            // Out of boundaries in positive direction.
            (Vec2::new(68.0, 62.0), Vec2::new(4.0, 62.0)),
            (Vec2::new(3.0, 80.0), Vec2::new(3.0, 16.0)),
            // Exact boundary is converted to 0.0
            (Vec2::new(7.0, 128.0), Vec2::new(7.0, 0.0)),
            (Vec2::new(128.0, 8.0), Vec2::new(0.0, 8.0)),
            (Vec2::new(256.0, 192.0), Vec2::new(0.0, 0.0)),
            // Out of boundaries in negative direction.
            (Vec2::new(-4.0, -61.5), Vec2::new(60.0, 2.5)),
            (Vec2::new(-129.0, -0.995), Vec2::new(63.0, 63.005)),
        ];
        const SIZE: UVec2 = UVec2::new(64, 64);

        let mode = BitmapRepeatMode::Repeat;

        for (input, output) in CASES.iter() {
            eprintln!("{:?}", mode.apply(*input, SIZE));
            assert!(mode.apply(*input, SIZE).approx_eq(*output, None));
        }
    }
}
