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

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use bevy::ecs::world::Command;
use bevy::prelude::*;
use save::SaveError;

use crate::layer;

mod save;

pub struct SessionPlugin;

impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Session>()
            .init_resource::<Session>()
            .add_systems(Startup, startup_system);
    }
}

// RESOURCES

#[derive(Debug, Default, Resource, Reflect)]
#[reflect(Resource)]
pub struct Session {
    loaded_from: Option<PathBuf>,
    // undo_stack: Vec<usize>,
    // unsaved_action_idx: usize,
}

impl Session {
    pub fn get_file_path(&self) -> Option<Cow<Path>> {
        self.loaded_from.as_ref().map(|p| Cow::from(p.as_path()))
    }

    pub fn has_save_file(&self) -> bool {
        self.loaded_from.is_some()
    }

    pub fn has_unsaved_changes(&self) -> bool {
        unimplemented!()
    }

    pub fn save(&self) -> Result<(), SessionError> {
        match &self.loaded_from {
            Some(path) => {
                info!("Saving to '{}'", path.to_str().unwrap());
                save::save(path.as_path(), vec![]).map_err(|e| SessionError::SaveError(e))
            }
            None => Err(SessionError::NoFilePath),
        }
    }

    pub fn set_file_path<T: Into<PathBuf>>(&mut self, path: T) {
        self.loaded_from = Some(path.into())
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
// FIXME: Either use this or remove.
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

fn startup_system(mut commands: Commands) {
    commands.add(InitializeNewSession);
}

// LIB

pub enum SessionError {
    NoFilePath,
    SaveError(SaveError),
}

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
