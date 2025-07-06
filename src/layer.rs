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

use crate::undo;

mod actions;
mod components;

pub use actions::*;
pub use components::*;

// PLUGIN

pub struct LayerPlugin;

impl Plugin for LayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(components::LayerComponentsPlugin);
        app.add_systems(
            Update,
            normalize_layer_ordering_system.run_if(any_match_filter::<Changed<LayerOrder>>),
        );
    }

    fn finish(&self, app: &mut App) {
        if !app.is_plugin_added::<undo::UndoPlugin>() {
            app.add_plugins(undo::UndoPlugin);
        }
    }
}

// SYSTEMS

fn normalize_layer_ordering_system(mut layers: Query<&mut LayerOrder>) {
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
}

// LIB

/// This is intended to be called to create the initial layer only.  It does
/// not emit LayerChange::Added event.
pub fn create_initial_layer(world: &mut World) {
    let layer = Layer::default();
    world.spawn(LayerBundle {
        name: layer.name_component(),
        layer,
        layer_order: LayerOrder(0),
        height_map: HeightMap::default(),
    });
}
