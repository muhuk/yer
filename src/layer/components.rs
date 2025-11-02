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

use std::fmt::{self, Display};
use std::ops::RangeInclusive;

use bevy::ecs::{lifecycle::HookContext, world::DeferredWorld};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::id::LayerId;
use crate::math::{Alpha, Sample, Sampler2D};

// FIXME: Circular dependency.
use super::mask::MaskBundle;

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

#[derive(Bundle, Deserialize, Serialize)]
pub struct LayerBundle {
    pub layer: Layer,
    pub name: Name,
    pub height_map: HeightMap,
}

impl LayerBundle {
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

#[derive(Component, Clone, Debug, Deserialize, Reflect, Serialize)]
#[require(Layer)]
pub enum HeightMap {
    Constant(f32),
}

impl Default for HeightMap {
    fn default() -> Self {
        Self::Constant(0.0)
    }
}

impl Sampler2D for HeightMap {
    fn sample(&self, _position: Vec2, _base: &Sample) -> Sample {
        match self {
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

    pub(super) fn new_id() -> LayerId {
        LayerId::now_v7()
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self::new(Self::new_id())
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

#[derive(Clone, Component, Debug, Reflect)]
pub struct NeedsLayerOrderNormalization;

// LIB

fn layer_order_on_remove_hook(mut world: DeferredWorld, HookContext { .. }: HookContext) {
    world.commands().spawn(NeedsLayerOrderNormalization);
}
