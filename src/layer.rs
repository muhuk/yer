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

use bevy::prelude::*;

use crate::math::{Sample, Sampler2D};
use crate::undo;

mod actions;
mod components;
mod mask;

pub use actions::{
    CreateLayerAction, DeleteLayerAction, HeightMapConstantUpdateHeightAction, RenameLayerAction,
    SwitchLayerPositionsAction, UpdateLayerAction,
};
pub use components::{
    HeightMap, Layer, LayerBundle, LayerOrder, NeedsLayerOrderNormalization, HEIGHT_RANGE,
    LAYER_SPACING,
};
pub use mask::{
    CreateMaskAction, DeleteMaskAction, Mask, MaskBundle, MaskCompositionMode, MaskOrder,
    MaskSource, UpdateMaskAction, UpdateMaskSourceAction,
};

// PLUGIN

pub struct LayerPlugin;

impl Plugin for LayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((components::LayerComponentsPlugin, mask::MaskPlugin));
        app.add_systems(
            FixedUpdate,
            normalize_layer_ordering_system.run_if(
                any_match_filter::<Changed<LayerOrder>>
                    .or(any_with_component::<NeedsLayerOrderNormalization>),
            ),
        );
    }

    fn finish(&self, app: &mut App) {
        if !app.is_plugin_added::<undo::UndoPlugin>() {
            app.add_plugins(undo::UndoPlugin::default());
        }
    }
}

// SYSTEMS

fn normalize_layer_ordering_system(
    mut commands: Commands,
    mut layers: Query<&mut LayerOrder>,
    needs_layer_order_normalization_query: Query<Entity, With<NeedsLayerOrderNormalization>>,
) {
    trace!("Normalizing layer ordering.");
    layers
        .iter_mut()
        .sort::<&LayerOrder>()
        .enumerate()
        .for_each(|(idx, mut layer_order)| {
            // Start from LAYER_SPACING (1-based) and increment for
            // as much as LAYER_SPACING at each layer.
            layer_order.bypass_change_detection().0 =
                u32::try_from(idx + 1).expect("There are too many layers.") * LAYER_SPACING;
        });
    needs_layer_order_normalization_query
        .iter()
        .for_each(|entity| commands.entity(entity).despawn());
}

// LIB

pub struct LayerSampler {
    pub height_map: HeightMap,
    pub masks: Vec<(Mask, MaskSource)>,
}

impl Sampler2D for LayerSampler {
    fn sample(&self, position: Vec2, base_sample: &Sample) -> Sample {
        let mut sample = self.height_map.sample(position, base_sample);

        // We need this condition to avoid multiplying the sample with zero.
        if !self.masks.is_empty() {
            let mut mask_multiplier: Option<f32> = None;
            for (mask, mask_source) in self.masks.iter() {
                mask_multiplier = Some(mask.combine(mask_multiplier, mask_source.sample(position)));
            }
            sample.multiply_alpha_mut(mask_multiplier.unwrap());
        }

        sample
    }
}

/// This is intended to be called to create the initial layer only.  It does
/// not emit LayerChange::Added event.
pub fn create_initial_layer(world: &mut World) {
    let layer = Layer::default();
    world.spawn((
        LayerBundle {
            name: layer.name_component(),
            layer,
            height_map: HeightMap::default(),
        },
        LayerOrder(0),
    ));
}
