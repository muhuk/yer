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
    /// This is used to track status of unsaved changes when
    /// `saved_action_idx` is set to `None`.  When a new project is created or
    /// a file is loaded this is set to `true`, as soon as an action is pushed
    /// to the undo stack this is set to `false`.
    new_project: bool,
    /// The index in undo stack for the last saved action relative to the last
    /// applied action.  `None` if we don't have an action to refer to as the
    /// last saved action.
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
        !self.new_project && self.saved_action_idx != Some(0)
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
        {
            let mut session = world.resource_mut::<Session>();
            session.loaded_from = None;
            session.saved_action_idx = None;
            session.new_project = true;
        }
        clear_session(world);
        layer::create_initial_layer(world);
    }
}

pub struct LoadSession(pub PathBuf);

impl Command for LoadSession {
    fn apply(self, world: &mut World) {
        let path = self.0;
        info!("Loading file '{}'", path.to_str().unwrap());
        clear_session(world);
        match save::load(path.as_path(), world).map_err(|e| SessionError::SaveError(e)) {
            Ok(_) => world.resource_mut::<Session>().set_file_path(path),
            Err(e) => error!(error = &e as &dyn core::error::Error),
        }
    }
}

pub struct SaveSession(pub Option<PathBuf>);

impl Command for SaveSession {
    fn apply(self, world: &mut World) {
        let path = self
            .0
            .or_else(|| world.resource::<Session>().loaded_from.clone());

        match path {
            Some(path) => {
                info!("Saving to '{}'", path.to_str().unwrap());
                match save::save(path.as_path(), world).map_err(|e| SessionError::SaveError(e)) {
                    Ok(_) => {
                        let mut session = world.resource_mut::<Session>();
                        session.saved_action_idx = Some(0);
                        session.set_file_path(path)
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
            (undo::UndoEvent::ActionPushed, None) => {
                session.new_project = false;
            }
            (undo::UndoEvent::ActionPushed, Some(idx)) => {
                session.saved_action_idx = Some(idx - 1);
                session.new_project = false;
            }
            // Decrement the index if redo
            (undo::UndoEvent::ActionReapplied, None) => (),
            (undo::UndoEvent::ActionReapplied, Some(idx)) => {
                session.saved_action_idx = Some(idx - 1);
            }
            // Increment the index if undo
            (undo::UndoEvent::ActionReverted, None) => (),
            (undo::UndoEvent::ActionReverted, Some(idx)) => {
                session.saved_action_idx = Some(idx + 1);
            }
            (undo::UndoEvent::StackCleared, _) => {
                if session.has_save_file() {
                    // We have just loaded a save file.
                    session.saved_action_idx = None;
                    session.new_project = true;
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
