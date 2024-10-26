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
use thiserror::Error;

use crate::layer;
use crate::undo;

mod save;

pub struct SessionPlugin;

impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Session>()
            .init_resource::<Session>()
            .add_systems(Startup, startup_system)
            .add_systems(Update, process_undo_events_system);
    }
}

// RESOURCES

#[derive(Debug, Default, Resource, Reflect)]
#[reflect(Resource)]
pub struct Session {
    loaded_from: Option<PathBuf>,
    /// The index in undo stack for the last saved action relative to the last
    /// applied action.  `None` if the session has not been saved yet.
    saved_action_idx: Option<i32>,
}

impl Session {
    pub fn get_file_path(&self) -> Option<Cow<Path>> {
        self.loaded_from.as_ref().map(|p| Cow::from(p.as_path()))
    }

    pub fn has_save_file(&self) -> bool {
        self.loaded_from.is_some()
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.saved_action_idx != Some(0)
    }

    pub fn save(&self, commands: &mut Commands) -> Result<(), SessionError> {
        match &self.loaded_from {
            Some(_) => {
                commands.add(SaveSession);
                Ok(())
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
        // The new session is not saved yet.
        world.resource_mut::<Session>().loaded_from = None;
        clear_session(world);
        world.commands().add(layer::CreateLayer::OnTop);
    }
}

struct SaveSession;

impl Command for SaveSession {
    fn apply(self, world: &mut World) {
        let path: Option<PathBuf> = world.resource::<Session>().loaded_from.clone();

        match path {
            Some(path) => {
                info!("Saving to '{}'", path.to_str().unwrap());
                match save::save(path.as_path(), layer::LayerBundle::extract_all(world))
                    .map_err(|e| SessionError::SaveError(e))
                {
                    Ok(_) => {
                        world.resource_mut::<Session>().saved_action_idx = Some(0);
                    }
                    Err(e) => {
                        error!(error = &e as &dyn core::error::Error)
                    }
                }
            }
            None => error!(error = &SessionError::NoFilePath as &dyn core::error::Error),
        }
    }
}

// SYSTEMS

fn process_undo_events_system(
    mut session: ResMut<Session>,
    mut undo_events: EventReader<undo::UndoEvent>,
) {
    for event in undo_events.read() {
        match (event, session.saved_action_idx) {
            (undo::UndoEvent::ActionPushed, None) => (),
            (undo::UndoEvent::ActionPushed, Some(idx)) => session.saved_action_idx = Some(idx - 1),
            // TODO: Increment if Undo
            // TODO: Decrement if Redo
            (undo::UndoEvent::StackCleared, _) => {
                if session.has_save_file() {
                    // We have just loaded a save file.
                    session.saved_action_idx = Some(0)
                } else {
                    // We have created a new project.
                    session.saved_action_idx = None
                }
            }
        }
    }
}

fn startup_system(mut commands: Commands) {
    commands.add(InitializeNewSession);
}

// LIB

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("no file path")]
    NoFilePath,
    #[error("save error: {0}")]
    SaveError(save::SaveError),
}

fn clear_session(world: &mut World) {
    // Clear undo stack.
    undo::ClearStack.apply(world);

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
