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

use bevy::prelude::*;

use crate::math::{Sample, Sampler2D};
use crate::undo;

mod actions;
mod components;
mod mask;

pub use actions::{
    CreateLayerAction, DeleteLayerAction, HeightMapBitmapUpdateTransformAction,
    HeightMapConstantUpdateHeightAction, RenameLayerAction, SwitchLayerPositionsAction,
    UpdateLayerAction,
};
pub use components::{
    BitmapRepeatMode, HeightMap, Layer, LayerBundle, LayerOrder, NeedsLayerOrderNormalization,
    HEIGHT_RANGE,
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
            app.add_plugins(undo::UndoPlugin);
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
            layer_order.bypass_change_detection().0 = u32::try_from(idx + 1)
                .expect("There are too many layers.")
                * components::LAYER_SPACING;
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

// FIXME: Remove this.
pub fn create_test_bitmap_layer(world: &mut World) {
    use std::path::PathBuf;

    const TEST_FILE_PATH: &str = "test_assets/smiley_heightmap.png";

    let file_path: PathBuf = {
        let mut file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        file_path.push(TEST_FILE_PATH);
        file_path
    };

    let (image_format, dynamic_image) = {
        let reader = image::ImageReader::open(&file_path).unwrap();
        (reader.format().unwrap(), reader.decode().unwrap())
    };

    let bitmap = components::Bitmap::Embedded {
        image: (image_format, dynamic_image),
        original_path: file_path,
    };

    let top_layer_id: Option<crate::id::LayerId> = world
        .query::<(&Layer, &LayerOrder)>()
        .iter(world)
        .sort::<&LayerOrder>()
        .last()
        .map(|(l, _)| l.id());

    let layer_bundle = LayerBundle {
        name: Name::new("Test Bitmap Layer"),
        layer: Layer::default(),
        height_map: HeightMap::Bitmap {
            // FIXME: Implement loading of image
            bitmap,
            transform: crate::math::Transform2D::default(),
            repeat_mode: BitmapRepeatMode::Extend,
        },
    };

    world
        .commands()
        .queue(crate::undo::PushAction::from(CreateLayerAction::new(
            layer_bundle,
            top_layer_id,
        )));
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
