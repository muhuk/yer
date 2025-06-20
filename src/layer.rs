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

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;

use crate::undo;

mod actions;
mod components;
mod sample;

pub use actions::*;
pub use components::*;
pub use sample::*;

const NORMALIZE_ORDERING_INTERVAL_MS: u64 = 500;

// PLUGIN

pub struct LayerPlugin;

impl Plugin for LayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(components::LayerComponentsPlugin);
        app.add_systems(
            Update,
            normalize_layer_ordering_system.run_if(on_timer(Duration::from_millis(
                NORMALIZE_ORDERING_INTERVAL_MS,
            ))),
        );
    }

    fn finish(&self, app: &mut App) {
        if !app.is_plugin_added::<undo::UndoPlugin>() {
            app.add_plugins(undo::UndoPlugin);
        }
    }
}

// SYSTEMS

fn normalize_layer_ordering_system(mut layers: Query<&mut Layer>) {
    trace!("Normalizing layer ordering.");
    layers
        .iter_mut()
        .sort::<&Layer>()
        .enumerate()
        .for_each(|(idx, mut layer)| {
            // Start from LAYER_SPACING (1-based) and increment for
            // as much as LAYER_SPACING at each layer.
            layer.order =
                u32::try_from(idx + 1).expect("There are too many layers.") * LAYER_SPACING;
        });
}

// LIB

/// This is intended to be called to create the initial layer only.  It does
/// not emit LayerChange::Added event.
pub fn create_initial_layer(world: &mut World) {
    let layer = Layer::default();
    world.spawn(LayerBundle {
        name: layer.name_component(),
        layer,
        height_map: HeightMap::default(),
    });
}
