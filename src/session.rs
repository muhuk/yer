// Copyright © 2024 Atamert Ölçgen.
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

use std::path::PathBuf;

use bevy::ecs::world::Command;
use bevy::prelude::*;

use crate::layer;

mod save;

pub struct SessionPlugin;

impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Session>()
            .init_resource::<Session>()
            .add_systems(Startup, initialize_empty_session);
    }
}

// RESOURCES

#[derive(Debug, Default, Resource, Reflect)]
#[reflect(Resource)]
struct Session {
    loaded_from: Option<PathBuf>,
    // undo_stack: Vec<usize>,
    // unsaved_action_idx: usize,
}

impl Session {
    fn has_unsaved_changes(&self) -> bool {
        unimplemented!()
    }
}

// COMMANDS

/// Clears existing session and creates a new, empty one.
///
/// Note that the app should ask to save an edited project first before
/// clearing everything.  But this is not implemented yet because Undo system
/// is not implemented and there is no way of knowing whether suggesting a
/// save is necessary or not.
pub struct InitializeNewSession;

impl Command for InitializeNewSession {
    fn apply(self, world: &mut World) {
        clear_session(world);
        world.commands().add(layer::CreateLayer::OnTop);
    }
}

/// Starts a new multi-step workflow that may eventually save the currently
/// edited project to disk, or not.
pub struct StartSaveSessionFlow;

impl Command for StartSaveSessionFlow {
    fn apply(self, world: &mut World) {
        let session_res = world.resource::<Session>();
        match session_res.loaded_from {
            Some(_) => unimplemented!(), // We're good, just save
            None => unimplemented!(),    // Enter flow, show field dialog
        }
    }
}

// SYSTEMS

fn initialize_empty_session(mut commands: Commands) {
    commands.add(InitializeNewSession);
}

// LIB

fn clear_session(world: &mut World) {
    // TODO: Clear Undo stack.

    // Despawn all layers.
    {
        let layers: Vec<Entity> = world
            .query_filtered::<Entity, With<layer::Layer>>()
            .iter(world)
            .collect();
        layers.iter().for_each(|entity| {
            world.entity_mut(*entity).despawn_recursive();
        });
    }
}
